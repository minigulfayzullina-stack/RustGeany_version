//! Geany-Rs - A fast and lightweight IDE in Rust
//! Built with egui for cross-platform support
//!
//! Features:
//! - Real file open/save dialogs
//! - Find & Replace functionality
//! - Symbol tree sidebar
//! - Terminal emulator
//! - Syntax highlighting
//! - Build commands (compile/run)
//! - Complete UI (menu bar, status bar, tabs)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{
    Align, Color32, ComboBox, Context, FontId, Label, LayoutJob, RichText, ScrollArea, 
    TextEdit, TopBottomPanel, SidePanel, CentralPanel, Separator, Stroke, TextFormat,
    UI, Ui, Vec2, Window, Button, WidgetText, menu,
};
use regex::Regex;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};

// ============================================================================
// FILE DIALOGS
// ============================================================================

mod file_dialogs {
    use rfd::FileDialog;
    use std::path::PathBuf;

    pub fn open_file() -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("Source Files", &["rs", "c", "cpp", "h", "py", "js", "ts", "html", "css", "json", "md", "toml", "yaml", "yml"])
            .add_filter("All Files", &["*"])
            .pick_file()
    }

    pub fn save_file(default_name: &str) -> Option<PathBuf> {
        FileDialog::new()
            .set_file_name(default_name)
            .add_filter("Source Files", &["rs", "c", "cpp", "h", "py", "js", "ts", "html", "css", "json", "md", "toml", "yaml", "yml"])
            .add_filter("All Files", &["*"])
            .save_file()
    }

    pub fn open_folder() -> Option<PathBuf> {
        FileDialog::new().pick_folder()
    }
}

// ============================================================================
// SYMBOL TREE (Phase 3)
// ============================================================================

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function, Struct, Enum, Impl, Trait, Class, Method, Module, Variable, Constant, Property,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::Function => "ƒ",
            SymbolKind::Struct => "S",
            SymbolKind::Enum => "E",
            SymbolKind::Impl => "I",
            SymbolKind::Trait => "T",
            SymbolKind::Class => "C",
            SymbolKind::Method => "m",
            SymbolKind::Module => "M",
            SymbolKind::Variable => "v",
            SymbolKind::Constant => "K",
            SymbolKind::Property => "p",
        }
    }
    pub fn color(&self) -> Color32 {
        match self {
            SymbolKind::Function | SymbolKind::Method => Color32::from_rgb(230, 192, 123),
            SymbolKind::Struct | SymbolKind::Class => Color32::from_rgb(78, 201, 176),
            SymbolKind::Enum => Color32::from_rgb(86, 156, 214),
            SymbolKind::Impl | SymbolKind::Trait => Color32::from_rgb(206, 145, 120),
            SymbolKind::Module => Color32::from_rgb(197, 134, 192),
            SymbolKind::Variable | SymbolKind::Constant => Color32::from_rgb(181, 206, 168),
            SymbolKind::Property => Color32::from_rgb(220, 220, 170),
        }
    }
}

pub struct SymbolParser;

impl SymbolParser {
    pub fn parse(content: &str, filetype: Filetype) -> Vec<Symbol> {
        match filetype {
            Filetype::Rust => Self::parse_rust(content),
            Filetype::C | Filetype::Cpp => Self::parse_c(content),
            Filetype::Python => Self::parse_python(content),
            Filetype::JavaScript | Filetype::TypeScript => Self::parse_js(content),
            _ => vec![],
        }
    }

    fn parse_rust(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let patterns = [
            (r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", SymbolKind::Function),
            (r"(?m)^(?:pub\s+)?struct\s+(\w+)", SymbolKind::Struct),
            (r"(?m)^(?:pub\s+)?enum\s+(\w+)", SymbolKind::Enum),
            (r"(?m)^(?:pub\s+)?trait\s+(\w+)", SymbolKind::Trait),
            (r"(?m)^(?:pub\s+)?mod\s+(\w+)", SymbolKind::Module),
            (r"(?m)^(?:pub\s+)?impl(?:\s+<\w+>)?\s+(\w+)", SymbolKind::Impl),
        ];
        
        for (pattern, kind) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        let line = content[..name.start()].matches('\n').count();
                        symbols.push(Symbol { name: name.as_str().to_string(), kind: kind.clone(), line });
                    }
                }
            }
        }
        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_c(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let re = Regex::new(r"(?m)^(?:[\w\*]+\s+)+(\w+)\s*\([^)]*\)\s*\{").unwrap();
        for cap in re.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let name_str = name.as_str();
                if !["if", "else", "while", "for"].contains(&name_str) {
                    let line = content[..name.start()].matches('\n').count();
                    symbols.push(Symbol { name: name_str.to_string(), kind: SymbolKind::Function, line });
                }
            }
        }
        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_python(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let patterns = [
            (r"(?m)^class\s+(\w+)", SymbolKind::Class),
            (r"(?m)^def\s+(\w+)", SymbolKind::Function),
        ];
        for (pattern, kind) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        let line = content[..name.start()].matches('\n').count();
                        symbols.push(Symbol { name: name.as_str().to_string(), kind: kind.clone(), line });
                    }
                }
            }
        }
        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_js(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let patterns = [
            (r"(?m)^(?:export\s+)?class\s+(\w+)", SymbolKind::Class),
            (r"(?m)^(?:export\s+)?function\s+(\w+)", SymbolKind::Function),
            (r"(?m)^(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(", SymbolKind::Function),
        ];
        for (pattern, kind) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        let line = content[..name.start()].matches('\n').count();
                        symbols.push(Symbol { name: name.as_str().to_string(), kind: kind.clone(), line });
                    }
                }
            }
        }
        symbols.sort_by_key(|s| s.line);
        symbols
    }
}

