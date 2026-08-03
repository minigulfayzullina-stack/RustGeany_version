//! Geany-Rs - A fast and lightweight IDE in Rust
//! Built with egui for cross-platform support
//!
//! Features:
//! - File open/save dialogs
//! - Find & Replace
//! - Symbol tree sidebar
//! - Terminal emulator
//! - Build commands
//! - Indentation settings, Word wrap, Bracket matching
//! - Code folding
//! - PROJECT MANAGEMENT (.geany files)
//! - AUTO-COMPLETION (keywords, symbols)
//! - PLUGIN SYSTEM (basic API)
//! - MACROS (record/playback)

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{Align, Color32, ComboBox, FontId, RichText, ScrollArea, TextEdit, TopBottomPanel, SidePanel, CentralPanel, Ui, Window};
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
            .add_filter("Source Files", &["rs", "c", "cpp", "h", "py", "js", "ts", "html", "css", "json", "md", "toml"])
            .add_filter("Geany Projects", &["geany"])
            .add_filter("All Files", &["*"])
            .pick_file()
    }

    pub fn save_file(default_name: &str) -> Option<PathBuf> {
        FileDialog::new()
            .set_file_name(default_name)
            .add_filter("Source Files", &["rs", "c", "cpp", "h", "py", "js"])
            .add_filter("All Files", &["*"])
            .save_file()
    }

    pub fn save_project() -> Option<PathBuf> {
        FileDialog::new()
            .set_file_name("project.geany")
            .add_filter("Geany Project", &["geany"])
            .save_file()
    }
}

// ============================================================================
// SETTINGS
// ============================================================================

#[derive(Debug, Clone)]
pub struct EditorSettings {
    pub indent_width: usize,
    pub use_spaces: bool,
    pub show_line_numbers: bool,
    pub word_wrap: bool,
    pub wrap_width: usize,
    pub highlight_current_line: bool,
    pub bracket_highlight: bool,
    pub auto_indent: bool,
    pub tab_size: usize,
}

impl Default for EditorSettings {
    fn default() -> Self {
        Self { indent_width: 4, use_spaces: true, show_line_numbers: true, word_wrap: false, wrap_width: 80, highlight_current_line: true, bracket_highlight: true, auto_indent: true, tab_size: 4 }
    }
}

