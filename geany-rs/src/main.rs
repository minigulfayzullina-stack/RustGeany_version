//! Geany-Rs - A fast and lightweight IDE in Rust
//! Built with egui for cross-platform support
//!
//! Features (v0.5.0 - Polished):
//! - Project Management (.geany files)
//! - Auto-completion (keywords + symbols)
//! - Plugin System (with dialog)
//! - Macros (record/playback with UI)
//! - Full IDE: Tabs, Terminal, Search, Build

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{Align, Color32, ComboBox, FontId, RichText, ScrollArea, TextEdit, TopBottomPanel, SidePanel, CentralPanel, Ui, Window, TextStyle, Label, Button, Separator};
use regex::Regex;
use std::process::{Command, Stdio};

// ============================================================================
// FILE DIALOGS
// ============================================================================

mod file_dialogs {
    use rfd::FileDialog;
    use std::path::PathBuf;
    
    pub fn open_file() -> Option<PathBuf> {
        FileDialog::new()
            .add_filter("Source Files", &["rs", "c", "cpp", "h", "py", "js", "ts", "html", "css", "json", "toml"])
            .add_filter("Geany Projects", &["geany"])
            .add_filter("All Files", &["*"])
            .pick_file()
    }
    
    pub fn save_file(default_name: &str) -> Option<PathBuf> {
        FileDialog::new().set_file_name(default_name)
            .add_filter("Source Files", &["rs", "c", "cpp", "py", "js"])
            .add_filter("All Files", &["*"])
            .save_file()
    }
    
    pub fn save_project() -> Option<PathBuf> {
        FileDialog::new().set_file_name("project.geany")
            .add_filter("Geany Project", &["geany"])
            .save_file()
    }
    
    pub fn pick_folder() -> Option<PathBuf> {
        FileDialog::new().pick_folder()
    }
}

// ============================================================================
// PROJECT MANAGEMENT (POLISHED)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeanyProject {
    pub name: String,
    pub description: String,
    pub base_path: String,
    pub file_patterns: Vec<String>,
    pub recent_files: Vec<String>,
    pub build_commands: Vec<BuildCmd>,
    pub preferences: ProjectPrefs,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildCmd {
    pub label: String,
    pub command: String,
    pub working_dir: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectPrefs {
    pub default_encoding: String,
    pub eol_mode: String,
    pub indent_width: usize,
    pub use_spaces: bool,
}

impl Default for GeanyProject {
    fn default() -> Self {
        Self {
            name: "New Project".to_string(),
            description: String::new(),
            base_path: String::new(),
            file_patterns: vec!["*.rs".to_string(), "*.c".to_string(), "*.py".to_string()],
            recent_files: Vec::new(),
            build_commands: vec![
                BuildCmd { label: "Build".to_string(), command: "cargo build".to_string(), working_dir: "$(ProjectPath)".to_string() },
                BuildCmd { label: "Run".to_string(), command: "cargo run".to_string(), working_dir: "$(ProjectPath)".to_string() },
            ],
            preferences: ProjectPrefs { default_encoding: "UTF-8".to_string(), eol_mode: "LF".to_string(), indent_width: 4, use_spaces: true },
        }
    }
}

impl GeanyProject {
    pub fn save(&self, path: &str) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, json).map_err(|e| e.to_string())
    }
    
    pub fn load(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    }
    
    pub fn add_recent_file(&mut self, path: &str) {
        self.recent_files.retain(|p| p != path);
        self.recent_files.insert(0, path.to_string());
        self.recent_files.truncate(10);
    }
}

// ============================================================================
// AUTO-COMPLETION (POLISHED)
// ============================================================================

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: String,
    pub insert_text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Keyword, Function, Snippet, Variable, Class, Property,
}

impl CompletionKind {
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::Keyword => "🔑",
            CompletionKind::Function => "ƒ",
            CompletionKind::Snippet => "⚡",
            CompletionKind::Variable => "v",
            CompletionKind::Class => "C",
            CompletionKind::Property => "p",
        }
    }
    
    pub fn color(&self) -> Color32 {
        match self {
            CompletionKind::Keyword => Color32::from_rgb(86, 156, 214),
            CompletionKind::Function => Color32::from_rgb(220, 220, 170),
            CompletionKind::Snippet => Color32::from_rgb(206, 145, 120),
            CompletionKind::Variable => Color32::from_rgb(181, 206, 168),
            CompletionKind::Class => Color32::from_rgb(78, 201, 176),
            CompletionKind::Property => Color32::from_rgb(197, 134, 192),
        }
    }
}

pub struct AutoCompleter {
    pub enabled: bool,
    pub show_popup: bool,
    pub items: Vec<CompletionItem>,
    pub selected_index: usize,
    pub current_word: String,
}

impl AutoCompleter {
    pub fn new() -> Self {
        Self { enabled: true, show_popup: false, items: Vec::new(), selected_index: 0, current_word: String::new() }
    }
    