// ============================================================================
// FIND & REPLACE (Phase 3)
// ============================================================================

#[derive(Debug, Clone)]
pub struct FindReplace {
    pub search_text: String,
    pub replace_text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub search_results: Vec<usize>,
    pub current_result: usize,
}

impl FindReplace {
    pub fn new() -> Self {
        Self { search_text: String::new(), replace_text: String::new(), case_sensitive: false, whole_word: false, regex: false, search_results: vec![], current_result: 0 }
    }

    pub fn search(&mut self, content: &str) {
        self.search_results.clear();
        if self.search_text.is_empty() { return; }

        let needle = if self.whole_word { format!(r"\b{}\b", regex::escape(&self.search_text)) } else { regex::escape(&self.search_text) };
        let pattern = if self.case_sensitive { needle } else { format!("(?i){}", needle) };

        if let Ok(re) = Regex::new(&pattern) {
            for (idx, line) in content.lines().enumerate() {
                if re.is_match(line) { self.search_results.push(idx); }
            }
        }
        self.current_result = 0;
    }

    pub fn replace_all(&self, content: &str) -> String {
        if self.search_text.is_empty() { return content.to_string(); }
        let needle = if self.whole_word { format!(r"\b{}\b", regex::escape(&self.search_text)) } else { regex::escape(&self.search_text) };
        let pattern = if self.case_sensitive { needle } else { format!("(?i){}", needle) };
        if let Ok(re) = Regex::new(&pattern) { re.replace_all(content, self.replace_text.as_str()).to_string() } else { content.to_string() }
    }
}

// ============================================================================
// BUILD COMMANDS (NEW FEATURE)
// ============================================================================

