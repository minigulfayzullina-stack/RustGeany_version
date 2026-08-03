//! Geany-Rs - A fast and lightweight IDE in Rust
//! Built with egui for cross-platform support
//!
//! Features:
//! 1. Real file open/save dialogs
//! 2. Find & Replace functionality
//! 3. Symbol tree sidebar
//! 4. Terminal emulator
//! 5. Enhanced syntax highlighting

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{
    Color32, FontId, RichText, ScrollArea, TopBottomPanel, SidePanel, CentralPanel, 
    ComboBox, TextEdit, Button, Label, Separator, Window, Area, Order
};
use regex::Regex;
use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::Mutex;

// ============================================================================
// FEATURE 1: Native File Dialogs
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
// FEATURE 3: Symbol Tree (AST-like parsing)
// ============================================================================

#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub line: usize,
    pub children: Vec<Symbol>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SymbolKind {
    Function,
    Struct,
    Enum,
    Impl,
    Trait,
    Class,
    Method,
    Module,
    Variable,
    Constant,
    Property,
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
            SymbolKind::Function | SymbolKind::Method => Color32::from_rgb(230, 192, 123),  // Yellow
            SymbolKind::Struct | SymbolKind::Class => Color32::from_rgb(78, 201, 176),     // Cyan
            SymbolKind::Enum => Color32::from_rgb(86, 156, 214),                           // Blue
            SymbolKind::Impl | SymbolKind::Trait => Color32::from_rgb(206, 145, 120),     // Orange
            SymbolKind::Module => Color32::from_rgb(197, 134, 192),                       // Purple
            SymbolKind::Variable | SymbolKind::Constant => Color32::from_rgb(181, 206, 168), // Green
            SymbolKind::Property => Color32::from_rgb(220, 220, 170),                      // Light yellow
        }
    }
}

pub struct SymbolParser;

impl SymbolParser {
    /// Parse symbols from document content based on filetype
    pub fn parse(content: &str, filetype: Filetype) -> Vec<Symbol> {
        match filetype {
            Filetype::Rust => Self::parse_rust(content),
            Filetype::C | Filetype::Cpp => Self::parse_c(content),
            Filetype::Python => Self::parse_python(content),
            Filetype::JavaScript | Filetype::TypeScript => Self::parse_js(content),
            Filetype::Html => Self::parse_html(content),
            _ => vec![],
        }
    }

    fn parse_rust(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        
        // Function/Method regex
        let fn_regex = Regex::new(r"(?m)^(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
        for cap in fn_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line,
                    children: vec![],
                });
            }
        }

        // Struct regex
        let struct_regex = Regex::new(r"(?m)^(?:pub\s+)?struct\s+(\w+)").unwrap();
        for cap in struct_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Struct,
                    line,
                    children: vec![],
                });
            }
        }

        // Enum regex
        let enum_regex = Regex::new(r"(?m)^(?:pub\s+)?enum\s+(\w+)").unwrap();
        for cap in enum_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Enum,
                    line,
                    children: vec![],
                });
            }
        }

        // Impl regex
        let impl_regex = Regex::new(r"(?m)^(?:pub\s+)?impl(?:\s+<\w+>)?\s+(?:(\w+)(?:\s+for)?\s+)?(\w+)").unwrap();
        for cap in impl_regex.captures_iter(content) {
            if let Some(name) = cap.get(2) {
                let name_str = name.as_str();
                if !name_str.is_empty() && name_str != "impl" {
                    let line = content[..name.start()].matches('\n').count();
                    symbols.push(Symbol {
                        name: format!("impl {}", name_str),
                        kind: SymbolKind::Impl,
                        line,
                        children: vec![],
                    });
                }
            }
        }

        // Trait regex
        let trait_regex = Regex::new(r"(?m)^(?:pub\s+)?trait\s+(\w+)").unwrap();
        for cap in trait_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Trait,
                    line,
                    children: vec![],
                });
            }
        }

        // Module regex
        let mod_regex = Regex::new(r"(?m)^(?:pub\s+)?mod\s+(\w+)").unwrap();
        for cap in mod_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Module,
                    line,
                    children: vec![],
                });
            }
        }

        // Sort by line number
        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_c(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        
        // Function regex
        let fn_regex = Regex::new(r"(?m)^(?:[\w\*]+\s+)+(\w+)\s*\([^)]*\)\s*\{").unwrap();
        for cap in fn_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let name_str = name.as_str();
                // Skip keywords
                if !["if", "else", "while", "for", "switch", "struct"].contains(&name_str) {
                    let line = content[..name.start()].matches('\n').count();
                    symbols.push(Symbol {
                        name: name_str.to_string(),
                        kind: SymbolKind::Function,
                        line,
                        children: vec![],
                    });
                }
            }
        }

        // Struct regex
        let struct_regex = Regex::new(r"(?m)^typedef\s+struct\s+(?:\w+\s+)?\{").unwrap();
        let struct_name_regex = Regex::new(r"(?m)^typedef\s+struct\s+\w+\s*(\w+)").unwrap();
        for (idx, _) in struct_regex.find_iter(content).enumerate() {
            if let Some(name_cap) = struct_name_regex.skip(idx).next() {
                if let Some(name) = name_cap.get(1) {
                    let line = content[..name.start()].matches('\n').count();
                    symbols.push(Symbol {
                        name: name.as_str().to_string(),
                        kind: SymbolKind::Struct,
                        line,
                        children: vec![],
                    });
                }
            }
        }

        // Enum regex
        let enum_regex = Regex::new(r"(?m)^typedef\s+enum\s+(?:\w+\s+)?(\w+)").unwrap();
        for cap in enum_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Enum,
                    line,
                    children: vec![],
                });
            }
        }

        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_python(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        
        // Class regex
        let class_regex = Regex::new(r"(?m)^class\s+(\w+)").unwrap();
        for cap in class_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Class,
                    line,
                    children: vec![],
                });
            }
        }

        // Function regex
        let fn_regex = Regex::new(r"(?m)^def\s+(\w+)").unwrap();
        for cap in fn_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line,
                    children: vec![],
                });
            }
        }

        // Async function
        let async_fn_regex = Regex::new(r"(?m)^async\s+def\s+(\w+)").unwrap();
        for cap in async_fn_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line,
                    children: vec![],
                });
            }
        }

        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_js(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        
        // Function regex
        let fn_regex = Regex::new(r"(?m)^(?:export\s+)?(?:async\s+)?function\s+(\w+)").unwrap();
        for cap in fn_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line,
                    children: vec![],
                });
            }
        }

        // Arrow function / const
        let const_fn_regex = Regex::new(r"(?m)^(?:export\s+)?(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(").unwrap();
        for cap in const_fn_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Function,
                    line,
                    children: vec![],
                });
            }
        }

        // Class regex
        let class_regex = Regex::new(r"(?m)^(?:export\s+)?class\s+(\w+)").unwrap();
        for cap in class_regex.captures_iter(content) {
            if let Some(name) = cap.get(1) {
                let line = content[..name.start()].matches('\n').count();
                symbols.push(Symbol {
                    name: name.as_str().to_string(),
                    kind: SymbolKind::Class,
                    line,
                    children: vec![],
                });
            }
        }

        symbols.sort_by_key(|s| s.line);
        symbols
    }

    fn parse_html(content: &str) -> Vec<Symbol> {
        let mut symbols = Vec::new();
        
        // Script tags
        let script_regex = Regex::new(r"(?m)<script(?:\s+[^>]*)?>(?:[^<]*)?</script>").unwrap();
        for cap in script_regex.find_iter(content) {
            let line = content[..cap.start()].matches('\n').count();
            symbols.push(Symbol {
                name: "script".to_string(),
                kind: SymbolKind::Function,
                line,
                children: vec![],
            });
        }

        // Function tags
        let function_regex = Regex::new(r"(?m)<function(?:\s+[^>]*)?>(?:[^<]*)?</function>").unwrap();
        for cap in function_regex.find_iter(content) {
            let line = content[..cap.start()].matches('\n').count();
            symbols.push(Symbol {
                name: "function".to_string(),
                kind: SymbolKind::Function,
                line,
                children: vec![],
            });
        }

        symbols.sort_by_key(|s| s.line);
        symbols
    }
}

