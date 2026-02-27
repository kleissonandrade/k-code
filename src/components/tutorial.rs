use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::layout::centered_rect;
use crate::theme::Theme;

pub struct TutorialComponent {
    pub scroll: usize,
}

impl TutorialComponent {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(1);
    }

    pub fn scroll_down(&mut self) {
        self.scroll += 1;
    }

    pub fn reset(&mut self) {
        self.scroll = 0;
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = centered_rect(75, 85, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" k-code Tutorial (? to toggle) ")
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

        let accent = theme.ui.accent;
        let fg = theme.ui.foreground;
        let dim = theme.ui.line_number;
        let bg = theme.ui.popup_bg;

        let base = Style::default().fg(fg).bg(bg);
        let header = Style::default().fg(accent).bg(bg).add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        let key_style = Style::default().fg(accent).bg(bg).add_modifier(Modifier::BOLD);
        let desc_style = Style::default().fg(fg).bg(bg);
        let dim_style = Style::default().fg(dim).bg(bg);

        let sections = vec![
            ("", vec![
                ("", "Welcome to k-code! A terminal code editor built with Rust."),
                ("", "Navigate this help with j/k or arrow keys. Press ? or Esc to close."),
            ]),
            ("\u{f121} NAVIGATION", vec![
                ("h / \u{2190}", "Move cursor left"),
                ("j / \u{2193}", "Move cursor down"),
                ("k / \u{2191}", "Move cursor up"),
                ("l / \u{2192}", "Move cursor right"),
                ("0", "Go to beginning of line"),
                ("$", "Go to end of line"),
                ("w", "Move to next word"),
                ("b", "Move to previous word"),
                ("gg", "Go to first line"),
                ("G", "Go to last line"),
                ("PageUp", "Scroll page up"),
                ("PageDown", "Scroll page down"),
                (":<number>", "Go to specific line number"),
            ]),
            ("\u{f044} EDITING", vec![
                ("i", "Enter INSERT mode (start typing)"),
                ("o", "Open new line below and enter INSERT mode"),
                ("I", "Insert at beginning of line"),
                ("A", "Append at end of line"),
                ("x", "Delete character under cursor"),
                ("dd", "Delete current line"),
                ("yy", "Yank (copy) current line"),
                ("p", "Paste yanked text"),
                ("u", "Undo last change"),
                ("Ctrl+r", "Redo last undone change"),
                ("Esc", "Return to NORMAL mode"),
            ]),
            ("\u{f0c7} FILE OPERATIONS", vec![
                ("Ctrl+s", "Save current file"),
                (":w", "Save current file (command mode)"),
                (":q", "Quit k-code"),
                (":wq", "Save and quit"),
                (":q!", "Force quit without saving"),
                ("Ctrl+t", "Create new empty file tab"),
                (":e <path>", "Open file at path"),
                ("Tab", "Switch to next open file"),
                ("Shift+Tab", "Switch to previous open file"),
            ]),
            ("\u{f002} SEARCH", vec![
                ("/", "Open search bar in current file"),
                ("Ctrl+f", "Open search bar in current file"),
                ("n", "Go to next search match"),
                ("N", "Go to previous search match"),
                ("Space+f", "Open global search across all files"),
                ("Ctrl+p", "Open fuzzy file finder"),
                ("Esc", "Close search / fuzzy finder"),
            ]),
            ("\u{f50d} GLOBAL SEARCH", vec![
                ("Space+f", "Open global search across all files"),
                ("\u{2191}/\u{2193}", "Navigate results list"),
                ("Enter", "Open selected file"),
                ("Tab", "Toggle collapse all match lines"),
                ("Click \u{25b8}/\u{25be}", "Collapse/expand individual file matches"),
                ("Scroll", "Scroll preview pane"),
                ("Esc", "Close global search"),
            ]),
            ("\u{f1bb} FILE TREE", vec![
                ("Ctrl+b", "Toggle file tree sidebar"),
                ("j / k", "Navigate tree (when focused)"),
                ("Enter", "Open file / expand directory"),
                ("h", "Collapse directory"),
                ("l", "Expand directory"),
            ]),
            ("\u{e702} GIT INTEGRATION", vec![
                ("Ctrl+g", "Open Git panel"),
                ("Tab", "Switch Git panel tabs (Status/Branches/Stash/Log/Actions)"),
                ("a", "Stage all files"),
                ("s", "Stage selected file"),
                ("c", "Start commit (type message, Enter to confirm)"),
                ("p", "Push to remote"),
                ("Enter", "Checkout branch / Pop stash"),
                ("n", "Create new branch (in Branches tab)"),
                ("r", "Refresh GitHub Actions (in Actions tab)"),
                ("o", "Open action in browser (in Actions tab)"),
                ("Esc", "Close Git panel"),
            ]),
            ("\u{f489} TERMINAL", vec![
                ("Space+j", "Toggle integrated terminal"),
                ("Ctrl+`", "Toggle terminal / return focus to editor"),
                ("Click +", "Open new terminal tab"),
                ("Click tab", "Switch between terminal tabs"),
                ("Scroll", "Scroll terminal history"),
                ("", "All keys are forwarded to the shell when terminal is focused"),
            ]),
            ("\u{e7a8} CODE INTELLIGENCE", vec![
                ("gd", "Go to definition of symbol under cursor"),
            ]),
            ("\u{f53f} THEMES", vec![
                ("Space+t", "Cycle through available themes"),
                (":theme", "Switch theme (command mode)"),
            ]),
            ("\u{f128} MODES", vec![
                ("NORMAL", "Default mode - navigate and execute commands"),
                ("INSERT", "Type text into the editor"),
                ("VISUAL", "Select text character by character"),
                ("V-LINE", "Select text line by line"),
                ("COMMAND", "Execute commands with ':'"),
                ("SEARCH", "Search within current file with '/'"),
            ]),
        ];

        let mut lines: Vec<Line> = Vec::new();

        for (title, entries) in &sections {
            if !title.is_empty() {
                lines.push(Line::from(vec![]));
                lines.push(Line::from(vec![
                    Span::styled("  ", base),
                    Span::styled(*title, header),
                ]));
                lines.push(Line::from(vec![]));
            }

            for (key, desc) in entries {
                if key.is_empty() {
                    lines.push(Line::from(vec![
                        Span::styled("  ", base),
                        Span::styled(*desc, dim_style),
                    ]));
                } else {
                    let key_width = 16;
                    let padded_key = format!("{:>width$}", key, width = key_width);
                    lines.push(Line::from(vec![
                        Span::styled("  ", base),
                        Span::styled(padded_key, key_style),
                        Span::styled("  ", base),
                        Span::styled(*desc, desc_style),
                    ]));
                }
            }
        }

        lines.push(Line::from(vec![]));
        lines.push(Line::from(vec![
            Span::styled("  ", base),
            Span::styled(
                "Made with \u{2764} in Rust │ k-code v0.1.0",
                dim_style,
            ),
        ]));
        lines.push(Line::from(vec![]));

        let max_scroll = lines.len().saturating_sub(inner.height as usize);
        let scroll = self.scroll.min(max_scroll);

        let paragraph = Paragraph::new(lines)
            .scroll((scroll as u16, 0))
            .style(base);

        frame.render_widget(paragraph, inner);
    }
}

impl Default for TutorialComponent {
    fn default() -> Self {
        Self::new()
    }
}
