use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActivityItem {
    FileTree,
    Git,
    Search,
}

// Each icon block: 1 blank line + 1 icon line + 1 blank line = 3 lines per item
const LINES_PER_ITEM: usize = 3;

pub struct ActivityBarComponent {
    pub active: ActivityItem,
}

impl ActivityBarComponent {
    pub fn new() -> Self {
        Self {
            active: ActivityItem::FileTree,
        }
    }

    pub fn item_at_y(&self, x: u16, y: u16, area: Rect) -> Option<ActivityItem> {
        if x < area.x || x >= area.x + area.width || y < area.y || y >= area.y + area.height {
            return None;
        }
        let rel_y = (y - area.y) as usize;
        let items = [ActivityItem::FileTree, ActivityItem::Git, ActivityItem::Search];
        for (idx, item) in items.iter().enumerate() {
            let start = idx * LINES_PER_ITEM;
            let end = start + LINES_PER_ITEM;
            if rel_y >= start && rel_y < end {
                return Some(*item);
            }
        }
        None
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if area.height == 0 || area.width == 0 {
            return;
        }

        let bg = theme.ui.file_tree_bg;
        let fg = theme.ui.line_number;
        let accent = theme.ui.accent;
        let w = area.width as usize;

        let items = [
            (ActivityItem::FileTree, "\u{f07c}"),  // nf-fa-folder_open (bigger)
            (ActivityItem::Git, "\u{f113}"),       // nf-fa-github_alt (bigger)
            (ActivityItem::Search, "\u{f50d}"),    // nf-md-magnify (bigger)
        ];

        let blank_style = Style::default().bg(bg);
        let blank_line = Line::from(Span::styled(" ".repeat(w), blank_style));

        let mut lines: Vec<Line> = Vec::new();

        for (item, icon) in &items {
            let is_active = *item == self.active;
            let icon_style = if is_active {
                Style::default()
                    .fg(accent)
                    .bg(bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(ratatui::style::Color::White)
                    .bg(bg)
            };

            let indicator = if is_active { "▎" } else { " " };
            let indicator_style = Style::default().fg(accent).bg(bg);

            // 1 blank line above
            lines.push(blank_line.clone());

            // Icon line: indicator + padding + icon + padding
            let pad_left = (w.saturating_sub(2)) / 2;
            let pad_right = w.saturating_sub(1 + pad_left + 1);
            lines.push(Line::from(vec![
                Span::styled(indicator, indicator_style),
                Span::styled(" ".repeat(pad_left.saturating_sub(1)), blank_style),
                Span::styled(*icon, icon_style),
                Span::styled(" ".repeat(pad_right), blank_style),
            ]));

            // 1 blank line below
            lines.push(blank_line.clone());
        }

        // Fill remaining height
        for _ in lines.len()..area.height as usize {
            lines.push(blank_line.clone());
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, area);

        // Draw subtle vertical separator on the right edge
        let sep_x = area.x + area.width.saturating_sub(1);
        let sep_style = Style::default().fg(theme.ui.border).bg(bg);
        for row in area.y..area.y + area.height {
            let sep_area = Rect::new(sep_x, row, 1, 1);
            frame.render_widget(Paragraph::new("│").style(sep_style), sep_area);
        }
    }
}

impl Default for ActivityBarComponent {
    fn default() -> Self {
        Self::new()
    }
}