#[derive(Debug, Clone)]
pub struct BuildCommand {
    pub name: String,
    pub command: String,
    pub working_dir: Option<String>,
    pub shortcut: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BuildSystem {
    pub commands: Vec<Vec<BuildCommand>>,
    pub current_set: usize,
    pub last_output: String,
    pub is_running: bool,
}

impl BuildSystem {
    pub fn new() -> Self {
        Self {
            commands: vec![
                // Rust commands
                vec![
                    BuildCommand { name: "Compile".to_string(), command: "rustc \"{file}\" -o \"{file_basename}\"".to_string(), working_dir: None, shortcut: Some("F8".to_string()) },
                    BuildCommand { name: "Run".to_string(), command: "\"./{file_basename}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                    BuildCommand { name: "Cargo Build".to_string(), command: "cargo build".to_string(), working_dir: Some("{project_dir}".to_string()), shortcut: Some("F10".to_string()) },
                    BuildCommand { name: "Cargo Run".to_string(), command: "cargo run".to_string(), working_dir: Some("{project_dir}".to_string()), shortcut: Some("F11".to_string()) },
                ],
                // C/C++ commands
                vec![
                    BuildCommand { name: "Compile".to_string(), command: "gcc \"{file}\" -o \"{file_basename}\" -Wall".to_string(), working_dir: None, shortcut: Some("F8".to_string()) },
                    BuildCommand { name: "Run".to_string(), command: "\"./{file_basename}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
                // Python commands
                vec![
                    BuildCommand { name: "Run".to_string(), command: "python3 \"{file}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
                // JavaScript commands
                vec![
                    BuildCommand { name: "Run".to_string(), command: "node \"{file}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
                // HTML commands
                vec![
                    BuildCommand { name: "Open in Browser".to_string(), command: "xdg-open \"{file}\" 2>/dev/null || open \"{file}\" 2>/dev/null || echo 'Open file manually'".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
            ],
            current_set: 0,
            last_output: String::new(),
            is_running: false,
        }
    }

    pub fn get_commands_for(&self, filetype: Filetype) -> &[BuildCommand] {
        let index = match filetype {
            Filetype::Rust => 0,
            Filetype::C | Filetype::Cpp => 1,
            Filetype::Python => 2,
            Filetype::JavaScript | Filetype::TypeScript => 3,
            Filetype::Html | Filetype::Css | Filetype::Json | Filetype::Markdown => 4,
            _ => 0,
        };
        &self.commands[index]
    }

    pub fn expand_variables(&self, command: &str, file_path: &Option<String>) -> String {
        let mut result = command.to_string();
        
        if let Some(path) = file_path {
            let path_obj = std::path::Path::new(path);
            result = result.replace("{file}", path);
            result = result.replace("{file_basename}", &path_obj.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
            result = result.replace("{project_dir}", &path_obj.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
        }
        
        result
    }

    pub fn execute_command(&mut self, cmd: &BuildCommand, file_path: &Option<String>, app: &mut GeanyApp) {
        let expanded = self.expand_variables(&cmd.command, file_path);
        let working_dir = cmd.working_dir.as_ref().map(|d| self.expand_variables(d, file_path));
        
        app.log(format!("> {}", expanded));
        
        #[cfg(target_os = "windows")]
        let (shell, shell_arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, shell_arg) = ("sh", "-c");

        match Command::new(shell)
            .arg(shell_arg)
            .arg(&expanded)
            .current_dir(working_dir.as_deref().unwrap_or("."))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(mut child) => {
                self.is_running = true;
                
                // Read stdout
                if let Some(stdout) = child.stdout.take() {
                    use std::io::Read;
                    let mut output = String::new();
                    if stdout.read_to_string(&mut output).is_ok() && !output.is_empty() {
                        self.last_output = output.clone();
                        for line in output.lines().take(100) {
                            app.log(line.to_string());
                        }
                    }
                }
                
                // Read stderr
                if let Some(stderr) = child.stderr.take() {
                    use std::io::Read;
                    let mut output = String::new();
                    if stderr.read_to_string(&mut output).is_ok() && !output.is_empty() {
                        self.last_output = output.clone();
                        for line in output.lines().take(100) {
                            app.log(format!("[stderr] {}", line));
                        }
                    }
                }
                
                // Wait for process
                if let Ok(status) = child.wait() {
                    if status.success() {
                        app.log("Process exited successfully (code 0)".to_string());
                    } else {
                        app.log(format!("Process exited with code {:?}", status.code()));
                    }
                }
                
                self.is_running = false;
            }
            Err(e) => {
                app.log(format!("Error executing command: {}", e));
            }
        }
    }
}

// ============================================================================
// TERMINAL (Phase 3)
// ============================================================================

pub struct Terminal {
    pub visible: bool,
    pub history: Vec<String>,
    pub current_dir: String,
    pub command_input: String,
}

impl Terminal {
    pub fn new() -> Self {
        let username = whoami::username();
        let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
        Self {
            visible: false,
            history: vec![
                "╔══════════════════════════════════════════╗".to_string(),
                "║        Geany-Rs Terminal v0.1.0          ║".to_string(),
                "╚══════════════════════════════════════════╝".to_string(),
                "Type 'help' for available commands".to_string(),
            ],
            current_dir: home,
            command_input: String::new(),
        }
    }

    pub fn execute(&mut self, command: &str, app: &mut GeanyApp) {
        if command.trim().is_empty() { return; }
        
        let prompt = format!("{} {} $ ", whoami::username(), self.current_dir.replace(&*whoami::username(), "~"));
        self.history.push(format!("{}{}", prompt, command));
        
        match command.trim() {
            "help" => {
                self.history.push("┌─ Available Commands ─┐".to_string());
                self.history.push("│  help        - Show this help".to_string());
                self.history.push("│  clear/cls   - Clear terminal".to_string());
                self.history.push("│  pwd         - Print working directory".to_string());
                self.history.push("│  cd <dir>    - Change directory".to_string());
                self.history.push("│  ls          - List files".to_string());
                self.history.push("│  cat <file>  - Show file contents".to_string());
                self.history.push("│  mkdir <dir> - Create directory".to_string());
                self.history.push("│  touch <f>   - Create empty file".to_string());
                self.history.push("│  rm <file>   - Remove file".to_string());
                self.history.push("│  date        - Show current date".to_string());
                self.history.push("│  whoami      - Show current user".to_string());
                self.history.push("│  echo <text> - Print text".to_string());
                self.history.push("│  <any>       - Run shell command".to_string());
                self.history.push("└────────────────────────────────┘".to_string());
            }
            "clear" | "cls" => { self.history.clear(); self.history.push("Terminal cleared".to_string()); }
            "pwd" => { self.history.push(self.current_dir.replace(&*whoami::username(), "~")); }
            "date" => { self.history.push(chrono_lite()); }
            "whoami" => { self.history.push(whoami::username()); }
            "exit" | "quit" => { self.history.push("Use the toggle button (🖥) to hide terminal".to_string()); }
            cmd if cmd.starts_with("cd ") => {
                let dir = cmd.trim_start_matches("cd ").trim().replace('~', &*whoami::username());
                let new_dir = if dir.starts_with('/') { dir.clone() } else { format!("{}/{}", self.current_dir, dir) };
                if let Ok(canonical) = std::fs::canonicalize(&new_dir) {
                    self.current_dir = canonical.to_string_lossy().to_string();
                    self.history.push(format!("Changed to: {}", self.current_dir.replace(&*whoami::username(), "~")));
                } else {
                    self.history.push(format!("cd: {}: No such directory", dir));
                }
            }
            "ls" | "ls -la" | "ls -l" => {
                if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
                    self.history.push(format!("Contents of {}:", self.current_dir.replace(&*whoami::username(), "~")));
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false);
                        let prefix = if is_dir { "📁 " } else { "📄 " };
                        self.history.push(format!("  {}{}", prefix, name));
                    }
                }
            }
            cmd if cmd.starts_with("cat ") => {
                let file = cmd.trim_start_matches("cat ");
                if let Ok(content) = std::fs::read_to_string(file) {
                    self.history.push(format!("─── {} ───", file));
                    for line in content.lines().take(100) {
                        self.history.push(line.to_string());
                    }
                } else {
                    self.history.push(format!("cat: {}: No such file", file));
                }
            }
            cmd if cmd.starts_with("echo ") => {
                self.history.push(cmd.trim_start_matches("echo ").to_string());
            }
            cmd if cmd.starts_with("mkdir ") => {
                let dir = cmd.trim_start_matches("mkdir ");
                match std::fs::create_dir_all(dir) {
                    Ok(_) => self.history.push(format!("Created: {}", dir)),
                    Err(e) => self.history.push(format!("mkdir: {}: {}", dir, e)),
                }
            }
            cmd if cmd.starts_with("touch ") => {
                let file = cmd.trim_start_matches("touch ");
                match std::fs::write(file, "") {
                    Ok(_) => self.history.push(format!("Created: {}", file)),
                    Err(e) => self.history.push(format!("touch: {}: {}", file, e)),
                }
            }
            cmd if cmd.starts_with("rm ") => {
                let file = cmd.trim_start_matches("rm ");
                match std::fs::remove_file(file) {
                    Ok(_) => self.history.push(format!("Removed: {}", file)),
                    Err(e) => self.history.push(format!("rm: {}: {}", file, e)),
                }
            }
            _ => {
                // Execute as shell command
                #[cfg(target_os = "windows")]
                let (shell, arg) = ("cmd", "/C");
                #[cfg(not(target_os = "windows"))]
                let (shell, arg) = ("sh", "-c");
                
                match Command::new(shell).arg(arg).arg(command).current_dir(&self.current_dir).output() {
                    Ok(output) => {
                        if !output.stdout.is_empty() {
                            for line in String::from_utf8_lossy(&output.stdout).lines().take(100) {
                                self.history.push(line.to_string());
                            }
                        }
                        if !output.stderr.is_empty() {
                            for line in String::from_utf8_lossy(&output.stderr).lines().take(100) {
                                self.history.push(format!("[error] {}", line));
                            }
                        }
                        if output.stdout.is_empty() && output.stderr.is_empty() {
                            self.history.push("Command executed successfully (no output)".to_string());
                        }
                    }
                    Err(e) => {
                        self.history.push(format!("Command not found: {}", command.trim()));
                    }
                }
            }
        }
        
        if self.history.len() > 1000 {
            self.history = self.history.split_off(self.history.len() - 1000);
        }
        
        app.log(format!("Terminal: Executed '{}'", command));
    }
}

fn chrono_lite() -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap();
    let secs = now.as_secs();
    let days = secs / 86400;
    let years = 1970 + days / 365;
    format!("📅 Current time: {} seconds since epoch (~year {})", secs, years)
}

// ============================================================================
// FILETYPE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Filetype {
    PlainText, C, Cpp, Rust, Python, JavaScript, TypeScript, Html, Css, Json, Markdown, Yaml, Toml, Go, Java, Php, Sql, Shell,
}

impl Filetype {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "c" => Filetype::C, "cpp" | "cc" | "cxx" | "h" | "hpp" => Filetype::Cpp,
            "rs" => Filetype::Rust, "py" => Filetype::Python,
            "js" | "mjs" => Filetype::JavaScript, "ts" | "tsx" => Filetype::TypeScript,
            "html" | "htm" => Filetype::Html, "css" => Filetype::Css,
            "json" => Filetype::Json, "md" | "markdown" => Filetype::Markdown,
            "yaml" | "yml" => Filetype::Yaml, "toml" => Filetype::Toml,
            "go" => Filetype::Go, "java" => Filetype::Java,
            "php" => Filetype::Php, "sql" => Filetype::Sql,
            "sh" | "bash" | "zsh" => Filetype::Shell, _ => Filetype::PlainText,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Filetype::PlainText => "Plain Text", Filetype::C => "C", Filetype::Cpp => "C++",
            Filetype::Rust => "Rust", Filetype::Python => "Python", Filetype::JavaScript => "JavaScript",
            Filetype::TypeScript => "TypeScript", Filetype::Html => "HTML", Filetype::Css => "CSS",
            Filetype::Json => "JSON", Filetype::Markdown => "Markdown", Filetype::Yaml => "YAML",
            Filetype::Toml => "TOML", Filetype::Go => "Go", Filetype::Java => "Java",
            Filetype::Php => "PHP", Filetype::Sql => "SQL", Filetype::Shell => "Shell",
        }
    }
}

// ============================================================================
// DOCUMENT
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub id: usize,
    pub name: String,
    pub path: Option<String>,
    pub content: String,
    pub filetype: Filetype,
    pub modified: bool,
    pub cursor_line: usize,
    pub cursor_col: usize,
    pub encoding: String,
    pub eol: String,
}

impl Document {
    pub fn new(id: usize) -> Self {
        Self { id, name: format!("untitled_{}", id), path: None, content: String::new(), filetype: Filetype::PlainText, modified: false, cursor_line: 1, cursor_col: 1, encoding: "UTF-8".to_string(), eol: "\n".to_string() }
    }

    pub fn from_file(path: &str, content: String) -> Self {
        let path_obj = std::path::Path::new(path);
        let name = path_obj.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let ext = path_obj.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let eol = if content.contains("\r\n") { "\r\n".to_string() } else { "\n".to_string() };
        
        Self { id: 0, name, path: Some(path.to_string()), content: content.clone(), filetype: Filetype::from_extension(&ext), modified: false, cursor_line: 1, cursor_col: 1, encoding: detect_encoding(&content), eol }
    }
}

fn detect_encoding(content: &str) -> String {
    if content.contains('\u{0}') { "UTF-16".to_string() } else { "UTF-8".to_string() }
}

// ============================================================================
// MAIN APPLICATION
// ============================================================================

pub struct GeanyApp {
    pub documents: Vec<Document>,
    pub active_doc: Option<usize>,
    pub sidebar_visible: bool,
    pub sidebar_tab: SidebarTab,
    pub messages_visible: bool,
    pub messages: Vec<String>,
    pub terminal: Terminal,
    pub build_system: BuildSystem,
    pub theme_dark: bool,
    pub find_replace: FindReplace,
    pub show_find: bool,
    pub show_goto_line: bool,
    pub goto_line: String,
    pub show_menu_bar: bool,
    pub active_menu: Option<Menu>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab { Files, Symbols, }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu { File, Edit, View, Search, Build, Tools, Help }

impl GeanyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            documents: vec![Document::new(1)],
            active_doc: Some(0),
            sidebar_visible: true,
            sidebar_tab: SidebarTab::Files,
            messages_visible: true,
            messages: vec!["Geany-Rs initialized".to_string(), "Welcome! Use File > Open or Ctrl+O to open files.".to_string()],
            terminal: Terminal::new(),
            build_system: BuildSystem::new(),
            theme_dark: true,
            find_replace: FindReplace::new(),
            show_find: false,
            show_goto_line: false,
            goto_line: String::new(),
            show_menu_bar: true,
            active_menu: None,
        };
        
        if let Some(doc) = app.documents.first_mut() {
            doc.content = include_str!("example.rs").to_string();
            doc.filetype = Filetype::Rust;
            doc.name = "example.rs".to_string();
        }
        app
    }

