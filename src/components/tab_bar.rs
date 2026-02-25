use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use k_buffer::Document;

use crate::icons::icon_for_extension;
use crate::theme::Theme;

pub struct TabBarComponent;

// Tab format: " {icon} {name}{modified} × "
// Width:       1 + 2 + name.len() + modified.len() + 2 + 1 = name.len() + modified.len() + 6
fn tab_width(doc: &Document) -> usize {
    let name = doc.filename();
    let modified = if doc.modified { " ●" } else { "" };
    1 + 2 + name.len() + modified.len() + 2 + 1
}

impl TabBarComponent {
    /// Returns (tab_index, is_close_button)
    pub fn hit_test(documents: &[Document], x: u16, area_x: u16) -> Option<(usize, bool)> {
        let rel_x = (x.saturating_sub(area_x)) as usize;
        let mut pos = 0usize;
        for (idx, doc) in documents.iter().enumerate() {
            let tw = tab_width(doc);
            let separator = if idx < documents.len() - 1 { 1 } else { 0 };
            let total = tw + separator;
            if rel_x < pos + total {
                // Check if click is on the × button (last 3 chars of tab: "× ")
                let close_start = pos + tw - 3; // "× " starts 3 chars from end
                let is_close = rel_x >= close_start && rel_x < pos + tw;
                return Some((idx, is_close));
            }
            pos += total;
        }
        None
    }

    /// Backwards compatible: returns just the tab index
    pub fn tab_at_x(documents: &[Document], x: u16, area_x: u16) -> Option<usize> {
        Self::hit_test(documents, x, area_x).map(|(idx, _)| idx)
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
            let close_style = Style::default()
                .fg(theme.ui.line_number)
                .bg(bg);

            spans.push(Span::styled(" ", style));
            spans.push(Span::styled(
                format!("{} ", icon.icon),
                Style::default().fg(icon.color).bg(bg),
            ));
            spans.push(Span::styled(format!("{}{}", name, modified), style));
            spans.push(Span::styled(" \u{f00d} ", close_style)); // × close button

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