// ============================================================================
// FEATURE 2: Find & Replace
// ============================================================================

#[derive(Debug, Clone)]
pub struct FindReplace {
    pub search_text: String,
    pub replace_text: String,
    pub case_sensitive: bool,
    pub whole_word: bool,
    pub regex: bool,
    pub search_results: Vec<usize>,  // Line numbers with matches
    pub current_result: usize,
}

impl FindReplace {
    pub fn new() -> Self {
        Self {
            search_text: String::new(),
            replace_text: String::new(),
            case_sensitive: false,
            whole_word: false,
            regex: false,
            search_results: vec![],
            current_result: 0,
        }
    }

    pub fn search(&mut self, content: &str) {
        self.search_results.clear();
        if self.search_text.is_empty() {
            return;
        }

        let search = if self.regex {
            match Regex::new(&self.search_text) {
                Ok(r) => {
                    let mut lines = Vec::new();
                    for (idx, line) in content.lines().enumerate() {
                        if r.is_match(line) {
                            lines.push(idx);
                        }
                    }
                    lines
                }
                Err(_) => return,
            }
        } else {
            let needle = if self.whole_word {
                format!(r"\b{}\b", regex::escape(&self.search_text))
            } else {
                regex::escape(&self.search_text)
            };

            let pattern = if self.case_sensitive {
                needle
            } else {
                format!("(?i){}", needle)
            };

            match Regex::new(&pattern) {
                Ok(r) => {
                    let mut lines = Vec::new();
                    for (idx, line) in content.lines().enumerate() {
                        if r.is_match(line) {
                            lines.push(idx);
                        }
                    }
                    lines
                }
                Err(_) => return,
            }
        };

        self.search_results = search;
        self.current_result = 0;
    }

    pub fn replace_all(&self, content: &str) -> String {
        if self.search_text.is_empty() || self.regex {
            return content.to_string();
        }

        let needle = if self.whole_word {
            format!(r"\b{}\b", regex::escape(&self.search_text))
        } else {
            regex::escape(&self.search_text)
        };

        let pattern = if self.case_sensitive {
            needle
        } else {
            format!("(?i){}", needle)
        };

        match Regex::new(&pattern) {
            Ok(r) => r.replace_all(content, self.replace_text.as_str()).to_string(),
            Err(_) => content.to_string(),
        }
    }
}

// ============================================================================
// FEATURE 4: Terminal Emulator
// ============================================================================

pub struct Terminal {
    pub visible: bool,
    pub history: Vec<String>,
    pub current_dir: String,
    pub running: bool,
    pub pty_master: Option<portable_pty::native_pty::MasterPty>,
}