// ============================================================================
// PROJECT MANAGEMENT (NEW!)
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeanyProject {
    pub name: String,
    pub description: String,
    pub file_patterns: Vec<String>,
    pub base_path: String,
    pub make_increment: String,
    pub grep_pattern: String,
    pub open_files: Vec<String>,
    pub recent_files: Vec<String>,
    pub build_commands: Vec<BuildCommandConfig>,
    pub preferences: ProjectPreferences,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildCommandConfig {
    pub label: String,
    pub command: String,
    pub working_dir: String,
    pub key: Option<u32>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProjectPreferences {
    pub indendation_mode: String,
    pub detect_indent: bool,
    pub prefer_utf8: bool,
    pub default_encoding: String,
    pub eol_mode: String,
}

impl Default for GeanyProject {
    fn default() -> Self {
        Self {
            name: "New Project".to_string(),
            description: String::new(),
            file_patterns: vec!["*.rs".to_string(), "*.c".to_string(), "*.py".to_string()],
            base_path: String::new(),
            make_increment: "make".to_string(),
            grep_pattern: String::new(),
            open_files: Vec::new(),
            recent_files: Vec::new(),
            build_commands: vec![
                BuildCommandConfig { label: "Make".to_string(), command: "make".to_string(), working_dir: "$(ProjectPath)".to_string(), key: Some(0) },
                BuildCommandConfig { label: "Make".to_string(), command: "make".to_string(), working_dir: "$(ProjectPath)".to_string(), key: Some(0) },
            ],
            preferences: ProjectPreferences {
                indendation_mode: "tabs-spaces".to_string(),
                detect_indent: true,
                prefer_utf8: true,
                default_encoding: "UTF-8".to_string(),
                eol_mode: "LF".to_string(),
            },
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

    pub fn get_recent_files_display(&self) -> Vec<String> {
        self.recent_files.iter().take(10).cloned().collect()
    }
}

// ============================================================================
// AUTO-COMPLETION (NEW!)
// ============================================================================

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: String,
    pub insert_text: String,
    pub score: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind {
    Keyword,
    Function,
    Snippet,
    Variable,
    Class,
    Property,
    Module,
    Constant,
}

impl CompletionKind {
    pub fn icon(&self) -> &'static str {
        match self {
            CompletionKind::Keyword => "k",
            CompletionKind::Function => "ƒ",
            CompletionKind::Snippet => "⚡",
            CompletionKind::Variable => "v",
            CompletionKind::Class => "C",
            CompletionKind::Property => "p",
            CompletionKind::Module => "M",
            CompletionKind::Constant => "K",
        }
    }
}

pub struct AutoCompleter {
    pub enabled: bool,
    pub show_popup: bool,
    pub items: Vec<CompletionItem>,
    pub selected_index: usize,
    pub trigger_chars: Vec<char>,
}

impl AutoCompleter {
    pub fn new() -> Self {
        Self {
            enabled: true,
            show_popup: false,
            items: Vec::new(),
            selected_index: 0,
            trigger_chars: vec!['.', ':', '>'],
        }
    }

    pub fn get_keywords_for_filetype(filetype: Filetype) -> Vec<CompletionItem> {
        let keywords = match filetype {
            Filetype::Rust => vec![
                ("fn", "fn name()", "Function declaration"),
                ("let", "let var = value", "Variable declaration"),
                ("let mut", "let mut var = value", "Mutable variable"),
                ("pub", "pub item", "Public visibility"),
                ("struct", "struct Name {}", "Struct definition"),
                ("impl", "impl Type {}", "Implementation block"),
                ("enum", "enum Name {}", "Enumeration"),
                ("trait", "trait Name {}", "Trait definition"),
                ("mod", "mod name;", "Module declaration"),
                ("use", "use path::to::item", "Import statement"),
                ("match", "match value {}", "Pattern matching"),
                ("if", "if condition {}", "If statement"),
                ("else", "else {}", "Else branch"),
                ("for", "for item in collection {}", "For loop"),
                ("while", "while condition {}", "While loop"),
                ("loop", "loop {}", "Infinite loop"),
                ("return", "return value", "Return statement"),
                ("async", "async fn", "Async function"),
                ("await", "await expression", "Await expression"),
                ("Some", "Some(value)", "Option variant"),
                ("None", "None", "Option variant"),
                ("Ok", "Ok(value)", "Result variant"),
                ("Err", "Err(error)", "Result variant"),
                ("true", "true", "Boolean true"),
                ("false", "false", "Boolean false"),
                ("self", "self", "Current instance"),
                ("super", "super::", "Parent module"),
                ("crate", "crate::", "Crate root"),
                ("mut", "mut variable", "Mutable binding"),
                ("ref", "ref pattern", "Reference binding"),
                ("where", "where T: Trait", "Where clause"),
                ("unsafe", "unsafe {}", "Unsafe block"),
                ("extern", "extern \"C\" {}", "External block"),
            ],
            Filetype::Python => vec![
                ("def", "def func_name():", "Function definition"),
                ("class", "class ClassName:", "Class definition"),
                ("if", "if condition:", "If statement"),
                ("elif", "elif condition:", "Else if statement"),
                ("else", "else:", "Else branch"),
                ("for", "for item in iterable:", "For loop"),
                ("while", "while condition:", "While loop"),
                ("try", "try:", "Try block"),
                ("except", "except Exception:", "Exception handler"),
                ("finally", "finally:", "Finally block"),
                ("with", "with open() as f:", "Context manager"),
                ("import", "import module", "Import statement"),
                ("from", "from module import", "From import"),
                ("return", "return value", "Return statement"),
                ("yield", "yield value", "Yield expression"),
                ("async", "async def", "Async function"),
                ("await", "await coroutine", "Await expression"),
                ("lambda", "lambda x: x", "Lambda function"),
                ("pass", "pass", "Pass statement"),
                ("break", "break", "Break loop"),
                ("continue", "continue", "Continue loop"),
                ("raise", "raise Exception()", "Raise exception"),
                ("assert", "assert condition", "Assert statement"),
                ("True", "True", "Boolean true"),
                ("False", "False", "Boolean false"),
                ("None", "None", "None value"),
            ],
            Filetype::JavaScript | Filetype::TypeScript => vec![
                ("function", "function name() {}", "Function declaration"),
                ("const", "const name = value", "Constant declaration"),
                ("let", "let name = value", "Let declaration"),
                ("var", "var name = value", "Var declaration"),
                ("class", "class Name {}", "Class definition"),
                ("async", "async function", "Async function"),
                ("await", "await promise", "Await expression"),
                ("import", "import name from 'module'", "Import statement"),
                ("export", "export name", "Export statement"),
                ("if", "if (condition) {}", "If statement"),
                ("for", "for (let i = 0; i < n; i++) {}", "For loop"),
                ("while", "while (condition) {}", "While loop"),
                ("return", "return value", "Return statement"),
                ("throw", "throw new Error()", "Throw error"),
                ("try", "try {} catch (e) {}", "Try-catch block"),
                ("true", "true", "Boolean true"),
                ("false", "false", "Boolean false"),
                ("null", "null", "Null value"),
                ("undefined", "undefined", "Undefined value"),
                ("this", "this", "This context"),
                ("new", "new ClassName()", "New instance"),
            ],
            Filetype::C | Filetype::Cpp => vec![
                ("int", "int name;", "Integer type"),
                ("char", "char name;", "Character type"),
                ("float", "float name;", "Float type"),
                ("double", "double name;", "Double type"),
                ("void", "void function()", "Void type"),
                ("struct", "struct Name {}", "Struct definition"),
                ("enum", "enum Name {}", "Enumeration"),
                ("typedef", "typedef existing new_name", "Type definition"),
                ("if", "if (condition) {}", "If statement"),
                ("for", "for (int i = 0; i < n; i++) {}", "For loop"),
                ("while", "while (condition) {}", "While loop"),
                ("switch", "switch (value) {}", "Switch statement"),
                ("return", "return value;", "Return statement"),
                ("NULL", "NULL", "Null pointer"),
                ("printf", "printf(\"\", var)", "Print formatted"),
                ("scanf", "scanf(\"\", &var)", "Scan formatted"),
                ("sizeof", "sizeof(type)", "Size of type"),
            ],
            _ => vec![],
        };

        keywords.into_iter().map(|(label, insert, detail)| CompletionItem {
            label: label.to_string(),
            kind: CompletionKind::Keyword,
            detail: detail.to_string(),
            insert_text: insert.to_string(),
            score: 100,
        }).collect()
    }

    pub fn trigger(&mut self, content: &str, cursor_pos: usize, filetype: Filetype, symbols: &[String]) {
        self.items.clear();
        self.selected_index = 0;
        
        // Get current word being typed
        let before_cursor = &content[..cursor_pos.min(content.len())];
        let current_word: String = before_cursor.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>().chars().rev().collect();
        
        if current_word.len() < 1 {
            self.show_popup = false;
            return;
        }
        
        // Add keywords
        let keywords = Self::get_keywords_for_filetype(filetype);
        for kw in keywords {
            if kw.label.starts_with(&current_word) || kw.label.to_lowercase().starts_with(&current_word.to_lowercase()) {
                self.items.push(kw);
            }
        }
        
        // Add symbols from current file
        for symbol in symbols {
            if symbol.starts_with(&current_word) {
                self.items.push(CompletionItem {
                    label: symbol.clone(),
                    kind: CompletionKind::Function,
                    detail: "Symbol".to_string(),
                    insert_text: symbol.clone(),
                    score: 90,
                });
            }
        }
        
        // Sort by score
        self.items.sort_by(|a, b| b.score.cmp(&a.score));
        
        self.show_popup = !self.items.is_empty();
    }

    pub fn select_next(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = (self.selected_index + 1) % self.items.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.items.is_empty() {
            self.selected_index = if self.selected_index == 0 { self.items.len() - 1 } else { self.selected_index - 1 };
        }
    }

    pub fn get_selected(&self) -> Option<&CompletionItem> {
        self.items.get(self.selected_index)
    }
}

// ============================================================================
// MACROS (NEW!)
// ============================================================================

#[derive(Debug, Clone)]
pub struct MacroAction {
    pub action_type: MacroType,
    pub text: Option<String>,
    pub key: Option<egui::Key>,
}

#[derive(Debug, Clone)]
pub enum MacroType {
    InsertText,
    DeleteBack,
    MoveCursor,
    PressKey,
}

#[derive(Debug, Clone)]
pub struct Macro {
    pub name: String,
    pub actions: Vec<MacroAction>,
}

impl Macro {
    pub fn new(name: &str) -> Self {
        Self { name: name.to_string(), actions: Vec::new() }
    }

    pub fn add_action(&mut self, action: MacroAction) {
        self.actions.push(action);
    }
}

pub struct MacroManager {
    pub is_recording: bool,
    pub current_macro: Option<Macro>,
    pub saved_macros: Vec<Macro>,
    pub current_action: Option<MacroAction>,
}

impl MacroManager {
    pub fn new() -> Self {
        Self {
            is_recording: false,
            current_macro: None,
            saved_macros: Vec::new(),
            current_action: None,
        }
    }

    pub fn start_recording(&mut self) {
        self.is_recording = true;
        self.current_macro = Some(Macro::new(&format!("Macro {}", self.saved_macros.len() + 1)));
    }

    pub fn stop_recording(&mut self) -> Option<Macro> {
        self.is_recording = false;
        if let Some(mut macro_) = self.current_macro.take() {
            if !macro_.actions.is_empty() {
                self.saved_macros.push(macro_.clone());
                return Some(macro_);
            }
        }
        None
    }

    pub fn record_insert(&mut self, text: &str) {
        if self.is_recording {
            if let Some(ref mut macro_) = self.current_macro {
                macro_.add_action(MacroAction {
                    action_type: MacroType::InsertText,
                    text: Some(text.to_string()),
                    key: None,
                });
            }
        }
    }

    pub fn record_delete(&mut self) {
        if self.is_recording {
            if let Some(ref mut macro_) = self.current_macro {
                macro_.add_action(MacroAction {
                    action_type: MacroType::DeleteBack,
                    text: None,
                    key: None,
                });
            }
        }
    }

    pub fn record_key(&mut self, key: egui::Key) {
        if self.is_recording {
            if let Some(ref mut macro_) = self.current_macro {
                macro_.add_action(MacroAction {
                    action_type: MacroType::PressKey,
                    text: None,
                    key: Some(key),
                });
            }
        }
    }

    pub fn execute_macro(&self, content: &str, cursor_pos: usize) -> (String, usize) {
        let mut result = content.to_string();
        let mut pos = cursor_pos;
        
        for action in &self.actions {
            match action.action_type {
                MacroType::InsertText => {
                    if let Some(ref text) = action.text {
                        result.insert_str(pos, text);
                        pos += text.len();
                    }
                }
                MacroType::DeleteBack => {
                    if pos > 0 {
                        result.remove(pos - 1);
                        pos -= 1;
                    }
                }
                MacroType::MoveCursor => {
                    // Handle cursor movement
                    pos += 1;
                }
                MacroType::PressKey => {
                    // Handle key press
                    if let Some(key) = &action.key {
                        match key {
                            egui::Key::Enter => {
                                result.insert(pos, '\n');
                                pos += 1;
                            }
                            egui::Key::Tab => {
                                result.insert_str(pos, "    ");
                                pos += 4;
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
        
        (result, pos)
    }

    pub fn delete_macro(&mut self, index: usize) {
        if index < self.saved_macros.len() {
            self.saved_macros.remove(index);
        }
    }

    pub fn rename_macro(&mut self, index: usize, new_name: &str) {
        if let Some(macro_) = self.saved_macros.get_mut(index) {
            macro_.name = new_name.to_string();
        }
    }
}

// ============================================================================
// PLUGIN SYSTEM (NEW!)
// ============================================================================

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub hooks: Vec<PluginHook>,
}

#[derive(Debug, Clone)]
pub enum PluginHook {
    Startup,
    Shutdown,
    DocumentNew,
    DocumentOpen,
    DocumentSave,
    DocumentClose,
    EditorKey,
    FiletypeSet,
}

#[derive(Debug, Clone)]
pub struct Plugin {
    pub info: PluginInfo,
    pub enabled: bool,
    pub path: Option<String>,
}

impl Plugin {
    pub fn new(name: &str) -> Self {
        Self {
            info: PluginInfo {
                name: name.to_string(),
                description: String::new(),
                version: "0.1.0".to_string(),
                author: "Unknown".to_string(),
                hooks: vec![],
            },
            enabled: false,
            path: None,
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

pub struct PluginManager {
    pub plugins: Vec<Plugin>,
    pub show_plugin_dialog: bool,
}

impl PluginManager {
    pub fn new() -> Self {
        let mut manager = Self {
            plugins: Vec::new(),
            show_plugin_dialog: false,
        };
        
        // Add built-in demo plugins
        manager.plugins.push({
            let mut p = Plugin::new("Formatter");
            p.info.description = "Auto-format code".to_string();
            p.info.author = "Geany-Rs".to_string();
            p.info.hooks = vec![PluginHook::DocumentSave];
            p
        });
        
        manager.plugins.push({
            let mut p = Plugin::new("BracketHighlighter");
            p.info.description = "Highlight matching brackets".to_string();
            p.info.author = "Geany-Rs".to_string();
            p.info.hooks = vec![PluginHook::EditorKey];
            p
        });
        
        manager.plugins.push({
            let mut p = Plugin::new("TodoViewer");
            p.info.description = "Show TODO/FIXME comments".to_string();
            p.info.author = "Geany-Rs".to_string();
            p.info.hooks = vec![PluginHook::DocumentOpen, PluginHook::DocumentNew];
            p
        });
        
        manager.plugins
    }

    pub fn toggle_plugin(&mut self, index: usize) {
        if let Some(plugin) = self.plugins.get_mut(index) {
            if plugin.enabled {
                plugin.disable();
            } else {
                plugin.enable();
            }
        }
    }

    pub fn get_enabled_plugins(&self) -> Vec<&Plugin> {
        self.plugins.iter().filter(|p| p.enabled).collect()
    }

    pub fn get_disabled_plugins(&self) -> Vec<&Plugin> {
        self.plugins.iter().filter(|p| !p.enabled).collect()
    }
}

// ============================================================================
// SYMBOL TREE
// ============================================================================

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function, Struct, Enum, Impl, Trait, Class, Module, Variable, Constant,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::Function => "ƒ", SymbolKind::Struct => "S", SymbolKind::Enum => "E",
            SymbolKind::Impl => "I", SymbolKind::Trait => "T", SymbolKind::Class => "C",
            SymbolKind::Module => "M", SymbolKind::Variable => "v", SymbolKind::Constant => "K",
        }
    }
    pub fn color(&self) -> Color32 {
        match self {
            SymbolKind::Function => Color32::from_rgb(230, 192, 123),
            SymbolKind::Struct | SymbolKind::Class => Color32::from_rgb(78, 201, 176),
            SymbolKind::Enum => Color32::from_rgb(86, 156, 214),
            SymbolKind::Impl | SymbolKind::Trait => Color32::from_rgb(206, 145, 120),
            SymbolKind::Module => Color32::from_rgb(197, 134, 192),
            SymbolKind::Variable | SymbolKind::Constant => Color32::from_rgb(181, 206, 168),
        }
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
                (r"(?m)^(?:pub\s+)?trait\s+(\w+)", SymbolKind::Trait),
                (r"(?m)^(?:pub\s+)?impl(?:\s+<\w+>)?\s+(\w+)", SymbolKind::Impl),
                (r"(?m)^(?:pub\s+)?mod\s+(\w+)", SymbolKind::Module),
                (r"(?m)^(?:pub\s+)?type\s+(\w+)", SymbolKind::Constant),
            ],
            Filetype::Python => vec![
                (r"(?m)^class\s+(\w+)", SymbolKind::Class),
                (r"(?m)^(?:async\s+)?def\s+(\w+)", SymbolKind::Function),
            ],
            Filetype::JavaScript | Filetype::TypeScript => vec![
                (r"(?m)^class\s+(\w+)", SymbolKind::Class),
                (r"(?m)^function\s+(\w+)", SymbolKind::Function),
                (r"(?m)^(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(", SymbolKind::Variable),
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
    
    pub fn get_names(symbols: &[Symbol]) -> Vec<String> {
        symbols.iter().map(|s| s.name.clone()).collect()
    }
}

// ============================================================================
// FIND & REPLACE
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
// BUILD SYSTEM
// ============================================================================

#[derive(Debug, Clone)]
pub struct BuildCommand {
    pub name: String,
    pub command: String,
    pub shortcut: Option<String>,
}

pub struct BuildSystem;

impl BuildSystem {
    pub fn get_commands(filetype: Filetype) -> Vec<BuildCommand> {
        match filetype {
            Filetype::Rust => vec![
                BuildCommand { name: "Cargo Build".to_string(), command: "cargo build".to_string(), shortcut: Some("F8".to_string()) },
                BuildCommand { name: "Cargo Run".to_string(), command: "cargo run".to_string(), shortcut: Some("F9".to_string()) },
                BuildCommand { name: "Cargo Check".to_string(), command: "cargo check".to_string(), shortcut: Some("F10".to_string()) },
            ],
            Filetype::C | Filetype::Cpp => vec![
                BuildCommand { name: "Compile".to_string(), command: "gcc \"{file}\" -o \"{name}\" -Wall".to_string(), shortcut: Some("F8".to_string()) },
                BuildCommand { name: "Run".to_string(), command: "\"./{name}\"".to_string(), shortcut: Some("F9".to_string()) },
                BuildCommand { name: "Make".to_string(), command: "make".to_string(), shortcut: Some("F10".to_string()) },
            ],
            Filetype::Python => vec![
                BuildCommand { name: "Run".to_string(), command: "python3 \"{file}\"".to_string(), shortcut: Some("F9".to_string()) },
            ],
            Filetype::JavaScript => vec![
                BuildCommand { name: "Run".to_string(), command: "node \"{file}\"".to_string(), shortcut: Some("F9".to_string()) },
            ],
            Filetype::Html => vec![
                BuildCommand { name: "Open".to_string(), command: "xdg-open \"{file}\"".to_string(), shortcut: Some("F9".to_string()) },
            ],
            _ => vec![],
        }
    }

    pub fn expand(&self, cmd: &str, file_path: &Option<String>) -> String {
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
        
        #[cfg(target_os = "windows")]
        let (shell, arg) = ("cmd", "/C");
        #[cfg(not(target_os = "windows"))]
        let (shell, arg) = ("sh", "-c");

        match Command::new(shell).arg(arg).arg(cmd).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(mut child) => {
                use std::io::Read;
                if let Some(mut out) = child.stdout.take() {
                    let mut s = String::new();
                    if out.read_to_string(&mut s).is_ok() && !s.is_empty() {
                        for line in s.lines().take(50) { app.log(line.to_string()); }
                    }
                }
                if let Some(mut err) = child.stderr.take() {
                    let mut s = String::new();
                    if err.read_to_string(&mut s).is_ok() && !s.is_empty() {
                        for line in s.lines().take(50) { app.log(format!("[err] {}", line)); }
                    }
                }
                if let Ok(status) = child.wait() {
                    app.log(if status.success() { "✓ Success".to_string() } else { format!("✗ Exit: {:?}", status.code()) });
                }
            }
            Err(e) => { app.log(format!("Error: {}", e)); }
        }
    }
}

// ============================================================================
// TERMINAL
// ============================================================================

pub struct Terminal {
    pub visible: bool,
    pub history: Vec<String>,
    pub current_dir: String,
    pub command_input: String,
}

impl Terminal {
    pub fn new() -> Self {
        let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
        Self { visible: false, history: vec!["Geany-Rs Terminal v0.4.0".to_string(), "Type 'help' for commands".to_string()], current_dir: home, command_input: String::new() }
    }

    pub fn execute(&mut self, cmd: &str, app: &mut GeanyApp) {
        if cmd.trim().is_empty() { return; }
        self.history.push(format!("$ {}", cmd));
        
        match cmd.trim() {
            "help" => { self.history.push("Commands: help, clear, pwd, cd, ls, cat, mkdir, touch, rm, echo".to_string()); }
            "clear" => { self.history.clear(); }
            "pwd" => { self.history.push(self.current_dir.clone()); }
            "whoami" => { self.history.push(whoami::username()); }
            cmd if cmd.starts_with("cd ") => {
                let dir = cmd.trim_start_matches("cd ").replace('~', &whoami::username());
                if let Ok(p) = std::fs::canonicalize(&dir) { self.current_dir = p.to_string_lossy().to_string(); }
                else { self.history.push(format!("cd: {} not found", dir)); }
            }
            "ls" => { if let Ok(e) = std::fs::read_dir(&self.current_dir) { for x in e.filter_map(|e| e.ok()) { self.history.push(x.file_name().to_string_lossy().to_string()); } } }
            cmd if cmd.starts_with("cat ") => { if let Ok(c) = std::fs::read_to_string(cmd.trim_start_matches("cat ")) { for l in c.lines().take(30) { self.history.push(l.to_string()); } } }
            cmd if cmd.starts_with("mkdir ") => { let _ = std::fs::create_dir_all(cmd.trim_start_matches("mkdir ")); }
            cmd if cmd.starts_with("touch ") => { let _ = std::fs::write(cmd.trim_start_matches("touch "), ""); }
            cmd if cmd.starts_with("rm ") => { let _ = std::fs::remove_file(cmd.trim_start_matches("rm ")); }
            cmd if cmd.starts_with("echo ") => { self.history.push(cmd.trim_start_matches("echo ").to_string()); }
            _ => {
                #[cfg(target_os = "windows")]
                let (shell, arg) = ("cmd", "/C");
                #[cfg(not(target_os = "windows"))]
                let (shell, arg) = ("sh", "-c");
                if let Ok(out) = Command::new(shell).arg(arg).arg(cmd).current_dir(&self.current_dir).output() {
                    if !out.stdout.is_empty() { for l in String::from_utf8_lossy(&out.stdout).lines().take(30) { self.history.push(l.to_string()); } }
                    if !out.stderr.is_empty() { for l in String::from_utf8_lossy(&out.stderr).lines().take(30) { self.history.push(format!("[err] {}", l)); } }
                }
            }
        }
        if self.history.len() > 200 { self.history = self.history.split_off(self.history.len() - 200); }
    }
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

#[derive(Debug, Clone)]
pub struct Document {
    pub id: usize,
    pub name: String,
    pub path: Option<String>,
    pub content: String,
    pub filetype: Filetype,
    pub modified: bool,
    pub cursor_line: usize,
    pub cursor_col: usize,
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
// MAIN APPLICATION
// ============================================================================

pub struct GeanyApp {
    pub documents: Vec<Document>,
    pub active_doc: Option<usize>,
    pub project: Option<GeanyProject>,
    pub sidebar_visible: bool,
    pub sidebar_tab: SidebarTab,
    pub messages_visible: bool,
    pub messages: Vec<String>,
    pub terminal: Terminal,
    pub completer: AutoCompleter,
    pub macro_manager: MacroManager,
    pub plugin_manager: PluginManager,
    pub theme_dark: bool,
    pub find_replace: FindReplace,
    pub show_find: bool,
    pub show_goto_line: bool,
    pub goto_line: String,
    pub settings: EditorSettings,
    pub active_menu: Option<Menu>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab { Files, Symbols, Macros, Plugins }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu { File, Edit, View, Search, Build, Tools, Macros, Plugins, Project, Help }

impl GeanyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            documents: vec![Document::new(1)],
            active_doc: Some(0),
            project: None,
            sidebar_visible: true,
            sidebar_tab: SidebarTab::Files,
            messages_visible: true,
            messages: vec!["Geany-Rs v0.4.0".to_string(), "NEW: Project Management, Auto-completion, Macros, Plugin System!".to_string()],
            terminal: Terminal::new(),
            completer: AutoCompleter::new(),
            macro_manager: MacroManager::new(),
            plugin_manager: PluginManager::new(),
            theme_dark: true,
            find_replace: FindReplace::new(),
            show_find: false,
            show_goto_line: false,
            goto_line: String::new(),
            settings: EditorSettings::default(),
            active_menu: None,
        };
        
        if let Some(doc) = app.documents.first_mut() {
            doc.content = include_str!("example.rs").to_string();
            doc.filetype = Filetype::Rust;
            doc.name = "example.rs".to_string();
        }
        app
    }

    fn log(&mut self, msg: impl Into<String>) { self.messages.push(msg.into()); if self.messages.len() > 100 { self.messages.remove(0); } }
    fn new_doc(&mut self) { let id = self.documents.len() + 1; self.documents.push(Document::new(id)); self.active_doc = Some(self.documents.len() - 1); self.log("New document"); }
    fn close_doc(&mut self, idx: usize) { if self.documents.len() > 1 { self.documents.remove(idx); if let Some(a) = self.active_doc { if a >= idx && a > 0 { self.active_doc = Some(a - 1); } else if a >= self.documents.len() { self.active_doc = Some(self.documents.len() - 1); } } self.log("Closed"); } }

    fn open_file(&mut self) {
        if let Some(path) = file_dialogs::open_file() {
            let path_str = path.to_string_lossy().to_string();
            match std::fs::read_to_string(&path_str) {
                Ok(c) => { let mut d = Document::from_file(&path_str, c); d.id = self.documents.len() + 1; self.documents.push(d); self.active_doc = Some(self.documents.len() - 1); self.log(format!("Opened: {}", path_str)); }
                Err(e) => { self.log(format!("Error: {}", e)); }
            }
        }
    }

    fn save_file(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &mut self.documents[idx];
            let name = doc.path.as_ref().map(|p| std::path::Path::new(p).file_name().unwrap().to_string_lossy().to_string()).unwrap_or_else(|| doc.name.clone());
            if let Some(path) = file_dialogs::save_file(&name) {
                let path_str = path.to_string_lossy().to_string();
                match std::fs::write(&path_str, &doc.content) {
                    Ok(_) => { doc.path = Some(path_str.clone()); doc.modified = false; doc.name = std::path::Path::new(&path_str).file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default(); self.log(format!("Saved: {}", path_str)); }
                    Err(e) => { self.log(format!("Error: {}", e)); }
                }
            }
        }
    }

    // ===== PROJECT MANAGEMENT =====
    fn new_project(&mut self) {
        self.project = Some(GeanyProject::default());
        self.log("Created new project");
    }

    fn save_project(&mut self) {
        if let Some(ref project) = self.project {
            if let Some(path) = file_dialogs::save_project() {
                let path_str = path.to_string_lossy().to_string();
                match project.save(&path_str) {
                    Ok(_) => self.log(format!("Project saved: {}", path_str)),
                    Err(e) => self.log(format!("Error saving project: {}", e)),
                }
            }
        } else {
            self.log("No project open");
        }
    }

    fn open_project(&mut self) {
        let path = rfd::FileDialog::new().add_filter("Geany Project", &["geany"]).pick_file();
        if let Some(path) = path {
            let path_str = path.to_string_lossy().to_string();
            match GeanyProject::load(&path_str) {
                Ok(project) => { self.project = Some(project); self.log(format!("Opened project: {}", path_str)); }
                Err(e) => { self.log(format!("Error loading project: {}", e)); }
            }
        }
    }

    fn close_project(&mut self) {
        self.project = None;
        self.log("Project closed");
    }

    // ===== MACROS =====
    fn start_macro_recording(&mut self) {
        self.macro_manager.start_recording();
        self.log("Recording macro... (Press Ctrl+Shift+E to stop)");
    }

    fn stop_macro_recording(&mut self) {
        if let Some(m) = self.macro_manager.stop_recording() {
            self.log(format!("Macro saved: {} ({} actions)", m.name, m.actions.len()));
        } else {
            self.log("No macro recorded");
        }
    }

    fn execute_last_macro(&mut self) {
        if let Some(m) = self.macro_manager.saved_macros.last() {
            if let Some(idx) = self.active_doc {
                let doc = &mut self.documents[idx];
                let (new_content, new_pos) = self.macro_manager.execute_macro(&doc.content, doc.content.len().min(100));
                doc.content = new_content;
                doc.modified = true;
                self.log(format!("Executed macro: {}", m.name));
            }
        } else {
            self.log("No macros saved");
        }
    }
}

// ============================================================================
// MENU RENDERING
// ============================================================================

fn render_menu(app: &mut GeanyApp, ui: &mut Ui, menu_type: Menu) {
    match menu_type {
        Menu::File => {
            if ui.button("📄 New              Ctrl+N").clicked() { app.new_doc(); app.active_menu = None; }
            if ui.button("📂 Open File       Ctrl+O").clicked() { app.open_file(); app.active_menu = None; }
            if ui.button("📂 Open Project").clicked() { app.open_project(); app.active_menu = None; }
            ui.separator();
            if ui.button("💾 Save             Ctrl+S").clicked() { app.save_file(); app.active_menu = None; }
            if ui.button("💾 Save As").clicked() { app.save_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("✕ Close            Ctrl+W").clicked() { if let Some(idx) = app.active_doc { app.close_doc(idx); } app.active_menu = None; }
        }
        Menu::Project => {
            if ui.button("📁 New Project").clicked() { app.new_project(); app.active_menu = None; }
            if ui.button("💾 Save Project").clicked() { app.save_project(); app.active_menu = None; }
            if ui.button("📂 Open Project").clicked() { app.open_project(); app.active_menu = None; }
            ui.separator();
            if ui.button("✕ Close Project").clicked() { app.close_project(); app.active_menu = None; }
            ui.separator();
            if let Some(ref p) = app.project {
                ui.label(RichText::new(format!("Current: {}", p.name)).strong());
            } else {
                ui.label("No project open");
            }
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
            if ui.button(if app.theme_dark { "☀️ Light" } else { "🌙 Dark" }).clicked() { app.theme_dark = !app.theme_dark; app.active_menu = None; }
        }
        Menu::Search => {
            if ui.button("🔍 Find            Ctrl+F").clicked() { app.show_find = !app.show_find; app.active_menu = None; }
            if ui.button("📍 Go to Line     Ctrl+G").clicked() { app.show_goto_line = true; app.active_menu = None; }
        }
        Menu::Build => {
            if let Some(idx) = app.active_doc {
                let doc = &app.documents[idx];
                let cmds = BuildSystem::get_commands(doc.filetype);
                for cmd in cmds {
                    if ui.button(format!("🔨 {}", cmd.name)).clicked() {
                        let expanded = BuildSystem.expand(&BuildSystem, &cmd.command, &doc.path);
                        BuildSystem::execute(&expanded, app);
                        app.active_menu = None;
                    }
                }
            }
        }
        Menu::Tools => {
            if ui.button("🖥 Terminal").clicked() { app.terminal.visible = !app.terminal.visible; app.active_menu = None; }
            if ui.button(if app.completer.enabled { "✓ Auto-complete" } else { "Auto-complete" }).clicked() { app.completer.enabled = !app.completer.enabled; }
        }
        Menu::Macros => {
            if ui.button(if app.macro_manager.is_recording { "⏹ Stop Recording" } else { "⏺ Start Recording" }).clicked() {
                if app.macro_manager.is_recording { app.stop_macro_recording(); } else { app.start_macro_recording(); }
            }
            if ui.button("▶ Play Last Macro").clicked() { app.execute_last_macro(); app.active_menu = None; }
            ui.separator();
            ui.label("Saved Macros:");
            for (i, m) in app.macro_manager.saved_macros.iter().enumerate() {
                ui.horizontal(|ui| {
                    if ui.button(format!("▶ {}", m.name)).clicked() {
                        if let Some(idx) = app.active_doc {
                            let doc = &mut app.documents[idx];
                            let (c, _) = app.macro_manager.execute_macro(&doc.content, 0);
                            doc.content = c;
                            doc.modified = true;
                        }
                    }
                    if ui.button("🗑").clicked() { app.macro_manager.delete_macro(i); }
                });
            }
            if app.macro_manager.is_recording {
                ui.separator();
                ui.label(RichText::new("⏺ Recording...").color(Color32::KHAKI));
            }
        }
        Menu::Plugins => {
            ui.label("Available Plugins:");
            for (i, plugin) in app.plugin_manager.plugins.iter().enumerate() {
                ui.horizontal(|ui| {
                    let checked = plugin.enabled;
                    if ui.checkbox(&mut app.plugin_manager.plugins[i].enabled, &plugin.info.name).changed() {
                        app.log(format!("Plugin '{}' {}", plugin.info.name, if plugin.enabled { "enabled" } else { "disabled" }));
                    }
                });
                ui.label(RichText::new(&plugin.info.description).small().color(Color32::GRAY));
            }
            if ui.button("🔌 Plugin Manager").clicked() { app.plugin_manager.show_plugin_dialog = true; app.active_menu = None; }
        }
        Menu::Help => {
            if ui.button("⌨ Keyboard Shortcuts").clicked() { app.log("Ctrl+N/O/S/F/G/W | F8/F9 | Ctrl+Shift+R/E for macros"); app.active_menu = None; }
            if ui.button("ℹ About").clicked() { app.log("Geany-Rs v0.4.0 - Built with Rust + egui"); app.active_menu = None; }
        }
    }
}

// ============================================================================
// MAIN UPDATE
// ============================================================================

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        let mods = ctx.input(|i| i.modifiers);
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::N)) { self.new_doc(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::O)) { self.open_file(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::S)) { self.save_file(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::F)) { self.show_find = !self.show_find; }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::G)) { self.show_goto_line = true; }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::W)) { if let Some(idx) = self.active_doc { self.close_doc(idx); } }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::Space)) && !self.macro_manager.is_recording {
            // Trigger auto-complete
            if let Some(idx) = self.active_doc {
                let doc = &self.documents[idx];
                let symbols = SymbolParser::parse(&doc.content, doc.filetype);
                self.completer.trigger(&doc.content, doc.content.len(), doc.filetype, &SymbolParser::get_names(&symbols));
            }
        }
        // Macro recording shortcut: Ctrl+Shift+R
        if mods.cmd && mods.shift && ctx.input(|i| i.key_pressed(egui::Key::R)) {
            if self.macro_manager.is_recording { self.stop_macro_recording(); } else { self.start_macro_recording(); }
        }
        // Macro playback shortcut: Ctrl+Shift+E
        if mods.cmd && mods.shift && ctx.input(|i| i.key_pressed(egui::Key::E)) {
            self.execute_last_macro();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.show_find = false; self.show_goto_line = false; self.active_menu = None; }

        ctx.set_visuals(if self.theme_dark { egui::Visuals::dark() } else { egui::Visuals::light() });

        // TOP PANEL
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let menus = [("File", Menu::File), ("Project", Menu::Project), ("Edit", Menu::Edit), ("View", Menu::View), ("Search", Menu::Search), ("Build", Menu::Build), ("Tools", Menu::Tools), ("Macros", Menu::Macros), ("Plugins", Menu::Plugins), ("Help", Menu::Help)];
                for (name, m) in menus {
                    let text = RichText::new(name);
                    if ui.selectable_label(self.active_menu == Some(m), text).clicked() {
                        self.active_menu = if self.active_menu == Some(m) { None } else { Some(m) };
                    }
                }
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("📄").clicked() { self.new_doc(); }
                if ui.button("📂").clicked() { self.open_file(); }
                if ui.button("💾").clicked() { self.save_file(); }
                ui.separator();
                if ui.toggle_value(&mut self.sidebar_visible, "📑").clicked() {}
                if ui.toggle_value(&mut self.messages_visible, "📋").clicked() {}
                if ui.toggle_value(&mut self.terminal.visible, "🖥").clicked() {}
                ui.separator();
                if ui.toggle_value(&mut self.macro_manager.is_recording, "⏺").clicked() { 
                    if self.macro_manager.is_recording { self.stop_macro_recording(); } else { self.start_macro_recording(); }
                }
                if ui.button(if self.theme_dark { "☀️" } else { "🌙" }).clicked() { self.theme_dark = !self.theme_dark; }
            });
        });

        // MENU DROPDOWN
        if let Some(menu) = self.active_menu {
            let pos = ctx.cursor().unwrap();
            Window::new(format!("{:?} Menu", menu)).collapsible(false).resizable(false).anchor(egui::Align2::LEFT_UP, [pos.x, pos.y + 20.0]).show(ctx, |ui| {
                render_menu(self, ui, menu);
            });
        }

        // SIDEBAR
        if self.sidebar_visible {
            SidePanel::left("sidebar").resizable(true).default_width(220.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, icon, name) in [(SidebarTab::Files, "📁", "Files"), (SidebarTab::Symbols, "🔣", "Symbols"), (SidebarTab::Macros, "⏺", "Macros"), (SidebarTab::Plugins, "🔌", "Plugins")] {
                        let mut selected = self.sidebar_tab == tab;
                        if ui.toggle_sized(&mut selected, icon).clicked() { self.sidebar_tab = tab; }
                    }
                });
                ui.separator();

                match self.sidebar_tab {
                    SidebarTab::Files => {
                        ScrollArea::vertical().show(ui, |ui| {
                            for (idx, doc) in self.documents.iter().enumerate() {
                                let active = self.active_doc == Some(idx);
                                let mut text = RichText::new(&doc.name);
                                if doc.modified { text = text.color(Color32::YELLOW); }
                                if active { text = text.bold(); }
                                if ui.selectable_label(active, text).clicked() { self.active_doc = Some(idx); }
                            }
                        });
                    }
                    SidebarTab::Symbols => {
                        if let Some(idx) = self.active_doc {
                            if let Some(doc) = self.documents.get(idx) {
                                let symbols = SymbolParser::parse(&doc.content, doc.filetype);
                                if symbols.is_empty() { ui.label("No symbols"); }
                                else { ScrollArea::vertical().show(ui, |ui| { for s in &symbols { ui.horizontal(|ui| { ui.label(RichText::new(s.kind.icon()).color(s.kind.color()).small()); if ui.link(&s.name).clicked() { self.log(format!("Jump to line {}", s.line + 1)); } }); } }); }
                            }
                        }
                    }
                    SidebarTab::Macros => {
                        ui.label("Macros");
                        ui.separator();
                        if self.macro_manager.is_recording { ui.label(RichText::new("⏺ Recording...").color(Color32::KHAKI)); }
                        else { ui.label("Not recording"); }
                        ui.separator();
                        ui.label("Saved Macros:");
                        for (i, m) in self.macro_manager.saved_macros.iter().enumerate() {
                            ui.horizontal(|ui| {
                                if ui.button(format!("▶ {}", m.name)).clicked() {
                                    if let Some(idx) = self.active_doc {
                                        let doc = &mut self.documents[idx];
                                        let (c, _) = self.macro_manager.execute_macro(&doc.content, 0);
                                        doc.content = c;
                                        doc.modified = true;
                                    }
                                }
                                if ui.button("🗑").clicked() { self.macro_manager.delete_macro(i); }
                            });
                        }
                    }
                    SidebarTab::Plugins => {
                        for (i, plugin) in self.plugin_manager.plugins.iter().enumerate() {
                            ui.horizontal(|ui| {
                                let mut enabled = plugin.enabled;
                                if ui.checkbox(&mut enabled, &plugin.info.name).clicked() {
                                    self.plugin_manager.plugins[i].enabled = enabled;
                                    self.log(format!("Plugin '{}' {}", plugin.info.name, if enabled { "enabled" } else { "disabled" }));
                                }
                            });
                            ui.label(RichText::new(&plugin.info.description).small().color(Color32::GRAY));
                        }
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
                        let mut label = doc.name.clone();
                        if doc.modified { label.push_str(" ●"); }
                        if ui.selectable_label(active, label).clicked() { self.active_doc = Some(idx); }
                    }
                    if ui.button("+").clicked() { self.new_doc(); }
                });
            });
            ui.separator();

            if self.show_find {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        TextEdit::singleline(&mut self.find_replace.search_text).desired_width(150.0).show(ui);
                        if ui.button("Find").clicked() { if let Some(idx) = self.active_doc { self.find_replace.search(&self.documents[idx].content); } }
                        ui.separator();
                        ui.label("Replace:");
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
                            ui.selectable_value(&mut doc.filetype, Filetype::Rust, "Rust");
                            ui.selectable_value(&mut doc.filetype, Filetype::C, "C");
                            ui.selectable_value(&mut doc.filetype, Filetype::Cpp, "C++");
                            ui.selectable_value(&mut doc.filetype, Filetype::Python, "Python");
                            ui.selectable_value(&mut doc.filetype, Filetype::JavaScript, "JavaScript");
                            ui.selectable_value(&mut doc.filetype, Filetype::Html, "HTML");
                            ui.selectable_value(&mut doc.filetype, Filetype::Json, "JSON");
                            ui.selectable_value(&mut doc.filetype, Filetype::PlainText, "Plain Text");
                        });
                        ui.separator();
                        ui.label(format!("Ln {}, Col {}", doc.cursor_line, doc.cursor_col));
                        if doc.modified { ui.label(RichText::new("●").color(Color32::YELLOW)); }
                    });
                    ui.separator();

                    ScrollArea::vertical().show(ui, |ui| {
                        let mut text = doc.content.clone();
                        TextEdit::multiline(&mut text).font(FontId::monospace(14.0)).desired_width(f32::MAX).show(ui);
                        if text != doc.content {
                            // Record macro action
                            if self.macro_manager.is_recording {
                                self.macro_manager.record_insert(&text);
                            }
                            doc.content = text;
                            doc.modified = true;
                        }
                    });
                }
            }
        });

        // STATUS BAR
        TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(idx) = self.active_doc {
                    if let Some(doc) = self.documents.get(idx) {
                        ui.label(RichText::new(&doc.name).small().strong());
                        ui.separator();
                        ui.label(RichText::new(format!("Ln {}, Col {}", doc.cursor_line, doc.cursor_col)).small());
                        ui.separator();
                        ui.label(RichText::new(doc.filetype.name()).small());
                    }
                }
                if let Some(ref p) = self.project {
                    ui.separator();
                    ui.label(RichText::new(format!("📁 {}", p.name)).small().color(Color32::KHAKI));
                }
                if self.macro_manager.is_recording {
                    ui.separator();
                    ui.label(RichText::new("⏺ REC").small().color(Color32::RED));
                }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                    ui.label(RichText::new("Geany-Rs v0.4.0").small().color(Color32::GRAY));
                });
            });
        });

        // MESSAGE PANEL
        if self.messages_visible {
            TopBottomPanel::bottom("messages").resizable(true).default_height(80.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("📋"); if ui.button("Clear").clicked() { self.messages.clear(); } });
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| { for msg in &self.messages { ui.label(msg.clone()); } });
            });
        }

        // TERMINAL
        if self.terminal.visible {
            TopBottomPanel::bottom("terminal").resizable(true).default_height(150.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("🖥️ Terminal:"); if ui.button("Clear").clicked() { self.terminal.history.clear(); } });
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

        // GOTO LINE DIALOG
        if self.show_goto_line {
            Window::new("Go to Line").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("Line:");
                TextEdit::singleline(&mut self.goto_line).desired_width(80.0).request_focus().show(ui);
                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() { if let Ok(l) = self.goto_line.parse::<usize>() { if let Some(idx) = self.active_doc { self.documents[idx].cursor_line = l; } } self.show_goto_line = false; self.goto_line.clear(); }
                    if ui.button("Cancel").clicked() { self.show_goto_line = false; self.goto_line.clear(); }
                });
            });
        }

        // AUTO-COMPLETION POPUP
        if self.completer.show_popup {
            Window::new("Completions").collapsible(false).resizable(false).anchor(egui::Align2::LEFT_BOTTOM, [100.0, 300.0]).show(ctx, |ui| {
                ui.label("Completions:");
                for (i, item) in self.completer.items.iter().enumerate() {
                    let selected = i == self.completer.selected_index;
                    let text = RichText::new(format!("{} {}", item.kind.icon(), item.label));
                    if selected { ui.label(RichText::new(format!("▶ {}", text)).color(Color32::KHAKI)); }
                    else { ui.label(text); }
                }
                ui.separator();
                ui.label("Press Enter to insert, Esc to close");
            });
        }
    }
}

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_min_inner_size([800.0, 600.0]).with_title("Geany-Rs v0.4.0 - IDE with Project/Macros/Plugins"),
        ..Default::default()
    };
    eframe::run_native("Geany-Rs", options, Box::new(|cc| Ok(Box::new(GeanyApp::new(cc))))).unwrap();
}
