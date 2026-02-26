use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use k_buffer::Document;
use k_syntax::Highlighter;

use crate::mode::EditorMode;
use crate::theme::Theme;

pub struct Viewport {
    pub top_line: usize,
    pub left_col: usize,
    pub height: usize,
    pub width: usize,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            top_line: 0,
            left_col: 0,
            height: 24,
            width: 80,
        }
    }
}

pub struct EditorComponent {
    pub viewport: Viewport,
    gutter_width: u16,
    pub search_query: Option<String>,
    pub search_matches: Vec<(usize, usize)>,
    pub current_match: usize,
}

impl EditorComponent {
    pub fn new() -> Self {
        Self {
            viewport: Viewport::default(),
            gutter_width: 4,
            search_query: None,
            search_matches: Vec::new(),
            current_match: 0,
        }
    }

    pub fn ensure_cursor_visible(&mut self, doc: &Document, scroll_padding: usize) {
        let line = doc.cursor.line;
        let padding = scroll_padding.min(self.viewport.height / 2);

        if line < self.viewport.top_line + padding {
            self.viewport.top_line = line.saturating_sub(padding);
        }
        if line >= self.viewport.top_line + self.viewport.height - padding {
            self.viewport.top_line = line.saturating_sub(self.viewport.height - padding - 1);
        }
    }

    pub fn update_viewport_size(&mut self, area: Rect, line_count: usize) {
        self.viewport.height = area.height as usize;
        self.gutter_width = if line_count > 0 {
            (line_count.to_string().len() as u16).max(3) + 2
        } else {
            5
        };
        self.viewport.width = area.width.saturating_sub(self.gutter_width) as usize;
    }

    pub fn search(&mut self, query: &str, doc: &Document) {
        self.search_matches.clear();
        self.current_match = 0;

        if query.is_empty() {
            self.search_query = None;
            return;
        }

        self.search_query = Some(query.to_string());
        let query_lower = query.to_lowercase();

        for line_idx in 0..doc.line_count() {
            if let Some(line_text) = doc.get_line(line_idx) {
                let line_lower = line_text.to_lowercase();
                let mut start = 0;
                while let Some(pos) = line_lower[start..].find(&query_lower) {
                    self.search_matches.push((line_idx, start + pos));
                    start += pos + 1;
                }
            }
        }
    }