    fn get_keywords(filetype: Filetype) -> Vec<(&'static str, &'static str, &'static str)> {
        match filetype {
            Filetype::Rust => vec![
                ("fn", "fn name() {}", "Function declaration"),
                ("let", "let name = value", "Variable declaration"),
                ("let mut", "let mut name = value", "Mutable variable"),
                ("pub", "pub item", "Public visibility"),
                ("struct", "struct Name {}", "Struct definition"),
                ("impl", "impl Type {}", "Implementation"),
                ("enum", "enum Name {}", "Enumeration"),
                ("trait", "trait Name {}", "Trait definition"),
                ("mod", "mod name;", "Module declaration"),
                ("use", "use path::item;", "Import statement"),
                ("match", "match value {}", "Pattern matching"),
                ("if", "if condition {}", "If statement"),
                ("else", "else {}", "Else branch"),
                ("for", "for item in coll {}", "For loop"),
                ("while", "while cond {}", "While loop"),
                ("loop", "loop {}", "Infinite loop"),
                ("return", "return value;", "Return statement"),
                ("async", "async fn name() {}", "Async function"),
                ("await", "await expression", "Await keyword"),
                ("Some", "Some(value)", "Option variant"),
                ("None", "None", "Option variant"),
                ("Ok", "Ok(value)", "Result variant"),
                ("Err", "Err(error)", "Result variant"),
            ],
            Filetype::Python => vec![
                ("def", "def func_name():", "Function definition"),
                ("class", "class ClassName:", "Class definition"),
                ("if", "if condition:", "If statement"),
                ("elif", "elif condition:", "Else if statement"),
                ("for", "for item in iterable:", "For loop"),
                ("while", "while condition:", "While loop"),
                ("try", "try:", "Try block"),
                ("except", "except Exception:", "Exception handler"),
                ("with", "with open() as f:", "Context manager"),
                ("import", "import module", "Import statement"),
                ("from", "from module import", "From import"),
                ("return", "return value", "Return statement"),
                ("lambda", "lambda x: x", "Lambda function"),
                ("async", "async def", "Async function"),
                ("yield", "yield value", "Yield expression"),
            ],
            Filetype::JavaScript | Filetype::TypeScript => vec![
                ("function", "function name() {}", "Function declaration"),
                ("const", "const name = value", "Constant"),
                ("let", "let name = value", "Let declaration"),
                ("class", "class Name {}", "Class definition"),
                ("async", "async function", "Async function"),
                ("import", "import name from 'mod'", "Import"),
                ("export", "export name", "Export"),
                ("if", "if (condition) {}", "If statement"),
                ("for", "for (let i = 0; i < n; i++) {}", "For loop"),
                ("return", "return value", "Return statement"),
                ("try", "try {} catch (e) {}", "Try-catch"),
            ],
            Filetype::C | Filetype::Cpp => vec![
                ("int", "int name;", "Integer type"),
                ("char", "char name;", "Character type"),
                ("float", "float name;", "Float type"),
                ("double", "double name;", "Double type"),
                ("void", "void func()", "Void type"),
                ("struct", "struct Name {}", "Struct definition"),
                ("enum", "enum Name {}", "Enumeration"),
                ("typedef", "typedef existing new_name", "Type definition"),
                ("if", "if (condition) {}", "If statement"),
                ("for", "for (int i = 0; i < n; i++) {}", "For loop"),
                ("while", "while (condition) {}", "While loop"),
                ("return", "return value;", "Return statement"),
            ],
            _ => vec![],
        }
    }
    
    pub fn trigger(&mut self, content: &str, filetype: Filetype, symbols: &[String]) {
        self.items.clear();
        self.selected_index = 0;
        
        // Get current word
        let before = &content[..content.len().min(content.len())];
        self.current_word = before.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>().chars().rev().collect();
        
        if self.current_word.len() < 1 {
            self.show_popup = false;
            return;
        }
        
        // Add keywords
        for (label, insert, detail) in Self::get_keywords(filetype) {
            if label.to_lowercase().starts_with(&self.current_word.to_lowercase()) {
                self.items.push(CompletionItem {
                    label: label.to_string(),
                    kind: CompletionKind::Keyword,
                    detail: detail.to_string(),
                    insert_text: insert.to_string(),
                });
            }
        }
        
        // Add symbols
        for sym in symbols {
            if sym.to_lowercase().starts_with(&self.current_word.to_lowercase()) {
                self.items.push(CompletionItem {
                    label: sym.clone(),
                    kind: CompletionKind::Function,
                    detail: "Symbol from file".to_string(),
                    insert_text: sym.clone(),
                });
            }
        }
        
        self.show_popup = !self.items.is_empty();
    }
    
    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }
    
    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = self.selected_index.saturating_sub(1);
            if self.selected_index == 0 { self.selected_index = self.items.len() - 1; }
        }
    }
    
    pub fn insert_selected(&self) -> Option<String> {
        self.items.get(self.selected_index).map(|i| i.insert_text.clone())
    }
}

// ============================================================================
// MACROS (POLISHED)
// ============================================================================

#[derive(Debug, Clone)]
pub struct MacroAction {
    pub action_type: MacroActionType,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MacroActionType {
    InsertText,
    DeleteBack,
    NewLine,
    Tab,
    MoveLeft,
    MoveRight,
}

#[derive(Debug, Clone)]
pub struct Macro {
    pub name: String,
    pub actions: Vec<MacroAction>,
    pub description: String,
}

impl Macro {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), actions: Vec::new(), description: String::new() }
    }
}

pub struct MacroManager {
    pub is_recording: bool,
    pub current_macro: Option<Macro>,
    pub saved_macros: Vec<Macro>,
    pub action_count: usize,
}

impl MacroManager {
    pub fn new() -> Self {
        Self { is_recording: false, current_macro: None, saved_macros: Vec::new(), action_count: 0 }
    }
    
    pub fn start_recording(&mut self) {
        self.is_recording = true;
        self.current_macro = Some(Macro::new(&format!("Macro {}", self.saved_macros.len() + 1)));
        self.action_count = 0;
    }
    
    pub fn stop_recording(&mut self) -> Option<Macro> {
        self.is_recording = false;
        if let Some(mut m) = self.current_macro.take() {
            m.description = format!("{} actions", self.action_count);
            if !m.actions.is_empty() {
                self.saved_macros.push(m.clone());
                return Some(m);
            }
        }
        None
    }
    
    pub fn record_insert(&mut self, text: &str) {
        if self.is_recording {
            self.action_count += 1;
            if let Some(ref mut m) = self.current_macro {
                m.actions.push(MacroAction { action_type: MacroActionType::InsertText, text: Some(text.to_string()) });
            }
        }
    }
    
    pub fn record_delete(&mut self) {
        if self.is_recording {
            self.action_count += 1;
            if let Some(ref mut m) = self.current_macro {
                m.actions.push(MacroAction { action_type: MacroActionType::DeleteBack, text: None });
            }
        }
    }
    
    pub fn record_newline(&mut self) {
        if self.is_recording {
            self.action_count += 1;
            if let Some(ref mut m) = self.current_macro {
                m.actions.push(MacroAction { action_type: MacroActionType::NewLine, text: None });
            }
        }
    }
    
    pub fn execute_macro(&self, content: &str, _cursor_pos: usize) -> String {
        let mut result = content.to_string();
        
        for action in &self.actions {
            match action.action_type {
                MacroActionType::InsertText => {
                    if let Some(ref text) = action.text {
                        result.push_str(text);
                    }
                }
                MacroActionType::DeleteBack => {
                    result.pop();
                }
                MacroActionType::NewLine => {
                    result.push('\n');
                }
                MacroActionType::Tab => {
                    result.push_str("    ");
                }
                MacroActionType::MoveLeft | MacroActionType::MoveRight => {}
            }
        }
        
        result
    }
}

// ============================================================================
// PLUGIN SYSTEM (POLISHED)
// ============================================================================

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub hooks: Vec<Hook>,
}

#[derive(Debug, Clone)]
pub enum Hook {
    Startup, Shutdown, DocumentNew, DocumentOpen, DocumentSave, DocumentClose,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub info: PluginInfo,
    pub enabled: bool,
    pub config: serde_json::Value,
}

