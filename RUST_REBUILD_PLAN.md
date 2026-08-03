# Geany Rust Rebuild Plan

## Project Overview

**Geany** is a fast and lightweight IDE originally written in C using GTK+ 3 and Scintilla.  
**Goal**: Rebuilt in Rust for improved memory safety, concurrency, and modern tooling.

---

## Repository Analysis

### Original Architecture Components

| Component | Technology | LOC | Purpose |
|----------|------------|-----|---------|
| **UI Framework** | GTK+ 3 | - | Main window, dialogs, menus |
| **Text Editor** | Scintilla | ~50K | Syntax highlighting, code editing |
| **Plugin System** | C/GModule | - | Extensibility |
| **Build System** | Autotools/Meson | - | Compilation |
| **Internationalization** | gettext | - | i18n support |

### Key Source Modules (44 C files)

| Module | Files | Responsibility |
|--------|-------|----------------|
| Core | `main.c`, `libmain.c` | Application entry, initialization |
| Documents | `document.c`, `sciwrappers.c` | File operations, editor integration |
| Editor | `editor.c` | Scintilla interaction, editing features |
| UI | `callbacks.c`, `dialogs.c`, `sidebar.c`, `toolbar.c`, `notebook.c` | User interface components |
| Search | `search.c` | Find/replace, regex search |
| Project | `project.c`, `keyfile.c` | Project management, settings |
| Build | `build.c` | Compile/run commands |
| Plugins | `plugins.c`, `pluginutils.c` | Plugin loading/management |
| Utilities | `utils.c`, `log.c`, `spawn.c` | Common utilities |
| Symbols | `symbols.c`, `tagmanager/` | Symbol tree, ctags integration |
| Terminal | `vte.c` | Embedded terminal (VTE) |
| Highlighting | `highlighting.c` | Syntax coloring |

---

## Rust Technology Stack

### Recommended Crates

| Category | Crate | Purpose |
|----------|-------|---------|
| **GUI Framework** | `egui` + `eframe` | Pure Rust immediate mode GUI (chosen!) |
| **Text Editor** | `egui_code_editor` / custom | Syntax highlighting editor |
| **Async Runtime** | `tokio` | Async I/O, concurrency |
| **Plugin System** | `rustic-plugin` (new) | Plugin API design |
| **Serialization** | `serde` + `serde_json` | Config/project files |
| **Logging** | `tracing` | Structured logging |
| **CLI Args** | `clap` | Command-line argument parsing |
| **File Watching** | `notify` | File change monitoring |
| **Terminal** | `xtermjs` / custom | Terminal emulation |
| **Internationalization** | `fluent-rs` / `rust-i18n` | Modern i18n |

### Why egui?

- ✅ **Pure Rust** - No C dependencies, easier compilation
- ✅ **Immediate mode** - Simple mental model, easy to reason about
- ✅ **Cross-platform** - Works on Windows, macOS, Linux, and Web
- ✅ **Lightweight** - Minimal dependencies
- ✅ **Fast** - Immediate rendering, no retained widgets
- ✅ **Customizable** - Full control over UI rendering

---

## Phase 1: Project Setup (Week 1-2)

### 1.1 Initialize Rust Workspace

```bash
# Create workspace structure
mkdir geany-rs && cd geany-rs
cargo init --workspace

# Add workspace members
cargo new --lib geany-core        # Core application logic
cargo new --lib geany-ui          # egui UI components
cargo new --lib geany-plugin     # Plugin API
cargo new --bin geany             # Main binary
```

### 1.2 Cargo Workspace Configuration

```toml
# Cargo.toml
[workspace]
members = [
    "geany-core",
    "geany-ui", 
    "geany-plugin",
    "geany",
]
resolver = "2"

[workspace.dependencies]
eframe = "0.32"
egui = "0.32"
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
tracing = "0.1"
```

### 1.3 Core Dependencies