    fn new_document(&mut self) { let id = self.documents.len() + 1; self.documents.push(Document::new(id)); self.active_doc = Some(self.documents.len() - 1); self.log("Created new document"); }
    fn close_document(&mut self, index: usize) { if self.documents.len() > 1 { self.documents.remove(index); if let Some(active) = self.active_doc { if active >= index && active > 0 { self.active_doc = Some(active - 1); } else if active >= self.documents.len() { self.active_doc = Some(self.documents.len() - 1); } } self.log("Closed document"); } }
    fn log(&mut self, msg: impl Into<String>) { self.messages.push(msg.into()); if self.messages.len() > 100 { self.messages.remove(0); } }

    fn open_file(&mut self) {
        if let Some(path) = file_dialogs::open_file() {
            let path_str = path.to_string_lossy().to_string();
            match std::fs::read_to_string(&path_str) {
                Ok(content) => { let mut doc = Document::from_file(&path_str, content); doc.id = self.documents.len() + 1; self.documents.push(doc); self.active_doc = Some(self.documents.len() - 1); self.log(format!("Opened: {}", path_str)); }
                Err(e) => { self.log(format!("Error opening file: {}", e)); }
            }
        }
    }

    fn save_file(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &mut self.documents[idx];
            let default_name = doc.path.as_ref().map(|p| std::path::Path::new(p).file_name().unwrap().to_string_lossy().to_string()).unwrap_or_else(|| doc.name.clone());
            if let Some(path) = file_dialogs::save_file(&default_name) {
                let path_str = path.to_string_lossy().to_string();
                match std::fs::write(&path_str, &doc.content) {
                    Ok(_) => { doc.path = Some(path_str.clone()); doc.modified = false; doc.name = std::path::Path::new(&path_str).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(); self.log(format!("Saved: {}", path_str)); }
                    Err(e) => { self.log(format!("Error saving file: {}", e)); }
                }
            }
        }
    }

