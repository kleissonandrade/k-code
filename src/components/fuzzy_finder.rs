use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use ignore::WalkBuilder;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::icons::icon_for_extension;
use crate::layout::centered_rect;
use crate::theme::Theme;

pub struct FuzzyFinderComponent {
    pub input: String,
    pub cursor_pos: usize,
    all_files: Vec<PathBuf>,
    filtered: Vec<(PathBuf, i64)>,
    pub selected: usize,
    root: PathBuf,
    matcher: SkimMatcherV2,
}

impl FuzzyFinderComponent {
    pub fn new(root: PathBuf) -> Self {
        let mut finder = Self {
            input: String::new(),
            cursor_pos: 0,
            all_files: Vec::new(),
            filtered: Vec::new(),
            selected: 0,
            root,
            matcher: SkimMatcherV2::default(),
        };
        finder.load_files();
        finder
    }

    pub fn load_files(&mut self) {
        self.all_files.clear();
        let walker = WalkBuilder::new(&self.root)
            .hidden(true)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                self.all_files.push(entry.into_path());
            }
        }
        self.filter();
    }

    pub fn reset(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.selected = 0;
        self.filter();
    }

    pub fn insert_char(&mut self, c: char) {
        self.input.insert(self.cursor_pos, c);
        self.cursor_pos += 1;
        self.selected = 0;
        self.filter();
    }

    pub fn delete_char(&mut self) {
        if self.cursor_pos > 0 {
            self.cursor_pos -= 1;
            self.input.remove(self.cursor_pos);
            self.selected = 0;
            self.filter();
        }
    }

    fn filter(&mut self) {
        if self.input.is_empty() {
            self.filtered = self
                .all_files
                .iter()
                .take(100)
                .map(|p| (p.clone(), 0))
                .collect();
            return;
        }

        let mut scored: Vec<(PathBuf, i64)> = self
            .all_files
            .iter()
            .filter_map(|path| {
                let relative = path
                    .strip_prefix(&self.root)
                    .ok()
                    .and_then(|p| p.to_str())
                    .unwrap_or("");
                self.matcher
                    .fuzzy_match(relative, &self.input)
                    .map(|score| (path.clone(), score))
            })
            .collect();

        scored.sort_by(|a, b| b.1.cmp(&a.1));
        self.filtered = scored.into_iter().take(50).collect();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_path(&self) -> Option<&Path> {
        self.filtered
            .get(self.selected)
            .map(|(p, _)| p.as_path())
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = centered_rect(60, 60, area);

        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Find File (Ctrl+P) ")
            .title_style(Style::default().fg(theme.ui.accent).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.ui.popup_border))
            .style(Style::default().bg(theme.ui.popup_bg));

        let inner = block.inner(popup_area);
        frame.render_widget(block, popup_area);

        if inner.height < 3 {
            return;
        }

        // Search input line
        let input_line = Line::from(vec![
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
        ]);

        let separator = Line::from(Span::styled(
            "─".repeat(inner.width as usize),
            Style::default().fg(theme.ui.border).bg(theme.ui.popup_bg),
        ));

        let max_results = (inner.height as usize).saturating_sub(2);
        let mut lines = vec![input_line, separator];

        for (idx, (path, _score)) in self.filtered.iter().enumerate().take(max_results) {
            let is_selected = idx == self.selected;
            let relative = path
                .strip_prefix(&self.root)
                .ok()
                .and_then(|p| p.to_str())
                .unwrap_or("");

            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let icon = icon_for_extension(ext);

            let bg = if is_selected {
                theme.ui.file_tree_selected_bg
            } else {
                theme.ui.popup_bg
            };

            let fg = if is_selected {
                theme.ui.file_tree_selected_fg
            } else {
                theme.ui.foreground
            };

            let mut spans = vec![
                Span::styled(" ", Style::default().bg(bg)),
                Span::styled(
                    format!("{} ", icon.icon),
                    Style::default().fg(icon.color).bg(bg),
                ),
                Span::styled(relative, Style::default().fg(fg).bg(bg)),
            ];

            let content_width: usize = spans.iter().map(|s| s.content.len()).sum();
            let remaining = (inner.width as usize).saturating_sub(content_width);
            if remaining > 0 {
                spans.push(Span::styled(
                    " ".repeat(remaining),
                    Style::default().bg(bg),
                ));
            }

            lines.push(Line::from(spans));
        }

        let paragraph = Paragraph::new(lines);
        frame.render_widget(paragraph, inner);

        // Cursor
        let cursor_x = inner.x + 3 + self.cursor_pos as u16;
        let cursor_y = inner.y;
        if cursor_x < inner.x + inner.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}
