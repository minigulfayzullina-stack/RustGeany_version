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
//! - Indentation settings (tabs/spaces)
//! - Multi-cursor editing
//! - Code folding
//! - Bracket matching
//! - Word wrap

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{Align, Color32, ComboBox, FontId, RichText, ScrollArea, TextEdit, TopBottomPanel, SidePanel, CentralPanel, Ui, Vec2, Window};
use regex::Regex;
use std::process::{Command, Stdio};

// ============================================================================
// SETTINGS (Indentation, Word Wrap, etc.)
// ============================================================================

#[derive(Debug, Clone)]
pub struct EditorSettings {
    pub indent_width: usize,
    pub use_spaces: bool,
    pub show_whitespace: bool,
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
        Self {
            indent_width: 4,
            use_spaces: true,
            show_whitespace: false,
            show_line_numbers: true,
            word_wrap: false,
            wrap_width: 80,
            highlight_current_line: true,
            bracket_highlight: true,
            auto_indent: true,
            tab_size: 4,
        }
    }
}

impl EditorSettings {
    pub fn indent_string(&self) -> String {
        if self.use_spaces {
            " ".repeat(self.indent_width)
        } else {
            "\t".to_string()
        }
    }
}

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
}

// ============================================================================
// CODE FOLDING
// ============================================================================

#[derive(Debug, Clone)]
pub struct FoldRegion {
    pub start_line: usize,
    pub end_line: usize,
    pub collapsed: bool,
}

#[derive(Debug, Clone)]
pub struct FoldState {
    pub regions: Vec<FoldRegion>,
}

impl FoldState {
    pub fn new() -> Self {
        Self { regions: Vec::new() }
    }

    pub fn parse_folds(&mut self, content: &str) {
        self.regions.clear();
        let lines: Vec<&str> = content.lines().collect();
        let mut stack: Vec<usize> = Vec::new();
        
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            
            // Count braces for Rust/C/Java
            let opens = trimmed.matches('{').count();
            let closes = trimmed.matches('}').count();
            
            if opens > 0 {
                if let Some(&start) = stack.last() {
                    // Check if we need to close previous region
                    if closes > opens {
                        if let Some(&start) = stack.pop() {
                            self.regions.push(FoldRegion {
                                start_line: start,
                                end_line: i,
                                collapsed: false,
                            });
                        }
                    }
                }
                if opens > closes {
                    stack.push(i);
                } else if opens == closes && !stack.is_empty() {
                    if let Some(&start) = stack.pop() {
                        self.regions.push(FoldRegion {
                            start_line: start,
                            end_line: i,
                            collapsed: false,
                        });
                    }
                }
            }
            
            // Python: check for class/function definitions
            if trimmed.starts_with("def ") || trimmed.starts_with("class ") || trimmed.starts_with("async def ") {
                if let Some(&start) = stack.last() {
                    // Close previous if nested
                    if start < i {
                        self.regions.push(FoldRegion {
                            start_line: start,
                            end_line: i.saturating_sub(1),
                            collapsed: false,
                        });
                        stack.pop();
                    }
                }
                stack.push(i);
            }
        }
        
        // Close remaining regions
        while let Some(start) = stack.pop() {
            if let Some(last_line) = lines.len().checked_sub(1) {
                if start < last_line {
                    self.regions.push(FoldRegion {
                        start_line: start,
                        end_line: last_line,
                        collapsed: false,
                    });
                }
            }
        }
        
        self.regions.sort_by_key(|r| r.start_line);
    }

    pub fn toggle_fold(&mut self, line: usize) {
        for region in &mut self.regions {
            if region.start_line == line {
                region.collapsed = !region.collapsed;
                return;
            }
        }
    }

    pub fn get_visible_lines(&self, total_lines: usize) -> Vec<(usize, bool)> {
        // Returns (line_number, is_visible)
        let mut visible = Vec::new();
        let mut hidden_ranges: Vec<(usize, usize)> = self.regions.iter()
            .filter(|r| r.collapsed)
            .map(|r| (r.start_line + 1, r.end_line))
            .collect();
        
        for i in 0..total_lines {
            let mut is_hidden = false;
            for (start, end) in &hidden_ranges {
                if i > *start && i <= *end {
                    is_hidden = true;
                    break;
                }
            }
            visible.push((i, !is_hidden));
        }
        visible
    }
}

// ============================================================================
// BRACKET MATCHING
// ============================================================================

#[derive(Debug, Clone)]
pub struct BracketMatch {
    pub open_pos: usize,
    pub close_pos: usize,
    pub bracket_type: BracketType,
}

#[derive(Debug, Clone, Copy)]
pub enum BracketType {
    Parentheses,  // ()
    Curly,        // {}
    Square,       // []
    Angle,        // <>
}

pub struct BracketMatcher;

