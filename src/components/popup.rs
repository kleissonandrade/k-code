use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

use crate::layout::centered_rect;
use crate::theme::Theme;

pub struct PopupComponent {
    pub title: String,
    pub message: String,
    pub visible: bool,
}

impl PopupComponent {
    pub fn new() -> Self {
        Self {
            title: String::new(),
            message: String::new(),
            visible: false,
        }
    }

    pub fn show(&mut self, title: &str, message: &str) {
        self.title = title.to_string();
        self.message = message.to_string();
        self.visible = true;
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if !self.visible {
            return;
        }

        let popup_area = centered_rect(40, 20, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(format!(" {} ", self.title))
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

        let text = Paragraph::new(self.message.as_str())
            .style(
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.popup_bg),
            )
            .wrap(Wrap { trim: false });

        frame.render_widget(text, inner);
    }
}

impl Default for PopupComponent {
    fn default() -> Self {
        Self::new()
    }
}