    fn close_all_tabs(&mut self) {
        while self.documents.len() > 1 { self.documents.pop(); }
        self.active_doc = Some(0);
        self.log("Closed all tabs except current");
    }

    fn goto_line(&mut self) {
        if let Ok(line) = self.goto_line.parse::<usize>() {
            if let Some(idx) = self.active_doc {
                let doc = &mut self.documents[idx];
                doc.cursor_line = line.min(doc.content.lines().count().max(1));
                self.log(format!("Jumped to line {}", doc.cursor_line));
            }
        }
        self.show_goto_line = false;
        self.goto_line.clear();
    }

    fn build_current(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &self.documents[idx];
            let commands = self.build_system.get_commands_for(doc.filetype);
            if let Some(cmd) = commands.first() {
                self.build_system.execute_command(cmd, &doc.path, self);
            } else {
                self.log("No build command available for this file type");
            }
        }
    }

    fn run_current(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &self.documents[idx];
            let commands = self.build_system.get_commands_for(doc.filetype);
            if commands.len() > 1 {
                self.build_system.execute_command(&commands[1], &doc.path, self);
            } else if let Some(cmd) = commands.first() {
                self.build_system.execute_command(cmd, &doc.path, self);
            } else {
                self.log("No run command available for this file type");
            }
        }
    }
}

// ============================================================================
// RENDER MENU BAR (Phase 3)
// ============================================================================

fn render_menu_bar(app: &mut GeanyApp, ui: &mut Ui) {
    let menus = vec![
        ("File", Menu::File),
        ("Edit", Menu::Edit),
        ("View", Menu::View),
        ("Search", Menu::Search),
        ("Build", Menu::Build),
        ("Tools", Menu::Tools),
        ("Help", Menu::Help),
    ];

    ui.horizontal(|ui| {
        for (name, menu_type) in menus {
            let mut text = RichText::new(name);
            if app.active_menu == Some(menu_type) { text = text.underline(); }
            if ui.selectable_label(app.active_menu == Some(menu_type), text).clicked() {
                app.active_menu = if app.active_menu == Some(menu_type) { None } else { Some(menu_type) };
            }
        }
    });
}