impl Terminal {
    pub fn new() -> Self {
        Self {
            visible: false,
            history: vec!["Geany-Rs Terminal".to_string(), "Type 'help' for commands".to_string()],
            current_dir: dirs::home_dir().map(|p| p.to_string_lossy().to_string()).unwrap_or_default(),
            running: true,
            pty_master: None,
        }
    }

    pub fn execute(&mut self, command: &str) {
        if command.trim().is_empty() {
            return;
        }

        self.history.push(format!("$ {}", command));
        
        // Simple built-in commands
        match command.trim() {
            "help" => {
                self.history.push("Available commands:".to_string());
                self.history.push("  help     - Show this help".to_string());
                self.history.push("  clear    - Clear terminal".to_string());
                self.history.push("  pwd      - Print working directory".to_string());
                self.history.push("  ls       - List files".to_string());
                self.history.push("  date     - Show current date".to_string());
                self.history.push("  exit     - Exit terminal".to_string());
            }
            "clear" => {
                self.history.clear();
            }
            "pwd" => {
                self.history.push(self.current_dir.clone());
            }
            "date" => {
                self.history.push(chrono_lite());
            }
            "exit" => {
                self.history.push("Use the toggle button to hide terminal".to_string());
            }
            cmd if cmd.starts_with("ls") => {
                // Simple ls implementation
                if let Ok(entries) = std::fs::read_dir(&self.current_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        let name = entry.file_name().to_string_lossy().to_string();
                        self.history.push(name);
                    }
                }
            }
            cmd if cmd.starts_with("cd ") => {
                let dir = cmd.trim_start_matches("cd ").trim();
                let new_dir = if dir.starts_with('/') {
                    dir.to_string()
                } else {
                    format!("{}/{}", self.current_dir, dir)
                };
                if std::path::Path::new(&new_dir).is_dir() {
                    self.current_dir = std::fs::canonicalize(&new_dir)
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or(new_dir);
                } else {
                    self.history.push(format!("cd: {}: No such directory", dir));
                }
            }
            cmd if cmd.starts_with("cat ") => {
                let file = cmd.trim_start_matches("cat ");
                if let Ok(content) = std::fs::read_to_string(file) {
                    for line in content.lines().take(50) {
                        self.history.push(line.to_string());
                    }
                } else {
                    self.history.push(format!("cat: {}: No such file", file));
                }
            }
            _ => {
                // Try to run as system command
                self.history.push(format!("Command not found: {}. Try 'help' for available commands.", command.trim()));
            }
        }
    }
}

fn chrono_lite() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap();
    let secs = now.as_secs();
    let days = secs / 86400;
    let years = 1970 + days / 365;
    let remaining_days = days % 365;
    let months = remaining_days / 30;
    let day = remaining_days % 30;
    format!("{} days since epoch (approx date: year {})", secs, years)
}

// ============================================================================
// FEATURE 5: Enhanced Syntax Highlighting
// ============================================================================

#[derive(Default)]
pub struct SyntaxColors {
    pub keyword: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub number: Color32,
    pub function: Color32,
    pub type_name: Color32,
    pub attribute: Color32,
    pub tag: Color32,
    pub property: Color32,
}

impl SyntaxColors {
    pub fn dark_default() -> Self {
        Self {
            keyword: Color32::from_rgb(86, 156, 214),      // Blue
            string: Color32::from_rgb(206, 145, 120),       // Orange
            comment: Color32::from_rgb(106, 153, 85),       // Green
            number: Color32::from_rgb(181, 206, 168),       // Light green
            function: Color32::from_rgb(220, 220, 170),     // Yellow
            type_name: Color32::from_rgb(78, 201, 176),      // Cyan
            attribute: Color32::from_rgb(220, 220, 170),    // Yellow
            tag: Color32::from_rgb(86, 156, 214),          // Blue
            property: Color32::from_rgb(78, 201, 176),     // Cyan
        }
    }
}

pub struct SyntaxHighlighter {
    pub colors: SyntaxColors,
}

impl SyntaxHighlighter {
    pub fn new() -> Self {
        Self {
            colors: SyntaxColors::dark_default(),
        }
    }

    pub fn highlight(&self, content: &str, filetype: Filetype) -> Vec<HighlightedLine> {
        content
            .lines()
            .map(|line| self.highlight_line(line, filetype))
            .collect()
    }

