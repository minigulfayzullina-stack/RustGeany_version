# Geany-Rs

A fast and lightweight IDE built with Rust and egui.

## Features

- ✅ Pure Rust implementation (no C dependencies)
- ✅ Cross-platform (Windows, macOS, Linux)
- ✅ Syntax highlighting for Rust, C, C++, Python, JavaScript, and more
- ✅ Tab-based document management
- ✅ Dark/Light theme support
- ✅ Sidebar with file list
- ✅ Message/Output panel

## Building

```bash
cd geany-rs
cargo build --release
```

## Running

```bash
cargo run --release
```

## Keyboard Shortcuts

- `Ctrl+N` - New document
- `Ctrl+O` - Open file
- `Ctrl+S` - Save file

## Screenshots

The IDE features:
- Top toolbar with file operations
- Left sidebar showing open documents
- Main editor area with syntax highlighting
- Bottom message panel

## Architecture

```
geany-rs/
├── Cargo.toml
├── src/
│   └── main.rs          # Main application with egui UI
└── README.md
```

## Technology Stack

- **egui** - Immediate mode GUI framework
- **eframe** - eframe native runtime
- **serde** - Serialization for config/project files
- **parking_lot** - Efficient locking

## Future Plans

- [ ] Real file system integration
- [ ] Project management
- [ ] Find and replace
- [ ] Symbol tree/sidebar
- [ ] Terminal panel
- [ ] Plugin system
- [ ] More language support

## License

GPL-2+