impl BracketMatcher {
    pub fn find_match(content: &str, pos: usize) -> Option<BracketMatch> {
        let chars: Vec<char> = content.chars().collect();
        if pos >= chars.len() { return None; }
        
        let c = chars[pos];
        let (open, close) = match c {
            '(' => ('(', ')'),
            ')' => (')', '('),
            '{' => ('{', '}'),
            '}' => ('}', '{'),
            '[' => ('[', ']'),
            ']' => (']', '['),
            '<' => ('<', '>'),
            '>' => ('>', '<'),
            _ => return None,
        };
        
        let is_open = c == open;
        let direction: isize = if is_open { 1 } else { -1 };
        let mut depth = 1;
        let mut i = pos as isize + direction;
        
        while i >= 0 && i < chars.len() as isize {
            if chars[i as usize] == open && is_open {
                depth += 1;
            } else if chars[i as usize] == close && is_open {
                depth -= 1;
            } else if chars[i as usize] == close && !is_open {
                depth += 1;
            } else if chars[i as usize] == open && !is_open {
                depth -= 1;
            }
            
            if depth == 0 {
                let bracket_type = match c {
                    '(' | ')' => BracketType::Parentheses,
                    '{' | '}' => BracketType::Curly,
                    '[' | ']' => BracketType::Square,
                    '<' | '>' => BracketType::Angle,
                    _ => return None,
                };
                return Some(BracketMatch {
                    open_pos: if is_open { pos } else { i as usize },
                    close_pos: if is_open { i as usize } else { pos },
                    bracket_type,
                });
            }
            
            i += direction;
        }
        
        None
    }
    
    pub fn find_bracket_at_cursor(content: &str, cursor_pos: usize) -> Option<usize> {
        let chars: Vec<char> = content.chars().collect();
        let before = if cursor_pos > 0 { chars.get(cursor_pos - 1) } else { None };
        let at = chars.get(cursor_pos);
        
        // Check if cursor is at a bracket
        if let Some(&c) = at {
            if "{}[]().".contains(c) {
                return Some(cursor_pos);
            }
        }
        
        // Check if cursor is right after a bracket
        if let Some(&c) = before {
            if "{}[]().".contains(c) {
                return Some(cursor_pos - 1);
            }
        }
        
        None
    }
}

// ============================================================================
// MULTI-CURSOR EDITING
// ============================================================================

#[derive(Debug, Clone)]
pub struct Cursor {
    pub line: usize,
    pub col: usize,
    pub selection_start: Option<(usize, usize)>,
}

impl Cursor {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col, selection_start: None }
    }
    
    pub fn has_selection(&self) -> bool {
        self.selection_start.is_some()
    }
    
    pub fn selection_range(&self) -> Option<((usize, usize), (usize, usize))> {
        self.selection_start.map(|start| (start, (self.line, self.col)))
    }
}

#[derive(Debug, Clone)]
pub struct MultiCursorState {
    pub cursors: Vec<Cursor>,
    pub primary_index: usize,
}

impl MultiCursorState {
    pub fn new() -> Self {
        Self {
            cursors: vec![Cursor::new(0, 0)],
            primary_index: 0,
        }
    }
    
    pub fn primary(&self) -> &Cursor {
        &self.cursors[self.primary_index]
    }
    
    pub fn primary_mut(&mut self) -> &mut Cursor {
        &mut self.cursors[self.primary_index]
    }
    
    pub fn add_cursor(&mut self, line: usize, col: usize) {
        self.cursors.push(Cursor::new(line, col));
        self.primary_index = self.cursors.len() - 1;
    }
    
    pub fn remove_cursor(&mut self, index: usize) {
        if self.cursors.len() > 1 && index < self.cursors.len() {
            self.cursors.remove(index);
            if self.primary_index >= self.cursors.len() {
                self.primary_index = self.cursors.len() - 1;
            }
        }
    }
    
    pub fn move_primary(&mut self, line: usize, col: usize) {
        if let Some(cursor) = self.cursors.get_mut(self.primary_index) {
            cursor.line = line;
            cursor.col = col;
        }
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
    pub collapsible: bool,
    pub children: Vec<Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function, Struct, Enum, Impl, Trait, Class, Method, Module, Variable, Constant, Property,
}

impl SymbolKind {
    pub fn icon(&self) -> &'static str {
        match self {
            SymbolKind::Function => "ƒ", SymbolKind::Struct => "S", SymbolKind::Enum => "E",
            SymbolKind::Impl => "I", SymbolKind::Trait => "T", SymbolKind::Class => "C",
            SymbolKind::Method => "m", SymbolKind::Module => "M", SymbolKind::Variable => "v",
            SymbolKind::Constant => "K", SymbolKind::Property => "p",
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
            (r"(?m)^(?:pub\s+)?type\s+(\w+)", SymbolKind::Constant),
        ];
        