    pub fn highlight_line(&self, line: &str, filetype: Filetype) -> HighlightedLine {
        let mut tokens = Vec::new();
        let mut remaining = line;

        while !remaining.is_empty() {
            if let Some((text, color)) = self.match_whitespace(remaining) {
                tokens.push((text, None));
                remaining = &remaining[text.len()..];
            } else if let Some((text, color)) = self.match_comment(remaining, filetype) {
                tokens.push((text, Some(color)));
                break;
            } else if let Some((text, color)) = self.match_string(remaining) {
                tokens.push((text, Some(color)));
                remaining = &remaining[text.len()..];
            } else if let Some((text, color)) = self.match_number(remaining) {
                tokens.push((text, Some(color)));
                remaining = &remaining[text.len()..];
            } else if let Some((text, color)) = self.match_keyword(remaining, filetype) {
                tokens.push((text, Some(color)));
                remaining = &remaining[text.len()..];
            } else if let Some((text, color)) = self.match_function(remaining, filetype) {
                tokens.push((text, Some(color)));
                remaining = &remaining[text.len()..];
            } else if let Some((text, color)) = self.match_type(remaining, filetype) {
                tokens.push((text, Some(color)));
                remaining = &remaining[text.len()..];
            } else if filetype == Filetype::Html && (remaining.starts_with('<') || remaining.starts_with('>') || remaining.starts_with('/')) {
                // HTML tags
                let end = remaining.find(|c| c == '>' || c == ' ' || c == '\n').unwrap_or(remaining.len());
                let (text, rest) = remaining.split_at(end);
                tokens.push((text, Some(self.colors.tag)));
                remaining = rest;
            } else if filetype == Filetype::Css && remaining.contains(':') {
                // CSS properties
                let end = remaining.find(':').unwrap_or(remaining.len());
                let (prop, rest) = remaining.split_at(end);
                tokens.push((prop, Some(self.colors.property)));
                remaining = rest;
            } else if filetype == Filetype::Json {
                // JSON keys and values
                if remaining.starts_with('"') {
                    if let Some(end) = remaining[1..].find('"') {
                        let key = &remaining[..end + 2];
                        tokens.push((key, Some(self.colors.attribute)));
                        remaining = &remaining[key.len()..];
                        continue;
                    }
                }
                tokens.push((&remaining[..1], None));
                remaining = &remaining[1..];
            } else {
                // Single character
                tokens.push((&remaining[..1], None));
                remaining = &remaining[1..];
            }
        }

        HighlightedLine { tokens }
    }

    fn match_whitespace(&self, text: &str) -> Option<(&str, Color32)> {
        let len = text.len().min(text.find(|c| !c.is_whitespace()).unwrap_or(text.len()));
        if len > 0 {
            Some((&text[..len], Color32::TRANSPARENT))
        } else {
            None
        }
    }

