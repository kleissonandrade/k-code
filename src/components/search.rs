use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

pub struct SearchComponent {
    pub input: String,
    pub cursor_pos: usize,
    pub match_count: usize,
    pub current_match: usize,
}

impl SearchComponent {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            match_count: 0,
            current_match: 0,
        }
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.match_count = 0;
        self.current_match = 0;
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let match_info = if self.match_count > 0 {
            format!(" [{}/{}]", self.current_match + 1, self.match_count)
        } else if !self.input.is_empty() {
            " [No matches]".to_string()
        } else {
            String::new()
        };

        let spans = vec![
            Span::styled(
                " / ",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.status_bar_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &self.input,
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.status_bar_bg),
            ),
            Span::styled(
                &match_info,
                Style::default()
                    .fg(theme.ui.line_number)
                    .bg(theme.ui.status_bar_bg),
            ),
        ];

        let content_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let remaining = (area.width as usize).saturating_sub(content_width);

        let mut all_spans = spans;
        all_spans.push(Span::styled(
            " ".repeat(remaining),
            Style::default().bg(theme.ui.status_bar_bg),
        ));

        let line = Line::from(all_spans);
        let paragraph = Paragraph::new(vec![line]);
        frame.render_widget(paragraph, area);

        // Set cursor position in search bar
        let cursor_x = area.x + 3 + self.cursor_pos as u16;
        let cursor_y = area.y;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

impl Default for SearchComponent {
    fn default() -> Self {
        Self::new()
    }
}
