//! Geany-Rs - A fast and lightweight IDE in Rust
//! Built with egui for cross-platform support

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use egui::{Color32, FontId, TextFormat, TextStyle, Ui, RichText, Label, ScrollArea, TopBottomPanel, SidePanel, CentralPanel, Window};
use std::collections::HashMap;

// ============================================================================
// Core Data Structures
// ============================================================================

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Document {
    pub id: usize,
    pub name: String,
    pub path: Option<String>,
    pub content: String,
    pub filetype: Filetype,
    pub modified: bool,
}

impl Document {
    pub fn new(id: usize) -> Self {
        Self {
            id,
            name: format!("untitled_{}", id),
            path: None,
            content: String::new(),
            filetype: Filetype::PlainText,
            modified: false,
        }
    }

    pub fn from_file(path: &str, content: String) -> Self {
        let name = std::path::Path::new(path)
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string());
        
        let filetype = Filetype::from_extension(
            std::path::Path::new(path)
                .extension()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default()
                .as_str()
        );

        Self {
            id: 0,
            name,
            path: Some(path.to_string()),
            content,
            filetype,
            modified: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Filetype {
    PlainText,
    C,
    Cpp,
    Rust,
    Python,
    JavaScript,
    Html,
    Css,
    Json,
    Markdown,
}

impl Filetype {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "c" => Filetype::C,
            "cpp" | "cc" | "cxx" | "h" | "hpp" => Filetype::Cpp,
            "rs" => Filetype::Rust,
            "py" => Filetype::Python,
            "js" | "mjs" => Filetype::JavaScript,
            "html" | "htm" => Filetype::Html,
            "css" => Filetype::Css,
            "json" => Filetype::Json,
            "md" | "markdown" => Filetype::Markdown,
            _ => Filetype::PlainText,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Filetype::PlainText => "Plain Text",
            Filetype::C => "C",
            Filetype::Cpp => "C++",
            Filetype::Rust => "Rust",
            Filetype::Python => "Python",
            Filetype::JavaScript => "JavaScript",
            Filetype::Html => "HTML",
            Filetype::Css => "CSS",
            Filetype::Json => "JSON",
            Filetype::Markdown => "Markdown",
        }
    }
}

// ============================================================================
// Syntax Highlighting
// ============================================================================

#[derive(Default)]
pub struct SyntaxColors {
    pub keyword: Color32,
    pub string: Color32,
    pub comment: Color32,
    pub number: Color32,
    pub function: Color32,
    pub type_name: Color32,
}

