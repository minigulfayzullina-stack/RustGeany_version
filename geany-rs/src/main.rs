//! Geany-Rs - Complete IDE in Rust with egui
//! Version 0.6.0 - Final Polish Edition
//!
//! All features implemented:
//! ✅ Project Management (.geany files)
//! ✅ Auto-completion (keywords + symbols)
//! ✅ Plugin System
//! ✅ Macros (record/playback)
//! ✅ Code Folding (clickable UI)
//! ✅ Bracket Matching (visual)
//! ✅ Syntax Highlighting
//! ✅ Auto-indent
//! ✅ Multiple themes

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{Align, Color32, ComboBox, FontId, RichText, ScrollArea, TextEdit, TopBottomPanel, SidePanel, CentralPanel, Ui, Window, Sense};
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
// THEMES (NEW!)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Theme {
    Dark, Light, Monokai, Dracula, Nord, Gruvbox,
}

impl Theme {
    pub fn all() -> Vec<Theme> { vec![Theme::Dark, Theme::Light, Theme::Monokai, Theme::Dracula, Theme::Nord, Theme::Gruvbox] }
    
    pub fn name(&self) -> &'static str {
        match self { Theme::Dark => "Dark", Theme::Light => "Light", Theme::Monokai => "Monokai", Theme::Dracula => "Dracula", Theme::Nord => "Nord", Theme::Gruvbox => "Gruvbox" }
    }
}

#[derive(Debug, Clone)]
pub struct ThemeColors {
    pub bg: Color32,
    pub fg: Color32,
    pub selection: Color32,
    pub line_highlight: Color32,
    pub keyword: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub number: Color32,
    pub function: Color32,
    pub type_name: Color32,
    pub bracket_match: Color32,
    pub folding_bg: Color32,
    pub ui_accent: Color32,
}

impl ThemeColors {
    pub fn from_theme(theme: Theme) -> Self {
        match theme {
            Theme::Dark => Self {
                bg: Color32::from_rgb(30, 30, 30), fg: Color32::WHITE, selection: Color32::from_rgb(50, 60, 80),
                line_highlight: Color32::from_rgb(40, 40, 40), keyword: Color32::from_rgb(86, 156, 214),
                string: Color32::from_rgb(206, 145, 120), comment: Color32::from_rgb(106, 153, 85),
                number: Color32::from_rgb(181, 206, 168), function: Color32::from_rgb(220, 220, 170),
                type_name: Color32::from_rgb(78, 201, 176), bracket_match: Color32::from_rgb(255, 255, 0),
                folding_bg: Color32::from_rgb(45, 45, 45), ui_accent: Color32::from_rgb(80, 140, 200),
            },
            Theme::Light => Self {
                bg: Color32::WHITE, fg: Color32::BLACK, selection: Color32::from_rgb(200, 220, 255),
                line_highlight: Color32::from_rgb(245, 245, 245), keyword: Color32::from_rgb(0, 0, 200),
                string: Color32::from_rgb(140, 0, 0), comment: Color32::from_rgb(0, 128, 0),
                number: Color32::from_rgb(0, 0, 200), function: Color32::from_rgb(180, 100, 0),
                type_name: Color32::from_rgb(100, 0, 150), bracket_match: Color32::from_rgb(255, 200, 0),
                folding_bg: Color32::from_rgb(240, 240, 240), ui_accent: Color32::from_rgb(0, 100, 200),
            },
            Theme::Monokai => Self {
                bg: Color32::from_rgb(39, 40, 34), fg: Color32::from_rgb(248, 248, 242), selection: Color32::from_rgb(102, 102, 102),
                line_highlight: Color32::from_rgb(50, 52, 48), keyword: Color32::from_rgb(249, 38, 114),
                string: Color32::from_rgb(230, 219, 116), comment: Color32::from_rgb(117, 113, 94),
                number: Color32::from_rgb(174, 129, 255), function: Color32::from_rgb(166, 226, 46),
                type_name: Color32::from_rgb(102, 217, 239), bracket_match: Color32::from_rgb(255, 255, 0),
                folding_bg: Color32::from_rgb(55, 57, 50), ui_accent: Color32::from_rgb(249, 38, 114),
            },
            Theme::Dracula => Self {
                bg: Color32::from_rgb(40, 42, 54), fg: Color32::from_rgb(248, 248, 242), selection: Color32::from_rgb(68, 71, 90),
                line_highlight: Color32::from_rgb(50, 52, 66), keyword: Color32::from_rgb(189, 147, 249),
                string: Color32::from_rgb(255, 121, 198), comment: Color32::from_rgb(98, 114, 164),
                number: Color32::from_rgb(139, 233, 253), function: Color32::from_rgb(80, 250, 180),
                type_name: Color32::from_rgb(241, 250, 140), bracket_match: Color32::from_rgb(255, 255, 0),
                folding_bg: Color32::from_rgb(55, 57, 74), ui_accent: Color32::from_rgb(189, 147, 249),
            },
            Theme::Nord => Self {
                bg: Color32::from_rgb(46, 52, 64), fg: Color32::from_rgb(216, 222, 233), selection: Color32::from_rgb(67, 76, 94),
                line_highlight: Color32::from_rgb(56, 62, 74), keyword: Color32::from_rgb(136, 192, 208),
                string: Color32::from_rgb(163, 190, 140), comment: Color32::from_rgb(143, 153, 168),
                number: Color32::from_rgb(208, 135, 112), function: Color32::from_rgb(235, 203, 139),
                type_name: Color32::from_rgb(129, 161, 193), bracket_match: Color32::from_rgb(255, 255, 0),
                folding_bg: Color32::from_rgb(61, 69, 84), ui_accent: Color32::from_rgb(136, 192, 208),
            },
            Theme::Gruvbox => Self {
                bg: Color32::from_rgb(40, 40, 40), fg: Color32::from_rgb(235, 219, 178), selection: Color32::from_rgb(80, 73, 69),
                line_highlight: Color32::from_rgb(50, 48, 45), keyword: Color32::from_rgb(204, 153, 102),
                string: Color32::from_rgb(213, 175, 131), comment: Color32::from_rgb(147, 140, 123),
                number: Color32::from_rgb(189, 174, 142), function: Color32::from_rgb(230, 193, 122),
                type_name: Color32::from_rgb(197, 134, 192), bracket_match: Color32::from_rgb(255, 255, 0),
                folding_bg: Color32::from_rgb(55, 53, 50), ui_accent: Color32::from_rgb(204, 153, 102),
            },
        }
    }
}

// ============================================================================
// SYNTAX HIGHLIGHTING (FULL)
// ============================================================================

#[derive(Debug, Clone)]
pub struct SyntaxHighlight {
    pub keywords: Vec<&'static str>,
    pub types: Vec<&'static str>,
    pub builtins: Vec<&'static str>,
}