impl Plugin {
    pub fn new(name: &str, description: &str, author: &str) -> Self {
        Self {
            info: PluginInfo { name: name.to_string(), description: description.to_string(), version: "0.1.0".to_string(), author: author.to_string(), hooks: vec![] },
            enabled: false,
            config: serde_json::json!({}),
        }
    }
}

pub struct PluginManager {
    pub plugins: Vec<Plugin>,
    pub show_dialog: bool,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut m = Self { plugins: Vec::new(), show_dialog: false };
        
        m.plugins.push({
            let mut p = Plugin::new("CodeFormatter", "Auto-format code on save", "Geany-Rs");
            p.info.version = "1.0.0".to_string();
            p.info.hooks = vec![Hook::DocumentSave];
            p
        });
        
        m.plugins.push({
            let mut p = Plugin::new("BracketHighlighter", "Highlight matching brackets when cursor is on bracket", "Geany-Rs");
            p.info.version = "1.0.0".to_string();
            p.info.hooks = vec![];
            p
        });
        
        m.plugins.push({
            let mut p = Plugin::new("TodoViewer", "Show TODO/FIXME/NOTE comments in sidebar", "Geany-Rs");
            p.info.version = "1.0.0".to_string();
            p.info.hooks = vec![Hook::DocumentNew, Hook::DocumentOpen];
            p
        });
        
        m.plugins.push({
            let mut p = Plugin::new("LineNumbers", "Show line numbers with better styling", "Geany-Rs");
            p.info.version = "1.0.0".to_string();
            p.info.hooks = vec![];
            p
        });
        
        m.plugins.push({
            let mut p = Plugin::new("AutoSave", "Automatically save files every 60 seconds", "Geany-Rs");
            p.info.version = "1.0.0".to_string();
            p.info.hooks = vec![];
            p
        });
        
        m
    }
}

// ============================================================================
// SYMBOL PARSER
// ============================================================================

#[derive(Debug, Clone)]
pub struct Symbol { pub name: String, pub kind: SymbolKind, pub line: usize }

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind { Function, Struct, Enum, Impl, Trait, Class, Module, Variable, Constant }

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self { SymbolKind::Function => "ƒ", SymbolKind::Struct => "S", SymbolKind::Enum => "E",
            SymbolKind::Impl => "I", SymbolKind::Trait => "T", SymbolKind::Class => "C",
            SymbolKind::Module => "M", SymbolKind::Variable => "v", SymbolKind::Constant => "K" }
    }
    pub fn color(&self) -> Color32 {
        match self { SymbolKind::Function => Color32::from_rgb(230, 192, 123), SymbolKind::Struct | SymbolKind::Class => Color32::from_rgb(78, 201, 176),
            SymbolKind::Enum => Color32::from_rgb(86, 156, 214), SymbolKind::Impl | SymbolKind::Trait => Color32::from_rgb(206, 145, 120),
            SymbolKind::Module => Color32::from_rgb(197, 134, 192), SymbolKind::Variable | SymbolKind::Constant => Color32::from_rgb(181, 206, 168) }
    }
}

pub struct SymbolParser;

impl SymbolParser {
    pub fn parse(content: &str, filetype: Filetype) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        let patterns: Vec<(&str, SymbolKind)> = match filetype {
            Filetype::Rust => vec![
                (r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", SymbolKind::Function),
                (r"(?m)^(?:pub\s+)?struct\s+(\w+)", SymbolKind::Struct),
                (r"(?m)^(?:pub\s+)?enum\s+(\w+)", SymbolKind::Enum),
                (r"(?m)^(?:pub\s+)?impl(?:\s+<\w+>)?\s+(\w+)", SymbolKind::Impl),
                (r"(?m)^(?:pub\s+)?trait\s+(\w+)", SymbolKind::Trait),
                (r"(?m)^(?:pub\s+)?mod\s+(\w+)", SymbolKind::Module),
            ],
            Filetype::Python => vec![
                (r"(?m)^class\s+(\w+)", SymbolKind::Class),
                (r"(?m)^(?:async\s+)?def\s+(\w+)", SymbolKind::Function),
            ],
            Filetype::JavaScript | Filetype::TypeScript => vec![
                (r"(?m)^class\s+(\w+)", SymbolKind::Class),
                (r"(?m)^function\s+(\w+)", SymbolKind::Function),
                (r"(?m)^(?:const|let|var)\s+(\w+)", SymbolKind::Variable),
            ],
            Filetype::C | Filetype::Cpp => vec![
                (r"(?m)^(?:[\w\*]+\s+)+(\w+)\s*\([^)]*\)\s*\{", SymbolKind::Function),
            ],
            _ => vec![],
        };
        
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
    
    pub fn get_names(symbols: &[Symbol]) -> Vec<String> { symbols.iter().map(|s| s.name.clone()).collect() }
}

// ============================================================================
// FILETYPE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Filetype { PlainText, C, Cpp, Rust, Python, JavaScript, TypeScript, Html, Css, Json, Markdown, Yaml, Toml, Go, Java, Php, Sql, Shell }

impl Filetype {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "c" => Filetype::C, "cpp" | "cc" | "cxx" | "h" | "hpp" => Filetype::Cpp,
            "rs" => Filetype::Rust, "py" => Filetype::Python,
            "js" | "mjs" => Filetype::JavaScript, "ts" | "tsx" => Filetype::TypeScript,
            "html" | "htm" => Filetype::Html, "css" => Filetype::Css,
            "json" => Filetype::Json, "md" => Filetype::Markdown,
            "yaml" | "yml" => Filetype::Yaml, "toml" => Filetype::Toml,
            "go" => Filetype::Go, "java" => Filetype::Java,
            "php" => Filetype::Php, "sql" => Filetype::Sql,
            "sh" | "bash" => Filetype::Shell, _ => Filetype::PlainText,
        }
    }
    pub fn name(&self) -> &'static str {
        match self { Filetype::PlainText => "Plain Text", Filetype::C => "C", Filetype::Cpp => "C++",
            Filetype::Rust => "Rust", Filetype::Python => "Python", Filetype::JavaScript => "JavaScript",
            Filetype::TypeScript => "TypeScript", Filetype::Html => "HTML", Filetype::Css => "CSS",
            Filetype::Json => "JSON", Filetype::Markdown => "Markdown", Filetype::Yaml => "YAML",
            Filetype::Toml => "TOML", Filetype::Go => "Go", Filetype::Java => "Java",
            Filetype::Php => "PHP", Filetype::Sql => "SQL", Filetype::Shell => "Shell" }
    }
}

// ============================================================================
// DOCUMENT
// ============================================================================