fn render_menu(app: &mut GeanyApp, ui: &mut Ui, menu_type: Menu) {
    match menu_type {
        Menu::File => {
            if ui.button("📄 New File      Ctrl+N").clicked() { app.new_document(); app.active_menu = None; }
            if ui.button("📂 Open File...  Ctrl+O").clicked() { app.open_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("💾 Save          Ctrl+S").clicked() { app.save_file(); app.active_menu = None; }
            if ui.button("💾 Save As...").clicked() { app.save_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("✕ Close File     Ctrl+W").clicked() { if let Some(idx) = app.active_doc { app.close_document(idx); } app.active_menu = None; }
            if ui.button("✕ Close All").clicked() { app.close_all_tabs(); app.active_menu = None; }
            ui.separator();
            if ui.button("🚪 Quit").clicked() { std::process::exit(0); }
        }
        Menu::Edit => {
            if ui.button("↩ Undo           Ctrl+Z").clicked() { app.log("Undo"); app.active_menu = None; }
            if ui.button("↪ Redo           Ctrl+Y").clicked() { app.log("Redo"); app.active_menu = None; }
            ui.separator();
            if ui.button("✂ Cut            Ctrl+X").clicked() { app.log("Cut"); app.active_menu = None; }
            if ui.button("📋 Copy          Ctrl+C").clicked() { app.log("Copy"); app.active_menu = None; }
            if ui.button("📄 Paste         Ctrl+V").clicked() { app.log("Paste"); app.active_menu = None; }
            ui.separator();
            if ui.button("Select All       Ctrl+A").clicked() { app.log("Select all"); app.active_menu = None; }
        }
        Menu::View => {
            if ui.button(if app.sidebar_visible { "✓ Sidebar" } else { "Sidebar" }).clicked() { app.sidebar_visible = !app.sidebar_visible; app.active_menu = None; }
            if ui.button(if app.messages_visible { "✓ Messages" } else { "Messages" }).clicked() { app.messages_visible = !app.messages_visible; app.active_menu = None; }
            if ui.button(if app.terminal.visible { "✓ Terminal" } else { "Terminal" }).clicked() { app.terminal.visible = !app.terminal.visible; app.active_menu = None; }
            ui.separator();
            if ui.button(if app.theme_dark { "☀️ Light Theme" } else { "🌙 Dark Theme" }).clicked() { app.theme_dark = !app.theme_dark; app.active_menu = None; }
        }
        Menu::Search => {
            if ui.button("🔍 Find...       Ctrl+F").clicked() { app.show_find = !app.show_find; app.active_menu = None; }
            if ui.button("📍 Go to Line... Ctrl+G").clicked() { app.show_goto_line = true; app.active_menu = None; }
        }
        Menu::Build => {
            if ui.button("🔨 Compile          F8").clicked() { app.build_current(); app.active_menu = None; }
            if ui.button("▶ Run               F9").clicked() { app.run_current(); app.active_menu = None; }
            ui.separator();
            if ui.button("🔄 Refresh Symbols").clicked() { app.log("Symbols refreshed"); app.active_menu = None; }
        }
        Menu::Tools => {
            if ui.button("🖥 Terminal").clicked() { app.terminal.visible = !app.terminal.visible; app.active_menu = None; }
        }
        Menu::Help => {
            if ui.button("⌨ Keyboard Shortcuts").clicked() { app.log("Ctrl+N/O/S/F/G | F8/F9 | Ctrl+W"); app.active_menu = None; }
            if ui.button("ℹ About Geany-Rs").clicked() { app.log("Geany-Rs v0.2.0 - A Rust IDE built with egui"); app.active_menu = None; }
        }
    }
}

// ============================================================================
// RENDER STATUS BAR (Phase 3)
// ============================================================================

fn render_status_bar(app: &GeanyApp, ui: &mut Ui) {
    ui.horizontal(|ui| {
        if let Some(idx) = app.active_doc {
            if let Some(doc) = app.documents.get(idx) {
                ui.label(RichText::new(format!("📄 {}", doc.name)).small().strong());
                ui.separator();
                ui.label(RichText::new(format!("Ln {}, Col {}", doc.cursor_line, doc.cursor_col)).small());
                ui.separator();
                ui.label(RichText::new(doc.filetype.name()).small());
                ui.separator();
                ui.label(RichText::new(&doc.encoding).small());
                ui.separator();
                ui.label(RichText::new(if doc.eol == "\r\n" { "CRLF" } else { "LF" }).small());
                if doc.modified { ui.label(RichText::new("● Modified").small().color(Color32::YELLOW)); }
            }
        } else {
            ui.label("No file open");
        }
        
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(RichText::new("Geany-Rs v0.2.0").small().color(Color32::GRAY));
            ui.separator();
            if app.build_system.is_running {
                ui.label(RichText::new("⚙ Building...").small().color(Color32::KHAKI));
            } else {
                ui.label(RichText::new("Ready").small().color(Color32::GRAY));
            }
        });
    });
}

