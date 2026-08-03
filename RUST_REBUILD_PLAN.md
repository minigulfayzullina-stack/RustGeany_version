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
| **GUI Framework** | `gtk4-rs` | Rust bindings for GTK4 (successor to GTK3) |
| **Text Editor** | `scintilla.rs` | Scintilla bindings for Rust |
| **Async Runtime** | `tokio` | Async I/O, concurrency |
| **Plugin System** | `rustic-plugin` (new) | Plugin API design |
| **Serialization** | `serde` + `serde_json` | Config/project files |
| **Logging** | `tracing` | Structured logging |
| **CLI Args** | `clap` | Command-line argument parsing |
| **File Watching** | `notify` | File change monitoring |
| **Terminal** | `vte` | VTE widget bindings |
| **Internationalization** | `fluent-rs` / `rust-i18n` | Modern i18n |

---

## Phase 1: Project Setup (Week 1-2)

### 1.1 Initialize Rust Workspace

```bash
# Create workspace structure
mkdir geany-rs && cd geany-rs
cargo init --workspace

# Add workspace members
cargo new --lib geany-core        # Core application logic
cargo new --lib geany-ui          # GTK UI components
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
gtk = "0.9"
scintilla = "5.5"
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

[build-dependencies]
pkg-config = "0.3"
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

### 3.1 GTK4 Integration

```rust
// geany-ui/src/main_window.rs
use gtk::prelude::*;
use gtk::{Application, ApplicationWindow, Builder};
use std::sync::Arc;

pub struct MainWindow {
    window: ApplicationWindow,
    notebook: gtk::Notebook,
    sidebar: Sidebar,
    toolbar: Toolbar,
    msgwin: MessageWindow,
}

impl MainWindow {
    pub fn new(app: &Application) -> Self {
        let builder = Builder::from_resource("/org/geany/ui/main.ui");
        
        let window: ApplicationWindow = builder.object("main_window")
            .expect("Failed to get main_window");
            
        let notebook: gtk::Notebook = builder.object("notebook")
            .expect("Failed to get notebook");
            
        // ... setup other components
        
        Self { window, notebook, sidebar, toolbar, msgwin }
    }
}
```

### 3.2 UI Resource Structure

```
geany-ui/resources/
├── Cargo.toml
├── build.rs
├── data/
│   ├── main.ui           # GTK4 UI definition
│   ├── geany.css         # Styles
│   ├── colorschemes/     # Editor themes
│   └── filedefs/         # Filetype configs
└── src/
    ├── lib.rs
    ├── main_window.rs
    ├── sidebar.rs
    ├── toolbar.rs
    ├── notebook.rs
    ├── dialogs.rs
    ├── callbacks.rs
    └── completion.rs
```

### 3.3 Scintilla Integration

```rust
// geany-ui/src/editor_widget.rs
use gtk::prelude::*;
use scintilla::Scintilla;

pub struct EditorView {
    scilla: Scintilla,
    doc_id: DocumentId,
}

impl EditorView {
    pub fn new() -> Self {
        let scilla = Scintilla::new();
        scilla.set_id("editor".to_string());
        
        Self { scilla, doc_id: 0 }
    }
    
    pub fn set_text(&self, text: &str) {
        self.scilla.set_text(text);
    }
    
    pub fn get_text(&self) -> String {
        self.scilla.text()
    }
    
    pub fn connect_modified<F>(&self, callback: F)
    where
        F: Fn(&Scintilla, i32, i32, i32) + 'static,
    {
        self.scilla.connect_modify(callback);
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
| GTK3 → GTK4 | Use GTK4 with compatible widgets; some GTK3 APIs differ |
| Scintilla C → Rust | Use existing `scintilla` crate or create bindings |
| Plugin ABI compatibility | Create FFI layer for existing C plugins |
| Large codebase | Incremental migration, maintain C/Gtk2 version |
| Performance | Use `parking_lot` for locking, async for I/O |

### Dependencies to Evaluate

```toml
# Consider these alternatives
vte = "0.14"          # Terminal widget
tree-sitter = "0.24"  # Alternative to ctags for symbol parsing
lsp-types = "0.96"    # Language Server Protocol support
syntect = "5"          # Syntax highlighting (alternative to Scintilla)
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

1. **Decision Point**: GTK3 vs GTK4 vs pure custom renderer
2. **Proof of Concept**: Create minimal editor with Scintilla in Rust
3. **Team Planning**: Assign modules to team members
4. **CI/CD Setup**: Configure GitHub Actions for multi-platform builds
5. **Documentation**: Migrate developer docs, API reference
