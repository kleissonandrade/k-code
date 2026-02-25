use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use syntect::parsing::{SyntaxReference, SyntaxSet};
use syntect::easy::HighlightLines;

use crate::languages::LanguageDetector;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
    current_theme: String,
}

#[derive(Debug, Clone)]
pub struct HighlightedLine {
    pub spans: Vec<HighlightSpan>,
}

#[derive(Debug, Clone)]
pub struct HighlightSpan {
    pub text: String,
    pub fg: (u8, u8, u8),
    pub style_bold: bool,
    pub style_italic: bool,
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
            current_theme: "base16-ocean.dark".to_string(),
        }
    }

    pub fn set_theme(&mut self, name: &str) {
        if self.theme_set.themes.contains_key(name) {
            self.current_theme = name.to_string();
        }
    }

    pub fn available_themes(&self) -> Vec<String> {
        self.theme_set.themes.keys().cloned().collect()
    }

    pub fn highlight_line(
        &self,
        line: &str,
        extension: Option<&str>,
    ) -> HighlightedLine {
        let syntax = extension
            .and_then(|ext| LanguageDetector::extension_to_syntect_name(ext))
            .and_then(|name| self.syntax_set.find_syntax_by_name(name))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.current_theme)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        self.do_highlight(line, syntax, theme)
    }

    pub fn highlight_lines(
        &self,
        lines: &[&str],
        extension: Option<&str>,
    ) -> Vec<HighlightedLine> {
        let syntax = extension
            .and_then(|ext| LanguageDetector::extension_to_syntect_name(ext))
            .and_then(|name| self.syntax_set.find_syntax_by_name(name))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = self
            .theme_set
            .themes
            .get(&self.current_theme)
            .unwrap_or_else(|| self.theme_set.themes.values().next().unwrap());

        let mut h = HighlightLines::new(syntax, theme);
        lines
            .iter()
            .map(|line| {
                let line_with_newline = if line.ends_with('\n') {
                    line.to_string()
                } else {
                    format!("{}\n", line)
                };
                match h.highlight_line(&line_with_newline, &self.syntax_set) {
                    Ok(ranges) => HighlightedLine {
                        spans: ranges
                            .into_iter()
                            .map(|(style, text)| HighlightSpan {
                                text: text.to_string(),
                                fg: (style.foreground.r, style.foreground.g, style.foreground.b),
                                style_bold: style
                                    .font_style
                                    .contains(syntect::highlighting::FontStyle::BOLD),
                                style_italic: style
                                    .font_style
                                    .contains(syntect::highlighting::FontStyle::ITALIC),
                            })
                            .collect(),
                    },
                    Err(_) => HighlightedLine {
                        spans: vec![HighlightSpan {
                            text: line.to_string(),
                            fg: (200, 200, 200),
                            style_bold: false,
                            style_italic: false,
                        }],
                    },
                }
            })
            .collect()
    }

    fn do_highlight(
        &self,
        line: &str,
        syntax: &SyntaxReference,
        theme: &SyntectTheme,
    ) -> HighlightedLine {
        let mut h = HighlightLines::new(syntax, theme);
        let line_with_newline = if line.ends_with('\n') {
            line.to_string()
        } else {
            format!("{}\n", line)
        };
        match h.highlight_line(&line_with_newline, &self.syntax_set) {
            Ok(ranges) => HighlightedLine {
                spans: ranges
                    .into_iter()
                    .map(|(style, text)| HighlightSpan {
                        text: text.to_string(),
                        fg: (style.foreground.r, style.foreground.g, style.foreground.b),
                        style_bold: style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::BOLD),
                        style_italic: style
                            .font_style
                            .contains(syntect::highlighting::FontStyle::ITALIC),
                    })
                    .collect(),
            },
            Err(_) => HighlightedLine {
                spans: vec![HighlightSpan {
                    text: line.to_string(),
                    fg: (200, 200, 200),
                    style_bold: false,
                    style_italic: false,
                }],
            },
        }
    }
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}
