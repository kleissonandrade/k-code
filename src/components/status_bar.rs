use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use k_buffer::Document;

use crate::mode::EditorMode;
use crate::theme::Theme;

pub struct StatusBarComponent;

impl StatusBarComponent {
    pub fn render(
        frame: &mut Frame,
        area: Rect,
        mode: EditorMode,
        doc: Option<&Document>,
        branch: &str,
        theme: &Theme,
        status_message: &str,
    ) {
        let mut spans = Vec::new();

        // Mode indicator
        let mode_style = Style::default()
            .fg(theme.ui.status_bar_mode_fg)
            .bg(theme.ui.status_bar_mode_bg)
            .add_modifier(Modifier::BOLD);

        spans.push(Span::styled(
            format!(" {} ", mode.display_name()),
            mode_style,
        ));

        let bar_style = Style::default()
            .fg(theme.ui.status_bar_fg)
            .bg(theme.ui.status_bar_bg);

        // File info
        if let Some(doc) = doc {
            let modified = if doc.modified { " [+]" } else { "" };
            spans.push(Span::styled(
                format!(" {}{} ", doc.display_path(), modified),
                bar_style,
            ));
        } else {
            spans.push(Span::styled(" [No file] ", bar_style));
        }

        // Status message
        if !status_message.is_empty() {
            spans.push(Span::styled(
                format!(" {} ", status_message),
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.status_bar_bg),
            ));
        }

        // Calculate left side width
        let left_width: usize = spans.iter().map(|s| s.content.len()).sum();

        // Right side: cursor position + branch
        let right_text = if let Some(doc) = doc {
            let lang = doc
                .language
                .as_deref()
                .unwrap_or("");
            if !branch.is_empty() {
                format!(
                    " {} │ Ln {}, Col {} │ \u{e702} {} ",
                    lang,
                    doc.cursor.line + 1,
                    doc.cursor.col + 1,
                    branch
                )
            } else {
                format!(
                    " {} │ Ln {}, Col {} ",
                    lang,
                    doc.cursor.line + 1,
                    doc.cursor.col + 1,
                )
            }
        } else {
            String::new()
        };

        let right_width = right_text.len();
        let padding = (area.width as usize).saturating_sub(left_width + right_width);

        spans.push(Span::styled(" ".repeat(padding), bar_style));
        spans.push(Span::styled(right_text, bar_style));

        let line = Line::from(spans);
        let paragraph = Paragraph::new(vec![line]);
        frame.render_widget(paragraph, area);
    }
}