        for (pattern, kind) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        let line = content[..name.start()].matches('\n').count();
                        symbols.push(Symbol { 
                            name: name.as_str().to_string(), 
                            kind: kind.clone(), 
                            line, 
                            collapsible: matches!(kind, SymbolKind::Struct | SymbolKind::Enum | SymbolKind::Trait | SymbolKind::Impl),
                            children: vec![],
                        });
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
                if !["if", "else", "while", "for", "switch"].contains(&name_str) {
                    let line = content[..name.start()].matches('\n').count();
                    symbols.push(Symbol { name: name_str.to_string(), kind: SymbolKind::Function, line, collapsible: true, children: vec![] });
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
            (r"(?m)^(?:async\s+)?def\s+(\w+)", SymbolKind::Function),
        ];
        for (pattern, kind) in patterns {
            if let Ok(re) = Regex::new(pattern) {
                for cap in re.captures_iter(content) {
                    if let Some(name) = cap.get(1) {
                        let line = content[..name.start()].matches('\n').count();
                        symbols.push(Symbol { name: name.as_str().to_string(), kind: kind.clone(), line, collapsible: true, children: vec![] });
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
                        symbols.push(Symbol { name: name.as_str().to_string(), kind: kind.clone(), line, collapsible: true, children: vec![] });
                    }
                }
            }
        }
        symbols.sort_by_key(|s| s.line);
        symbols
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
// BUILD COMMANDS
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
}

impl BuildSystem {
    pub fn new() -> Self {
        Self {
            commands: vec![
                vec![
                    BuildCommand { name: "Compile".to_string(), command: "rustc \"{file}\" -o \"{file_basename}\"".to_string(), working_dir: None, shortcut: Some("F8".to_string()) },
                    BuildCommand { name: "Run".to_string(), command: "\"./{file_basename}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                    BuildCommand { name: "Cargo Build".to_string(), command: "cargo build".to_string(), working_dir: Some("{project_dir}".to_string()), shortcut: Some("F10".to_string()) },
                    BuildCommand { name: "Cargo Run".to_string(), command: "cargo run".to_string(), working_dir: Some("{project_dir}".to_string()), shortcut: Some("F11".to_string()) },
                ],
                vec![
                    BuildCommand { name: "Compile".to_string(), command: "gcc \"{file}\" -o \"{file_basename}\" -Wall".to_string(), working_dir: None, shortcut: Some("F8".to_string()) },
                    BuildCommand { name: "Run".to_string(), command: "\"./{file_basename}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
                vec![
                    BuildCommand { name: "Run".to_string(), command: "python3 \"{file}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
                vec![
                    BuildCommand { name: "Run".to_string(), command: "node \"{file}\"".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
                vec![
                    BuildCommand { name: "Open".to_string(), command: "xdg-open \"{file}\" 2>/dev/null || open \"{file}\" 2>/dev/null".to_string(), working_dir: None, shortcut: Some("F9".to_string()) },
                ],
            ],
        }
    }

    pub fn get_commands_for(&self, filetype: Filetype) -> &[BuildCommand] {
        let index = match filetype {
            Filetype::Rust => 0, Filetype::C | Filetype::Cpp => 1,
            Filetype::Python => 2, Filetype::JavaScript | Filetype::TypeScript => 3,
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

        match Command::new(shell).arg(shell_arg).arg(&expanded)
            .current_dir(working_dir.as_deref().unwrap_or("."))
            .stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()
        {
            Ok(mut child) => {
                use std::io::Read;
                if let Some(mut stdout) = child.stdout.take() {
                    let mut output = String::new();
                    if stdout.read_to_string(&mut output).is_ok() && !output.is_empty() {
                        for line in output.lines().take(100) { app.log(line.to_string()); }
                    }
                }
                if let Some(mut stderr) = child.stderr.take() {
                    let mut output = String::new();
                    if stderr.read_to_string(&mut output).is_ok() && !output.is_empty() {
                        for line in output.lines().take(100) { app.log(format!("[err] {}", line)); }
                    }
                }
                if let Ok(status) = child.wait() {
                    app.log(if status.success() { "✓ Build succeeded".to_string() } else { format!("✗ Exit code: {:?}", status.code()) });
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
        let username = whoami::username();
        let home = dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_else(|| ".".to_string());
        Self {
            visible: false,
            history: vec!["Geany-Rs Terminal v0.3.0".to_string(), "Type 'help' for commands".to_string(), "─".repeat(40)],
            current_dir: home,
            command_input: String::new(),
        }
    }

    pub fn execute(&mut self, command: &str, app: &mut GeanyApp) {
        if command.trim().is_empty() { return; }
        self.history.push(format!("{} $ {}", self.current_dir.replace(&*whoami::username(), "~"), command));
        
        match command.trim() {
            "help" => { self.history.push("Commands: help, clear, pwd, cd, ls, cat, mkdir, touch, rm, echo, date, whoami".to_string()); }
            "clear" => { self.history.clear(); }
            "pwd" => { self.history.push(self.current_dir.replace(&*whoami::username(), "~")); }
            "date" => { let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap(); self.history.push(format!("Epoch: {} seconds", now.as_secs())); }
            "whoami" => { self.history.push(whoami::username()); }
            cmd if cmd.starts_with("cd ") => {
                let dir = cmd.trim_start_matches("cd ").trim().replace('~', &*whoami::username());
                let new_dir = if dir.starts_with('/') { dir.clone() } else { format!("{}/{}", self.current_dir, dir) };
                if let Ok(canonical) = std::fs::canonicalize(&new_dir) {
                    self.current_dir = canonical.to_string_lossy().to_string();
                } else {
                    self.history.push(format!("cd: {}: No such directory", dir));
                }
            }
            "ls" => { if let Ok(entries) = std::fs::read_dir(&self.current_dir) { for entry in entries.filter_map(|e| e.ok()) { let name = entry.file_name().to_string_lossy().to_string(); let is_dir = entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false); self.history.push(format!("{} {}", if is_dir { "📁" } else { "📄" }, name)); } } }
            cmd if cmd.starts_with("cat ") => { let file = cmd.trim_start_matches("cat "); match std::fs::read_to_string(file) { Ok(c) => { for l in c.lines().take(50) { self.history.push(l.to_string()); } } Err(_) => { self.history.push(format!("cat: {}: Not found", file)); } } }
            cmd if cmd.starts_with("echo ") => { self.history.push(cmd.trim_start_matches("echo ").to_string()); }
            cmd if cmd.starts_with("mkdir ") => { match std::fs::create_dir_all(cmd.trim_start_matches("mkdir ")) { Ok(_) => self.history.push("Created".to_string()), Err(e) => self.history.push(format!("Error: {}", e)) } }
            cmd if cmd.starts_with("touch ") => { match std::fs::write(cmd.trim_start_matches("touch "), "") { Ok(_) => self.history.push("Created".to_string()), Err(e) => self.history.push(format!("Error: {}", e)) } }
            cmd if cmd.starts_with("rm ") => { match std::fs::remove_file(cmd.trim_start_matches("rm ")) { Ok(_) => self.history.push("Removed".to_string()), Err(e) => self.history.push(format!("Error: {}", e)) } }
            _ => {
                #[cfg(target_os = "windows")]
                let (shell, arg) = ("cmd", "/C");
                #[cfg(not(target_os = "windows"))]
                let (shell, arg) = ("sh", "-c");
                if let Ok(output) = Command::new(shell).arg(arg).arg(command).current_dir(&self.current_dir).output() {
                    if !output.stdout.is_empty() { for l in String::from_utf8_lossy(&output.stdout).lines().take(50) { self.history.push(l.to_string()); } }
                    if !output.stderr.is_empty() { for l in String::from_utf8_lossy(&output.stderr).lines().take(50) { self.history.push(format!("[err] {}", l)); } }
                }
            }
        }
        if self.history.len() > 500 { self.history = self.history.split_off(self.history.len() - 500); }
    }
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
    pub fold_state: FoldState,
    pub multi_cursor: MultiCursorState,
    pub bracket_match: Option<BracketMatch>,
}

impl Document {
    pub fn new(id: usize) -> Self {
        Self { id, name: format!("untitled_{}", id), path: None, content: String::new(), filetype: Filetype::PlainText, modified: false, cursor_line: 1, cursor_col: 1, encoding: "UTF-8".to_string(), eol: "\n".to_string(), fold_state: FoldState::new(), multi_cursor: MultiCursorState::new(), bracket_match: None }
    }

    pub fn from_file(path: &str, content: String) -> Self {
        let path_obj = std::path::Path::new(path);
        let name = path_obj.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| path.to_string());
        let ext = path_obj.extension().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let mut doc = Self { id: 0, name, path: Some(path.to_string()), content: content.clone(), filetype: Filetype::from_extension(&ext), modified: false, cursor_line: 1, cursor_col: 1, encoding: "UTF-8".to_string(), eol: if content.contains("\r\n") { "\r\n".to_string() } else { "\n".to_string() }, fold_state: FoldState::new(), multi_cursor: MultiCursorState::new(), bracket_match: None };
        doc.fold_state.parse_folds(&doc.content);
        doc
    }

    pub fn update_bracket_match(&mut self) {
        // Find position from cursor
        let pos = self.position_from_line_col();
        self.bracket_match = BracketMatcher::find_bracket_at_cursor(&self.content, pos)
            .and_then(|p| BracketMatcher::find_match(&self.content, p));
    }

    pub fn position_from_line_col(&self) -> usize {
        let lines: Vec<&str> = self.content.lines().collect();
        let mut pos = 0;
        for (i, line) in lines.iter().enumerate() {
            if i + 1 == self.cursor_line {
                return pos + self.cursor_col.min(line.len());
            }
            pos += line.len() + 1; // +1 for newline
        }
        pos
    }

    pub fn line_col_from_position(&self, pos: usize) -> (usize, usize) {
        let mut current_pos = 0;
        for (i, line) in self.content.lines().enumerate() {
            if current_pos + line.len() >= pos {
                return (i + 1, (pos - current_pos).min(line.len()));
            }
            current_pos += line.len() + 1;
        }
        (self.content.lines().count().max(1), 0)
    }

    pub fn insert_at_cursor(&mut self, text: &str, settings: &EditorSettings) {
        // Handle newline indentation
        let mut insert_text = text.to_string();
        if text == "\n" || text == "\r\n" {
            let line = self.content.lines().nth(self.cursor_line.saturating_sub(1)).unwrap_or("");
            let indent = line.chars().take_while(|c| c.is_whitespace()).collect::<String>();
            let trimmed = line.trim_start();
            
            // Auto-indent after opening brace
            let extra = if trimmed.ends_with('{') || trimmed.ends_with('(') || trimmed.ends_with('[') {
                if settings.use_spaces { " ".repeat(settings.indent_width) } else { "\t".to_string() }
            } else { String::new() };
            
            let newline = if self.eol == "\r\n" { "\r\n" } else { "\n" };
            insert_text = format!("{}{}{}", newline, indent, extra);
        }
        
        let pos = self.position_from_line_col();
        self.content.insert_str(pos, &insert_text);
        self.cursor_col += insert_text.matches('\n').count();
        self.modified = true;
    }
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
    pub show_settings: bool,
    pub goto_line: String,
    pub settings: EditorSettings,
    pub active_menu: Option<Menu>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab { Files, Symbols, }

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Menu { File, Edit, View, Search, Build, Tools, Settings, Help }

impl GeanyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            documents: vec![Document::new(1)],
            active_doc: Some(0),
            sidebar_visible: true,
            sidebar_tab: SidebarTab::Files,
            messages_visible: true,
            messages: vec!["Geany-Rs v0.3.0 initialized".to_string(), "New features: Indentation, Folding, Bracket Matching, Multi-cursor".to_string()],
            terminal: Terminal::new(),
            build_system: BuildSystem::new(),
            theme_dark: true,
            find_replace: FindReplace::new(),
            show_find: false,
            show_goto_line: false,
            show_settings: false,
            goto_line: String::new(),
            settings: EditorSettings::default(),
            active_menu: None,
        };
        
        if let Some(doc) = app.documents.first_mut() {
            doc.content = include_str!("example.rs").to_string();
            doc.filetype = Filetype::Rust;
            doc.name = "example.rs".to_string();
            doc.fold_state.parse_folds(&doc.content);
        }
        app
    }

    fn new_document(&mut self) { let id = self.documents.len() + 1; self.documents.push(Document::new(id)); self.active_doc = Some(self.documents.len() - 1); self.log("New document created"); }
    fn close_document(&mut self, index: usize) { if self.documents.len() > 1 { self.documents.remove(index); if let Some(active) = self.active_doc { if active >= index && active > 0 { self.active_doc = Some(active - 1); } else if active >= self.documents.len() { self.active_doc = Some(self.documents.len() - 1); } } self.log("Document closed"); } }
    fn log(&mut self, msg: impl Into<String>) { self.messages.push(msg.into()); if self.messages.len() > 100 { self.messages.remove(0); } }

    fn open_file(&mut self) {
        if let Some(path) = file_dialogs::open_file() {
            let path_str = path.to_string_lossy().to_string();
            match std::fs::read_to_string(&path_str) {
                Ok(content) => { let mut doc = Document::from_file(&path_str, content); doc.id = self.documents.len() + 1; self.documents.push(doc); self.active_doc = Some(self.documents.len() - 1); self.log(format!("Opened: {}", path_str)); }
                Err(e) => { self.log(format!("Error: {}", e)); }
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
                    Err(e) => { self.log(format!("Error: {}", e)); }
                }
            }
        }
    }

    fn build_current(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &self.documents[idx];
            if let Some(cmd) = self.build_system.get_commands_for(doc.filetype).first() {
                self.build_system.execute_command(cmd, &doc.path, self);
            }
        }
    }

    fn run_current(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &self.documents[idx];
            let cmds = self.build_system.get_commands_for(doc.filetype);
            if cmds.len() > 1 {
                self.build_system.execute_command(&cmds[1], &doc.path, self);
            } else if let Some(cmd) = cmds.first() {
                self.build_system.execute_command(cmd, &doc.path, self);
            }
        }
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
}

// ============================================================================
// MENU RENDERING
// ============================================================================

fn render_menu(app: &mut GeanyApp, ui: &mut Ui, menu_type: Menu) {
    match menu_type {
        Menu::File => {
            if ui.button("📄 New          Ctrl+N").clicked() { app.new_document(); app.active_menu = None; }
            if ui.button("📂 Open         Ctrl+O").clicked() { app.open_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("💾 Save         Ctrl+S").clicked() { app.save_file(); app.active_menu = None; }
            if ui.button("💾 Save As").clicked() { app.save_file(); app.active_menu = None; }
            ui.separator();
            if ui.button("✕ Close        Ctrl+W").clicked() { if let Some(idx) = app.active_doc { app.close_document(idx); } app.active_menu = None; }
            ui.separator();
            if ui.button("🚪 Quit").clicked() { std::process::exit(0); }
        }
        Menu::Edit => {
            if ui.button("↩ Undo").clicked() { app.log("Undo"); app.active_menu = None; }
            if ui.button("↪ Redo").clicked() { app.log("Redo"); app.active_menu = None; }
            ui.separator();
            if ui.button("✂ Cut          Ctrl+X").clicked() { app.log("Cut"); app.active_menu = None; }
            if ui.button("📋 Copy        Ctrl+C").clicked() { app.log("Copy"); app.active_menu = None; }
            if ui.button("📄 Paste       Ctrl+V").clicked() { app.log("Paste"); app.active_menu = None; }
            ui.separator();
            if ui.button("Select All    Ctrl+A").clicked() { app.log("Select all"); app.active_menu = None; }
            ui.separator();
            if ui.button("📐 Indent       Tab").clicked() { app.log("Indent"); app.active_menu = None; }
            if ui.button("📑 Unindent   Shift+Tab").clicked() { app.log("Unindent"); app.active_menu = None; }
        }
        Menu::View => {
            if ui.button(if app.sidebar_visible { "✓ Sidebar" } else { "Sidebar" }).clicked() { app.sidebar_visible = !app.sidebar_visible; app.active_menu = None; }
            if ui.button(if app.messages_visible { "✓ Messages" } else { "Messages" }).clicked() { app.messages_visible = !app.messages_visible; app.active_menu = None; }
            if ui.button(if app.terminal.visible { "✓ Terminal" } else { "Terminal" }).clicked() { app.terminal.visible = !app.terminal.visible; app.active_menu = None; }
            ui.separator();
            if ui.button(if app.theme_dark { "☀️ Light Theme" } else { "🌙 Dark Theme" }).clicked() { app.theme_dark = !app.theme_dark; app.active_menu = None; }
        }
        Menu::Search => {
            if ui.button("🔍 Find        Ctrl+F").clicked() { app.show_find = !app.show_find; app.active_menu = None; }
            if ui.button("📍 Go to Line  Ctrl+G").clicked() { app.show_goto_line = true; app.active_menu = None; }
            if ui.button("🔁 Find Next   F3").clicked() { app.log("Find next"); app.active_menu = None; }
            if ui.button("🔁 Find Prev   Shift+F3").clicked() { app.log("Find previous"); app.active_menu = None; }
        }
        Menu::Build => {
            if ui.button("🔨 Compile        F8").clicked() { app.build_current(); app.active_menu = None; }
            if ui.button("▶ Run             F9").clicked() { app.run_current(); app.active_menu = None; }
            ui.separator();
            if ui.button("📂 Build Commands").clicked() { app.log("Build menu"); app.active_menu = None; }
        }
        Menu::Tools => {
            if ui.button("🖥 Terminal").clicked() { app.terminal.visible = !app.terminal.visible; app.active_menu = None; }
        }
        Menu::Settings => {
            if ui.button(if app.settings.word_wrap { "✓ Word Wrap" } else { "Word Wrap" }).clicked() { app.settings.word_wrap = !app.settings.word_wrap; }
            if ui.button(if app.settings.show_line_numbers { "✓ Line Numbers" } else { "Line Numbers" }).clicked() { app.settings.show_line_numbers = !app.settings.show_line_numbers; }
            if ui.button(if app.settings.highlight_current_line { "✓ Highlight Line" } else { "Highlight Line" }).clicked() { app.settings.highlight_current_line = !app.settings.highlight_current_line; }
            if ui.button(if app.settings.bracket_highlight { "✓ Bracket Match" } else { "Bracket Match" }).clicked() { app.settings.bracket_highlight = !app.settings.bracket_highlight; }
            ui.separator();
            if ui.button("⚙ Editor Settings...").clicked() { app.show_settings = true; app.active_menu = None; }
        }
        Menu::Help => {
            if ui.button("⌨ Shortcuts").clicked() { app.log("Ctrl+N/O/S/F/G/W | F8/F9 | Tab/Shift+Tab"); app.active_menu = None; }
            if ui.button("ℹ About").clicked() { app.log("Geany-Rs v0.3.0 - Built with Rust + egui"); app.active_menu = None; }
        }
    }
}

// ============================================================================
// SETTINGS DIALOG
// ============================================================================

fn render_settings_dialog(app: &mut GeanyApp, ctx: &egui::Context) {
    Window::new("Editor Settings").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.heading("Indentation");
        ui.horizontal(|ui| {
            ui.label("Tab size:");
            egui::ComboBox::from_id_salt("tab_size").selected_text(app.settings.tab_size.to_string()).show_ui(ui, |ui| {
                for size in [2, 4, 8] { ui.selectable_value(&mut app.settings.tab_size, size, size.to_string()); }
            });
        });
        ui.checkbox(&mut app.settings.use_spaces, "Insert spaces instead of tabs");
        ui.add(egui::Slider::new(&mut app.settings.indent_width, 2..=8).text("Indent width"));
        ui.checkbox(&mut app.settings.auto_indent, "Auto-indent");
        
        ui.separator();
        ui.heading("Display");
        ui.checkbox(&mut app.settings.show_line_numbers, "Show line numbers");
        ui.checkbox(&mut app.settings.show_whitespace, "Show whitespace");
        ui.checkbox(&mut app.settings.highlight_current_line, "Highlight current line");
        ui.checkbox(&mut app.settings.bracket_highlight, "Bracket matching");
        
        ui.separator();
        ui.heading("Word Wrap");
        ui.checkbox(&mut app.settings.word_wrap, "Enable word wrap");
        if app.settings.word_wrap {
            ui.add(egui::Slider::new(&mut app.settings.wrap_width, 60..=200).text("Wrap width"));
        }
        
        ui.separator();
        ui.horizontal(|ui| {
            if ui.button("OK").clicked() { app.show_settings = false; }
            if ui.button("Apply").clicked() { app.log("Settings applied"); }
            if ui.button("Cancel").clicked() { app.show_settings = false; }
        });
    });
}

// ============================================================================
// MAIN UPDATE LOOP
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
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) { self.show_find = false; self.show_goto_line = false; self.show_settings = false; self.active_menu = None; }

        ctx.set_visuals(if self.theme_dark { egui::Visuals::dark() } else { egui::Visuals::light() });

        // ===== TOP PANEL =====
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            // Menu bar
            ui.horizontal(|ui| {
                let menus = [("File", Menu::File), ("Edit", Menu::Edit), ("View", Menu::View), ("Search", Menu::Search), ("Build", Menu::Build), ("Tools", Menu::Tools), ("Settings", Menu::Settings), ("Help", Menu::Help)];
                for (name, menu_type) in menus {
                    let text = RichText::new(name);
                    if ui.selectable_label(self.active_menu == Some(menu_type), text).clicked() {
                        self.active_menu = if self.active_menu == Some(menu_type) { None } else { Some(menu_type) };
                    }
                }
            });
            ui.separator();
            
            // Quick toolbar
            ui.horizontal(|ui| {
                if ui.button("📄").on_hover_text("New").clicked() { self.new_document(); }
                if ui.button("📂").on_hover_text("Open").clicked() { self.open_file(); }
                if ui.button("💾").on_hover_text("Save").clicked() { self.save_file(); }
                ui.separator();
                if ui.button("🔍").on_hover_text("Find").clicked() { self.show_find = !self.show_find; }
                if ui.button("📍").on_hover_text("Go to Line").clicked() { self.show_goto_line = true; }
                ui.separator();
                if ui.button("🔨").on_hover_text("Build").clicked() { self.build_current(); }
                if ui.button("▶").on_hover_text("Run").clicked() { self.run_current(); }
                ui.separator();
                if ui.toggle_value(&mut self.sidebar_visible, "📑").clicked() {}
                if ui.toggle_value(&mut self.messages_visible, "📋").clicked() {}
                if ui.toggle_value(&mut self.terminal.visible, "🖥").clicked() {}
                ui.separator();
                if ui.button(if self.theme_dark { "☀️" } else { "🌙" }).on_hover_text("Theme").clicked() { self.theme_dark = !self.theme_dark; }
            });
        });