impl SyntaxColors {
    pub fn dark_default() -> Self {
        Self {
            keyword: Color32::from_rgb(86, 156, 214),    // Blue
            string: Color32::from_rgb(206, 145, 120),   // Orange
            comment: Color32::from_rgb(106, 153, 85),   // Green
            number: Color32::from_rgb(181, 206, 168),    // Light green
            function: Color32::from_rgb(220, 220, 170), // Yellow
            type_name: Color32::from_rgb(78, 201, 176), // Cyan
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

    /// Highlight a line of code and return styled text
    pub fn highlight_line(&self, line: &str, filetype: Filetype) -> Vec<(String, Option<Color32>)> {
        let mut result = Vec::new();
        let mut remaining = line;
        
        while !remaining.is_empty() {
            // Try to match different patterns
            if let Some((text, color)) = self.match_comment(remaining, filetype) {
                result.push((text, Some(color)));
                remaining = &remaining[remaining.len()..];
                break;
            } else if let Some((text, color)) = self.match_string(remaining) {
                result.push((text, Some(color)));
                remaining = &remaining[remaining.len()..];
                break;
            } else if let Some((text, color)) = self.match_keyword(remaining, filetype) {
                result.push((text, Some(color)));
                remaining = &remaining[remaining.len()..];
                break;
            } else if let Some((text, color)) = self.match_number(remaining) {
                result.push((text, Some(color)));
                remaining = &remaining[remaining.len()..];
                break;
            } else {
                // Single character without highlighting
                let (ch, rest) = remaining.char_at(0).map_or(("", ""), |(c, r)| (c, r));
                result.push((ch.to_string(), None));
                remaining = rest;
            }
        }
        
        // If there's remaining text, process it recursively
        if !remaining.is_empty() {
            let rest = self.highlight_line(remaining, filetype);
            result.extend(rest);
        }
        
        result
    }

    fn match_comment(&self, text: &str, filetype: Filetype) -> Option<(&str, Color32)> {
        // Check for single-line comments
        if let Some(idx) = text.find("//") {
            return Some((&text[idx..], self.colors.comment));
        }
        // Check for # comments (Python, etc.)
        if let Some(idx) = text.find('#') {
            return Some((&text[idx..], self.colors.comment));
        }
        None
    }

    fn match_string(&self, text: &str) -> Option<(&str, Color32)> {
        let mut chars = text.chars();
        let quote = chars.next()?;
        if quote != '"' && quote != '\'' {
            return None;
        }
        let rest: String = chars.collect();
        if let Some(end) = rest.find(quote) {
            let string_content = &text[..end + 2];
            return Some((string_content, self.colors.string));
        }
        Some((text, self.colors.string))
    }

    fn match_keyword(&self, text: &str, filetype: Filetype) -> Option<(&str, Color32)> {
        let keywords = self.get_keywords(filetype);
        
        for kw in keywords {
            if text.starts_with(kw) {
                let after = &text[kw.len()..];
                // Make sure it's a whole word
                if after.is_empty() || !Self::is_identifier_char(after.chars().next()?) {
                    return Some((kw, self.colors.keyword));
                }
            }
        }
        None
    }

    fn match_number(&self, text: &str) -> Option<(&str, Color32)> {
        let mut chars = text.chars();
        let first = chars.next()?;
        if !first.is_ascii_digit() {
            return None;
        }
        
        let mut end = 1;
        for ch in chars {
            if ch.is_ascii_digit() || ch == '.' || ch == 'x' || ch.is_ascii_hexdigit() {
                end += 1;
            } else {
                break;
            }
        }
        Some((&text[..end], self.colors.number))
    }

    fn get_keywords(&self, filetype: Filetype) -> Vec<&'static str> {
        match filetype {
            Filetype::Rust => vec![
                "fn", "let", "mut", "pub", "impl", "struct", "enum", "trait", "use", "mod",
                "crate", "self", "super", "match", "if", "else", "while", "for", "loop",
                "return", "break", "continue", "true", "false", "Option", "Result", "Vec",
                "String", "Box", "Rc", "Arc", "RefCell", "const", "static", "async", "await",
            ],
            Filetype::C | Filetype::Cpp => vec![
                "int", "char", "float", "double", "void", "long", "short", "unsigned", "signed",
                "const", "static", "extern", "struct", "enum", "union", "typedef", "sizeof",
                "if", "else", "switch", "case", "default", "for", "while", "do", "break",
                "continue", "return", "goto", "NULL", "true", "false", "class", "public",
                "private", "protected", "virtual", "template", "namespace", "using", "try",
                "catch", "throw", "new", "delete", "this", "auto",
            ],
            Filetype::Python => vec![
                "def", "class", "if", "elif", "else", "for", "while", "try", "except",
                "finally", "with", "as", "import", "from", "return", "yield", "raise",
                "pass", "break", "continue", "True", "False", "None", "and", "or", "not",
                "in", "is", "lambda", "global", "nonlocal", "async", "await", "print",
            ],
            Filetype::JavaScript => vec![
                "function", "const", "let", "var", "if", "else", "for", "while", "do",
                "switch", "case", "default", "break", "continue", "return", "throw",
                "try", "catch", "finally", "class", "extends", "new", "this", "super",
                "import", "export", "from", "as", "async", "await", "true", "false",
                "null", "undefined", "typeof", "instanceof", "yield", "static", "get",
                "set", "of", "in",
            ],
            _ => vec![],
        }
    }

    fn is_identifier_char(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }
}

// ============================================================================
// Main Application
// ============================================================================

pub struct GeanyApp {
    pub documents: Vec<Document>,
    pub active_doc: Option<usize>,
    pub sidebar_visible: bool,
    pub messages_visible: bool,
    pub show_open_dialog: bool,
    pub show_save_dialog: bool,
    pub messages: Vec<String>,
    pub highlighter: SyntaxHighlighter,
    pub theme_dark: bool,
}

impl GeanyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Use default fonts (eframe provides them)
        // Fonts are configured automatically

        let mut app = Self {
            documents: vec![Document::new(1)],
            active_doc: Some(0),
            sidebar_visible: true,
            messages_visible: true,
            show_open_dialog: false,
            show_save_dialog: false,
            messages: vec!["Geany-Rs initialized".to_string()],
            highlighter: SyntaxHighlighter::new(),
            theme_dark: true,
        };
        
        // Set initial document content as example
        if let Some(doc) = app.documents.first_mut() {
            doc.content = r#"// Welcome to Geany-Rs!
// A fast and lightweight IDE built with Rust and egui

fn main() {
    println!("Hello, Geany-Rs!");
    
    let message = "Syntax highlighting is working!";
    println!("{}", message);
    
    // Try editing this file
    let numbers = vec![1, 2, 3, 4, 5];
    for num in numbers {
        println!("Number: {}", num);
    }
}
"#.to_string();
            doc.filetype = Filetype::Rust;
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
        // Keep only last 100 messages
        if self.messages.len() > 100 {
            self.messages.remove(0);
        }
    }

    fn save_current_document(&mut self) {
        if let Some(idx) = self.active_doc {
            let doc = &mut self.documents[idx];
            doc.modified = false;
            self.log_message(format!("Saved: {}", doc.name));
        }
    }

    fn open_file(&mut self) {
        // In a real app, this would use native file dialog
        // For now, we'll simulate opening a file
        let content = r#"// New Rust file
fn example() {
    println!("Hello from new file!");
}
"#.to_string();
        
        let mut doc = Document::from_file("new_file.rs", content);
        doc.id = self.documents.len() + 1;
        self.documents.push(doc);
        self.active_doc = Some(self.documents.len() - 1);
        self.log_message("Opened: new_file.rs".to_string());
    }
}

