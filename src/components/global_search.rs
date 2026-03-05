use std::collections::HashSet;
use std::path::PathBuf;

use ignore::WalkBuilder;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use k_syntax::Highlighter;

use crate::icons::icon_for_extension;
use crate::layout::centered_rect;
use crate::theme::Theme;

pub enum ClickResult {
    None,
    Handled,
    OpenFile,
}

#[derive(Clone)]
pub struct SearchFileResult {
    pub path: PathBuf,
    pub matches: Vec<(usize, String)>, // (line_number, line_text)
}

pub struct GlobalSearchComponent {
    pub input: String,
    pub cursor_pos: usize,
    root: PathBuf,
    results: Vec<SearchFileResult>,
    pub selected: usize,
    pub hovered: Option<usize>,
    preview_lines: Vec<String>,
    preview_match_line: usize,
    preview_path: Option<PathBuf>,
    scroll_offset: usize,
    visible_height: usize,
    pub collapsed: bool,
    collapsed_files: HashSet<usize>,
    collapse_btn_area: Option<(u16, u16)>, // (x, y) of the toggle button
    left_pane_x: u16,
    preview_scroll: usize,
    preview_height: usize,
    right_pane_area: Option<Rect>,
}

impl GlobalSearchComponent {
    pub fn new(root: PathBuf) -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
            root,
            results: Vec::new(),
            selected: 0,
            hovered: None,
            preview_lines: Vec::new(),
            preview_match_line: 0,
            preview_path: None,
            scroll_offset: 0,
            visible_height: 0,
            collapsed: false,
            collapsed_files: HashSet::new(),
            collapse_btn_area: None,
            left_pane_x: 0,
            preview_scroll: 0,
            preview_height: 0,
            right_pane_area: None,
        }
    }

    pub fn set_root(&mut self, new_root: PathBuf) {
        self.root = new_root;
        self.results.clear();
    }

    pub fn reset(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
        self.results.clear();
        self.selected = 0;
        self.hovered = None;
        self.preview_lines.clear();
        self.preview_match_line = 0;
        self.preview_path = None;
        self.scroll_offset = 0;
        self.collapsed_files.clear();
        self.preview_scroll = 0;
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

    pub fn trigger_search(&mut self) {
        self.search();
    }

    pub fn toggle_collapsed(&mut self) {
        self.collapsed = !self.collapsed;
        self.collapsed_files.clear();
        self.scroll_offset = 0;
        self.ensure_visible();
    }

    /// Check if a specific file's matches are hidden.
    fn is_file_collapsed(&self, idx: usize) -> bool {
        if self.collapsed {
            // Global collapse: individual files can be expanded
            !self.collapsed_files.contains(&idx)
        } else {
            // Global expanded: individual files can be collapsed
            self.collapsed_files.contains(&idx)
        }
    }

    pub fn toggle_file_collapsed(&mut self, idx: usize) {
        if self.collapsed_files.contains(&idx) {
            self.collapsed_files.remove(&idx);
        } else {
            self.collapsed_files.insert(idx);
        }
        self.ensure_visible();
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.ensure_visible();
            self.load_preview();
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.results.len() {
            self.selected += 1;
            self.ensure_visible();
            self.load_preview();
        }
    }

    /// Compute the starting row of result at `idx`.
    fn row_start_of(&self, idx: usize) -> usize {
        let mut row = 0;
        for i in 0..idx.min(self.results.len()) {
            row += self.item_height(i);
        }
        row
    }

    /// Height of the item at `idx` in rows.
    fn item_height(&self, idx: usize) -> usize {
        if self.is_file_collapsed(idx) {
            1
        } else {
            1 + self.results[idx].matches.len()
        }
    }

    /// Ensure the selected item is visible within the scroll viewport.
    fn ensure_visible(&mut self) {
        if self.results.is_empty() || self.visible_height == 0 {
            return;
        }
        let sel_start = self.row_start_of(self.selected);
        let sel_height = self.item_height(self.selected);
        let sel_end = sel_start + sel_height;

        // If selected item starts above viewport, scroll up
        if sel_start < self.scroll_offset {
            self.scroll_offset = sel_start;
        }
        // If selected item ends below viewport, scroll down
        if sel_end > self.scroll_offset + self.visible_height {
            self.scroll_offset = sel_end.saturating_sub(self.visible_height);
        }
    }

    pub fn preview_scroll_up(&mut self, lines: usize) {
        self.preview_scroll = self.preview_scroll.saturating_sub(lines);
    }

    pub fn preview_scroll_down(&mut self, lines: usize) {
        let max = self.preview_lines.len().saturating_sub(self.preview_height);
        self.preview_scroll = (self.preview_scroll + lines).min(max);
    }

    pub fn is_in_preview(&self, x: u16, y: u16) -> bool {
        if let Some(area) = self.right_pane_area {
            x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
        } else {
            false
        }
    }

    pub fn selected_result(&self) -> Option<&SearchFileResult> {
        self.results.get(self.selected)
    }

    fn search(&mut self) {
        self.results.clear();
        self.selected = 0;
        self.scroll_offset = 0;
        self.collapsed_files.clear();
        self.hovered = None;

        if self.input.len() < 2 {
            self.preview_lines.clear();
            self.preview_path = None;
            return;
        }

        let query_lower = self.input.to_lowercase();

        let walker = WalkBuilder::new(&self.root)
            .hidden(false)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }

            let path = entry.into_path();

            // Skip large files (> 512KB)
            if let Ok(meta) = std::fs::metadata(&path) {
                if meta.len() > 512 * 1024 {
                    continue;
                }
            }

            // Skip binary files by checking extension
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if matches!(
                ext,
                "png" | "jpg" | "jpeg" | "gif" | "bmp" | "ico" | "svg"
                    | "woff" | "woff2" | "ttf" | "eot"
                    | "zip" | "tar" | "gz" | "bz2" | "xz"
                    | "exe" | "dll" | "so" | "dylib"
                    | "pdf" | "doc" | "docx"
                    | "mp3" | "mp4" | "avi" | "mov"
                    | "o" | "a" | "lib" | "rlib" | "rmeta"
                    | "lock"
            ) {
                continue;
            }

            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let mut matches = Vec::new();
            for (line_idx, line) in content.lines().enumerate() {
                if line.to_lowercase().contains(&query_lower) {
                    matches.push((line_idx, line.to_string()));
                    if matches.len() >= 5 {
                        break;
                    }
                }
            }

            if !matches.is_empty() {
                self.results.push(SearchFileResult {
                    path,
                    matches,
                });
            }

            if self.results.len() >= 100 {
                break;
            }
        }

        // Sort by number of matches descending
        self.results.sort_by(|a, b| b.matches.len().cmp(&a.matches.len()));

        self.load_preview();
    }

    fn load_preview(&mut self) {
        if let Some(result) = self.results.get(self.selected) {
            if self.preview_path.as_ref() == Some(&result.path) {
                // Already loaded, just update match line and scroll
                if let Some(&(line, _)) = result.matches.first() {
                    self.preview_match_line = line;
                    self.preview_scroll = line.saturating_sub(self.preview_height / 2);
                }
                return;
            }

            if let Ok(content) = std::fs::read_to_string(&result.path) {
                self.preview_lines = content.lines().map(|l| l.to_string()).collect();
                self.preview_path = Some(result.path.clone());
                if let Some(&(line, _)) = result.matches.first() {
                    self.preview_match_line = line;
                    self.preview_scroll = line.saturating_sub(self.preview_height / 2);
                }
            }
        } else {
            self.preview_lines.clear();
            self.preview_path = None;
            self.preview_scroll = 0;
        }
    }

    pub fn hover_at(&mut self, x: u16, y: u16, area: Rect) -> bool {
        let popup_area = centered_rect(80, 80, area);
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(popup_area);

        if x < inner.x || x >= inner.x + inner.width || y < inner.y || y >= inner.y + inner.height
        {
            self.hovered = None;
            return false;
        }

        // Left pane starts at inner.x, takes 40%
        let left_width = (inner.width * 40) / 100;

        // Only hover on left pane, below input (row 0) and separator (row 1)
        let rel_y = (y - inner.y) as usize;
        if x < inner.x + left_width && rel_y >= 2 {
            let item_idx = self.item_at_row(rel_y - 2 + self.scroll_offset);
            if let Some(idx) = item_idx {
                if idx < self.results.len() {
                    self.hovered = Some(idx);
                    // Load preview for hovered item
                    if let Some(result) = self.results.get(idx) {
                        if self.preview_path.as_ref() != Some(&result.path) {
                            if let Ok(content) = std::fs::read_to_string(&result.path) {
                                self.preview_lines =
                                    content.lines().map(|l| l.to_string()).collect();
                                self.preview_path = Some(result.path.clone());
                            }
                        }
                        if let Some(&(line, _)) = result.matches.first() {
                            self.preview_match_line = line;
                        }
                    }
                    return true;
                }
            }
        }

        self.hovered = None;
        true
    }

    pub fn click_at(&mut self, x: u16, y: u16, area: Rect) -> ClickResult {
        // Check collapse button click
        if let Some((btn_x, btn_y)) = self.collapse_btn_area {
            if y == btn_y && x >= btn_x && x < btn_x + 4 {
                self.toggle_collapsed();
                return ClickResult::Handled;
            }
        }

        let popup_area = centered_rect(80, 80, area);
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(popup_area);

        if x < inner.x || x >= inner.x + inner.width || y < inner.y || y >= inner.y + inner.height
        {
            return ClickResult::None;
        }

        let left_width = (inner.width * 40) / 100;
        let rel_y = (y - inner.y) as usize;

        if x < inner.x + left_width && rel_y >= 2 {
            let abs_row = rel_y - 2 + self.scroll_offset;
            if let Some(idx) = self.item_at_row(abs_row) {
                if idx < self.results.len() {
                    // Check if click is on the arrow icon (first char of header row)
                    let row_in_item = abs_row - self.row_start_of(idx);
                    if row_in_item == 0 && x >= self.left_pane_x && x < self.left_pane_x + 2 {
                        self.toggle_file_collapsed(idx);
                        return ClickResult::Handled;
                    }
                    self.selected = idx;
                    self.load_preview();
                    return ClickResult::OpenFile;
                }
            }
        }
        ClickResult::Handled
    }

    /// Map a row offset in the results pane to a result index.
    fn item_at_row(&self, row: usize) -> Option<usize> {
        let mut current_row = 0;
        for (idx, _result) in self.results.iter().enumerate() {
            let h = self.item_height(idx);
            if row >= current_row && row < current_row + h {
                return Some(idx);
            }
            current_row += h;
        }
        None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme, highlighter: &Highlighter) {
        let popup_area = centered_rect(80, 80, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Search in Files (Space+F) ")
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

        if inner.height < 4 || inner.width < 20 {
            return;
        }

        // Layout: input row, separator, content area, footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Input
                Constraint::Length(1), // Separator
                Constraint::Min(1),    // Content (left + right panes)
                Constraint::Length(1), // Footer
            ])
            .split(inner);

        let input_area = chunks[0];
        let sep_area = chunks[1];
        let content_area = chunks[2];
        let footer_area = chunks[3];

        // --- Input line ---
        let file_count = if self.results.is_empty() && !self.input.is_empty() {
            " [No matches]".to_string()
        } else if !self.results.is_empty() {
            let total_matches: usize = self.results.iter().map(|r| r.matches.len()).sum();
            format!(" [{} files, {} matches]", self.results.len(), total_matches)
        } else {
            String::new()
        };

        let input_spans = vec![
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
            Span::styled(
                &file_count,
                Style::default()
                    .fg(theme.ui.line_number)
                    .bg(theme.ui.popup_bg),
            ),
        ];
        frame.render_widget(Paragraph::new(Line::from(input_spans)), input_area);

        // --- Separator ---
        let sep = Line::from(Span::styled(
            "─".repeat(sep_area.width as usize),
            Style::default()
                .fg(theme.ui.border)
                .bg(theme.ui.popup_bg),
        ));
        frame.render_widget(Paragraph::new(sep), sep_area);

        // --- Split content: left (40%) results, right (60%) preview ---
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(content_area);

        let left_area = panes[0];
        let right_area = panes[1];

        // --- Collapse toggle button aligned with file list ---
        let collapse_icon = if self.collapsed { " \u{f066} " } else { " \u{f065} " };
        let collapse_fg = if self.collapsed {
            theme.ui.accent
        } else {
            theme.ui.line_number
        };
        let btn_x = left_area.x + left_area.width.saturating_sub(4);
        let btn_y = sep_area.y;
        self.collapse_btn_area = Some((btn_x, btn_y));
        let btn_area = Rect::new(btn_x, btn_y, 4, 1);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                collapse_icon,
                Style::default().fg(collapse_fg).bg(theme.ui.popup_bg),
            ))),
            btn_area,
        );

        self.visible_height = left_area.height as usize;
        self.left_pane_x = left_area.x;
        self.preview_height = right_area.height as usize;
        self.right_pane_area = Some(right_area);
        self.render_results(frame, left_area, theme);
        self.render_preview(frame, right_area, theme, highlighter);

        // --- Footer ---
        let footer_spans = vec![
            Span::styled(
                " Enter",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.popup_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                ": search/open  ",
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.popup_bg),
            ),
            Span::styled(
                "↑↓",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.popup_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                ": navigate  ",
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.popup_bg),
            ),
            Span::styled(
                "Esc",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.popup_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                ": close",
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.popup_bg),
            ),
        ];
        frame.render_widget(Paragraph::new(Line::from(footer_spans)), footer_area);

        // --- Cursor ---
        let cursor_x = input_area.x + 3 + self.cursor_pos as u16;
        let cursor_y = input_area.y;
        if cursor_x < input_area.x + input_area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }

    fn render_results(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let max_rows = area.height as usize;

        // Build all result rows first, then slice by scroll_offset
        let mut all_lines: Vec<Line> = Vec::new();

        for (idx, result) in self.results.iter().enumerate() {
            let is_selected = idx == self.selected;
            let is_hovered = self.hovered == Some(idx) && !is_selected;

            let relative = result
                .path
                .strip_prefix(&self.root)
                .ok()
                .and_then(|p| p.to_str())
                .unwrap_or("");

            let ext = result
                .path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            let icon = icon_for_extension(ext);

            let bg = if is_selected {
                theme.ui.file_tree_selected_bg
            } else if is_hovered {
                theme.ui.selection
            } else {
                theme.ui.popup_bg
            };

            let fg = if is_selected {
                theme.ui.file_tree_selected_fg
            } else if is_hovered {
                theme.ui.accent
            } else {
                theme.ui.foreground
            };

            let match_count = format!(" ({})", result.matches.len());

            // Arrow icon for per-file collapse
            let arrow = if self.is_file_collapsed(idx) { "▸" } else { "▾" };

            // File header line
            let mut header_spans = vec![
                Span::styled(
                    format!("{} ", arrow),
                    Style::default().fg(theme.ui.accent).bg(bg),
                ),
                Span::styled(
                    format!("{} ", icon.icon),
                    Style::default().fg(icon.color).bg(bg),
                ),
                Span::styled(
                    relative.to_string(),
                    Style::default()
                        .fg(fg)
                        .bg(bg)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    match_count,
                    Style::default().fg(theme.ui.line_number).bg(bg),
                ),
            ];

            // Fill remaining width
            let used: usize = header_spans.iter().map(|s| s.content.len()).sum();
            let remaining = (area.width as usize).saturating_sub(used);
            if remaining > 0 {
                header_spans.push(Span::styled(
                    " ".repeat(remaining),
                    Style::default().bg(bg),
                ));
            }

            all_lines.push(Line::from(header_spans));

            // Match lines (indented) — skip when file is collapsed
            if self.is_file_collapsed(idx) {
                continue;
            }
            for &(line_num, ref line_text) in &result.matches {
                let trimmed = line_text.trim();
                let max_text_len = (area.width as usize).saturating_sub(10);
                let display_text: String = if trimmed.len() > max_text_len {
                    format!("{}…", &trimmed[..max_text_len.saturating_sub(1)])
                } else {
                    trimmed.to_string()
                };

                let match_bg = if is_selected {
                    theme.ui.file_tree_selected_bg
                } else {
                    theme.ui.popup_bg
                };

                let mut match_spans = vec![
                    Span::styled("   ", Style::default().bg(match_bg)),
                    Span::styled(
                        format!("{}: ", line_num + 1),
                        Style::default().fg(theme.ui.line_number).bg(match_bg),
                    ),
                    Span::styled(
                        display_text,
                        Style::default().fg(theme.ui.foreground).bg(match_bg),
                    ),
                ];

                let used: usize = match_spans.iter().map(|s| s.content.len()).sum();
                let remaining = (area.width as usize).saturating_sub(used);
                if remaining > 0 {
                    match_spans.push(Span::styled(
                        " ".repeat(remaining),
                        Style::default().bg(match_bg),
                    ));
                }

                all_lines.push(Line::from(match_spans));
            }
        }

        // Apply scroll offset: skip rows, take visible portion
        let visible: Vec<Line> = all_lines
            .into_iter()
            .skip(self.scroll_offset)
            .take(max_rows)
            .collect();

        let mut lines = visible;

        // Fill remaining rows
        while lines.len() < max_rows {
            lines.push(Line::from(Span::styled(
                " ".repeat(area.width as usize),
                Style::default().bg(theme.ui.popup_bg),
            )));
        }

        frame.render_widget(Paragraph::new(lines), area);
    }

    fn render_preview(&self, frame: &mut Frame, area: Rect, theme: &Theme, highlighter: &Highlighter) {
        let height = area.height as usize;

        if self.preview_lines.is_empty() {
            // Empty preview
            let mut lines: Vec<Line> = Vec::new();
            let empty_msg = if self.input.is_empty() {
                "Type to search..."
            } else if self.results.is_empty() {
                "No matches found"
            } else {
                ""
            };

            if !empty_msg.is_empty() && height > 2 {
                for _ in 0..height / 2 {
                    lines.push(Line::from(Span::styled(
                        " ".repeat(area.width as usize),
                        Style::default().bg(theme.ui.background),
                    )));
                }
                let padding = (area.width as usize).saturating_sub(empty_msg.len()) / 2;
                lines.push(Line::from(Span::styled(
                    format!("{}{}", " ".repeat(padding), empty_msg),
                    Style::default()
                        .fg(theme.ui.line_number)
                        .bg(theme.ui.background),
                )));
            }

            while lines.len() < height {
                lines.push(Line::from(Span::styled(
                    " ".repeat(area.width as usize),
                    Style::default().bg(theme.ui.background),
                )));
            }

            frame.render_widget(Paragraph::new(lines), area);
            return;
        }

        // Use preview_scroll for viewport position
        let start = self.preview_scroll.min(self.preview_lines.len().saturating_sub(height));
        let end = (start + height).min(self.preview_lines.len());

        let line_count = self.preview_lines.len();
        let gutter_width = if line_count > 0 {
            line_count.to_string().len().max(3) + 2
        } else {
            5
        };

        let query_lower = self.input.to_lowercase();

        // Get match lines for the active result
        let active_idx = self.hovered.unwrap_or(self.selected);
        let match_lines: Vec<usize> = self
            .results
            .get(active_idx)
            .map(|r| r.matches.iter().map(|&(l, _)| l).collect())
            .unwrap_or_default();

        // Get file extension for syntax highlighting
        let ext = self
            .preview_path
            .as_ref()
            .and_then(|p| p.extension())
            .and_then(|e| e.to_str());

        // Syntax highlight visible lines
        let visible_texts: Vec<&str> = (start..end)
            .map(|i| self.preview_lines[i].as_str())
            .collect();
        let highlighted = highlighter.highlight_lines(&visible_texts, ext);

        let mut lines: Vec<Line> = Vec::new();

        for (idx, line_idx) in (start..end).enumerate() {
            let line_text = &self.preview_lines[line_idx];
            let is_match_line = match_lines.contains(&line_idx);

            let num_style = if is_match_line {
                Style::default().fg(theme.ui.line_number_active)
            } else {
                Style::default().fg(theme.ui.line_number)
            };

            let num_str = format!(
                "{:>width$} ",
                line_idx + 1,
                width = gutter_width - 2
            );

            let line_bg = if is_match_line {
                theme.ui.selection
            } else {
                theme.ui.background
            };

            let mut spans = vec![Span::styled(num_str, num_style.bg(line_bg))];

            // Build syntax-highlighted content spans
            let mut content_spans: Vec<Span> = Vec::new();
            if idx < highlighted.len() {
                for hl_span in &highlighted[idx].spans {
                    let mut style = Style::default()
                        .fg(Color::Rgb(hl_span.fg.0, hl_span.fg.1, hl_span.fg.2))
                        .bg(line_bg);
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
            } else {
                content_spans.push(Span::styled(
                    line_text.to_string(),
                    Style::default().fg(theme.ui.foreground).bg(line_bg),
                ));
            }

            // Overlay search match highlighting on top of syntax colors
            if is_match_line && !query_lower.is_empty() {
                content_spans =
                    apply_search_bg(content_spans, line_text, &query_lower, self.input.len(), theme.ui.search_match);
            }

            spans.extend(content_spans);
            lines.push(Line::from(spans));
        }

        // Fill remaining
        while lines.len() < height {
            lines.push(Line::from(Span::styled(
                " ".repeat(area.width as usize),
                Style::default().bg(theme.ui.background),
            )));
        }

        let preview_block = Block::default()
            .style(Style::default().bg(theme.ui.background));

        frame.render_widget(Paragraph::new(lines).block(preview_block), area);

        // --- Scrollbar ---
        let total = self.preview_lines.len();
        if total > height {
            let scrollbar_x = area.x + area.width - 1;
            let track_height = height as f64;
            let thumb_size = ((height as f64 / total as f64) * track_height).max(1.0) as u16;
            let max_scroll = total.saturating_sub(height);
            let thumb_pos = if max_scroll > 0 {
                ((start as f64 / max_scroll as f64) * (track_height - thumb_size as f64)) as u16
            } else {
                0
            };

            for row in 0..height as u16 {
                let ch = if row >= thumb_pos && row < thumb_pos + thumb_size {
                    "┃"
                } else {
                    "│"
                };
                let style = if row >= thumb_pos && row < thumb_pos + thumb_size {
                    Style::default().fg(theme.ui.accent).bg(theme.ui.background)
                } else {
                    Style::default().fg(theme.ui.border).bg(theme.ui.background)
                };
                let bar_area = Rect::new(scrollbar_x, area.y + row, 1, 1);
                frame.render_widget(Paragraph::new(Line::from(Span::styled(ch, style))), bar_area);
            }
        }
    }
}