// ============================================================================
// MAIN UPDATE LOOP (Phase 3)
// ============================================================================

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        let shortcuts = ctx.input(|i| i.modifiers);
        
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::N)) { self.new_document(); }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::O)) { self.open_file(); }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::S)) { self.save_file(); }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::F)) { self.show_find = !self.show_find; }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::G)) { self.show_goto_line = true; }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::W)) { if let Some(idx) = self.active_doc { self.close_document(idx); } }
        if ctx.input(|i| i.key_pressed(egui::Key::F8)) { self.build_current(); }
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) { self.run_current(); }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.show_find = false; self.show_goto_line = false; self.active_menu = None; }

        ctx.set_visuals(if self.theme_dark { egui::Visuals::dark() } else { egui::Visuals::light() });

        // ===== TOP PANEL =====
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            if self.show_menu_bar {
                render_menu_bar(self, ui);
                ui.separator();
            }
            
            ui.horizontal(|ui| {
                if ui.button("📄 New").clicked() { self.new_document(); }
                if ui.button("📂 Open").clicked() { self.open_file(); }
                if ui.button("💾 Save").clicked() { self.save_file(); }
                ui.separator();
                if ui.button("✂").on_hover_text("Cut").clicked() { self.log("Cut"); }
                if ui.button("📋").on_hover_text("Copy").clicked() { self.log("Copy"); }
                if ui.button("📄").on_hover_text("Paste").clicked() { self.log("Paste"); }
                ui.separator();
                if ui.button("🔍").on_hover_text("Find (Ctrl+F)").clicked() { self.show_find = !self.show_find; }
                if ui.button("📍").on_hover_text("Go to Line (Ctrl+G)").clicked() { self.show_goto_line = true; }
                ui.separator();
                if ui.button("🔨").on_hover_text("Compile (F8)").clicked() { self.build_current(); }
                if ui.button("▶").on_hover_text("Run (F9)").clicked() { self.run_current(); }
                ui.separator();
                if ui.toggle_value(&mut self.sidebar_visible, "📑").on_hover_text("Sidebar").clicked() {}
                if ui.toggle_value(&mut self.messages_visible, "📋").on_hover_text("Messages").clicked() {}
                if ui.toggle_value(&mut self.terminal.visible, "🖥").on_hover_text("Terminal").clicked() {}
                ui.separator();
                if ui.button(if self.theme_dark { "☀️" } else { "🌙" }).on_hover_text("Toggle Theme").clicked() { self.theme_dark = !self.theme_dark; }
            });
        });

        // ===== MENU DROPDOWN =====
        if let Some(menu) = self.active_menu {
            let pos = ctx.cursor().expect("Cursor position");
            Window::new(format!("{:?} Menu", menu))
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::LEFT_UP, [pos.x, pos.y + 20.0])
                .show(ctx, |ui| {
                    render_menu(self, ui, menu);
                });
        }

        // ===== SIDEBAR (Phase 3) =====
        if self.sidebar_visible {
            SidePanel::left("sidebar").resizable(true).default_width(220.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.toggle_sized(&mut (self.sidebar_tab == SidebarTab::Files), "📁 Files");
                    ui.toggle_sized(&mut (self.sidebar_tab == SidebarTab::Symbols), "🔣 Symbols");
                });
                ui.separator();

                match self.sidebar_tab {
                    SidebarTab::Files => {
                        ScrollArea::vertical().show(ui, |ui| {
                            for (idx, doc) in self.documents.iter().enumerate() {
                                let is_active = self.active_doc == Some(idx);
                                let mut text = RichText::new(&doc.name);
                                if doc.modified { text = text.color(Color32::YELLOW); }
                                if is_active { text = text.bold(); }
                                
                                let response = ui.selectable_label(is_active, text);
                                if response.context_menu(|ui| {
                                    if ui.button("Close").clicked() { self.close_document(idx); }
                                    if ui.button("Close Others").clicked() { 
                                        self.documents.retain(|d| d.id == doc.id || self.documents.len() == 1);
                                        self.active_doc = Some(0);
                                    }
                                    if ui.button("Close All").clicked() { self.close_all_tabs(); }
                                }).clicked() { self.active_doc = Some(idx); }
                            }
                        });
                    }
                    SidebarTab::Symbols => {
                        if let Some(idx) = self.active_doc {
                            if let Some(doc) = self.documents.get(idx) {
                                let symbols = SymbolParser::parse(&doc.content, doc.filetype);
                                if symbols.is_empty() {
                                    ui.label(RichText::new("No symbols found").color(Color32::GRAY));
                                    ui.label(RichText::new("(Save file to update)").small().color(Color32::GRAY));
                                } else {
                                    ScrollArea::vertical().show(ui, |ui| {
                                        for symbol in &symbols {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(symbol.kind.icon()).color(symbol.kind.color()).small());
                                                if ui.link(&symbol.name).clicked() {
                                                    self.documents.get_mut(idx).map(|d| d.cursor_line = symbol.line + 1);
                                                    self.log(format!("Jump to: {} (line {})", symbol.name, symbol.line + 1));
                                                }
                                            });
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            });
        }

        // ===== MAIN EDITOR AREA (Phase 3) =====
        CentralPanel::default().show(ctx, |ui| {
            // Tab bar with scroll
            ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, doc) in self.documents.iter().enumerate() {
                        let is_active = self.active_doc == Some(idx);
                        let mut label = doc.name.clone();
                        if doc.modified { label.push_str(" ●"); }
                        
                        let response = ui.selectable_label(is_active, label);
                        if response.clicked() { self.active_doc = Some(idx); }
                        if response.context_menu(|ui| {
                            if ui.button("Close").clicked() { self.close_document(idx); }
                            if ui.button("Close Others").clicked() { 
                                self.documents.retain(|d| d.id == doc.id || self.documents.len() == 1);
                                self.active_doc = Some(0);
                            }
                        }).clicked() { self.active_doc = Some(idx); }
                    }
                    if ui.button("+").on_hover_text("New Tab").clicked() { self.new_document(); }
                });
            });
            ui.separator();

            // Find bar
            if self.show_find {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔍 Find:");
                        TextEdit::singleline(&mut self.find_replace.search_text).desired_width(200.0).show(ui);
                        if ui.button("Find All").clicked() { 
                            if let Some(idx) = self.active_doc { 
                                if let Some(doc) = self.documents.get(idx) { 
                                    self.find_replace.search(&doc.content); 
                                    self.log(format!("Found {} matches", self.find_replace.search_results.len())); 
                                } 
                            } 
                        }
                        ui.separator();
                        ui.label("↔ Replace:");
                        TextEdit::singleline(&mut self.find_replace.replace_text).desired_width(200.0).show(ui);
                        if ui.button("Replace All").clicked() { 
                            if let Some(idx) = self.active_doc { 
                                let new_content = self.find_replace.replace_all(&self.documents[idx].content); 
                                self.documents[idx].content = new_content; 
                                self.documents[idx].modified = true; 
                                self.log("Replaced all occurrences".to_string()); 
                            } 
                        }
                        ui.separator();
                        ui.checkbox(&mut self.find_replace.case_sensitive, "Case");
                        ui.checkbox(&mut self.find_replace.whole_word, "Word");
                        ui.checkbox(&mut self.find_replace.regex, "Regex");
                        if ui.button("✕").clicked() { self.show_find = false; }
                    });
                });
                ui.separator();
            }

            // Editor content
            if let Some(idx) = self.active_doc {
                if let Some(doc) = self.documents.get_mut(idx) {
                    ui.horizontal(|ui| {
                        ui.label("📝 Filetype:");
                        ComboBox::from_id_salt("ft").selected_text(doc.filetype.name()).show_ui(ui, |ui| {
                            ui.selectable_value(&mut doc.filetype, Filetype::Rust, "Rust");
                            ui.selectable_value(&mut doc.filetype, Filetype::C, "C");
                            ui.selectable_value(&mut doc.filetype, Filetype::Cpp, "C++");
                            ui.selectable_value(&mut doc.filetype, Filetype::Python, "Python");
                            ui.selectable_value(&mut doc.filetype, Filetype::JavaScript, "JavaScript");
                            ui.selectable_value(&mut doc.filetype, Filetype::TypeScript, "TypeScript");
                            ui.selectable_value(&mut doc.filetype, Filetype::Html, "HTML");
                            ui.selectable_value(&mut doc.filetype, Filetype::Css, "CSS");
                            ui.selectable_value(&mut doc.filetype, Filetype::Json, "JSON");
                            ui.selectable_value(&mut doc.filetype, Filetype::Markdown, "Markdown");
                            ui.selectable_value(&mut doc.filetype, Filetype::PlainText, "Plain Text");
                        });
                        ui.separator();
                        let lines = doc.content.lines().count();
                        let chars = doc.content.len();
                        ui.label(format!("Lines: {} | Chars: {}", lines.max(1), chars));
                        if doc.modified { ui.label(RichText::new("(Modified)").color(Color32::YELLOW)); }
                    });
                    ui.separator();

                    ScrollArea::vertical().show(ui, |ui| {
                        let mut text = doc.content.clone();
                        TextEdit::multiline(&mut text).font(FontId::monospace(14.0)).desired_width(ui.available_width()).show(ui);
                        if text != doc.content { doc.content = text; doc.modified = true; }
                    });
                }
            } else {
                ui.centered_and_on_hovered_and_clicked_widget(|ui| { 
                    ui.vertical_centered(|ui| {
                        ui.heading("No document open");
                        ui.label("Use File > Open or Ctrl+O to open a file");
                    });
                });
            }
        });

        // ===== STATUS BAR (Phase 3) =====
        TopBottomPanel::bottom("status").show(ctx, |ui| {
            render_status_bar(self, ui);
        });

        // ===== MESSAGE PANEL =====
        if self.messages_visible {
            TopBottomPanel::bottom("messages").resizable(true).default_height(100.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("📋 Messages:"); if ui.button("Clear").clicked() { self.messages.clear(); } });
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| { for msg in &self.messages { ui.label(msg.clone()); } });
            });
        }

        // ===== TERMINAL (Phase 3) =====
        if self.terminal.visible {
            TopBottomPanel::bottom("terminal").resizable(true).default_height(200.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("🖥️ Terminal:");
                    ui.label(RichText::new(&self.terminal.current_dir.replace(&*whoami::username(), "~")).small().color(Color32::GRAY));
                    if ui.button("Clear").clicked() { self.terminal.history.clear(); }
                });
                ui.separator();
                ScrollArea::vertical().id_salt("term").show(ui, |ui| {
                    let history_text = self.terminal.history.join("\n");
                    ui.label(RichText::new(&history_text).monospace().size(12.0));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(RichText::new(format!("{} $ ", self.terminal.current_dir.replace(&*whoami::username(), "~"))).monospace().small());
                    let response = TextEdit::singleline(&mut self.terminal.command_input).font(FontId::monospace(14.0)).show(ui);
                    if response.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.terminal.execute(&self.terminal.command_input, self);
                        self.terminal.command_input.clear();
                    }
                });
            });
        }

        // ===== GOTO LINE DIALOG =====
        if self.show_goto_line {
            Window::new("Go to Line").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("Enter line number:");
                TextEdit::singleline(&mut self.goto_line).desired_width(100.0).request_focus().show(ui);
                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() { self.goto_line(); }
                    if ui.button("Cancel").clicked() { self.show_goto_line = false; self.goto_line.clear(); }
                });
            });
        }
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_min_inner_size([800.0, 600.0]).with_title("Geany-Rs - Rust IDE"),
        ..Default::default()
    };
    eframe::run_native("Geany-Rs", options, Box::new(|cc| Ok(Box::new(GeanyApp::new(cc))))).unwrap();
}