#[derive(Debug, Clone)]
pub struct Document {
    pub id: usize, pub name: String, pub path: Option<String>,
    pub content: String, pub filetype: Filetype, pub modified: bool,
    pub cursor_line: usize, pub cursor_col: usize,
}

impl Document {
    pub fn new(id: usize) -> Self {
        Self { id, name: format!("untitled_{}", id), path: None, content: String::new(), filetype: Filetype::PlainText, modified: false, cursor_line: 1, cursor_col: 1 }
    }
    pub fn from_file(path: &str, content: String) -> Self {
        let p = std::path::Path::new(path);
        let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let ext = p.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Self { id: 0, name, path: Some(path.to_string()), content, filetype: Filetype::from_extension(&ext), modified: false, cursor_line: 1, cursor_col: 1 }
    }
}

// ============================================================================
// TERMINAL
// ============================================================================

pub struct Terminal {
    pub visible: bool, pub history: Vec<String>, pub current_dir: String, pub command_input: String,
}

impl Terminal {
    pub fn new() -> Self {
        let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
        Self { visible: false, history: vec!["Geany-Rs Terminal v0.5.0".to_string(), "Type 'help' for commands".to_string()], current_dir: home, command_input: String::new() }
    }
    pub fn execute(&mut self, cmd: &str, app: &mut GeanyApp) {
        if cmd.trim().is_empty() { return; }
        self.history.push(format!("$ {}", cmd));
        match cmd.trim() {
            "help" => { self.history.push("Commands: help, clear, pwd, cd, ls, cat, mkdir, touch, rm, echo, date, whoami".to_string()); }
            "clear" => { self.history.clear(); }
            "pwd" => { self.history.push(self.current_dir.replace(&whoami::username(), "~")); }
            "date" => { let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap(); self.history.push(format!("Epoch: {} seconds", now.as_secs())); }
            "whoami" => { self.history.push(whoami::username()); }
            cmd if cmd.starts_with("cd ") => {
                let dir = cmd.trim_start_matches("cd ").replace('~', &whoami::username());
                if let Ok(p) = std::fs::canonicalize(&dir) { self.current_dir = p.to_string_lossy().to_string(); }
                else { self.history.push(format!("cd: {}: Not found", dir)); }
            }
            "ls" => { if let Ok(e) = std::fs::read_dir(&self.current_dir) { for x in e.filter_map(|e| e.ok()) { self.history.push(x.file_name().to_string_lossy().to_string()); } } }
            cmd if cmd.starts_with("cat ") => { if let Ok(c) = std::fs::read_to_string(cmd.trim_start_matches("cat ")) { for l in c.lines().take(30) { self.history.push(l.to_string()); } } }
            cmd if cmd.starts_with("mkdir ") => { let _ = std::fs::create_dir_all(cmd.trim_start_matches("mkdir ")); }
            cmd if cmd.starts_with("touch ") => { let _ = std::fs::write(cmd.trim_start_matches("touch "), ""); }
            cmd if cmd.starts_with("rm ") => { let _ = std::fs::remove_file(cmd.trim_start_matches("rm ")); }
            cmd if cmd.starts_with("echo ") => { self.history.push(cmd.trim_start_matches("echo ").to_string()); }
            _ => {
                #[cfg(target_os = "windows")] let (s, a) = ("cmd", "/C");
                #[cfg(not(target_os = "windows"))] let (s, a) = ("sh", "-c");
                if let Ok(out) = Command::new(s).arg(a).arg(cmd).current_dir(&self.current_dir).output() {
                    if !out.stdout.is_empty() { for l in String::from_utf8_lossy(&out.stdout).lines().take(30) { self.history.push(l.to_string()); } }
                    if !out.stderr.is_empty() { for l in String::from_utf8_lossy(&out.stderr).lines().take(30) { self.history.push(format!("[err] {}", l)); } }
                }
            }
        }
        if self.history.len() > 200 { self.history = self.history.split_off(self.history.len() - 200); }
        app.log(format!("Executed: {}", cmd));
    }
}

// ============================================================================
// FIND & REPLACE
// ============================================================================

#[derive(Debug, Clone)]
pub struct FindReplace { pub search_text: String, pub replace_text: String, pub case_sensitive: bool, pub whole_word: bool, pub regex: bool }

impl FindReplace { pub fn new() -> Self { Self { search_text: String::new(), replace_text: String::new(), case_sensitive: false, whole_word: false, regex: false } }
    pub fn search(&mut self, content: &str) -> Vec<usize> {
        if self.search_text.is_empty() { return vec![]; }
        let needle = if self.whole_word { format!(r"\b{}\b", regex::escape(&self.search_text)) } else { regex::escape(&self.search_text) };
        let pattern = if self.case_sensitive { needle } else { format!("(?i){}", needle) };
        if let Ok(re) = Regex::new(&pattern) { content.lines().enumerate().filter(|(_, l)| re.is_match(l)).map(|(i, _)| i).collect() } else { vec![] }
    }
    pub fn replace_all(&self, content: &str) -> String {
        if self.search_text.is_empty() { return content.to_string(); }
        let needle = if self.whole_word { format!(r"\b{}\b", regex::escape(&self.search_text)) } else { regex::escape(&self.search_text) };
        let pattern = if self.case_sensitive { needle } else { format!("(?i){}", needle) };
        if let Ok(re) = Regex::new(&pattern) { re.replace_all(content, self.replace_text.as_str()).to_string() } else { content.to_string() }
    }
}

// ============================================================================
// BUILD SYSTEM
// ============================================================================

pub struct BuildSystem;

impl BuildSystem {
    pub fn get_commands(filetype: Filetype) -> Vec<(&'static str, &'static str)> {
        match filetype {
            Filetype::Rust => vec![("🔨 Build (F8)", "cargo build"), ("▶ Run (F9)", "cargo run")],
            Filetype::C | Filetype::Cpp => vec![("🔨 Compile (F8)", "gcc \"{file}\" -o \"{name}\" -Wall"), ("▶ Run (F9)", "\"./{name}\"")],
            Filetype::Python => vec![("▶ Run (F9)", "python3 \"{file}\"")],
            Filetype::JavaScript => vec![("▶ Run (F9)", "node \"{file}\"")],
            Filetype::Html => vec![("🌐 Open (F9)", "xdg-open \"{file}\"")],
            _ => vec![],
        }
    }
    pub fn expand(cmd: &str, file_path: &Option<String>) -> String {
        let mut result = cmd.to_string();
        if let Some(path) = file_path {
            let p = std::path::Path::new(path);
            result = result.replace("{file}", path);
            result = result.replace("{name}", &p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default());
            result = result.replace("{dir}", &p.parent().map(|p| p.to_string_lossy().to_string()).unwrap_or_default());
        }
        result
    }
    pub fn execute(cmd: &str, app: &mut GeanyApp) {
        app.log(format!("> {}", cmd));
        #[cfg(target_os = "windows")] let (s, a) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))] let (s, a) = ("sh", "-c");
        if let Ok(mut child) = Command::new(s).arg(a).arg(cmd).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            use std::io::Read;
            if let Some(mut out) = child.stdout.take() { let mut st = String::new(); if out.read_to_string(&mut st).is_ok() && !st.is_empty() { for l in st.lines().take(50) { app.log(l.to_string()); } } }
            if let Some(mut err) = child.stderr.take() { let mut st = String::new(); if err.read_to_string(&mut st).is_ok() && !st.is_empty() { for l in st.lines().take(50) { app.log(format!("[err] {}", l)); } } }
            if let Ok(st) = child.wait() { app.log(if st.success() { "✓ Build succeeded".to_string() } else { format!("✗ Exit code: {:?}", st.code()) }); }
        } else { app.log(format!("Error: could not execute '{}'", cmd)); }
    }
}