/// Overlay search match background color on syntax-highlighted spans.
fn apply_search_bg<'a>(
    spans: Vec<Span<'a>>,
    line_text: &str,
    query_lower: &str,
    query_len: usize,
    match_bg: Color,
) -> Vec<Span<'a>> {
    if query_lower.is_empty() || spans.is_empty() {
        return spans;
    }

    // Build a flat array of (char, style) from existing spans
    let mut chars: Vec<(char, Style)> = Vec::new();
    for span in &spans {
        let style = span.style;
        for c in span.content.chars() {
            chars.push((c, style));
        }
    }

    // Find match positions in the original line (case-insensitive)
    let line_lower = line_text.to_lowercase();
    let mut search_start = 0;
    while let Some(pos) = line_lower[search_start..].find(query_lower) {
        let abs_pos = search_start + pos;
        let end = (abs_pos + query_len).min(chars.len());
        for item in chars.iter_mut().take(end).skip(abs_pos) {
            item.1 = item.1.bg(match_bg);
        }
        search_start = abs_pos + 1;
    }

    // Rebuild spans from the flat array, merging consecutive chars with same style
    let mut result: Vec<Span<'a>> = Vec::new();
    let mut current_text = String::new();
    let mut current_style: Option<Style> = None;

    for (c, style) in &chars {
        if current_style == Some(*style) {
            current_text.push(*c);
        } else {
            if let Some(s) = current_style {
                if !current_text.is_empty() {
                    result.push(Span::styled(std::mem::take(&mut current_text), s));
                }
            }
            current_text.push(*c);
            current_style = Some(*style);
        }
    }
    if let Some(s) = current_style {
        if !current_text.is_empty() {
            result.push(Span::styled(current_text, s));
        }
    }

    result
}
