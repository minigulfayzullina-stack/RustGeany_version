# Persistence Design — `settings.json`

Status: **design only** (no working code yet — depends on Phase 0/1).
Location: `dirs::config_dir()/geany-rs/settings.json` (Linux: `~/.config/geany-rs/settings.json`).

Goals:
- Restore the UI exactly as the user left it (theme, window, panels).
- Restore the editing session (open docs, active doc, per-doc cursor/folds).
- Keep everything human-readable JSON; fail soft on any parse error.

## Schema (v1)

```json
{
  "version": 1,
  "ui": {
    "theme": "dark",                // one of: dark|light|monokai|dracula|nord|gruvbox
    "window": { "width": 1400, "height": 900, "x": null, "y": null, "maximized": false },
    "sidebar": { "visible": true, "width": 230.0, "tab": "files" },
    "messages": { "visible": true, "height": 80.0 },
    "terminal": { "visible": false, "height": 150.0 },
    "line_numbers": true,
    "word_wrap": false,
    "font_size": 14.0
  },
  "editor": {
    "auto_save": false,
    "auto_save_interval_secs": 60,
    "tab_width": 4,
    "insert_spaces": true
  },
  "session": {
    "open_documents": [
      {
        "path": "/abs/path/file.rs",   // null for untitled
        "name": "file.rs",
        "filetype": "rust",            // explicit override; else derived from path
        "cursor_line": 1,
        "cursor_col": 1,
        "folds_collapsed": [12, 48],   // start lines of collapsed fold regions
        "content_unsaved": null        // full text for untitled / unsaved docs
      }
    ],
    "active_document": 0               // index into open_documents
  },
  "project": {
    "path": "/abs/path/project.geany", // last open project file, null if none
    "recent_projects": ["/abs/a.geany", "/abs/b.geany"]
  },
  "recent_files": ["/abs/x.rs", "/abs/y.py"],  // most-recent-first, cap 20
  "plugins": {
    "enabled": ["todo_viewer"],        // persisted per-plugin toggle state
    "settings": {}                     // free-form per-plugin key/values
  }
}
```

## Rules

1. **Versioning.** `version` is mandatory. On load: unknown version → attempt best-effort parse with defaults for missing keys; never crash. Bump the version only on breaking schema changes; add a `migrate(v) -> v+1` function per step.
2. **Fail soft.** Corrupt/partial JSON → log a message to the message panel and fall back to defaults. Never refuse to start because of settings.
3. **Untitled/unsaved docs.** Documents without a path persist their full text under `content_unsaved` so a crash never loses work. On load, restore them as modified.
4. **Atomic writes.** Write to `settings.json.tmp`, then rename — no half-written files on crash. Save on quit and on a debounced timer (e.g. 5s after the last settings-affecting change), not every frame.
5. **Paths.** Store absolute paths. On load, skip `open_documents` entries whose `path` no longer exists (with a notice), but keep untitled ones.
6. **Privacy.** Nothing leaves the machine; this file is local-only. Recent-file list is user-visible state — no surprises.

## Rust shape (maps to `serde` structs)

```rust
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Settings {
    pub version: u32,          // default 1
    pub ui: UiSettings,
    pub editor: EditorSettings,
    pub session: SessionState,
    pub project: ProjectState,
    pub recent_files: Vec<PathBuf>,
    pub plugins: PluginSettings,
}
```

`#[serde(default)]` on every struct/field so old files load fine with new fields missing.

## What is NOT persisted (deliberately)

- Undo/redo history, macro recordings (session-ephemeral).
- Terminal history beyond the current run (optional later opt-in).
- Window position on platforms that misbehave with it (restore size first, position as best-effort).
