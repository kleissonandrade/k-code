use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use k_buffer::Document;

use crate::icons::icon_for_extension;
use crate::theme::Theme;

pub struct TabBarComponent;

impl TabBarComponent {
    pub fn tab_at_x(documents: &[Document], x: u16, area_x: u16) -> Option<usize> {
        let rel_x = (x.saturating_sub(area_x)) as usize;
        let mut pos = 0usize;
        for (idx, doc) in documents.iter().enumerate() {
            let name = doc.filename();
            let modified = if doc.modified { " ●" } else { "" };
            // " " + icon + " " + name + modified + " " = 1 + 2 + name.len() + modified.len() + 1
            let tab_width = 1 + 2 + name.len() + modified.len() + 1;
            let separator = if idx < documents.len() - 1 { 1 } else { 0 };
            let total = tab_width + separator;
            if rel_x < pos + total {
                return Some(idx);
            }
            pos += total;
        }
        None
    }

    pub fn render(
        frame: &mut Frame,
        area: Rect,
        documents: &[Document],
        active_index: usize,
        theme: &Theme,
    ) {
        let mut spans = Vec::new();

        for (idx, doc) in documents.iter().enumerate() {
            let is_active = idx == active_index;
            let name = doc.filename();
            let modified = if doc.modified { " ●" } else { "" };

            let ext = doc
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let icon = icon_for_extension(ext);

            let (bg, fg) = if is_active {
                (theme.ui.tab_active_bg, theme.ui.tab_active_fg)
            } else {
                (theme.ui.tab_inactive_bg, theme.ui.tab_inactive_fg)
            };

            let style = Style::default().fg(fg).bg(bg);

            spans.push(Span::styled(" ", style));
            spans.push(Span::styled(
                format!("{} ", icon.icon),
                Style::default().fg(icon.color).bg(bg),
            ));
            spans.push(Span::styled(format!("{}{}", name, modified), style));
            spans.push(Span::styled(" ", style));

            if idx < documents.len() - 1 {
                spans.push(Span::styled(
                    "│",
                    Style::default().fg(theme.ui.border).bg(theme.ui.tab_inactive_bg),
                ));
            }
        }

        // Fill remaining width
        let content_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let remaining = (area.width as usize).saturating_sub(content_width);
        if remaining > 0 {
            spans.push(Span::styled(
                " ".repeat(remaining),
                Style::default().bg(theme.ui.tab_inactive_bg),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(vec![line]);
        frame.render_widget(paragraph, area);
    }
}