impl eframe::App for GeanyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle keyboard shortcuts
        let shortcuts = ctx.input(|i| i.modifiers);
        
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::N)) {
            self.new_document();
        }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::S)) {
            self.save_current_document();
        }
        if shortcuts.cmd && ctx.input(|i| i.key_pressed(egui::Key::O)) {
            self.open_file();
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
                    self.open_file();
                }
                if ui.button("💾 Save").clicked() {
                    self.save_current_document();
                }

                ui.separator();

                // View toggles
                if ui.toggle_value(&mut self.sidebar_visible, "📑 Sidebar").clicked() {
                    // Toggle sidebar
                }
                if ui.toggle_value(&mut self.messages_visible, "📋 Messages").clicked() {
                    // Toggle messages
                }

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
                .default_width(200.0)
                .show(ctx, |ui| {
                    ui.heading("Files");
                    ui.separator();

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
                });
        }

        // ===== MAIN EDITOR AREA =====
        CentralPanel::default().show(ctx, |ui| {
            // Tab bar
            ui.horizontal(|ui| {
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
                    
                    // Close button on hover
                    if is_active && ui.button("✕").on_hover_text("Close").clicked() {
                        self.close_document(idx);
                        continue;
                    }
                }
                
                // Add new tab button
                if ui.button("+").on_hover_text("New tab").clicked() {
                    self.new_document();
                }
            });
            
            ui.separator();

            // Editor content
            if let Some(idx) = self.active_doc {
                if let Some(doc) = self.documents.get_mut(idx) {
                    // Filetype indicator
                    ui.horizontal(|ui| {
                        ui.label("Filetype: ");
                        ui.label(RichText::new(doc.filetype.name()).color(Color32::KHAKI));
                    });
                    
                    ui.separator();

                    // Code editor with syntax highlighting
                    ScrollArea::vertical().id_salt("editor_scroll").show(ui, |ui| {
                        let text_format = TextFormat {
                            font: FontId::monospace(14.0),
                            color: Color32::WHITE,
                            ..Default::default()
                        };

                        let mut layouter = |ui: &Ui, string: &str, _wrap_width: f32| {
                            let mut layout = ui.fonts().layout_no_visual("#".to_string());
                            let height = layout.rect.height();

                            let mut underlines: Vec<egui::Stroke> = Vec::new();
                            let mut texts = Vec::new();
                            
                            for (line_idx, line) in string.lines().enumerate() {
                                let highlighted = self.highlighter.highlight_line(line, doc.filetype);
                                
                                for (text, color) in highlighted {
                                    let color = color.unwrap_or(Color32::WHITE);
                                    let text_format = TextFormat {
                                        font: FontId::monospace(14.0),
                                        color,
                                        ..Default::default()
                                    };
                                    let layout = ui.fonts().layout_no_visual(text.to_string());
                                    texts.push((layout, text_format.color));
                                }
                                
                                // Line ending
                                let layout = ui.fonts().layout_no_visual(" ".to_string());
                                texts.push((layout, Color32::TRANSPARENT));
                            }
                            
                            // Build final layout with all lines
                            let mut result = Vec::new();
                            for (line_idx, line) in string.lines().enumerate() {
                                let mut x = 0.0;
                                let highlighted = self.highlighter.highlight_line(line, doc.filetype);
                                
                                for (text, color) in highlighted {
                                    let color = color.unwrap_or(Color32::WHITE);
                                    let text_format = TextFormat {
                                        font: FontId::monospace(14.0),
                                        color,
                                        ..Default::default()
                                    };
                                    let galley = ui.fonts().layout_no_visual(text.to_string());
                                    result.push(egui::epaint::TextShape {
                                        pos: egui::pos2(x, line_idx as f32 * height),
                                        galley: Box::new(galley.galley),
                                        override_text_color: Some(color),
                                        underline: egui::Stroke::NONE,
                                        ..Default::default()
                                    });
                                    x += galley.galley.size.x;
                                }
                                // Add space at end of line for cursor
                                let galley = ui.fonts().layout_no_visual(" ".to_string());
                                result.push(egui::epaint::TextShape {
                                    pos: egui::pos2(x, line_idx as f32 * height),
                                    galley: Box::new(galley.galley),
                                    override_text_color: Some(Color32::TRANSPARENT),
                                    underline: egui::Stroke::NONE,
                                    ..Default::default()
                                });
                            }
                            
                            vec![egui::epaint::TextShape::from_galleys(
                                &[],
                                result,
                                egui::TextureFilter::Linear,
                            )]
                        };

                        egui::TextEdit::multiline(&mut doc.content)
                            .font(FontId::monospace(14.0))
                            .desired_width(ui.available_width())
                            .layouter(&mut layouter)
                            .show(ui);
                        
                        // Mark as modified when content changes
                        if ui.memory(|m| m.text_cursor().state.any_char_deleted()) {
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
                .default_height(100.0)
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
    }
}

// ============================================================================
// Entry Point
// ============================================================================

fn main() {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1200.0, 800.0])
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