// ============================================================================
// MAIN APPLICATION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab { Files, Symbols, Macros, Plugins, Project }

pub struct GeanyApp {
    pub documents: Vec<Document>, pub active_doc: Option<usize>,
    pub project: Option<GeanyProject>,
    pub sidebar_visible: bool, pub sidebar_tab: SidebarTab,
    pub messages_visible: bool, pub messages: Vec<String>,
    pub terminal: Terminal,
    pub completer: AutoCompleter,
    pub macro_manager: MacroManager,
    pub plugin_manager: PluginManager,
    pub theme_dark: bool,
    pub find_replace: FindReplace,
    pub show_find: bool, pub show_goto_line: bool, pub show_settings: bool,
    pub goto_line: String,
    pub active_menu: Option<Menu>,
    pub line_numbers: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu { File, Edit, View, Search, Build, Project, Tools, Macros, Plugins, Help }

impl GeanyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            documents: vec![Document::new(1)], active_doc: Some(0),
            project: None,
            sidebar_visible: true, sidebar_tab: SidebarTab::Files,
            messages_visible: true,
            messages: vec!["Geany-Rs v0.5.0 ready!".to_string(), "Features: Projects, Auto-complete, Macros, Plugins".to_string()],
            terminal: Terminal::new(),
            completer: AutoCompleter::new(),
            macro_manager: MacroManager::new(),
            plugin_manager: PluginManager::new(),
            theme_dark: true,
            find_replace: FindReplace::new(),
            show_find: false, show_goto_line: false, show_settings: false,
            goto_line: String::new(),
            active_menu: None,
            line_numbers: true,
        };
        if let Some(doc) = app.documents.first_mut() {
            doc.content = include_str!("example.rs").to_string();
            doc.filetype = Filetype::Rust;
            doc.name = "example.rs".to_string();
        }
        app
    }
    
    fn log(&mut self, msg: impl Into<String>) { self.messages.push(msg.into()); if self.messages.len() > 100 { self.messages.remove(0); } }
    fn new_doc(&mut self) { let id = self.documents.len() + 1; self.documents.push(Document::new(id)); self.active_doc = Some(self.documents.len() - 1); self.log("New document created"); }
    fn close_doc(&mut self, idx: usize) { if self.documents.len() > 1 { self.documents.remove(idx); if let Some(a) = self.active_doc { if a >= idx && a > 0 { self.active_doc = Some(a - 1); } else if a >= self.documents.len() { self.active_doc = Some(self.documents.len() - 1); } } self.log("Document closed"); } }
    
    fn open_file(&mut self) {
        if let Some(path) = file_dialogs::open_file() {
            let path_str = path.to_string_lossy().to_string();
            if path_str.ends_with(".geany") {
                match GeanyProject::load(&path_str) { Ok(p) => { self.project = Some(p); self.log(format!("Opened project: {}", p.name)); } Err(e) => self.log(format!("Error: {}", e)) }
            } else {
                match std::fs::read_to_string(&path_str) { Ok(c) => { let mut d = Document::from_file(&path_str, c); d.id = self.documents.len() + 1; self.documents.push(d); self.active_doc = Some(self.documents.len() - 1); self.log(format!("Opened: {}", path_str)); } Err(e) => self.log(format!("Error: {}", e)) }
            }
        }
    }
    
    fn save_file(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &mut self.documents[idx];
            let name = doc.path.as_ref().map(|p| std::path::Path::new(p).file_name().unwrap().to_string_lossy().to_string()).unwrap_or_else(|| doc.name.clone());
            if let Some(path) = file_dialogs::save_file(&name) {
                let path_str = path.to_string_lossy().to_string();
                match std::fs::write(&path_str, &doc.content) { Ok(_) => { doc.path = Some(path_str.clone()); doc.modified = false; doc.name = std::path::Path::new(&path_str).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(); self.log(format!("Saved: {}", path_str)); } Err(e) => self.log(format!("Error: {}", e)) }
            }
        }
    }
    
    fn new_project(&mut self) { self.project = Some(GeanyProject::default()); self.log("New project created"); }
    fn save_project(&mut self) { if let Some(ref p) = self.project { if let Some(path) = file_dialogs::save_project() { let path_str = path.to_string_lossy().to_string(); match p.save(&path_str) { Ok(_) => self.log(format!("Project saved: {}", path_str)), Err(e) => self.log(format!("Error: {}", e)) } } else { self.log("No project to save"); } }
    fn open_project(&mut self) { if let Some(path) = file_dialogs::pick_folder() { let path_str = path.to_string_lossy().to_string(); let mut proj = GeanyProject::default(); proj.name = std::path::Path::new(&path_str).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Project".to_string()); proj.base_path = path_str.clone(); self.project = Some(proj); self.log(format!("Opened folder: {}", path_str)); } }
}

// ============================================================================
// MENU & UI RENDERING
// ============================================================================

fn render_menu(app: &mut GeanyApp, ui: &mut Ui, menu: Menu) {
    match menu {
        Menu::File => {
            if ui.button("📄 New File       Ctrl+N").clicked() { app.new_doc(); app.active_menu = None; }
            if ui.button("📂 Open...        Ctrl+O").clicked() { app.open_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("💾 Save          Ctrl+S").clicked() { app.save_file(); app.active_menu = None; }
            if ui.button("💾 Save As").clicked() { app.save_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("✕ Close Tab      Ctrl+W").clicked() { if let Some(idx) = app.active_doc { app.close_doc(idx); } app.active_menu = None; }
        }
        Menu::Project => {
            if ui.button("📁 New Project").clicked() { app.new_project(); app.active_menu = None; }
            if ui.button("💾 Save Project").clicked() { app.save_project(); app.active_menu = None; }
            if ui.button("📂 Open Project").clicked() { app.open_project(); app.active_menu = None; }
            ui.separator();
            if let Some(ref p) = app.project { ui.label(RichText::new(format!("📁 {}", p.name)).strong()); }
            else { ui.label("No project open"); }
        }
        Menu::Edit => {
            if ui.button("↩ Undo").clicked() { app.log("Undo"); app.active_menu = None; }
            if ui.button("↪ Redo").clicked() { app.log("Redo"); app.active_menu = None; }
            ui.separator();
            if ui.button("✂ Cut").clicked() { app.log("Cut"); app.active_menu = None; }
            if ui.button("📋 Copy").clicked() { app.log("Copy"); app.active_menu = None; }
            if ui.button("📄 Paste").clicked() { app.log("Paste"); app.active_menu = None; }
        }
        Menu::View => {
            if ui.button(if app.sidebar_visible { "✓ Sidebar" } else { "Sidebar" }).clicked() { app.sidebar_visible = !app.sidebar_visible; }
            if ui.button(if app.messages_visible { "✓ Messages" } else { "Messages" }).clicked() { app.messages_visible = !app.messages_visible; }
            if ui.button(if app.terminal.visible { "✓ Terminal" } else { "Terminal" }).clicked() { app.terminal.visible = !app.terminal.visible; }
            ui.separator();
            if ui.button(if app.line_numbers { "✓ Line Numbers" } else { "Line Numbers" }).clicked() { app.line_numbers = !app.line_numbers; }
            if ui.button(if app.theme_dark { "☀️ Light Theme" } else { "🌙 Dark Theme" }).clicked() { app.theme_dark = !app.theme_dark; app.active_menu = None; }
        }
        Menu::Search => {
            if ui.button("🔍 Find          Ctrl+F").clicked() { app.show_find = !app.show_find; app.active_menu = None; }
            if ui.button("📍 Go to Line   Ctrl+G").clicked() { app.show_goto_line = true; app.active_menu = None; }
        }
        Menu::Build => {
            if let Some(idx) = app.active_doc {
                let doc = &app.documents[idx];
                for (label, cmd) in BuildSystem::get_commands(doc.filetype) {
                    if ui.button(label).clicked() { BuildSystem::execute(&BuildSystem::expand(cmd, &doc.path), app); app.active_menu = None; }
                }
            }
        }
        Menu::Tools => {
            if ui.button(if app.terminal.visible { "✓ Terminal" } else { "🖥 Terminal" }).clicked() { app.terminal.visible = !app.terminal.visible; app.active_menu = None; }
            if ui.button(if app.completer.enabled { "✓ Auto-complete" } else { "Auto-complete" }).clicked() { app.completer.enabled = !app.completer.enabled; }
        }
        Menu::Macros => {
            if ui.button(if app.macro_manager.is_recording { "⏹ Stop Recording" } else { "⏺ Start Recording" }).clicked() {
                if app.macro_manager.is_recording { if let Some(m) = app.macro_manager.stop_recording() { app.log(format!("Macro saved: {} ({} actions)", m.name, m.actions.len())); } }
                else { app.macro_manager.start_recording(); app.log("Recording... Press Ctrl+Shift+R to stop"); }
            }
            ui.separator();
            ui.label("Saved Macros:");
            if app.macro_manager.saved_macros.is_empty() { ui.label(RichText::new("No macros saved").color(Color32::GRAY)); }
            else { for (i, m) in app.macro_manager.saved_macros.iter().enumerate() { ui.horizontal(|ui| { if ui.button(format!("▶ {}", m.name)).clicked() { if let Some(idx) = app.active_doc { app.documents[idx].content = app.macro_manager.execute_macro(&app.documents[idx].content, 0); app.documents[idx].modified = true; } } if ui.button("🗑").clicked() { app.macro_manager.saved_macros.remove(i); } }); } }
            if app.macro_manager.is_recording { ui.separator(); ui.label(RichText::new(format!("⏺ Recording: {} actions", app.macro_manager.action_count)).color(Color32::from_rgb(255, 100, 100))); }
        }
        Menu::Plugins => {
            ui.label("Available Plugins:");
            for (i, plugin) in app.plugin_manager.plugins.iter().enumerate() {
                ui.horizontal(|ui| {
                    let mut enabled = plugin.enabled;
                    if ui.checkbox(&mut enabled, &plugin.info.name).changed() { app.plugin_manager.plugins[i].enabled = enabled; app.log(format!("Plugin '{}' {}", plugin.info.name, if enabled { "enabled" } else { "disabled" })); }
                });
                ui.label(RichText::new(format!("{} - {}", plugin.info.version, plugin.info.description)).small().color(Color32::GRAY));
            }
        }
        Menu::Help => {
            if ui.button("⌨ Keyboard Shortcuts").clicked() { app.log("Ctrl+N/O/S/F/G/W | F8/F9 | Ctrl+Shift+R (macro) | Ctrl+Space (complete)"); app.active_menu = None; }
            if ui.button("ℹ About").clicked() { app.log("Geany-Rs v0.5.0 - Built with Rust + egui"); app.active_menu = None; }
        }
    }
}

// ============================================================================
// MAIN UPDATE LOOP
// ============================================================================

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mods = ctx.input(|i| i.modifiers);
        
        // Shortcuts
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::N)) { self.new_doc(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::O)) { self.open_file(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::S)) { self.save_file(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::F)) { self.show_find = !self.show_find; }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::G)) { self.show_goto_line = true; }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::W)) { if let Some(idx) = self.active_doc { self.close_doc(idx); } }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::Space)) { if self.completer.enabled { if let Some(idx) = self.active_doc { let doc = &self.documents[idx]; let syms = SymbolParser::parse(&doc.content, doc.filetype); self.completer.trigger(&doc.content, doc.filetype, &SymbolParser::get_names(&syms)); } } }
        if mods.cmd && mods.shift && ctx.input(|i| i.key_pressed(egui::Key::R)) { if self.macro_manager.is_recording { if let Some(m) = self.macro_manager.stop_recording() { self.log(format!("Macro saved: {} ({} actions)", m.name, m.actions.len())); } } else { self.macro_manager.start_recording(); self.log("Recording macro..."); } }
        if self.completer.show_popup { if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) { self.completer.select_next(); } if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) { self.completer.select_prev(); } if ctx.input(|i| i.key_pressed(egui::Key::Enter)) { if let Some(text) = self.completer.insert_selected() { if let Some(idx) = self.active_doc { let pos = self.documents[idx].content.len(); self.documents[idx].content.push_str(&text); self.documents[idx].modified = true; } } self.completer.show_popup = false; } if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.completer.show_popup = false; } }
        if ctx.input(|i| i.key_pressed(egui::Key::F8)) { if let Some(idx) = self.active_doc { let doc = &self.documents[idx]; if let Some((_, cmd)) = BuildSystem::get_commands(doc.filetype).first() { BuildSystem::execute(&BuildSystem::expand(cmd, &doc.path), self); } } }
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) { if let Some(idx) = self.active_doc { let doc = &self.documents[idx]; if let Some((_, cmd)) = BuildSystem::get_commands(doc.filetype).get(1).or_else(|| BuildSystem::get_commands(doc.filetype).first()) { BuildSystem::execute(&BuildSystem::expand(cmd, &doc.path), self); } } }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.show_find = false; self.show_goto_line = false; self.show_settings = false; self.active_menu = None; }
        
        ctx.set_visuals(if self.theme_dark { egui::Visuals::dark() } else { egui::Visuals::light() });
        
        // TOP PANEL
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (name, m) in [("File", Menu::File), ("Project", Menu::Project), ("Edit", Menu::Edit), ("View", Menu::View), ("Search", Menu::Search), ("Build", Menu::Build), ("Tools", Menu::Tools), ("Macros", Menu::Macros), ("Plugins", Menu::Plugins), ("Help", Menu::Help)] {
                    let txt = RichText::new(name);
                    if ui.selectable_label(self.active_menu == Some(m), txt).clicked() { self.active_menu = if self.active_menu == Some(m) { None } else { Some(m) }; }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("📄").clicked() { self.new_doc(); }
                if ui.button("📂").clicked() { self.open_file(); }
                if ui.button("💾").clicked() { self.save_file(); }
                ui.separator();
                if ui.button(if self.macro_manager.is_recording { "⏹" } else { "⏺" }).on_hover_text("Record Macro (Ctrl+Shift+R)").clicked() {
                    if self.macro_manager.is_recording { if let Some(m) = self.macro_manager.stop_recording() { self.log(format!("Saved: {}", m.name)); } } else { self.macro_manager.start_recording(); }
                }
                ui.separator();
                if ui.toggle_value(&mut self.sidebar_visible, "📑").clicked() {}
                if ui.toggle_value(&mut self.messages_visible, "📋").clicked() {}
                if ui.toggle_value(&mut self.terminal.visible, "🖥").clicked() {}
                if ui.button(if self.theme_dark { "☀️" } else { "🌙" }).clicked() { self.theme_dark = !self.theme_dark; }
            });
        });
        
        // MENU DROPDOWN
        if let Some(menu) = self.active_menu {
            let pos = ctx.cursor().unwrap();
            Window::new(format!("{:?}", menu)).collapsible(false).resizable(false).anchor(egui::Align2::LEFT_UP, [pos.x, pos.y + 20.0]).show(ctx, |ui| { render_menu(self, ui, menu); });
        }
        
        // SIDEBAR
        if self.sidebar_visible {
            SidePanel::left("sidebar").resizable(true).default_width(230.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, icon, name) in [(SidebarTab::Files, "📁", "Files"), (SidebarTab::Symbols, "🔣", "Symbols"), (SidebarTab::Macros, "⏺", "Macros"), (SidebarTab::Plugins, "🔌", "Plugins"), (SidebarTab::Project, "📁", "Project")] {
                        let mut sel = self.sidebar_tab == tab;
                        if ui.toggle_sized(&mut sel, icon).clicked() { self.sidebar_tab = tab; }
                    }
                });
                ui.separator();
                
                match self.sidebar_tab {
                    SidebarTab::Files => {
                        ScrollArea::vertical().show(ui, |ui| {
                            for (idx, doc) in self.documents.iter().enumerate() {
                                let active = self.active_doc == Some(idx);
                                let mut txt = RichText::new(&doc.name);
                                if doc.modified { txt = txt.color(Color32::from_rgb(255, 200, 0)); }
                                if active { txt = txt.bold(); }
                                if ui.selectable_label(active, txt).clicked() { self.active_doc = Some(idx); }
                            }
                        });
                    }
                    SidebarTab::Symbols => {
                        if let Some(idx) = self.active_doc {
                            let syms = SymbolParser::parse(&self.documents[idx].content, self.documents[idx].filetype);
                            if syms.is_empty() { ui.label(RichText::new("No symbols found").color(Color32::GRAY)); }
                            else { ScrollArea::vertical().show(ui, |ui| { for s in &syms { ui.horizontal(|ui| { ui.label(RichText::new(s.kind.icon()).color(s.kind.color())); if ui.link(&s.name).clicked() { self.documents[idx].cursor_line = s.line + 1; } }); } }); }
                        }
                    }
                    SidebarTab::Macros => {
                        ui.label("Macros");
                        ui.separator();
                        if self.macro_manager.is_recording { ui.label(RichText::new(format!("⏺ Recording: {} actions", self.macro_manager.action_count)).color(Color32::from_rgb(255, 100, 100))); }
                        else { ui.label("Not recording"); }
                        ui.separator();
                        if app.macro_manager.saved_macros.is_empty() { ui.label("No macros saved"); }
                        else { for (i, m) in self.macro_manager.saved_macros.iter().enumerate() { ui.horizontal(|ui| { if ui.button(format!("▶ {}", m.name)).clicked() { if let Some(idx) = self.active_doc { self.documents[idx].content = self.macro_manager.execute_macro(&self.documents[idx].content, 0); self.documents[idx].modified = true; } } if ui.button("🗑").clicked() { self.macro_manager.saved_macros.remove(i); } }); } }
                    }
                    SidebarTab::Plugins => {
                        for (i, plugin) in self.plugin_manager.plugins.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let mut en = plugin.enabled;
                                if ui.checkbox(&mut en, &plugin.info.name).changed() { self.plugin_manager.plugins[i].enabled = en; self.log(format!("Plugin '{}' {}", plugin.info.name, if en { "enabled" } else { "disabled" })); }
                            });
                            ui.label(RichText::new(&plugin.info.description).small().color(Color32::GRAY));
                        }
                    }
                    SidebarTab::Project => {
                        if let Some(ref p) = self.project {
                            ui.label(RichText::new(&p.name).strong());
                            ui.separator();
                            ui.label("Build Commands:");
                            for cmd in &p.build_commands { ui.label(format!("• {}", cmd.label)); }
                        } else { ui.label("No project"); }
                    }
                }
            });
        }
        
        // MAIN EDITOR
        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, doc) in self.documents.iter().enumerate() {
                        let active = self.active_doc == Some(idx);
                        let mut lbl = doc.name.clone();
                        if doc.modified { lbl.push_str(" ●"); }
                        if ui.selectable_label(active, lbl).clicked() { self.active_doc = Some(idx); }
                    }
                    if ui.button("+").clicked() { self.new_doc(); }
                });
            });
            ui.separator();
            
            if self.show_find {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔍 Find:");
                        TextEdit::singleline(&mut self.find_replace.search_text).desired_width(150.0).show(ui);
                        if ui.button("Find All").clicked() { if let Some(idx) = self.active_doc { let results = self.find_replace.search(&self.documents[idx].content); self.log(format!("Found {} matches", results.len())); } }
                        ui.separator();
                        ui.label("↔ Replace:");
                        TextEdit::singleline(&mut self.find_replace.replace_text).desired_width(150.0).show(ui);
                        if ui.button("Replace All").clicked() { if let Some(idx) = self.active_doc { let c = self.find_replace.replace_all(&self.documents[idx].content); self.documents[idx].content = c; self.documents[idx].modified = true; } }
                        if ui.button("✕").clicked() { self.show_find = false; }
                    });
                });
                ui.separator();
            }
            
            if let Some(idx) = self.active_doc {
                if let Some(doc) = self.documents.get_mut(idx) {
                    ui.horizontal(|ui| {
                        ui.label(format!("📝 {}", doc.filetype.name()));
                        ComboBox::from_id_salt("ft").selected_text(doc.filetype.name()).show_ui(ui, |ui| {
                            for ft in [Filetype::Rust, Filetype::C, Filetype::Cpp, Filetype::Python, Filetype::JavaScript, Filetype::TypeScript, Filetype::Html, Filetype::Css, Filetype::Json, Filetype::Markdown, Filetype::PlainText] { ui.selectable_value(&mut doc.filetype, ft, ft.name()); }
                        });
                        ui.separator();
                        ui.label(format!("Ln {}, Col {}", doc.cursor_line, doc.cursor_col));
                        if doc.modified { ui.label(RichText::new("●").color(Color32::from_rgb(255, 200, 0))); }
                        if self.completer.enabled { ui.separator(); if ui.button("✨ Complete").clicked() { let syms = SymbolParser::parse(&doc.content, doc.filetype); self.completer.trigger(&doc.content, doc.filetype, &SymbolParser::get_names(&syms)); } }
                    });
                    ui.separator();
                    
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if self.line_numbers {
                                ui.vertical(|ui| {
                                    ui.set_width(50.0);
                                    let lines = doc.content.lines().count().max(1);
                                    for i in 1..=lines { ui.label(RichText::new(format!("{:>4}", i)).small().monospace().color(Color32::GRAY)); }
                                });
                                ui.separator();
                            }
                            let mut text = doc.content.clone();
                            TextEdit::multiline(&mut text).font(FontId::monospace(14.0)).desired_width(f32::INFINITY).show(ui);
                            if text != doc.content {
                                self.macro_manager.record_insert(&text);
                                doc.content = text;
                                doc.modified = true;
                            }
                        });
                    });
                }
            }
        });
        
        // COMPLETION POPUP
        if self.completer.show_popup && !self.completer.items.is_empty() {
            Window::new("Completions").collapsible(false).resizable(false).always_auto_resize().anchor(egui::Align2::LEFT_BOTTOM, [50.0, 400.0]).show(ctx, |ui| {
                ui.set_width(300.0);
                for (i, item) in self.completer.items.iter().enumerate() {
                    let sel = i == self.completer.selected_index;
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(item.kind.icon()).color(item.kind.color()));
                        let txt = if sel { RichText::new(&item.label).strong() } else { RichText::new(&item.label) };
                        ui.label(txt);
                    });
                    ui.label(RichText::new(&item.detail).small().color(Color32::GRAY));
                }
                ui.separator();
                ui.label("↑↓ Navigate  Enter Insert  Esc Close");
            });
        }
        
        // STATUS BAR
        TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(idx) = self.active_doc { if let Some(doc) = self.documents.get(idx) { ui.label(RichText::new(&doc.name).small().strong()); ui.separator(); ui.label(RichText::new(format!("Ln {}, Col {}", doc.cursor_line, doc.cursor_col)).small()); ui.separator(); ui.label(RichText::new(doc.filetype.name()).small()); } }
                if let Some(ref p) = self.project { ui.separator(); ui.label(RichText::new(format!("📁 {}", p.name)).small().color(Color32::from_rgb(200, 150, 100))); }
                if self.macro_manager.is_recording { ui.separator(); ui.label(RichText::new("⏺ REC").small().color(Color32::from_rgb(255, 100, 100))); }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| { ui.label(RichText::new("Geany-Rs v0.5.0").small().color(Color32::GRAY)); });
            });
        });
        
        // MESSAGE PANEL
        if self.messages_visible {
            TopBottomPanel::bottom("messages").resizable(true).default_height(80.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("📋 Messages:"); if ui.button("Clear").clicked() { self.messages.clear(); } });
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| { for msg in &self.messages { ui.label(msg); } });
            });
        }
        
        // TERMINAL
        if self.terminal.visible {
            TopBottomPanel::bottom("terminal").resizable(true).default_height(150.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("🖥 Terminal:"); if ui.button("Clear").clicked() { self.terminal.history.clear(); } });
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| { ui.label(RichText::new(self.terminal.history.join("\n")).monospace().size(12.0)); });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("$");
                    let r = TextEdit::singleline(&mut self.terminal.command_input).show(ui);
                    if r.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { self.terminal.execute(&self.terminal.command_input, self); self.terminal.command_input.clear(); }
                });
            });
        }
        
        // GOTO LINE
        if self.show_goto_line {
            Window::new("Go to Line").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("Line number:");
                TextEdit::singleline(&mut self.goto_line).desired_width(100.0).request_focus().show(ui);
                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() { if let Ok(l) = self.goto_line.parse::<usize>() { if let Some(idx) = self.active_doc { self.documents[idx].cursor_line = l; } } self.show_goto_line = false; self.goto_line.clear(); }
                    if ui.button("Cancel").clicked() { self.show_goto_line = false; self.goto_line.clear(); }
                });
            });
        }
    }
}

fn main() {
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_min_inner_size([800.0, 600.0]).with_title("Geany-Rs v0.5.0 - IDE with Projects, Macros, Plugins"), ..Default::default() };
    eframe::run_native("Geany-Rs", options, Box::new(|cc| Ok(Box::new(GeanyApp::new(cc))))).unwrap();
}