```toml
# geany-core/Cargo.toml
[package]
name = "geany-core"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { workspace = true }
tokio = { workspace = true }
tracing = { workspace = true }
thiserror = "2"
anyhow = "1"
parking_lot = "0.12"
once_cell = "1"
dirs = "6"
```

### 1.4 UI Dependencies (egui)

```toml
# geany-ui/Cargo.toml
[package]
name = "geany-ui"
version = "0.1.0"
edition = "2024"

[dependencies]
eframe = { workspace = true }
egui = { workspace = true }
parking_lot = "0.12"
dirs = "6"
```

---

## Phase 2: Core Architecture (Week 3-6)

### 2.1 Core Data Structures

```rust
// geany-core/src/document.rs
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: DocumentId,
    pub path: Option<PathBuf>,
    pub encoding: Encoding,
    pub filetype: FiletypeId,
    pub readonly: bool,
    pub encoding_changed: bool,
    pub last_save_writable: bool,
    pub disk_encoding: Encoding,
    pub valid: bool,
}

pub type DocumentId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Encoding {
    Utf8,
    Latin1,
    // ... other encodings
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FiletypeId {
    None,
    C,
    Cpp,
    Rust,
    Python,
    // ... 100+ filetypes
}
```

### 2.2 Application State

```rust
// geany-core/src/app.rs
use std::sync::Arc;
use parking_lot::RwLock;
use crate::document::Document;
use crate::project::Project;

pub struct App {
    pub debug_mode: bool,
    pub configdir: PathBuf,
    pub datadir: PathBuf,
    pub docdir: PathBuf,
    pub tm_workspace: TmWorkspace,
    pub project: Option<Arc<Project>>,
    pub documents: RwLock<DocumentList>,
}

pub struct DocumentList {
    pub items: Vec<Document>,
    pub active: Option<DocumentId>,
}
```

### 2.3 Module Structure

```
geany-core/src/
├── lib.rs
├── app.rs              # GeanyApp state
├── document.rs         # Document management
├── editor.rs           # Editor logic
├── project.rs          # Project handling
├── filetype.rs         # Filetype definitions
├── highlighting.rs     # Syntax highlighting
├── search.rs           # Search functionality
├── build.rs            # Build commands
├── spawn.rs            # Process spawning
├── keyfile.rs          # Config files (GLib KeyFile)
├── utils.rs            # Utility functions
├── log.rs              # Logging
├── symbols.rs          # Symbol tree
└── tm/
    ├── mod.rs
    ├── workspace.rs     # TagManager workspace
    ├── tag.rs           # Tag structures
    └── parser.rs        # ctags parser integration
```

---

## Phase 3: UI Layer (Week 7-12)

### 3.1 egui Integration

```rust
// geany-ui/src/main_window.rs
use egui::{CentralPanel, TopBottomPanel, SidePanel};
use eframe::egui;

pub struct GeanyApp {
    pub documents: Vec<Document>,
    pub active_doc: Option<usize>,
}

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top toolbar
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("New").clicked() {
                    self.new_document();
                }
                if ui.button("Open").clicked() {
                    // Open file dialog
                }
                if ui.button("Save").clicked() {
                    // Save current file
                }
            });
        });

        // Left sidebar (file tree, symbols)
        SidePanel::left("sidebar").show(ctx, |ui| {
            ui.label("Files");
            ui.separator();
            // File tree here
        });

        // Main editor area
        CentralPanel::default().show(ctx, |ui| {
            // Tab bar for open documents
            // Editor content
        });

        // Bottom message panel
        TopBottomPanel::bottom("messages").show(ctx, |ui| {
            ui.label("Messages");
        });
    }
}
```

### 3.2 UI Resource Structure

```
geany-ui/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── main_window.rs     # Main app with egui
│   ├── editor.rs          # Code editor widget
│   ├── sidebar.rs         # File tree, symbols panel
│   ├── toolbar.rs         # Top toolbar
│   ├── tabs.rs            # Document tabs
│   ├── messages.rs        # Bottom message panel
│   └── theme.rs           # Editor themes/colors
└── themes/                # Color schemes
```

