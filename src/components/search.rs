use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
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
            format!(" {}/{} ", self.current_match + 1, self.match_count)
        } else if !self.input.is_empty() {
            " No matches ".to_string()
        } else {
            String::new()
        };

        // icon(3) + input + match_info + padding
        let content_width = 3 + self.input.len() + match_info.len() + 1;
        let box_width = (content_width as u16).max(30).min(area.width.saturating_sub(2));
        let box_height: u16 = 3; // border + content + border

        // Position: top-right corner of the editor area
        let box_x = area.x + area.width.saturating_sub(box_width + 1);
        let box_y = area.y;

        let popup_area = Rect::new(box_x, box_y, box_width, box_height);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Find (Ctrl+F) ")
            .title_style(
                Style::default()
                    .fg(theme.ui.accent)
                    .add_modifier(Modifier::BOLD),
            )
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ui.popup_border))
            .style(Style::default().bg(theme.ui.popup_bg));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let mut spans = vec![
            Span::styled(
                " \u{f002} ",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.popup_bg),
            ),
            Span::styled(
                &self.input,
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.popup_bg),
            ),
        ];

        if !match_info.is_empty() {
            spans.push(Span::styled(
                &match_info,
                Style::default()
                    .fg(theme.ui.line_number)
                    .bg(theme.ui.popup_bg),
            ));
        }

        // Fill remaining space
        let used: usize = spans.iter().map(|s| s.content.len()).sum();
        let remaining = (inner.width as usize).saturating_sub(used);
        if remaining > 0 {
            spans.push(Span::styled(
                " ".repeat(remaining),
                Style::default().bg(theme.ui.popup_bg),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(vec![line]);
        frame.render_widget(paragraph, inner);

        // Set cursor position inside the input
        let cursor_x = inner.x + 3 + self.cursor_pos as u16;
        let cursor_y = inner.y;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

impl Default for SearchComponent {
    fn default() -> Self {
        Self::new()
    }
}