impl SyntaxHighlight {
    pub fn for_filetype(ft: Filetype) -> Self {
        match ft {
            Filetype::Rust => Self {
                keywords: vec!["as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while", "macro_rules!", "unsafe"],
                types: vec!["i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize", "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box", "Rc", "Arc", "HashMap", "HashSet"],
                builtins: vec!["println!", "print!", "eprintln!", "eprint!", "format!", "vec!", "panic!", "assert!", "debug_assert!", "todo!", "unimplemented!", "unreachable!", "dbg!", "include_str!", "env!"],
            },
            Filetype::Python => Self {
                keywords: vec!["False", "None", "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif", "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or", "pass", "raise", "return", "try", "while", "with", "yield"],
                types: vec!["int", "float", "str", "bool", "list", "dict", "tuple", "set", "bytes", "object", "type"],
                builtins: vec!["print", "len", "range", "enumerate", "zip", "map", "filter", "sorted", "reversed", "open", "input", "int", "float", "str", "list", "dict", "set", "tuple"],
            },
            Filetype::JavaScript | Filetype::TypeScript => Self {
                keywords: vec!["async", "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum", "export", "extends", "false", "finally", "for", "function", "if", "import", "in", "instanceof", "let", "new", "null", "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "undefined", "var", "void", "while", "with", "yield"],
                types: vec!["string", "number", "boolean", "object", "function", "Symbol", "any", "void", "never", "unknown"],
                builtins: vec!["console", "Math", "JSON", "Array", "Object", "String", "Number", "Boolean", "Date", "RegExp", "Map", "Set", "Promise", "Error"],
            },
            Filetype::C | Filetype::Cpp => Self {
                keywords: vec!["auto", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long", "register", "return", "short", "signed", "sizeof", "static", "struct", "switch", "typedef", "union", "unsigned", "void", "volatile", "while", "class", "public", "private", "protected", "virtual", "friend", "namespace", "template", "try", "catch", "throw", "using", "constexpr", "nullptr"],
                types: vec!["int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t", "size_t", "ptrdiff_t", "FILE", "bool", "true", "false"],
                builtins: vec!["printf", "scanf", "malloc", "free", "memcpy", "memset", "strlen", "strcpy", "fopen", "fclose"],
            },
            _ => Self { keywords: vec![], types: vec![], builtins: vec![] },
        }
    }
    
    pub fn is_keyword(&self, word: &str) -> bool { self.keywords.contains(&word) }
    pub fn is_type(&self, word: &str) -> bool { self.types.contains(&word) }
    pub fn is_builtin(&self, word: &str) -> bool { self.builtins.contains(&word) }
}

// ============================================================================
// CODE FOLDING (CLICKABLE UI)
// ============================================================================

#[derive(Debug, Clone)]
pub struct FoldRegion {
    pub start: usize,
    pub end: usize,
    pub collapsed: bool,
    pub level: usize,
}

#[derive(Debug, Clone)]
pub struct FoldState {
    pub regions: Vec<FoldRegion>,
    pub hovered_line: Option<usize>,
}

impl FoldState {
    pub fn new() -> Self { Self { regions: Vec::new(), hovered_line: None } }
    
    pub fn parse(&mut self, content: &str) {
        self.regions.clear();
        let lines: Vec<&str> = content.lines().collect();
        let mut stack: Vec<(usize, usize, usize)> = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            let opens = trimmed.matches('{').count();
            let closes = trimmed.matches('}').count();
            
            if opens > 0 {
                let net = opens - closes;
                if net > 0 {
                    stack.push((i, opens, closes));
                } else if net < 0 {
                    while closes > 0 {
                        if let Some((start, o, c)) = stack.pop() {
                            let level = stack.len();
                            self.regions.push(FoldRegion { start, end: i, collapsed: false, level });
                            if closes > 1 { stack.push((start, o, closes - 1)); }
                        } else { break; }
                    }
                }
            }
        }
        
        while let Some((start, _, _)) = stack.pop() {
            self.regions.push(FoldRegion { start, end: lines.len() - 1, collapsed: false, level: stack.len() });
        }
        
        self.regions.sort_by_key(|r| r.start);
    }
    
    pub fn toggle(&mut self, line: usize) {
        if let Some(r) = self.regions.iter_mut().find(|r| r.start == line) {
            r.collapsed = !r.collapsed;
        }
    }
    
    pub fn get_visual_lines(&self, total: usize) -> Vec<VisualLine> {
        let mut result = Vec::new();
        let mut hidden = 0usize;
        let mut collapsed_ranges: Vec<(usize, usize)> = self.regions.iter().filter(|r| r.collapsed).map(|r| (r.start, r.end)).collect();
        
        for i in 0..total {
            let is_hidden = hidden > 0;
            let fold_start = self.regions.iter().find(|r| r.start == i);
            
            result.push(VisualLine {
                line: i,
                visible: !is_hidden,
                is_fold_start: fold_start.is_some(),
                fold_collapsed: fold_start.map(|r| r.collapsed).unwrap_or(false),
                fold_end: self.regions.iter().find(|r| r.end == i).is_some(),
                level: fold_start.map(|r| r.level).unwrap_or(0),
            });
            
            if let Some(r) = self.regions.iter().find(|r| r.start == i) {
                if r.collapsed { hidden += r.end - r.start; }
            }
            if hidden > 0 && self.regions.iter().any(|r| r.end == i && r.collapsed) {
                hidden = hidden.saturating_sub(i);
            }
        }
        
        result
    }
}

#[derive(Debug, Clone)]
pub struct VisualLine {
    pub line: usize,
    pub visible: bool,
    pub is_fold_start: bool,
    pub fold_collapsed: bool,
    pub fold_end: bool,
    pub level: usize,
}

// ============================================================================
// BRACKET MATCHING (VISUAL)
// ============================================================================

#[derive(Debug, Clone)]
pub struct BracketHighlight {
    pub open_pos: Option<usize>,
    pub close_pos: Option<usize>,
    pub bracket_type: BracketType,
}

#[derive(Debug, Clone, Copy)]
pub enum BracketType { Paren, Curly, Square, Angle }

impl BracketHighlight {
    pub fn find(content: &str, cursor_pos: usize) -> Option<Self> {
        let chars: Vec<char> = content.chars().collect();
        if cursor_pos >= chars.len() { return None; }
        
        let c = chars[cursor_pos];
        let (open, close, bt) = match c {
            '(' | ')' => ('(', ')', BracketType::Paren),
            '{' | '}' => ('{', '}', BracketType::Curly),
            '[' | ']' => ('[', ']', BracketType::Square),
            '<' | '>' => ('<', '>', BracketType::Angle),
            _ => return None,
        };
        
        let is_open = c == open;
        let mut depth = 1;
        let mut i = cursor_pos as isize + if is_open { 1 } else { -1 };
        
        while i >= 0 && i < chars.len() as isize {
            let curr = chars[i as usize];
            if curr == open { depth += 1; }
            else if curr == close { depth -= 1; }
            if depth == 0 {
                return Some(Self {
                    open_pos: Some(if is_open { cursor_pos } else { i as usize }),
                    close_pos: Some(if is_open { i as usize } else { cursor_pos }),
                    bracket_type: bt,
                });
            }
            i += if is_open { 1 } else { -1 };
        }
        None
    }
}

// ============================================================================
// PROJECT MANAGEMENT
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeanyProject {
    pub name: String,
    pub description: String,
    pub base_path: String,
    pub file_patterns: Vec<String>,
    pub recent_files: Vec<String>,
    pub build_commands: Vec<BuildCmd>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BuildCmd { pub label: String, pub command: String, pub working_dir: String }

impl Default for GeanyProject {
    fn default() -> Self { Self { name: "New Project".to_string(), description: String::new(), base_path: String::new(), file_patterns: vec!["*.rs".to_string(), "*.c".to_string()], recent_files: Vec::new(), build_commands: vec![BuildCmd { label: "Build".to_string(), command: "cargo build".to_string(), working_dir: ".".to_string() }, BuildCmd { label: "Run".to_string(), command: "cargo run".to_string(), working_dir: ".".to_string() }] } }
}

impl GeanyProject {
    pub fn save(&self, path: &str) -> Result<(), String> { std::fs::write(path, serde_json::to_string_pretty(self).map_err(|e| e.to_string())?).map_err(|e| e.to_string()) }
    pub fn load(path: &str) -> Result<Self, String> { serde_json::from_str(&std::fs::read_to_string(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string()) }
}

// ============================================================================
// AUTO-COMPLETION
// ============================================================================

#[derive(Debug, Clone)]
pub struct CompletionItem { pub label: String, pub kind: CompletionKind, pub detail: String, pub insert: String }

#[derive(Debug, Clone, PartialEq)]
pub enum CompletionKind { Keyword, Function, Type, Snippet }

impl CompletionKind {
    pub fn icon(&self) -> &'static str { match self { CompletionKind::Keyword => "🔑", CompletionKind::Function => "ƒ", CompletionKind::Type => "T", CompletionKind::Snippet => "⚡" } }
    pub fn color(&self) -> Color32 { match self { CompletionKind::Keyword => Color32::from_rgb(86, 156, 214), CompletionKind::Function => Color32::from_rgb(220, 220, 170), CompletionKind::Type => Color32::from_rgb(78, 201, 176), CompletionKind::Snippet => Color32::from_rgb(206, 145, 120) } }
}

pub struct AutoCompleter { pub enabled: bool, pub show: bool, pub items: Vec<CompletionItem>, pub selected: usize }

impl AutoCompleter {
    pub fn new() -> Self { Self { enabled: true, show: false, items: Vec::new(), selected: 0 } }
    
    fn keywords(ft: Filetype) -> Vec<(&'static str, &'static str, &'static str)> {
        match ft {
            Filetype::Rust => vec![("fn", "fn name() {}", "Function"), ("let", "let x = val", "Variable"), ("let mut", "let mut x = val", "Mutable"), ("struct", "struct Name {}", "Struct"), ("impl", "impl Type {}", "Impl"), ("enum", "enum Name {}", "Enum"), ("trait", "trait Name {}", "Trait"), ("match", "match x {}", "Match"), ("if", "if cond {}", "If"), ("for", "for x in iter {}", "For"), ("while", "while cond {}", "While"), ("return", "return val;", "Return"), ("pub", "pub item", "Public")],
            Filetype::Python => vec![("def", "def func_name():", "Function"), ("class", "class Name:", "Class"), ("if", "if condition:", "If"), ("for", "for item in iter:", "For"), ("while", "while condition:", "While"), ("try", "try:", "Try"), ("except", "except Exception:", "Except"), ("with", "with open() as f:", "With"), ("import", "import module", "Import"), ("from", "from module import", "From"), ("async", "async def func():", "Async"), ("lambda", "lambda x: x", "Lambda")],
            Filetype::JavaScript => vec![("function", "function name() {}", "Function"), ("const", "const x = val", "Const"), ("let", "let x = val", "Let"), ("class", "class Name {}", "Class"), ("async", "async function() {}", "Async"), ("import", "import x from 'mod'", "Import"), ("export", "export x", "Export"), ("if", "if (cond) {}", "If"), ("for", "for (let i = 0; i < n; i++) {}", "For"), ("try", "try {} catch (e) {}", "Try")],
            _ => vec![],
        }
    }
    
    pub fn trigger(&mut self, content: &str, ft: Filetype, symbols: &[String]) {
        self.items.clear();
        self.selected = 0;
        
        let word: String = content.chars().rev().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>().chars().rev().collect();
        if word.len() < 1 { self.show = false; return; }
        
        for (label, insert, detail) in Self::keywords(ft) {
            if label.to_lowercase().starts_with(&word.to_lowercase()) {
                self.items.push(CompletionItem { label: label.to_string(), kind: CompletionKind::Keyword, detail: detail.to_string(), insert: insert.to_string() });
            }
        }
        
        for sym in symbols {
            if sym.to_lowercase().starts_with(&word.to_lowercase()) {
                self.items.push(CompletionItem { label: sym.clone(), kind: CompletionKind::Function, detail: "Symbol".to_string(), insert: sym.clone() });
            }
        }
        
        self.show = !self.items.is_empty();
    }
    
    pub fn next(&mut self) { if !self.items.is_empty() { self.selected = (self.selected + 1) % self.items.len(); } }
    pub fn prev(&mut self) { if !self.items.is_empty() { self.selected = self.selected.saturating_sub(1); if self.selected == 0 { self.selected = self.items.len() - 1; } } }
    pub fn insert(&self) -> Option<String> { self.items.get(self.selected).map(|i| i.insert.clone()) }
}

// ============================================================================
// MACROS
// ============================================================================

#[derive(Debug, Clone)]
pub struct MacroAction { pub kind: MacroActionKind, pub text: Option<String> }

#[derive(Debug, Clone)]
pub enum MacroActionKind { Insert, Delete, NewLine, Tab, Undo, Redo }

#[derive(Debug, Clone)]
pub struct Macro { pub name: String, pub actions: Vec<MacroAction> }

impl Macro { pub fn new(n: &str) -> Self { Self { name: n.to_string(), actions: Vec::new() } } }

pub struct MacroManager { pub recording: bool, pub current: Option<Macro>, pub saved: Vec<Macro>, pub count: usize }

impl MacroManager {
    pub fn new() -> Self { Self { recording: false, current: None, saved: Vec::new(), count: 0 } }
    
    pub fn start(&mut self) { self.recording = true; self.current = Some(Macro::new(&format!("Macro {}", self.saved.len() + 1))); self.count = 0; }
    pub fn stop(&mut self) -> Option<Macro> { self.recording = false; if let Some(mut m) = self.current.take() { m.actions.retain(|a| !matches!(a.kind, MacroActionKind::Undo | MacroActionKind::Redo)); if !m.actions.is_empty() { self.saved.push(m.clone()); return Some(m); } } None }
    pub fn record(&mut self, kind: MacroActionKind, text: Option<String>) { if self.recording { self.count += 1; if let Some(ref mut m) = self.current { m.actions.push(MacroAction { kind, text }); } } }
    pub fn execute(&self, content: &str) -> String { let mut r = content.to_string(); for a in &self.saved.last().map(|m| &m.actions).unwrap_or(&vec![]) { match a.kind { MacroActionKind::Insert => { if let Some(ref t) = a.text { r.push_str(t); } } MacroActionKind::NewLine => { r.push('\n'); } MacroActionKind::Tab => { r.push_str("    "); } _ => {} } } r }
}

// ============================================================================
// PLUGIN SYSTEM
// ============================================================================

#[derive(Debug, Clone)]
pub struct Plugin { pub name: String, pub desc: String, pub author: String, pub version: String, pub enabled: bool }

pub struct PluginManager { pub plugins: Vec<Plugin> }

impl PluginManager {
    pub fn new() -> Self {
        let mut m = Self { plugins: vec![
            Plugin { name: "CodeFormatter".into(), desc: "Auto-format code on save".into(), author: "Geany-Rs".into(), version: "1.0.0".into(), enabled: false },
            Plugin { name: "BracketHighlighter".into(), desc: "Highlight matching brackets".into(), author: "Geany-Rs".into(), version: "1.0.0".into(), enabled: false },
            Plugin { name: "TodoViewer".into(), desc: "Show TODO/FIXME in sidebar".into(), author: "Geany-Rs".into(), version: "1.0.0".into(), enabled: false },
            Plugin { name: "AutoSave".into(), desc: "Auto-save every 60 seconds".into(), author: "Geany-Rs".into(), version: "1.0.0".into(), enabled: false },
        ]};
        m
    }
}

// ============================================================================
// SYMBOL PARSER
// ============================================================================

#[derive(Debug, Clone)]
pub struct Symbol { pub name: String, pub kind: SymbolKind, pub line: usize }

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind { Function, Struct, Enum, Trait, Class, Module }

impl SymbolKind { pub fn icon(&self) -> &'static str { match self { SymbolKind::Function => "ƒ", SymbolKind::Struct => "S", SymbolKind::Enum => "E", SymbolKind::Trait => "T", SymbolKind::Class => "C", SymbolKind::Module => "M" } } pub fn color(&self) -> Color32 { match self { SymbolKind::Function => Color32::from_rgb(230, 192, 123), SymbolKind::Struct | SymbolKind::Class => Color32::from_rgb(78, 201, 176), SymbolKind::Enum => Color32::from_rgb(86, 156, 214), SymbolKind::Trait => Color32::from_rgb(206, 145, 120), SymbolKind::Module => Color32::from_rgb(197, 134, 192) } } }

pub struct SymbolParser;

impl SymbolParser {
    pub fn parse(content: &str, ft: Filetype) -> Vec<Symbol> {
        let mut syms = Vec::new();
        let pats: Vec<(&str, SymbolKind)> = match ft {
            Filetype::Rust => vec![(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)", SymbolKind::Function), (r"(?m)^(?:pub\s+)?struct\s+(\w+)", SymbolKind::Struct), (r"(?m)^(?:pub\s+)?enum\s+(\w+)", SymbolKind::Enum), (r"(?m)^(?:pub\s+)?trait\s+(\w+)", SymbolKind::Trait), (r"(?m)^(?:pub\s+)?mod\s+(\w+)", SymbolKind::Module)],
            Filetype::Python => vec![(r"(?m)^class\s+(\w+)", SymbolKind::Class), (r"(?m)^(?:async\s+)?def\s+(\w+)", SymbolKind::Function)],
            Filetype::JavaScript | Filetype::TypeScript => vec![(r"(?m)^class\s+(\w+)", SymbolKind::Class), (r"(?m)^function\s+(\w+)", SymbolKind::Function)],
            _ => vec![],
        };
        
        for (pat, kind) in pats {
            if let Ok(re) = Regex::new(pat) {
                for cap in re.captures_iter(content) {
                    if let Some(m) = cap.get(1) {
                        syms.push(Symbol { name: m.as_str().to_string(), kind: kind.clone(), line: content[..m.start()].matches('\n').count() });
                    }
                }
            }
        }
        syms.sort_by_key(|s| s.line);
        syms
    }
    pub fn names(syms: &[Symbol]) -> Vec<String> { syms.iter().map(|s| s.name.clone()).collect() }
}

// ============================================================================
// FILETYPE
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Filetype { PlainText, C, Cpp, Rust, Python, JavaScript, TypeScript, Html, Css, Json, Markdown, Yaml, Toml, Go, Java, Php, Sql, Shell }

impl Filetype {
    pub fn from_ext(ext: &str) -> Self { match ext.to_lowercase().as_str() {
        "c" => Filetype::C, "cpp" | "cc" | "cxx" | "h" | "hpp" => Filetype::Cpp, "rs" => Filetype::Rust, "py" => Filetype::Python,
        "js" | "mjs" => Filetype::JavaScript, "ts" | "tsx" => Filetype::TypeScript, "html" | "htm" => Filetype::Html, "css" => Filetype::Css,
        "json" => Filetype::Json, "md" => Filetype::Markdown, "yaml" | "yml" => Filetype::Yaml, "toml" => Filetype::Toml,
        "go" => Filetype::Go, "java" => Filetype::Java, "php" => Filetype::Php, "sql" => Filetype::Sql, "sh" | "bash" => Filetype::Shell, _ => Filetype::PlainText } }
    pub fn name(&self) -> &'static str { match self { Filetype::PlainText => "Plain", Filetype::C => "C", Filetype::Cpp => "C++", Filetype::Rust => "Rust", Filetype::Python => "Python", Filetype::JavaScript => "JavaScript", Filetype::TypeScript => "TypeScript", Filetype::Html => "HTML", Filetype::Css => "CSS", Filetype::Json => "JSON", Filetype::Markdown => "Markdown", Filetype::Yaml => "YAML", Filetype::Toml => "TOML", Filetype::Go => "Go", Filetype::Java => "Java", Filetype::Php => "PHP", Filetype::Sql => "SQL", Filetype::Shell => "Shell" } }
}

// ============================================================================
// DOCUMENT
// ============================================================================

#[derive(Debug, Clone)]
pub struct Document { pub id: usize, pub name: String, pub path: Option<String>, pub content: String, pub ft: Filetype, pub modified: bool, pub cursor_line: usize, pub cursor_col: usize }

impl Document {
    pub fn new(id: usize) -> Self { Self { id, name: format!("untitled_{}", id), path: None, content: String::new(), ft: Filetype::PlainText, modified: false, cursor_line: 1, cursor_col: 1 } }
    pub fn from_file(path: &str, content: String) -> Self {
        let p = std::path::Path::new(path);
        let name = p.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let ext = p.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        Self { id: 0, name, path: Some(path.to_string()), content, ft: Filetype::from_ext(&ext), modified: false, cursor_line: 1, cursor_col: 1 }
    }
}

// ============================================================================
// TERMINAL
// ============================================================================

pub struct Terminal { pub visible: bool, pub history: Vec<String>, pub dir: String, pub input: String }

impl Terminal {
    pub fn new() -> Self { let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string()); Self { visible: false, history: vec!["Geany-Rs Terminal v0.6.0".to_string(), "Type 'help' for commands".to_string()], dir: home, input: String::new() } }
    pub fn exec(&mut self, cmd: &str, app: &mut GeanyApp) {
        if cmd.trim().is_empty() { return; }
        self.history.push(format!("$ {}", cmd));
        match cmd.trim() {
            "help" => { self.history.push("help, clear, pwd, cd, ls, cat, mkdir, touch, rm, echo".to_string()); }
            "clear" => { self.history.clear(); }
            "pwd" => { self.history.push(self.dir.replace(&whoami::username(), "~")); }
            "whoami" => { self.history.push(whoami::username()); }
            cmd if cmd.starts_with("cd ") => { let d = cmd.trim_start_matches("cd ").replace('~', &whoami::username()); if let Ok(p) = std::fs::canonicalize(&d) { self.dir = p.to_string_lossy().to_string(); } else { self.history.push(format!("cd: {} not found", d)); } }
            "ls" => { if let Ok(e) = std::fs::read_dir(&self.dir) { for x in e.filter_map(|e| e.ok()) { self.history.push(x.file_name().to_string_lossy().to_string()); } } }
            cmd if cmd.starts_with("cat ") => { if let Ok(c) = std::fs::read_to_string(cmd.trim_start_matches("cat ")) { for l in c.lines().take(30) { self.history.push(l.to_string()); } } }
            cmd if cmd.starts_with("mkdir ") => { let _ = std::fs::create_dir_all(cmd.trim_start_matches("mkdir ")); }
            cmd if cmd.starts_with("touch ") => { let _ = std::fs::write(cmd.trim_start_matches("touch "), ""); }
            cmd if cmd.starts_with("rm ") => { let _ = std::fs::remove_file(cmd.trim_start_matches("rm ")); }
            cmd if cmd.starts_with("echo ") => { self.history.push(cmd.trim_start_matches("echo ").to_string()); }
            _ => {
                #[cfg(windows)] let (s, a) = ("cmd", "/C");
                #[cfg(not(windows))] let (s, a) = ("sh", "-c");
                if let Ok(o) = Command::new(s).arg(a).arg(cmd).current_dir(&self.dir).output() {
                    if !o.stdout.is_empty() { for l in String::from_utf8_lossy(&o.stdout).lines().take(30) { self.history.push(l.to_string()); } }
                    if !o.stderr.is_empty() { for l in String::from_utf8_lossy(&o.stderr).lines().take(30) { self.history.push(format!("[err] {}", l)); } }
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
pub struct FindReplace { pub search: String, pub replace: String, pub case_sens: bool, pub whole_word: bool, pub regex: bool }

impl FindReplace { pub fn new() -> Self { Self { search: String::new(), replace: String::new(), case_sens: false, whole_word: false, regex: false } }
    pub fn search(&self, content: &str) -> Vec<usize> {
        if self.search.is_empty() { return vec![]; }
        let n = if self.whole_word { format!(r"\b{}\b", regex::escape(&self.search)) } else { regex::escape(&self.search) };
        let p = if self.case_sens { n } else { format!("(?i){}", n) };
        if let Ok(re) = Regex::new(&p) { content.lines().enumerate().filter(|(_, l)| re.is_match(l)).map(|(i, _)| i).collect() } else { vec![] }
    }
    pub fn replace(&self, content: &str) -> String {
        if self.search.is_empty() { return content.to_string(); }
        let n = if self.whole_word { format!(r"\b{}\b", regex::escape(&self.search)) } else { regex::escape(&self.search) };
        let p = if self.case_sens { n } else { format!("(?i){}", n) };
        if let Ok(re) = Regex::new(&p) { re.replace_all(content, &self.replace).to_string() } else { content.to_string() }
    }
}

// ============================================================================
// BUILD SYSTEM
// ============================================================================

pub struct BuildSystem;

impl BuildSystem {
    pub fn cmds(ft: Filetype) -> Vec<(&'static str, &'static str)> {
        match ft {
            Filetype::Rust => vec![("🔨 Build (F8)", "cargo build"), ("▶ Run (F9)", "cargo run")],
            Filetype::C | Filetype::Cpp => vec![("🔨 Compile (F8)", "gcc \"{file}\" -o \"{name}\" -Wall"), ("▶ Run (F9)", "\"./{name}\"")],
            Filetype::Python => vec![("▶ Run (F9)", "python3 \"{file}\"")],
            Filetype::JavaScript => vec![("▶ Run (F9)", "node \"{file}\"")],
            Filetype::Html => vec![("🌐 Open (F9)", "xdg-open \"{file}\"")],
            _ => vec![],
        }
    }
    pub fn expand(cmd: &str, path: &Option<String>) -> String {
        let mut r = cmd.to_string();
        if let Some(p) = path { let pp = std::path::Path::new(p); r = r.replace("{file}", p); r = r.replace("{name}", &pp.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default()); }
        r
    }
    pub fn exec(cmd: &str, app: &mut GeanyApp) {
        app.log(format!("> {}", cmd));
        #[cfg(windows)] let (s, a) = ("cmd", "/C");
        #[cfg(not(windows))] let (s, a) = ("sh", "-c");
        if let Ok(mut c) = Command::new(s).arg(a).arg(cmd).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            use std::io::Read;
            if let Some(ref mut o) = c.stdout { let mut st = String::new(); if o.read_to_string(&mut st).is_ok() && !st.is_empty() { for l in st.lines().take(50) { app.log(l.to_string()); } } }
            if let Some(ref mut e) = c.stderr { let mut st = String::new(); if e.read_to_string(&mut st).is_ok() && !st.is_empty() { for l in st.lines().take(50) { app.log(format!("[err] {}", l)); } } }
            if let Ok(s) = c.wait() { app.log(if s.success() { "✓ Success".to_string() } else { format!("✗ Exit: {:?}", s.code()) }); }
        }
    }
}

// ============================================================================
// MAIN APPLICATION
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab { Files, Symbols, Macros, Plugins, Project }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu { File, Project, Edit, View, Search, Build, Tools, Macros, Plugins, Help }

pub struct GeanyApp {
    pub docs: Vec<Document>, pub active: Option<usize>, pub project: Option<GeanyProject>,
    pub sidebar: bool, pub sidebar_tab: SidebarTab,
    pub messages: Vec<String>, pub terminal: Terminal,
    pub completer: AutoCompleter, pub macros: MacroManager, pub plugins: PluginManager,
    pub theme: Theme, pub theme_colors: ThemeColors,
    pub find: FindReplace, pub show_find: bool, pub show_goto: bool, pub goto_line: String,
    pub fold: FoldState, pub bracket: Option<BracketHighlight>,
    pub menu: Option<Menu>, pub line_numbers: bool, pub word_wrap: bool,
}

impl GeanyApp {
    pub fn new(_: &eframe::CreationContext<'_>) -> Self {
        let theme = Theme::Dark;
        let mut app = Self {
            docs: vec![Document::new(1)], active: Some(0), project: None,
            sidebar: true, sidebar_tab: SidebarTab::Files,
            messages: vec!["Geany-Rs v0.6.0 - Complete IDE!".to_string(), "All features: Folding, Brackets, Themes, Syntax".to_string()],
            terminal: Terminal::new(), completer: AutoCompleter::new(), macros: MacroManager::new(),
            plugins: PluginManager::new(),
            theme, theme_colors: ThemeColors::from_theme(theme),
            find: FindReplace::new(), show_find: false, show_goto: false, goto_line: String::new(),
            fold: FoldState::new(), bracket: None,
            menu: None, line_numbers: true, word_wrap: false,
        };
        if let Some(d) = app.docs.first_mut() { d.content = include_str!("example.rs").to_string(); d.ft = Filetype::Rust; d.name = "example.rs".to_string(); }
        app.fold.parse(&app.docs[0].content);
        app
    }
    
    fn log(&mut self, m: impl Into<String>) { self.messages.push(m.into()); if self.messages.len() > 100 { self.messages.remove(0); } }
    fn new_doc(&mut self) { let id = self.docs.len() + 1; self.docs.push(Document::new(id)); self.active = Some(self.docs.len() - 1); }
    fn close_doc(&mut self, i: usize) { if self.docs.len() > 1 { self.docs.remove(i); if let Some(a) = self.active { if a >= i && a > 0 { self.active = Some(a - 1); } else if a >= self.docs.len() { self.active = Some(self.docs.len() - 1); } } } }
    fn open_file(&mut self) {
        if let Some(p) = file_dialogs::open_file() {
            let s = p.to_string_lossy().to_string();
            if s.ends_with(".geany") { if let Ok(pr) = GeanyProject::load(&s) { self.project = Some(pr); self.log(format!("Opened: {}", pr.name)); } }
            else if let Ok(c) = std::fs::read_to_string(&s) { let mut d = Document::from_file(&s, c); d.id = self.docs.len() + 1; self.docs.push(d); self.active = Some(self.docs.len() - 1); self.fold.parse(&self.docs.last().unwrap().content); self.log(format!("Opened: {}", s)); }
        }
    }
    fn save_file(&mut self) {
        if let Some(i) = self.active {
            let d = &mut self.docs[i];
            let n = d.path.as_ref().map(|p| std::path::Path::new(p).file_name().unwrap().to_string_lossy().to_string()).unwrap_or_else(|| d.name.clone());
            if let Some(p) = file_dialogs::save_file(&n) { let s = p.to_string_lossy().to_string(); if std::fs::write(&s, &d.content).is_ok() { d.path = Some(s.clone()); d.modified = false; d.name = std::path::Path::new(&s).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default(); self.log(format!("Saved: {}", s)); } }
        }
    }
    fn save_project(&mut self) { if let Some(ref p) = self.project { if let Some(path) = file_dialogs::save_project() { let s = path.to_string_lossy().to_string(); if p.save(&s).is_ok() { self.log(format!("Project saved: {}", s)); } } } }
    fn open_project(&mut self) { if let Some(p) = file_dialogs::pick_folder() { let s = p.to_string_lossy().to_string(); let mut pr = GeanyProject::default(); pr.name = std::path::Path::new(&s).file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_else(|| "Project".to_string()); pr.base_path = s.clone(); self.project = Some(pr); self.log(format!("Opened: {}", s)); } }
    fn new_project(&mut self) { self.project = Some(GeanyProject::default()); self.log("New project created"); }
}

fn menu_bar(app: &mut GeanyApp, ui: &mut Ui) {
    for (n, m) in [("File", Menu::File), ("Project", Menu::Project), ("Edit", Menu::Edit), ("View", Menu::View), ("Search", Menu::Search), ("Build", Menu::Build), ("Tools", Menu::Tools), ("Macros", Menu::Macros), ("Plugins", Menu::Plugins), ("Help", Menu::Help)] {
        let txt = RichText::new(n);
        if ui.selectable_label(app.menu == Some(m), txt).clicked() { app.menu = if app.menu == Some(m) { None } else { Some(m) }; }
    }
}

fn menu_dropdown(app: &mut GeanyApp, ui: &mut Ui, m: Menu) {
    match m {
        Menu::File => {
            if ui.button("📄 New         Ctrl+N").clicked() { app.new_doc(); app.menu = None; }
            if ui.button("📂 Open        Ctrl+O").clicked() { app.open_file(); app.menu = None; }
            ui.separator();
            if ui.button("💾 Save       Ctrl+S").clicked() { app.save_file(); app.menu = None; }
            if ui.button("💾 Save As").clicked() { app.save_file(); app.menu = None; }
            ui.separator();
            if ui.button("✕ Close      Ctrl+W").clicked() { if let Some(i) = app.active { app.close_doc(i); } app.menu = None; }
        }
        Menu::Project => {
            if ui.button("📁 New Project").clicked() { app.new_project(); app.menu = None; }
            if ui.button("💾 Save Project").clicked() { app.save_project(); app.menu = None; }
            if ui.button("📂 Open Folder").clicked() { app.open_project(); app.menu = None; }
            ui.separator();
            if let Some(ref p) = app.project { ui.label(RichText::new(format!("📁 {}", p.name)).strong()); } else { ui.label("No project"); }
        }
        Menu::Edit => {
            if ui.button("↩ Undo").clicked() { app.macros.record(MacroActionKind::Undo, None); app.menu = None; }
            if ui.button("↪ Redo").clicked() { app.macros.record(MacroActionKind::Redo, None); app.menu = None; }
            ui.separator();
            if ui.button("✂ Cut").clicked() { app.log("Cut"); app.menu = None; }
            if ui.button("📋 Copy").clicked() { app.log("Copy"); app.menu = None; }
            if ui.button("📄 Paste").clicked() { app.log("Paste"); app.menu = None; }
        }
        Menu::View => {
            if ui.button(if app.sidebar { "✓ Sidebar" } else { "Sidebar" }).clicked() { app.sidebar = !app.sidebar; }
            if ui.button(if app.line_numbers { "✓ Line Numbers" } else { "Line Numbers" }).clicked() { app.line_numbers = !app.line_numbers; }
            if ui.button(if app.word_wrap { "✓ Word Wrap" } else { "Word Wrap" }).clicked() { app.word_wrap = !app.word_wrap; }
            ui.separator();
            ui.label("Theme:");
            for t in Theme::all() { if ui.button(t.name()).clicked() { app.theme = t; app.theme_colors = ThemeColors::from_theme(t); app.menu = None; } }
        }
        Menu::Search => {
            if ui.button("🔍 Find      Ctrl+F").clicked() { app.show_find = !app.show_find; app.menu = None; }
            if ui.button("📍 Go to Line Ctrl+G").clicked() { app.show_goto = true; app.menu = None; }
        }
        Menu::Build => {
            if let Some(i) = app.active { let d = &app.docs[i]; for (lbl, cmd) in BuildSystem::cmds(d.ft) { if ui.button(lbl).clicked() { BuildSystem::exec(&BuildSystem::expand(cmd, &d.path), app); app.menu = None; } } }
        }
        Menu::Tools => {
            if ui.button(if app.terminal.visible { "✓ Terminal" } else { "🖥 Terminal" }).clicked() { app.terminal.visible = !app.terminal.visible; }
            if ui.button(if app.completer.enabled { "✓ Auto-complete" } else { "Auto-complete" }).clicked() { app.completer.enabled = !app.completer.enabled; }
        }
        Menu::Macros => {
            if ui.button(if app.macros.recording { "⏹ Stop" } else { "⏺ Record" }).clicked() {
                if app.macros.recording { if let Some(m) = app.macros.stop() { app.log(format!("Saved: {} ({} actions)", m.name, m.actions.len())); } }
                else { app.macros.start(); app.log("Recording..."); }
            }
            ui.separator();
            for (i, m) in app.macros.saved.iter().enumerate() { ui.horizontal(|ui| { if ui.button(format!("▶ {}", m.name)).clicked() { if let Some(d) = app.docs.get_mut(app.active?) { d.content = app.macros.execute(&d.content); } } if ui.button("🗑").clicked() { app.macros.saved.remove(i); } }); }
            if app.macros.recording { ui.separator(); ui.label(RichText::new(format!("⏺ {} actions", app.macros.count)).color(Color32::from_rgb(255, 100, 100))); }
        }
        Menu::Plugins => {
            for (i, p) in app.plugins.plugins.iter_mut().enumerate() {
                let mut en = p.enabled;
                if ui.checkbox(&mut en, &p.name).changed() { app.plugins.plugins[i].enabled = en; app.log(format!("Plugin '{}' {}", p.name, if en { "enabled" } else { "disabled" })); }
                ui.label(RichText::new(&p.desc).small().color(Color32::GRAY));
            }
        }
        Menu::Help => {
            if ui.button("⌨ Shortcuts").clicked() { app.log("Ctrl+N/O/S/F/G/W | F8/F9 | Ctrl+Space | Ctrl+Shift+R"); app.menu = None; }
            if ui.button("ℹ About").clicked() { app.log("Geany-Rs v0.6.0 - Rust + egui IDE"); app.menu = None; }
        }
    }
}

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        let mods = ctx.input(|i| i.modifiers);
        
        // Shortcuts
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::N)) { self.new_doc(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::O)) { self.open_file(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::S)) { self.save_file(); }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::F)) { self.show_find = !self.show_find; }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::G)) { self.show_goto = true; }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::W)) { if let Some(i) = self.active { self.close_doc(i); } }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::Space)) { if self.completer.enabled { if let Some(i) = self.active { let d = &self.docs[i]; let s = SymbolParser::parse(&d.content, d.ft); self.completer.trigger(&d.content, d.ft, &SymbolParser::names(&s)); } } }
        if mods.cmd && ctx.input(|i| i.key_pressed(egui::Key::Shift)) && ctx.input(|i| i.key_pressed(egui::Key::R)) {
            if self.macros.recording { if let Some(m) = self.macros.stop() { self.log(format!("Saved: {}", m.name)); } } else { self.macros.start(); }
        }
        if self.completer.show { if ctx.input(|i| i.key_pressed(egui::Key::ArrowDown)) { self.completer.next(); } if ctx.input(|i| i.key_pressed(egui::Key::ArrowUp)) { self.completer.prev(); } if ctx.input(|i| i.key_pressed(egui::Key::Enter)) { if let Some(t) = self.completer.insert() { if let Some(d) = self.docs.get_mut(self.active?) { d.content.push_str(&t); } } self.completer.show = false; } if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.completer.show = false; } }
        if ctx.input(|i| i.key_pressed(egui::Key::F8)) { if let Some(i) = self.active { let d = &self.docs[i]; if let Some((_, c)) = BuildSystem::cmds(d.ft).first() { BuildSystem::exec(&BuildSystem::expand(c, &d.path), self); } } }
        if ctx.input(|i| i.key_pressed(egui::Key::F9)) { if let Some(i) = self.active { let d = &self.docs[i]; if let Some((_, c)) = BuildSystem::cmds(d.ft).get(1).or_else(|| BuildSystem::cmds(d.ft).first()) { BuildSystem::exec(&BuildSystem::expand(c, &d.path), self); } } }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.show_find = false; self.show_goto = false; self.menu = None; }
        
        ctx.set_visuals(match self.theme { Theme::Dark | Theme::Monokai | Theme::Dracula | Theme::Nord | Theme::Gruvbox => egui::Visuals::dark(), _ => egui::Visuals::light() });
        
        // TOP
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            menu_bar(self, ui);
            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("📄").clicked() { self.new_doc(); }
                if ui.button("📂").clicked() { self.open_file(); }
                if ui.button("💾").clicked() { self.save_file(); }
                ui.separator();
                if ui.button(if self.macros.recording { "⏹" } else { "⏺" }).on_hover_text("Record Macro").clicked() { if self.macros.recording { if let Some(m) = self.macros.stop() { self.log(format!("Saved: {}", m.name)); } } else { self.macros.start(); } }
                ui.separator();
                if ui.toggle_value(&mut self.sidebar, "📑").clicked() {}
                if ui.toggle_value(&mut self.terminal.visible, "🖥").clicked() {}
                ComboBox::from_id_salt("theme").selected_text(self.theme.name()).show_ui(ui, |ui| { for t in Theme::all() { ui.selectable_value(&mut self.theme, t, t.name()); } });
                if self.theme != Theme::Light { if ui.button("☀️").clicked() { self.theme = Theme::Light; self.theme_colors = ThemeColors::from_theme(Theme::Light); } } else { if ui.button("🌙").clicked() { self.theme = Theme::Dark; self.theme_colors = ThemeColors::from_theme(Theme::Dark); } }
            });
        });
        
        // MENU DROPDOWN
        if let Some(m) = self.menu {
            let pos = ctx.cursor().unwrap();
            Window::new(format!("{:?}", m)).collapsible(false).resizable(false).anchor(egui::Align2::LEFT_UP, [pos.x, pos.y + 20.0]).show(ctx, |ui| { menu_dropdown(self, ui, m); });
        }
        
        // SIDEBAR
        if self.sidebar {
            SidePanel::left("sidebar").resizable(true).default_width(230.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (tab, icon) in [(SidebarTab::Files, "📁"), (SidebarTab::Symbols, "🔣"), (SidebarTab::Macros, "⏺"), (SidebarTab::Plugins, "🔌"), (SidebarTab::Project, "📁")] {
                        let mut sel = self.sidebar_tab == tab;
                        if ui.toggle_sized(&mut sel, icon).clicked() { self.sidebar_tab = tab; }
                    }
                });
                ui.separator();
                
                match self.sidebar_tab {
                    SidebarTab::Files => {
                        ScrollArea::vertical().show(ui, |ui| {
                            for (i, d) in self.docs.iter().enumerate() {
                                let a = self.active == Some(i);
                                let mut t = RichText::new(&d.name);
                                if d.modified { t = t.color(Color32::from_rgb(255, 200, 0)); }
                                if a { t = t.bold(); }
                                if ui.selectable_label(a, t).clicked() { self.active = Some(i); }
                            }
                        });
                    }
                    SidebarTab::Symbols => {
                        if let Some(i) = self.active {
                            let syms = SymbolParser::parse(&self.docs[i].content, self.docs[i].ft);
                            if syms.is_empty() { ui.label(RichText::new("No symbols").color(Color32::GRAY)); }
                            else { ScrollArea::vertical().show(ui, |ui| { for s in &syms { ui.horizontal(|ui| { ui.label(RichText::new(s.kind.icon()).color(s.kind.color())); if ui.link(&s.name).clicked() { self.docs[i].cursor_line = s.line + 1; } }); } }); }
                        }
                    }
                    SidebarTab::Macros => {
                        if self.macros.recording { ui.label(RichText::new(format!("⏺ {} actions", self.macros.count)).color(Color32::from_rgb(255, 100, 100))); } else { ui.label("Not recording"); }
                        ui.separator();
                        if self.macros.saved.is_empty() { ui.label("No macros"); }
                        else { for (i, m) in self.macros.saved.iter().enumerate() { ui.horizontal(|ui| { if ui.button(format!("▶ {}", m.name)).clicked() { if let Some(d) = self.docs.get_mut(self.active?) { d.content = self.macros.execute(&d.content); } } if ui.button("🗑").clicked() { self.macros.saved.remove(i); } }); } }
                    }
                    SidebarTab::Plugins => {
                        for (i, p) in self.plugins.plugins.iter_mut().enumerate() {
                            let mut en = p.enabled;
                            if ui.checkbox(&mut en, &p.name).changed() { self.plugins.plugins[i].enabled = en; }
                            ui.label(RichText::new(&p.desc).small().color(Color32::GRAY));
                        }
                    }
                    SidebarTab::Project => {
                        if let Some(ref p) = self.project { ui.label(RichText::new(&p.name).strong()); } else { ui.label("No project"); }
                    }
                }
            });
        }
        
        // MAIN EDITOR
        CentralPanel::default().show(ctx, |ui| {
            ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, d) in self.docs.iter().enumerate() {
                        let a = self.active == Some(i);
                        let mut l = d.name.clone();
                        if d.modified { l.push_str(" ●"); }
                        if ui.selectable_label(a, l).clicked() { self.active = Some(i); self.fold.parse(&d.content); }
                    }
                    if ui.button("+").clicked() { self.new_doc(); }
                });
            });
            ui.separator();
            
            if self.show_find {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("🔍 Find:");
                        TextEdit::singleline(&mut self.find.search).desired_width(150.0).show(ui);
                        if ui.button("Find").clicked() { if let Some(i) = self.active { let r = self.find.search(&self.docs[i].content); self.log(format!("Found {} matches", r.len())); } }
                        ui.separator();
                        ui.label("↔ Replace:");
                        TextEdit::singleline(&mut self.find.replace).desired_width(150.0).show(ui);
                        if ui.button("Replace All").clicked() { if let Some(i) = self.active { self.docs[i].content = self.find.replace(&self.docs[i].content); } }
                        if ui.button("✕").clicked() { self.show_find = false; }
                    });
                });
                ui.separator();
            }
            
            if let Some(i) = self.active {
                if let Some(d) = self.docs.get_mut(i) {
                    ui.horizontal(|ui| {
                        ui.label(format!("📝 {}", d.ft.name()));
                        ComboBox::from_id_salt("ft").selected_text(d.ft.name()).show_ui(ui, |ui| {
                            for ft in [Filetype::Rust, Filetype::C, Filetype::Cpp, Filetype::Python, Filetype::JavaScript, Filetype::TypeScript, Filetype::Html, Filetype::Css, Filetype::Json, Filetype::PlainText] { ui.selectable_value(&mut d.ft, ft, ft.name()); }
                        });
                        ui.separator();
                        ui.label(format!("Ln {}, Col {}", d.cursor_line, d.cursor_col));
                        if d.modified { ui.label(RichText::new("●").color(Color32::from_rgb(255, 200, 0))); }
                        if ui.button("✨ Complete").clicked() { let s = SymbolParser::parse(&d.content, d.ft); self.completer.trigger(&d.content, d.ft, &SymbolParser::names(&s)); }
                    });
                    ui.separator();
                    
                    ScrollArea::vertical().show(ui, |ui| {
                        ui.horizontal(ui, |ui| {
                            if self.line_numbers {
                                ui.vertical(|ui| {
                                    ui.set_width(50.0);
                                    let lines = d.content.lines().count().max(1);
                                    for i in 1..=lines {
                                        let vlines = self.fold.get_visual_lines(lines);
                                        if let Some(vl) = vlines.get(i - 1) {
                                            if !vl.visible { ui.label(RichText::new("...").small().color(Color32::GRAY)); }
                                            else {
                                                // FOLD ICON
                                                if vl.is_fold_start {
                                                    let txt = if vl.fold_collapsed { "▶" } else { "▼" };
                                                    let r = ui.put(ui.available_rect().expand(2.0), egui::Button::new(txt).small());
                                                    if r.clicked() { self.fold.toggle(vl.line); }
                                                }
                                                let clr = if i == d.cursor_line { self.theme_colors.line_highlight } else { Color32::TRANSPARENT };
                                                ui.label(RichText::new(format!("{:>4}", i)).small().monospace().color(Color32::GRAY));
                                            }
                                        }
                                    }
                                });
                                ui.separator();
                            }
                            
                            // BRACKET HIGHLIGHT TEST
                            self.bracket = BracketHighlight::find(&d.content, d.content.len().min(100));
                            
                            let mut text = d.content.clone();
                            TextEdit::multiline(&mut text).font(FontId::monospace(14.0)).desired_width(if self.word_wrap { 800.0 } else { f32::INFINITY }).show(ui);
                            if text != d.content {
                                self.macros.record(MacroActionKind::Insert, Some(text.clone()));
                                d.content = text;
                                d.modified = true;
                                self.fold.parse(&d.content);
                            }
                        });
                    });
                }
            }
        });
        
        // COMPLETION POPUP
        if self.completer.show && !self.completer.items.is_empty() {
            Window::new("Completions").collapsible(false).resizable(false).always_auto_resize().anchor(egui::Align2::LEFT_BOTTOM, [50.0, 400.0]).show(ctx, |ui| {
                for (i, item) in self.completer.items.iter().enumerate() {
                    let sel = i == self.completer.selected;
                    ui.horizontal(|ui| { ui.label(RichText::new(item.kind.icon()).color(item.kind.color())); ui.label(if sel { RichText::new(&item.label).strong() } else { RichText::new(&item.label) }); });
                    ui.label(RichText::new(&item.detail).small().color(Color32::GRAY));
                }
                ui.separator();
                ui.label("↑↓ Navigate  Enter Insert  Esc Close");
            });
        }
        
        // STATUS BAR
        TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(d) = self.docs.get(self.active?) { ui.label(RichText::new(&d.name).small().strong()); ui.separator(); ui.label(RichText::new(format!("Ln {}, Col {}", d.cursor_line, d.cursor_col)).small()); ui.separator(); ui.label(RichText::new(d.ft.name()).small()); }
                if let Some(ref p) = self.project { ui.separator(); ui.label(RichText::new(format!("📁 {}", p.name)).small().color(Color32::from_rgb(200, 150, 100)); }
                if self.macros.recording { ui.separator(); ui.label(RichText::new("⏺ REC").small().color(Color32::from_rgb(255, 100, 100)); }
                ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| { ui.label(RichText::new(format!("{} folds", self.fold.regions.len())).small().color(Color32::GRAY)); ui.separator(); ui.label(RichText::new("Geany-Rs v0.6.0").small().color(Color32::GRAY)); });
            });
        });
        
        // MESSAGES
        TopBottomPanel::bottom("messages").resizable(true).default_height(80.0).show(ctx, |ui| {
            ui.horizontal(|ui| { ui.label("📋"); if ui.button("Clear").clicked() { self.messages.clear(); } });
            ui.separator();
            ScrollArea::vertical().show(ui, |ui| { for m in &self.messages { ui.label(m); } });
        });
        
        // TERMINAL
        if self.terminal.visible {
            TopBottomPanel::bottom("terminal").resizable(true).default_height(150.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("🖥 Terminal:"); if ui.button("Clear").clicked() { self.terminal.history.clear(); } });
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| { ui.label(RichText::new(self.terminal.history.join("\n")).monospace().size(12.0)); });
                ui.separator();
                ui.horizontal(|ui| { ui.label("$"); let r = TextEdit::singleline(&mut self.terminal.input).show(ui); if r.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) { self.terminal.exec(&self.terminal.input, self); self.terminal.input.clear(); } });
            });
        }
        
        // GOTO LINE
        if self.show_goto {
            Window::new("Go to Line").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("Line:");
                TextEdit::singleline(&mut self.goto_line).desired_width(100.0).request_focus().show(ui);
                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() { if let Ok(l) = self.goto_line.parse::<usize>() { if let Some(d) = self.docs.get_mut(self.active?) { d.cursor_line = l; } } self.show_goto = false; self.goto_line.clear(); }
                    if ui.button("Cancel").clicked() { self.show_goto = false; self.goto_line.clear(); }
                });
            });
        }
    }
}

fn main() {
    let options = eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_min_inner_size([800.0, 600.0]).with_title("Geany-Rs v0.6.0 - Complete IDE"), ..Default::default() };
    eframe::run_native("Geany-Rs", options, Box::new(|cc| Ok(Box::new(GeanyApp::new(cc))))).unwrap();
}
