# Plugin API Design

Status: **design decision** (no working code yet). Replaces the current fake "plugins" (four inert checkboxes).

## Decision: in-process trait-object plugins, dynamic loading deferred

The plan in `RUST_REBUILD_PLAN.md` mentions `libloading`-based dynamic plugins with a C-style ABI. **Do not do that yet.** Reasons:

1. Rust has no stable ABI — dynamic plugins must be built with the exact same compiler + dependency versions, or they need a `repr(C)` + `abi_stable`-style shim. High cost, low benefit for v1.
2. Geany-Rs is a single binary; "plugins" are features toggled at runtime.
3. Immediate-mode egui makes a sandboxed JS/WASM plugin engine a big project of its own.

**Chosen design:** a `Plugin` trait; plugins are Rust structs registered in a `PluginRegistry` at startup. They ship *compiled into* the binary today; the registry is deliberately built so a dynamic loader can be added later without changing plugin code.

```rust
pub struct PluginInfo {
    pub id: &'static str,          // "todo_viewer" — stable, used in settings
    pub name: &'static str,        // "Todo Viewer" — display name
    pub description: &'static str,
    pub version: &'static str,
    pub author: &'static str,
}

pub struct PluginContext<'a> {
    pub documents: &'a [Document],
    pub active: Option<usize>,
    pub filetypes: &'a [Filetype],
    // later: commands, events, UI region handles
}

/// Implementors must be Send + 'static; state lives inside the plugin struct.
pub trait Plugin: Send + 'static {
    fn info(&self) -> PluginInfo;

    /// Called once when the plugin is enabled. Receives mutable access to
    /// the app-facing API (logging, status bar).
    fn activate(&mut self, api: &mut PluginApi) -> Result<(), PluginError>;

    /// Called on disable; must release resources.
    fn deactivate(&mut self);

    /// Called on every document change (debounced by the host).
    fn on_document_changed(&mut self, _ctx: &PluginContext) {}

    /// Called periodically (e.g. every 60s) for timers like autosave.
    fn on_tick(&mut self, _ctx: &PluginContext, _api: &mut PluginApi) {}

    /// Optional: contribute items to the sidebar (TODO list etc.).
    fn sidebar_panel(&mut self, _ui: &mut egui::Ui, _ctx: &PluginContext) {}
}
```

`PluginApi` is the *only* way plugins touch the host: log messages, add status-bar items, request a save, schedule an action. It never hands out `&mut GeanyApp` directly — that keeps the boundary explicit and is the seam a future dynamic/WASM loader would sit behind.

## Lifecycle & toggling

- `PluginRegistry` holds `Vec<Box<dyn Plugin>>` plus an `enabled: HashSet<PluginId>`.
- Toggle in the Plugins panel calls `activate`/`deactivate`; the enabled set persists via `plugins.enabled` in `settings.json` (see `docs/persistence.md`).
- Activation order = registration order; failures are reported to the message panel and the plugin stays disabled.

## First real plugins (replaces the four fakes)

| id | behavior |
|---|---|
| `auto_save` | `on_tick`: saves modified documents that have a path, every N secs (configurable). |
| `todo_viewer` | `on_document_changed` + `sidebar_panel`: scans for `TODO`/`FIXME`/`HACK` and lists them with line links. |
| `bracket_guide` | `on_document_changed`: feeds matching-bracket info to the editor highlighter. |
| `format_on_save` | hook on save event: runs the filetype's formatter (`rustfmt`, `black`, …) via the build system. |

## Migration path to loadable plugins (later, Phase 3+)

1. Stabilize `Plugin` trait (this doc) and `PluginApi`.
2. Extract both into a small `geany-plugin-api` crate with **no egui dependency** for the data surface (pass plain data, not `egui::Ui`, through the dynamic boundary; UI stays host-side).
3. Only then evaluate `abi_stable`/WASM. Until then: built-in registry only.