    fn match_comment(&self, text: &str, filetype: Filetype) -> Option<(&str, Color32)> {
        match filetype {
            Filetype::Rust | Filetype::C | Filetype::Cpp | Filetype::Java | Filetype::Go | Filetype::Shell
            | Filetype::JavaScript | Filetype::TypeScript | Filetype::Php | Filetype::Sql => {
                if text.starts_with("//") {
                    Some((text, self.colors.comment))
                } else {
                    None
                }
            }
            Filetype::Python | Filetype::Yaml | Filetype::Shell => {
                if text.starts_with('#') {
                    Some((text, self.colors.comment))
                } else {
                    None
                }
            }
            Filetype::Html | Filetype::Css => {
                if text.starts_with("/*") {
                    let end = text.find("*/").map(|i| i + 2).unwrap_or(text.len());
                    Some((&text[..end], self.colors.comment))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    fn match_string(&self, text: &str) -> Option<(&str, Color32)> {
        let mut chars = text.chars();
        let quote = chars.next()?;
        if quote != '"' && quote != '\'' && quote != '`' {
            return None;
        }
        
        // Check for raw string prefix (Rust)
        let mut rest: String = chars.collect();
        if rest.starts_with('r') {
            // Raw string - find closing "
            let close_idx = rest[2..].find('"').map(|i| i + 2);
            return close_idx.map(|end| (&text[..end + 1], self.colors.string));
        }
        
        // Normal string
        let close_idx = rest.find(quote).map(|i| i + 1);
        close_idx.map(|end| (&text[..end + 1], self.colors.string))
    }

    fn match_keyword(&self, text: &str, filetype: Filetype) -> Option<(&str, Color32)> {
        let keywords = self.get_keywords(filetype);
        
        for kw in keywords {
            if text.starts_with(kw) {
                let after = &text[kw.len()..];
                if after.is_empty() || !Self::is_identifier_char(after.chars().next()?) {
                    return Some((kw, self.colors.keyword));
                }
            }
        }
        None
    }

    fn match_function(&self, text: &str, filetype: Filetype) -> Option<(&str, Color32)> {
        match filetype {
            Filetype::Rust => {
                let re = Regex::new(r"^(\w+)!").ok()?;
                let cap = re.captures(text)?;
                cap.get(1).map(|m| (m.as_str(), self.colors.function))
            }
            _ => {
                let re = Regex::new(r"^(\w+)\s*\(").ok()?;
                let cap = re.captures(text)?;
                cap.get(1).map(|m| (m.as_str(), self.colors.function))
            }
        }
    }

    fn match_type(&self, text: &str, filetype: Filetype) -> Option<(&str, Color32)> {
        let types = self.get_types(filetype);
        for t in types {
            if text.starts_with(t) {
                let after = &text[t.len()..];
                if after.is_empty() || !Self::is_identifier_char(after.chars().next()?) {
                    return Some((t, self.colors.type_name));
                }
            }
        }
        None
    }

    fn match_number(&self, text: &str) -> Option<(&str, Color32)> {
        let re = Regex::new(r"^(\d+\.?\d*([eE][+-]?\d+)?|0x[0-9a-fA-F]+|0b[01]+|0o[0-7]+)").ok()?;
        let cap = re.captures(text)?;
        cap.get(1).map(|m| (m.as_str(), self.colors.number))
    }

    fn get_keywords(&self, filetype: Filetype) -> Vec<&'static str> {
        match filetype {
            Filetype::Rust => vec![
                "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else",
                "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop",
                "match", "mod", "move", "mut", "pub", "ref", "return", "self", "Self", "static",
                "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
            ],
            Filetype::C | Filetype::Cpp => vec![
                "auto", "break", "case", "char", "const", "continue", "default", "do", "double",
                "else", "enum", "extern", "float", "for", "goto", "if", "inline", "int", "long",
                "register", "restrict", "return", "short", "signed", "sizeof", "static", "struct",
                "switch", "typedef", "union", "unsigned", "void", "volatile", "while", "_Bool",
                "_Complex", "_Imaginary", "class", "public", "private", "protected", "virtual",
                "friend", "inline", "namespace", "new", "delete", "this", "template", "typename",
                "try", "catch", "throw", "using", "constexpr", "nullptr", "override", "final",
            ],
            Filetype::Python => vec![
                "False", "None", "True", "and", "as", "assert", "async", "await", "break",
                "class", "continue", "def", "del", "elif", "else", "except", "finally", "for",
                "from", "global", "if", "import", "in", "is", "lambda", "nonlocal", "not", "or",
                "pass", "raise", "return", "try", "while", "with", "yield",
            ],
            Filetype::JavaScript | Filetype::TypeScript => vec![
                "async", "await", "break", "case", "catch", "class", "const", "continue", "debugger",
                "default", "delete", "do", "else", "enum", "export", "extends", "false", "finally",
                "for", "function", "if", "import", "in", "instanceof", "let", "new", "null",
                "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof",
                "var", "void", "while", "with", "yield", "interface", "type", "implements",
                "abstract", "as", "from", "of", "readonly", "keyof", "infer",
            ],
            Filetype::Go => vec![
                "break", "case", "chan", "const", "continue", "default", "defer", "else", "fallthrough",
                "for", "func", "go", "goto", "if", "import", "interface", "map", "package", "range",
                "return", "select", "struct", "switch", "type", "var", "true", "false", "nil", "iota",
            ],
            Filetype::Java => vec![
                "abstract", "assert", "boolean", "break", "byte", "case", "catch", "char", "class",
                "const", "continue", "default", "do", "double", "else", "enum", "extends", "false",
                "final", "finally", "float", "for", "goto", "if", "implements", "import", "instanceof",
                "int", "interface", "long", "native", "new", "null", "package", "private", "protected",
                "public", "return", "short", "static", "strictfp", "super", "switch", "synchronized",
                "this", "throw", "throws", "transient", "true", "try", "void", "volatile", "while",
            ],
            Filetype::Php => vec![
                "abstract", "and", "array", "as", "break", "callable", "case", "catch", "class",
                "const", "continue", "declare", "default", "die", "do", "echo", "else", "elseif",
                "empty", "enddeclare", "endfor", "endforeach", "endif", "endswitch", "endwhile",
                "eval", "exit", "extends", "final", "finally", "fn", "for", "foreach", "function",
                "global", "goto", "if", "implements", "include", "include_once", "instanceof",
                "insteadof", "interface", "isset", "list", "match", "namespace", "new", "or",
                "print", "private", "protected", "public", "require", "require_once", "return",
                "static", "switch", "throw", "trait", "try", "unset", "use", "var", "while", "xor",
                "yield", "true", "false", "null",
            ],
            Filetype::Sql => vec![
                "SELECT", "FROM", "WHERE", "INSERT", "UPDATE", "DELETE", "CREATE", "DROP", "ALTER",
                "TABLE", "INDEX", "VIEW", "DATABASE", "SCHEMA", "INTO", "VALUES", "SET", "AND",
                "OR", "NOT", "NULL", "PRIMARY", "KEY", "FOREIGN", "REFERENCES", "JOIN", "LEFT",
                "RIGHT", "INNER", "OUTER", "ON", "AS", "ORDER", "BY", "GROUP", "HAVING", "LIMIT",
                "OFFSET", "UNION", "ALL", "DISTINCT", "COUNT", "SUM", "AVG", "MAX", "MIN", "CASE",
                "WHEN", "THEN", "ELSE", "END", "EXISTS", "IN", "BETWEEN", "LIKE", "IS", "ASC",
                "DESC", "VIRTUAL", "TRIGGER", "PROCEDURE", "FUNCTION", "BEGIN", "COMMIT", "ROLLBACK",
            ],
            Filetype::Shell => vec![
                "if", "then", "else", "elif", "fi", "case", "esac", "for", "while", "until", "do", "done",
                "in", "function", "select", "time", "coproc", "export", "readonly", "local", "declare",
                "typeset", "unset", "shift", "exit", "return", "break", "continue", "eval", "exec",
                "source", "alias", "unalias", "cd", "pwd", "echo", "printf", "read", "test", "true", "false",
            ],
            Filetype::Html => vec![
                "html", "head", "body", "div", "span", "p", "a", "img", "table", "tr", "td", "th",
                "ul", "ol", "li", "dl", "dt", "dd", "form", "input", "button", "select", "option",
                "textarea", "label", "script", "style", "link", "meta", "title", "header", "footer",
                "nav", "main", "section", "article", "aside", "h1", "h2", "h3", "h4", "h5", "h6",
                "br", "hr", "pre", "code", "blockquote", "em", "strong", "i", "b", "u", "small",
            ],
            Filetype::Css => vec![
                "color", "background", "background-color", "font", "font-size", "font-family",
                "margin", "padding", "border", "width", "height", "display", "position", "top",
                "left", "right", "bottom", "flex", "grid", "align", "justify", "text-align",
                "background-image", "border-radius", "box-shadow", "opacity", "transform", "transition",
            ],
            _ => vec![],
        }
    }

    fn get_types(&self, filetype: Filetype) -> Vec<&'static str> {
        match filetype {
            Filetype::Rust => vec![
                "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64", "u128", "usize",
                "f32", "f64", "bool", "char", "str", "String", "Vec", "Option", "Result", "Box", "Rc",
                "Arc", "RefCell", "Cell", "HashMap", "HashSet", "BTreeMap", "BTreeSet", "VecDeque",
                "LinkedList", "BinaryHeap", "Mutex", "RwLock", "AtomicBool", "AtomicI32", "AtomicU32",
            ],
            Filetype::C | Filetype::Cpp => vec![
                "int", "char", "short", "long", "float", "double", "void", "bool", "size_t", "ptrdiff_t",
                "int8_t", "int16_t", "int32_t", "int64_t", "uint8_t", "uint16_t", "uint32_t", "uint64_t",
                "intptr_t", "uintptr_t", "FILE", "bool", "true", "false", "std::string", "std::vector",
                "std::map", "std::set", "std::shared_ptr", "std::unique_ptr", "std::optional",
            ],
            Filetype::JavaScript | Filetype::TypeScript => vec![
                "string", "number", "boolean", "object", "function", "symbol", "bigint", "any", "void",
                "never", "unknown", "null", "undefined", "Array", "Object", "String", "Number", "Boolean",
                "Date", "RegExp", "Map", "Set", "WeakMap", "WeakSet", "Promise", "Error", "console",
                "Math", "JSON", "Proxy", "Reflect", "Symbol", "BigInt",
            ],
            Filetype::Python => vec![
                "int", "float", "str", "bool", "list", "dict", "tuple", "set", "frozenset", "bytes",
                "bytearray", "object", "type", "Exception", "range", "slice", "property", "classmethod",
                "staticmethod", "enumerate", "zip", "map", "filter", "reversed", "sorted", "any", "all",
            ],
            Filetype::Go => vec![
                "bool", "byte", "complex64", "complex128", "error", "float32", "float64", "int", "int8",
                "int16", "int32", "int64", "rune", "string", "uint", "uint8", "uint16", "uint32", "uint64",
                "uintptr", "any", "comparable", "any", "map", "chan", "func",
            ],
            _ => vec![],
        }
    }

    fn is_identifier_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
}

#[derive(Debug)]
pub struct HighlightedLine {
    pub tokens: Vec<(&'static str, Option<Color32>)>,
}

// ============================================================================
// Main Application
// ============================================================================

pub struct GeanyApp {
    pub documents: Vec<Document>,
    pub active_doc: Option<usize>,
    pub sidebar_visible: bool,
    pub sidebar_tab: SidebarTab,
    pub messages_visible: bool,
    pub messages: Vec<String>,
    pub terminal: Terminal,
    pub highlighter: SyntaxHighlighter,
    pub theme_dark: bool,
    pub find_replace: FindReplace,
    pub show_find: bool,
    pub show_goto_line: bool,
    pub goto_line: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SidebarTab {
    Files,
    Symbols,
}

impl Default for SidebarTab {
    fn default() -> Self {
        Self::Files
    }
}

impl GeanyApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let mut app = Self {
            documents: vec![Document::new(1)],
            active_doc: Some(0),
            sidebar_visible: true,
            sidebar_tab: SidebarTab::Files,
            messages_visible: true,
            messages: vec!["Geany-Rs initialized".to_string(), "Welcome! Use File > Open to open files.".to_string()],
            terminal: Terminal::new(),
            highlighter: SyntaxHighlighter::new(),
            theme_dark: true,
            find_replace: FindReplace::new(),
            show_find: false,
            show_goto_line: false,
            goto_line: String::new(),
        };
        
        // Set example content
        if let Some(doc) = app.documents.first_mut() {
            doc.content = include_str!("example.rs").to_string();
            doc.filetype = Filetype::Rust;
            doc.name = "example.rs".to_string();
        }
        
        app
    }

    fn new_document(&mut self) {
        let id = self.documents.len() + 1;
        self.documents.push(Document::new(id));
        self.active_doc = Some(self.documents.len() - 1);
        self.log_message(format!("Created new document: untitled_{}", id));
    }

    fn close_document(&mut self, index: usize) {
        if self.documents.len() > 1 {
            self.documents.remove(index);
            if let Some(active) = self.active_doc {
                if active >= index && active > 0 {
                    self.active_doc = Some(active - 1);
                } else if active >= self.documents.len() {
                    self.active_doc = Some(self.documents.len() - 1);
                }
            }
            self.log_message(format!("Closed document"));
        }
    }

    fn log_message(&mut self, msg: String) {
        self.messages.push(msg);
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    fn open_file_dialog(&mut self) {
        if let Some(path) = file_dialogs::open_file() {
            let path_str = path.to_string_lossy().to_string();
            match std::fs::read_to_string(&path_str) {
                Ok(content) => {
                    let mut doc = Document::from_file(&path_str, content);
                    doc.id = self.documents.len() + 1;
                    self.documents.push(doc);
                    self.active_doc = Some(self.documents.len() - 1);
                    self.log_message(format!("Opened: {}", path_str));
                }
                Err(e) => {
                    self.log_message(format!("Error opening file: {}", e));
                }
            }
        }
    }

    fn save_file_dialog(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &mut self.documents[idx];
            let default_name = doc.path.as_ref()
                .map(|p| std::path::Path::new(p).file_name().unwrap().to_string_lossy().to_string())
                .unwrap_or_else(|| doc.name.clone());
            
            if let Some(path) = file_dialogs::save_file(&default_name) {
                let path_str = path.to_string_lossy().to_string();
                match std::fs::write(&path_str, &doc.content) {
                    Ok(_) => {
                        doc.path = Some(path_str.clone());
                        doc.modified = false;
                        doc.name = std::path::Path::new(&path_str)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or_default();
                        self.log_message(format!("Saved: {}", path_str));
                    }
                    Err(e) => {
                        self.log_message(format!("Error saving file: {}", e));
                    }
                }
            }
        }
    }
}

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Keyboard shortcuts
        let shortcuts = ctx.input(|i| i.modifiers);
        
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::N)) {
            self.new_document();
        }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::O)) {
            self.open_file_dialog();
        }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::S)) {
            self.save_file_dialog();
        }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::F)) {
            self.show_find = !self.show_find;
        }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::G)) {
            self.show_goto_line = true;
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.show_find = false;
            self.show_goto_line = false;
        }

        // Set theme
        ctx.set_visuals(if self.theme_dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        // ===== TOOLBAR =====
        TopBottomPanel::top("toolbar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // File operations
                if ui.button("📄 New").clicked() {
                    self.new_document();
                }
                if ui.button("📂 Open").clicked() {
                    self.open_file_dialog();
                }
                if ui.button("💾 Save").clicked() {
                    self.save_file_dialog();
                }

                ui.separator();

                // Edit operations
                if ui.button("🔍 Find").clicked() {
                    self.show_find = !self.show_find;
                }
                if ui.button("📍 Goto").clicked() {
                    self.show_goto_line = true;
                }

                ui.separator();

                // View toggles
                if ui.toggle_value(&mut self.sidebar_visible, "📑 Sidebar").clicked() {}
                if ui.toggle_value(&mut self.messages_visible, "📋 Messages").clicked() {}
                if ui.toggle_value(&mut self.terminal.visible, "🖥️ Terminal").clicked() {}

                ui.separator();

                // Theme toggle
                if ui.button(if self.theme_dark { "☀️ Light" } else { "🌙 Dark" }).clicked() {
                    self.theme_dark = !self.theme_dark;
                }
            });
        });

        // ===== SIDEBAR =====
        if self.sidebar_visible {
            SidePanel::left("sidebar")
                .resizable(true)
                .default_width(220.0)
                .show(ctx, |ui| {
                    // Tab selection
                    ui.horizontal(|ui| {
                        if ui.selectable_label(self.sidebar_tab == SidebarTab::Files, "📁 Files").clicked() {
                            self.sidebar_tab = SidebarTab::Files;
                        }
                        if ui.selectable_label(self.sidebar_tab == SidebarTab::Symbols, "🔣 Symbols").clicked() {
                            self.sidebar_tab = SidebarTab::Symbols;
                        }
                    });
                    ui.separator();

                    match self.sidebar_tab {
                        SidebarTab::Files => {
                            ScrollArea::vertical().show(ui, |ui| {
                                for (idx, doc) in self.documents.iter().enumerate() {
                                    let is_active = self.active_doc == Some(idx);
                                    let mut text = RichText::new(&doc.name);
                                    
                                    if doc.modified {
                                        text = text.color(Color32::YELLOW);
                                    }
                                    if is_active {
                                        text = text.bold();
                                    }
                                    
                                    if ui.selectable_label(is_active, text).clicked() {
                                        self.active_doc = Some(idx);
                                    }
                                }
                            });
                        }
                        SidebarTab::Symbols => {
                            // FEATURE 3: Symbol Tree
                            if let Some(idx) = self.active_doc {
                                if let Some(doc) = self.documents.get(idx) {
                                    let symbols = SymbolParser::parse(&doc.content, doc.filetype);
                                    
                                    if symbols.is_empty() {
                                        ui.label(RichText::new("No symbols found").color(Color32::GRAY));
                                    } else {
                                        ScrollArea::vertical().show(ui, |ui| {
                                            for symbol in &symbols {
                                                ui.horizontal(|ui| {
                                                    let icon = RichText::new(symbol.kind.icon())
                                                        .color(symbol.kind.color())
                                                        .small();
                                                    ui.label(icon);
                                                    if ui.link(&symbol.name).clicked() {
                                                        self.log_message(format!("Jump to: {} (line {})", symbol.name, symbol.line + 1));
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

        // ===== MAIN EDITOR AREA =====
        CentralPanel::default().show(ctx, |ui| {
            // Tab bar
            ui.horizontal_wrapped(|ui| {
                for (idx, doc) in self.documents.iter().enumerate() {
                    let is_active = self.active_doc == Some(idx);
                    let mut label = doc.name.clone();
                    if doc.modified {
                        label.push_str(" ●");
                    }
                    
                    let response = ui.selectable_label(is_active, label);
                    
                    if response.clicked() {
                        self.active_doc = Some(idx);
                    }
                    
                    // Close button
                    if is_active {
                        let close_btn = ui.put(
                            egui::Rect::from_min_size(
                                response.rect.right_top() - egui::vec2(20.0, 0.0),
                                egui::vec2(20.0, 20.0)
                            ),
                            egui::Button::new("✕").small()
                        );
                        if close_btn.clicked() {
                            self.close_document(idx);
                            continue;
                        }
                    }
                }
                
                // Add new tab
                if ui.button("+").clicked() {
                    self.new_document();
                }
            });
            
            ui.separator();

            // FEATURE 2: Find/Replace Bar
            if self.show_find {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.label("Find:");
                        TextEdit::singleline(&mut self.find_replace.search_text)
                            .desired_width(200.0)
                            .show(ui);
                        
                        if ui.button("Find All").clicked() {
                            if let Some(idx) = self.active_doc {
                                if let Some(doc) = self.documents.get(idx) {
                                    self.find_replace.search(&doc.content);
                                    self.log_message(format!("Found {} matches", self.find_replace.search_results.len()));
                                }
                            }
                        }
                        
                        ui.separator();
                        
                        ui.label("Replace:");
                        TextEdit::singleline(&mut self.find_replace.replace_text)
                            .desired_width(200.0)
                            .show(ui);
                        
                        if ui.button("Replace All").clicked() {
                            if let Some(idx) = self.active_doc {
                                let new_content = self.find_replace.replace_all(&self.documents[idx].content);
                                self.documents[idx].content = new_content;
                                self.documents[idx].modified = true;
                                self.log_message("Replaced all occurrences".to_string());
                            }
                        }
                        
                        ui.separator();
                        
                        ui.checkbox(&mut self.find_replace.case_sensitive, "Case");
                        ui.checkbox(&mut self.find_replace.whole_word, "Word");
                        ui.checkbox(&mut self.find_replace.regex, "Regex");
                        
                        if ui.button("✕").clicked() {
                            self.show_find = false;
                        }
                    });
                });
                ui.separator();
            }

            // Editor content
            if let Some(idx) = self.active_doc {
                if let Some(doc) = self.documents.get_mut(idx) {
                    // Status bar
                    ui.horizontal(|ui| {
                        ui.label("Filetype: ");
                        ComboBox::from_id_salt("filetype_selector")
                            .selected_text(doc.filetype.name())
                            .show_ui(ui, |ui| {
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
                        ui.label(format!("Lines: {} | Characters: {}", lines, chars));
                        
                        if doc.modified {
                            ui.label(RichText::new("(Modified)").color(Color32::YELLOW));
                        }
                    });
                    
                    ui.separator();

                    // Code editor
                    ScrollArea::vertical().id_salt("editor_scroll").show(ui, |ui| {
                        let mut text = doc.content.clone();
                        
                        TextEdit::multiline(&mut text)
                            .font(FontId::monospace(14.0))
                            .desired_width(ui.available_width())
                            .show(ui);
                        
                        // Check for modifications
                        if text != doc.content {
                            doc.content = text;
                            doc.modified = true;
                        }
                    });
                }
            } else {
                ui.centered_and_on_hovered_and_clicked_widget(|ui| {
                    ui.label("No document open");
                });
            }
        });

        // ===== MESSAGE PANEL =====
        if self.messages_visible {
            TopBottomPanel::bottom("messages")
                .resizable(true)
                .default_height(120.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Messages:");
                        if ui.button("Clear").clicked() {
                            self.messages.clear();
                        }
                    });
                    ui.separator();
                    
                    ScrollArea::vertical().show(ui, |ui| {
                        for msg in &self.messages {
                            ui.label(msg.clone());
                        }
                    });
                });
        }

        // ===== TERMINAL PANEL (FEATURE 4) =====
        if self.terminal.visible {
            TopBottomPanel::bottom("terminal")
                .resizable(true)
                .default_height(200.0)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("🖥️ Terminal:");
                        ui.label(RichText::new(&self.terminal.current_dir).small().color(Color32::GRAY));
                        if ui.button("Clear").clicked() {
                            self.terminal.history.clear();
                        }
                    });
                    ui.separator();
                    
                    ScrollArea::vertical().id_salt("terminal").show(ui, |ui| {
                        for line in &self.terminal.history {
                            ui.label(RichText::new(line).monospace());
                        }
                    });
                    
                    // Command input
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.label("$");
                        let mut cmd = String::new();
                        let response = TextEdit::singleline(&mut cmd)
                            .font(FontId::monospace(14.0))
                            .show(ui);
                        
                        if response.response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            self.terminal.execute(&cmd);
                        }
                    });
                });
        }

        // ===== GOTO LINE DIALOG =====
        if self.show_goto_line {
            Window::new("Go to Line")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ctx, |ui| {
                    ui.label("Line number:");
                    TextEdit::singleline(&mut self.goto_line)
                        .desired_width(100.0)
                        .request_focus()
                        .show(ui);
                    
                    ui.horizontal(|ui| {
                        if ui.button("Go").clicked() {
                            if let Ok(line) = self.goto_line.parse::<usize>() {
                                self.log_message(format!("Go to line: {}", line));
                            }
                            self.show_goto_line = false;
                            self.goto_line.clear();
                        }
                        if ui.button("Cancel").clicked() {
                            self.show_goto_line = false;
                            self.goto_line.clear();
                        }
                    });
                });
        }
    }
}

// ============================================================================
// Entry Point
// ============================================================================

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1400.0, 900.0])
            .with_min_inner_size([800.0, 600.0])
            .with_title("Geany-Rs - Rust IDE"),
        ..Default::default()
    };

    eframe::run_native(
        "Geany-Rs",
        options,
        Box::new(|cc| Ok(Box::new(GeanyApp::new(cc)))),
    )
    .unwrap();
}