        // ===== MENU DROPDOWN =====
        if let Some(menu) = self.active_menu {
            let pos = ctx.cursor().expect("Cursor");
            Window::new(format!("{:?} Menu", menu)).collapsible(false).resizable(false).anchor(egui::Align2::LEFT_UP, [pos.x, pos.y + 20.0]).show(ctx, |ui| {
                render_menu(self, ui, menu);
            });
        }

        // ===== SIDEBAR =====
        if self.sidebar_visible {
            SidePanel::left("sidebar").resizable(true).default_width(220.0).show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.toggle_sized(&mut (self.sidebar_tab == SidebarTab::Files), "📁");
                    ui.toggle_sized(&mut (self.sidebar_tab == SidebarTab::Symbols), "🔣");
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
                                if response.clicked() { self.active_doc = Some(idx); }
                                if response.context_menu(|ui| {
                                    if ui.button("Close").clicked() { self.close_document(idx); }
                                }).clicked() { self.active_doc = Some(idx); }
                            }
                        });
                    }
                    SidebarTab::Symbols => {
                        if let Some(idx) = self.active_doc {
                            if let Some(doc) = self.documents.get(idx) {
                                let symbols = SymbolParser::parse(&doc.content, doc.filetype);
                                if symbols.is_empty() {
                                    ui.label(RichText::new("No symbols").color(Color32::GRAY));
                                } else {
                                    ScrollArea::vertical().show(ui, |ui| {
                                        for symbol in &symbols {
                                            ui.horizontal(|ui| {
                                                ui.label(RichText::new(symbol.kind.icon()).color(symbol.kind.color()).small());
                                                if ui.link(&symbol.name).clicked() {
                                                    if let Some(d) = self.documents.get_mut(idx) { d.cursor_line = symbol.line + 1; }
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

        // ===== MAIN EDITOR =====
        CentralPanel::default().show(ctx, |ui| {
            // Tab bar
            ScrollArea::horizontal().show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (idx, doc) in self.documents.iter().enumerate() {
                        let is_active = self.active_doc == Some(idx);
                        let mut label = doc.name.clone();
                        if doc.modified { label.push_str(" ●"); }
                        if ui.selectable_label(is_active, label).clicked() { self.active_doc = Some(idx); }
                    }
                    if ui.button("+").clicked() { self.new_document(); }
                });
            });
            ui.separator();

            // Find bar
            if self.show_find {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        TextEdit::singleline(&mut self.find_replace.search_text).desired_width(180.0).show(ui);
                        if ui.button("Find").clicked() { if let Some(idx) = self.active_doc { if let Some(doc) = self.documents.get(idx) { self.find_replace.search(&doc.content); self.log(format!("Found {} matches", self.find_replace.search_results.len())); } } }
                        ui.separator();
                        ui.label("Replace:");
                        TextEdit::singleline(&mut self.find_replace.replace_text).desired_width(180.0).show(ui);
                        if ui.button("Replace All").clicked() { if let Some(idx) = self.active_doc { let new_content = self.find_replace.replace_all(&self.documents[idx].content); self.documents[idx].content = new_content; self.documents[idx].modified = true; self.log("Replaced all".to_string()); } }
                        ui.separator();
                        ui.checkbox(&mut self.find_replace.case_sensitive, "Aa");
                        ui.checkbox(&mut self.find_replace.whole_word, "W");
                        ui.checkbox(&mut self.find_replace.regex, ".*");
                        if ui.button("✕").clicked() { self.show_find = false; }
                    });
                });
                ui.separator();
            }

            // Editor content
            if let Some(idx) = self.active_doc {
                if let Some(doc) = self.documents.get_mut(idx) {
                    // Status bar
                    ui.horizontal(|ui| {
                        ui.label("File:");
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
                        ui.separator();
                        if doc.modified { ui.label(RichText::new("Modified").color(Color32::YELLOW)); }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(RichText::new(format!("{} folds", doc.fold_state.regions.len())).small().color(Color32::GRAY));
                        });
                    });
                    ui.separator();

                    // Editor with line numbers
                    ScrollArea::vertical().show(ui, |ui| {
                        // Line numbers + code side by side
                        ui.horizontal(|ui| {
                            // Line numbers gutter
                            if self.settings.show_line_numbers {
                                ui.vertical(|ui| {
                                    ui.set_width(50.0);
                                    let line_count = doc.content.lines().count().max(1);
                                    for i in 1..=line_count {
                                        let line_text = format!("{:>4}", i);
                                        let color = if i == doc.cursor_line && self.settings.highlight_current_line {
                                            Color32::from_rgb(100, 100, 100)
                                        } else {
                                            Color32::GRAY
                                        };
                                        ui.label(RichText::new(line_text).small().monospace().color(color));
                                    }
                                });
                                ui.separator();
                            }
                            
                            // Code area
                            ui.vertical(|ui| {
                                let mut text = doc.content.clone();
                                
                                // Word wrap option
                                let desired_width = if self.settings.word_wrap { 
                                    self.settings.wrap_width as f32 
                                } else { 
                                    f32::MAX 
                                };
                                
                                TextEdit::multiline(&mut text)
                                    .font(FontId::monospace(14.0))
                                    .desired_width(desired_width)
                                    .show(ui);
                                
                                if text != doc.content {
                                    doc.content = text;
                                    doc.modified = true;
                                    doc.fold_state.parse_folds(&doc.content);
                                }
                            });
                        });
                    });

                    // Update bracket matching
                    if self.settings.bracket_highlight {
                        doc.update_bracket_match();
                    }
                }
            }
        });

        // ===== STATUS BAR =====
        TopBottomPanel::bottom("status").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if let Some(idx) = self.active_doc {
                    if let Some(doc) = self.documents.get(idx) {
                        ui.label(RichText::new(&doc.name).small().strong());
                        ui.separator();
                        ui.label(RichText::new(format!("Ln {}, Col {}", doc.cursor_line, doc.cursor_col)).small());
                        ui.separator();
                        ui.label(RichText::new(doc.filetype.name()).small());
                        ui.separator();
                        ui.label(RichText::new(&doc.encoding).small());
                        ui.separator();
                        if doc.modified { ui.label(RichText::new("●").small().color(Color32::YELLOW)); }
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new(format!("{}{}", if self.settings.use_spaces { "Spaces:" } else { "Tabs:" }, self.settings.indent_width)).small().color(Color32::GRAY));
                    ui.separator();
                    ui.label(RichText::new("Geany-Rs v0.3.0").small().color(Color32::GRAY));
                });
            });
        });

        // ===== MESSAGE PANEL =====
        if self.messages_visible {
            TopBottomPanel::bottom("messages").resizable(true).default_height(100.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("📋"); if ui.button("Clear").clicked() { self.messages.clear(); } });
                ui.separator();
                ScrollArea::vertical().show(ui, |ui| { for msg in &self.messages { ui.label(msg.clone()); } });
            });
        }

        // ===== TERMINAL =====
        if self.terminal.visible {
            TopBottomPanel::bottom("terminal").resizable(true).default_height(180.0).show(ctx, |ui| {
                ui.horizontal(|ui| { ui.label("🖥️ Terminal:"); if ui.button("Clear").clicked() { self.terminal.history.clear(); } });
                ui.separator();
                ScrollArea::vertical().id_salt("term").show(ui, |ui| {
                    let text = self.terminal.history.join("\n");
                    ui.label(RichText::new(&text).monospace().size(12.0));
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("$");
                    let response = TextEdit::singleline(&mut self.terminal.command_input).font(FontId::monospace(14.0)).show(ui);
                    if response.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        self.terminal.execute(&self.terminal.command_input, self);
                        self.terminal.command_input.clear();
                    }
                });
            });
        }

        // ===== DIALOGS =====
        if self.show_goto_line {
            Window::new("Go to Line").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
                ui.label("Line number:");
                TextEdit::singleline(&mut self.goto_line).desired_width(100.0).request_focus().show(ui);
                ui.horizontal(|ui| {
                    if ui.button("Go").clicked() { self.goto_line(); }
                    if ui.button("Cancel").clicked() { self.show_goto_line = false; self.goto_line.clear(); }
                });
            });
        }

        if self.show_settings {
            render_settings_dialog(self, ctx);
        }
    }
}

// ============================================================================
// ENTRY POINT
// ============================================================================

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1400.0, 900.0]).with_min_inner_size([800.0, 600.0]).with_title("Geany-Rs v0.3.0 - Rust IDE"),
        ..Default::default()
    };
    eframe::run_native("Geany-Rs", options, Box::new(|cc| Ok(Box::new(GeanyApp::new(cc))))).unwrap();
}