### 3.3 Editor Integration

```rust
// geany-ui/src/editor.rs
use egui::{Color32, FontId, RichText, TextEdit};
use std::sync::Arc;

pub struct CodeEditor {
    pub content: String,
    pub cursor_pos: (usize, usize),  // line, col
    pub syntax_highlight: SyntaxHighlight,
}

impl CodeEditor {
    pub fn new() -> Self {
        Self {
            content: String::new(),
            cursor_pos: (0, 0),
            syntax_highlight: SyntaxHighlight::default(),
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) {
        TextEdit::multiline(&mut self.content)
            .font(FontId::monospace(14.0))
            .code_editor()
            .show(ui);
    }
}
```

---

## Phase 4: Plugin System (Week 13-16)

### 4.1 Plugin API Design

```rust
// geany-plugin/src/lib.rs
pub mod api;
pub mod loader;

pub use api::*;
pub use loader::*;

// geany-plugin/src/api.rs
use geany_core::document::Document;

pub trait Plugin {
    fn load(&mut self) -> bool;
    fn unload(&mut self);
    fn activate(&self);
    fn deactivate(&self);
    fn configure(&self) -> bool;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GeanyPluginInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    pub author: &'static str,
}

pub struct GeanyFunctions {
    pub document_new: fn() -> DocumentId,
    pub document_open: fn(path: &Path) -> DocumentId,
    pub document_save: fn(id: DocumentId),
    pub editor_insert_text: fn(id: DocumentId, pos: i32, text: &str),
    // ... 500+ API functions
}
```

### 4.2 Plugin Loading

```rust
// geany-plugin/src/loader.rs
use libloading::{Library, Symbol};
use std::path::Path;

pub struct PluginLoader {
    libraries: HashMap<PathBuf, Library>,
}

impl PluginLoader {
    pub fn load_plugin<P: AsRef<Path>>(&mut self, path: P) -> Result<Box<dyn Plugin>> {
        let lib = unsafe { Library::new(path.as_ref())? };
        let plugin = unsafe {
            let register: Symbol<unsafe extern "C" fn(*mut GeanyPlugin, *const GeanyFunctions) -> i32> =
                lib.get(b"geany_plugin_register")?;
            // Initialize plugin
        };
        Ok(plugin)
    }
}
```

---

## Phase 5: Feature Migration (Week 17-24)

### 5.1 Priority Order