    pub fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        self.current_match = (self.current_match + 1) % self.search_matches.len();
    }

    pub fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }
        if self.current_match == 0 {
            self.current_match = self.search_matches.len() - 1;
        } else {
            self.current_match -= 1;
        }
    }

    pub fn current_match_position(&self) -> Option<(usize, usize)> {
        self.search_matches.get(self.current_match).copied()
    }

    /// Returns the Rect of the revert button if a diff toolbar is shown, for click detection.
    pub fn diff_revert_button_area(&self, area: Rect, doc: &Document) -> Option<Rect> {
        if doc.language.as_deref() != Some("diff") {
            return None;
        }
        // Revert button is at the right side of the first row
        // " ↩ Revert " = 10 chars
        let btn_width: u16 = 10;
        let btn_x = area.x + area.width.saturating_sub(btn_width + 1);
        Some(Rect::new(btn_x, area.y, btn_width, 1))
    }

    pub fn render(
        &self,
        frame: &mut Frame,
        area: Rect,
        doc: &Document,
        mode: EditorMode,
        theme: &Theme,
        highlighter: &Highlighter,
    ) {
        let is_diff = doc.language.as_deref() == Some("diff");

        // Diff toolbar takes 1 row at the top
        let (toolbar_area, editor_area) = if is_diff {
            let toolbar = Rect::new(area.x, area.y, area.width, 1);
            let editor = Rect::new(area.x, area.y + 1, area.width, area.height.saturating_sub(1));
            (Some(toolbar), editor)
        } else {
            (None, area)
        };

        // Render diff toolbar
        if let Some(tb_area) = toolbar_area {
            let file_name = doc
                .path
                .as_ref()
                .and_then(|p| p.to_str())
                .unwrap_or("")
                .strip_prefix("diff: ")
                .unwrap_or(
                    doc.path.as_ref().and_then(|p| p.to_str()).unwrap_or(""),
                );

            let btn_width: u16 = 10;
            let name_width = tb_area.width.saturating_sub(btn_width + 2);

            let toolbar_line = Line::from(vec![
                Span::styled(
                    format!(" \u{f440} {:<width$}", file_name, width = name_width as usize),
                    Style::default()
                        .fg(theme.ui.foreground)
                        .bg(theme.ui.tab_active_bg),
                ),
                Span::styled(
                    " \u{f0e2} Revert ",
                    Style::default()
                        .fg(theme.git.deleted)
                        .bg(theme.ui.tab_active_bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    " ",
                    Style::default().bg(theme.ui.tab_active_bg),
                ),
            ]);
            frame.render_widget(Paragraph::new(vec![toolbar_line]), tb_area);
        }

        let line_count = doc.line_count();
        let gutter_width = if line_count > 0 {
            (line_count.to_string().len() as u16).max(3) + 2
        } else {
            5
        };

        let visible_lines = editor_area.height as usize;
        let start = self.viewport.top_line;
        let end = (start + visible_lines).min(line_count);

        let ext = doc
            .path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str());

        // Collect visible lines for batch highlighting
        let line_texts: Vec<String> = (start..end)
            .map(|i| doc.get_line(i).unwrap_or_default())
            .collect();

        // Only use syntax highlighter for non-diff files
        let highlighted = if is_diff {
            Vec::new()
        } else {
            let line_refs: Vec<&str> = line_texts.iter().map(|s| s.as_str()).collect();
            highlighter.highlight_lines(&line_refs, ext)
        };

        let mut lines: Vec<Line> = Vec::with_capacity(visible_lines);

        for (idx, line_idx) in (start..end).enumerate() {
            let mut spans = Vec::new();

            // Line number gutter
            let is_current = line_idx == doc.cursor.line;
            let num_style = if is_current {
                Style::default().fg(theme.ui.line_number_active)
            } else {
                Style::default().fg(theme.ui.line_number)
            };
            let num_str = format!("{:>width$} ", line_idx + 1, width = gutter_width as usize - 2);
            spans.push(Span::styled(num_str, num_style));

            let mut content_spans: Vec<Span> = Vec::new();

            if is_diff {
                // Diff coloring based on line prefix
                let text = &line_texts[idx];
                let (fg, bg) = if text.starts_with('+') && !text.starts_with("+++") {
                    (theme.git.added, Color::Rgb(0, 40, 0))
                } else if text.starts_with('-') && !text.starts_with("---") {
                    (theme.git.deleted, Color::Rgb(40, 0, 0))
                } else if text.starts_with("@@") {
                    (theme.ui.accent, theme.ui.background)
                } else if text.starts_with("diff ") || text.starts_with("index ") || text.starts_with("+++") || text.starts_with("---") {
                    (theme.ui.accent, theme.ui.background)
                } else {
                    (theme.ui.foreground, theme.ui.background)
                };
                content_spans.push(Span::styled(text.clone(), Style::default().fg(fg).bg(bg)));
            } else {
                // Syntax-highlighted content
                if idx < highlighted.len() {
                    for hl_span in &highlighted[idx].spans {
                        let mut style = Style::default().fg(Color::Rgb(
                            hl_span.fg.0,
                            hl_span.fg.1,
                            hl_span.fg.2,
                        ));
                        if hl_span.style_bold {
                            style = style.add_modifier(Modifier::BOLD);
                        }
                        if hl_span.style_italic {
                            style = style.add_modifier(Modifier::ITALIC);
                        }

                        let text = hl_span.text.replace('\n', "");
                        if !text.is_empty() {
                            content_spans.push(Span::styled(text, style));
                        }
                    }
                }
            }

            // Apply search match highlighting
            if let Some(ref query) = self.search_query {
                if !self.search_matches.is_empty() {
                    content_spans = apply_search_highlights(
                        content_spans,
                        line_idx,
                        &self.search_matches,
                        self.current_match,
                        query.len(),
                        theme.ui.search_match,
                        theme.ui.search_current,
                    );
                }
            }

            spans.extend(content_spans);
            lines.push(Line::from(spans));
        }

        // Fill remaining lines with tildes
        for _ in end..(start + visible_lines) {
            lines.push(Line::from(vec![
                Span::styled(
                    format!("{:>width$} ", "~", width = gutter_width as usize - 2),
                    Style::default().fg(theme.ui.line_number),
                ),
            ]));
        }

        let editor_block = Block::default().style(
            Style::default()
                .bg(theme.ui.background)
                .fg(theme.ui.foreground),
        );

        let paragraph = Paragraph::new(lines).block(editor_block);
        frame.render_widget(paragraph, editor_area);

        // Render cursor
        if mode == EditorMode::Insert || mode == EditorMode::Normal {
            let cursor_y = editor_area.y + (doc.cursor.line.saturating_sub(self.viewport.top_line)) as u16;
            let cursor_x = editor_area.x + gutter_width + doc.cursor.col as u16;
            if cursor_y < editor_area.y + editor_area.height && cursor_x < editor_area.x + editor_area.width {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

impl Default for EditorComponent {
    fn default() -> Self {
        Self::new()
    }
}

fn apply_search_highlights<'a>(
    content_spans: Vec<Span<'a>>,
    line_idx: usize,
    search_matches: &[(usize, usize)],
    current_match_idx: usize,
    query_len: usize,
    match_bg: Color,
    current_bg: Color,
) -> Vec<Span<'a>> {
    // Collect match ranges on this line
    let mut ranges: Vec<(usize, usize, bool)> = Vec::new();
    for (i, &(ml, mc)) in search_matches.iter().enumerate() {
        if ml == line_idx {
            ranges.push((mc, mc + query_len, i == current_match_idx));
        }
    }

    if ranges.is_empty() {
        return content_spans;
    }

    // Build flat char array with styles
    let mut chars: Vec<(char, Style)> = Vec::new();
    for span in &content_spans {
        for ch in span.content.chars() {
            chars.push((ch, span.style));
        }
    }

    // Apply match highlight backgrounds
    for &(start, end, is_current) in &ranges {
        let bg = if is_current { current_bg } else { match_bg };
        let end = end.min(chars.len());
        for item in chars.iter_mut().take(end).skip(start) {
            item.1 = item.1.bg(bg);
        }
    }

    // Rebuild spans by grouping consecutive chars with same style
    let mut result: Vec<Span> = Vec::new();
    for (ch, style) in chars {
        if let Some(last) = result.last_mut() {
            if last.style == style {
                let mut s = last.content.to_string();
                s.push(ch);
                *last = Span::styled(s, style);
                continue;
            }
        }
        result.push(Span::styled(ch.to_string(), style));
    }

    result
}