| Priority | Feature | Complexity | Files |
|----------|---------|------------|-------|
| P0 | Document management | High | document.c |
| P0 | Scintilla integration | High | sciwrappers.c |
| P0 | Main window/UI | High | callbacks.c, ui_utils.c |
| P1 | Search/Replace | Medium | search.c |
| P1 | Project management | Medium | project.c, keyfile.c |
| P1 | Syntax highlighting | High | highlighting.c |
| P2 | Build system | Medium | build.c |
| P2 | Symbol tree | Medium | symbols.c, tagmanager/ |
| P2 | Plugin system | High | plugins.c |
| P3 | VTE terminal | Low | vte.c |
| P3 | i18n | Low | po/*.po |

### 5.2 Component Mapping

```
C Module                    → Rust Module
─────────────────────────────────────────
document.c                  → geany-core/document.rs
sciwrappers.c               → geany-ui/editor_widget.rs
editor.c                    → geany-core/editor.rs
callbacks.c                 → geany-ui/callbacks.rs
dialogs.c                   → geany-ui/dialogs.rs
sidebar.c                   → geany-ui/sidebar.rs
notebook.c                  → geany-ui/notebook.rs
toolbar.c                   → geany-ui/toolbar.rs
search.c                    → geany-core/search.rs
project.c                   → geany-core/project.rs
keyfile.c                   → geany-core/keyfile.rs
build.c                     → geany-core/build.rs
plugins.c                   → geany-plugin/loader.rs
utils.c                     → geany-core/utils.rs
spawn.c                     → geany-core/spawn.rs
symbols.c                   → geany-core/symbols.rs
tagmanager/*                → geany-core/tm/*.rs
vte.c                       → geany-ui/terminal.rs
highlighting.c              → geany-core/highlighting.rs
```

---

## Phase 6: Testing Strategy (Week 25-28)

### 6.1 Test Coverage

```rust
// geany-core/tests/document_test.rs
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_document_new() {
        let doc = Document::new();
        assert!(doc.path.is_none());
        assert_eq!(doc.encoding, Encoding::Utf8);
    }
    
    #[test]
    fn test_document_open() {
        let path = PathBuf::from("test.c");
        let doc = Document::open(&path).unwrap();
        assert_eq!(doc.filetype, FiletypeId::C);
    }
}
```

### 6.2 Integration Tests

```rust
// tests/integration/main_window_test.rs
use gtk::test::TestApp;

#[gtk::test]
async fn test_open_file() {
    let app = TestApp::new();
    app.open_file("test.c");
    app.await_render();
    
    assert!(app.active_document().is_some());
}
```

---

## Phase 7: Build & Release (Week 29-32)

### 7.1 Cross-Platform Build

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
linker = "gcc"

[target.x86_64-pc-windows-msvc]
linker = "msvc"

[target.aarch64-apple-darwin]
linker = "clang"
```

### 7.2 Release Configuration

```toml
# geany/Cargo.toml
[package]
name = "geany"
version = "2.0.0"
edition = "2024"

[[bin]]
name = "geany"
path = "src/main.rs"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true
```

---

## Implementation Notes

### Challenges & Solutions

| Challenge | Solution |
|-----------|----------|
| egui text editor | Use `egui_code_editor` crate or build custom |
| Syntax highlighting | Use `syntect` with egui's rich text |
| Terminal emulation | Use `xterm-rs` or embedded webview |
| Plugin compatibility | New pure-Rust plugin system (not C ABI) |
| Large codebase | Incremental migration, maintain C version |
| Performance | Use `parking_lot` for locking, async for I/O |

### Dependencies to Evaluate

```toml
# egui ecosystem
egui_code_editor = "0.3"   # Code editor with syntax highlighting
syntect = "5"               # Syntax highlighting engine
xterm = "0.1"               # Terminal widget
```

---

## Timeline Summary

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Phase 1: Setup | 2 weeks | Rust workspace, basic structure |
| Phase 2: Core | 4 weeks | Core data structures, state management |
| Phase 3: UI | 6 weeks | Main window, document tabs, editor |
| Phase 4: Plugin | 4 weeks | Plugin API, loader |
| Phase 5: Features | 8 weeks | All features migrated |
| Phase 6: Testing | 4 weeks | Unit, integration tests |
| Phase 7: Release | 4 weeks | Cross-platform builds |
| **Total** | **~32 weeks** | **Production-ready Rust IDE** |

---

## Repository Structure (Final)

```
geany-rs/
├── Cargo.toml              # Workspace
├── README.md
├── LICENSE                 # GPL-2+
├── geany/                  # Main binary
│   ├── Cargo.toml
│   └── src/main.rs
├── geany-core/             # Core library
│   ├── Cargo.toml
│   ├── build.rs
│   ├── src/
│   └── tests/
├── geany-ui/               # UI components
│   ├── Cargo.toml
│   ├── build.rs
│   ├── resources/          # UI files, themes
│   ├── data/               # Filedefs, templates
│   └── src/
├── geany-plugin/           # Plugin system
│   ├── Cargo.toml
│   └── src/
├── tests/                  # Integration tests
├── scripts/                # Build/utility scripts
└── doc/                    # Documentation
```

---

## Next Steps

1. **✅ Decision Made**: Using egui for pure Rust GUI
2. **Proof of Concept**: Create minimal editor with egui (see geany-rs/ folder)
3. **Team Planning**: Assign modules to team members
4. **CI/CD Setup**: Configure GitHub Actions for multi-platform builds
5. **Documentation**: Migrate developer docs, API reference
