use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs};
use ratatui::Frame;

use k_git::{BranchInfo, WorktreeInfo};

use crate::layout::centered_rect;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorktreeTab {
    List,
    Create,
}

impl WorktreeTab {
    pub fn next(&self) -> Self {
        match self {
            Self::List => Self::Create,
            Self::Create => Self::List,
        }
    }

    pub fn prev(&self) -> Self {
        self.next()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateField {
    Branch,
    BaseBranch,
}

pub struct WorktreePanelComponent {
    pub active_tab: WorktreeTab,
    pub worktrees: Vec<WorktreeInfo>,
    pub selected: usize,
    pub branch_input: String,
    pub base_branch_input: String,
    pub branches: Vec<BranchInfo>,
    pub creating: bool,
    pub create_field: CreateField,
    pub message: String,
    pub confirm_delete: bool,
    /// Stored (index, Rect) for each delete button rendered in the list
    pub delete_button_areas: Vec<(usize, Rect)>,
}

impl WorktreePanelComponent {
    pub fn new() -> Self {
        Self {
            active_tab: WorktreeTab::List,
            worktrees: Vec::new(),
            selected: 0,
            branch_input: String::new(),
            base_branch_input: "main".to_string(),
            branches: Vec::new(),
            creating: false,
            create_field: CreateField::Branch,
            message: String::new(),
            confirm_delete: false,
            delete_button_areas: Vec::new(),
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.branch_input.clear();
        self.base_branch_input = "main".to_string();
        self.creating = false;
        self.create_field = CreateField::Branch;
        self.message.clear();
        self.confirm_delete = false;
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.selected = 0;
        self.confirm_delete = false;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.selected = 0;
        self.confirm_delete = false;
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.confirm_delete = false;
    }

    pub fn move_down(&mut self) {
        let max = match self.active_tab {
            WorktreeTab::List => self.worktrees.len(),
            WorktreeTab::Create => 0,
        };
        if max > 0 && self.selected + 1 < max {
            self.selected += 1;
        }
        self.confirm_delete = false;
    }

    pub fn selected_worktree(&self) -> Option<&WorktreeInfo> {
        self.worktrees.get(self.selected)
    }

    /// Check if a mouse click hit a delete button. Returns the worktree index if so.
    pub fn click_delete_at(&self, x: u16, y: u16) -> Option<usize> {
        for &(idx, rect) in &self.delete_button_areas {
            if x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height {
                return Some(idx);
            }
        }
        None
    }

    pub fn render(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = centered_rect(75, 75, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Worktrees (Ctrl+W) ")
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

        if inner.height < 4 {
            return;
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(1),
                Constraint::Length(1),
            ])
            .split(inner);

        // Tabs
        let tab_titles = vec!["List", "Create"];
        let selected_idx = match self.active_tab {
            WorktreeTab::List => 0,
            WorktreeTab::Create => 1,
        };

        let tabs = Tabs::new(tab_titles)
            .select(selected_idx)
            .style(Style::default().fg(theme.ui.foreground).bg(theme.ui.popup_bg))
            .highlight_style(
                Style::default()
                    .fg(theme.ui.accent)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .divider("│");

        frame.render_widget(tabs, chunks[0]);

        // Separator
        let sep = Paragraph::new(Line::from(Span::styled(
            "─".repeat(chunks[1].width as usize),
            Style::default().fg(theme.ui.border).bg(theme.ui.popup_bg),
        )));
        frame.render_widget(sep, chunks[1]);

        // Content
        let content_area = chunks[2];
        match self.active_tab {
            WorktreeTab::List => self.render_list(frame, content_area, theme),
            WorktreeTab::Create => self.render_create(frame, content_area, theme, popup_area),
        }

        // Footer help
        let help_text = match self.active_tab {
            WorktreeTab::List => {
                if self.confirm_delete {
                    "y: confirm delete │ n/Esc: cancel"
                } else {
                    "Tab: switch │ n: new │ d: delete │ Enter: open terminal │ P: prune │ r: refresh │ Esc: close"
                }
            }
            WorktreeTab::Create => {
                "Tab: switch field │ Enter: create │ Esc: cancel"
            }
        };
        let footer = Paragraph::new(Line::from(Span::styled(
            help_text,
            Style::default()
                .fg(theme.ui.line_number)
                .bg(theme.ui.popup_bg),
        )));
        frame.render_widget(footer, chunks[3]);

        // Message overlay
        if !self.message.is_empty() {
            let msg_area = Rect::new(
                popup_area.x + 2,
                popup_area.y + popup_area.height.saturating_sub(3),
                popup_area.width.saturating_sub(4),
                1,
            );
            let msg = Paragraph::new(Line::from(Span::styled(
                &self.message,
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.popup_bg)
                    .add_modifier(Modifier::BOLD),
            )));
            frame.render_widget(msg, msg_area);
        }
    }

    fn render_list(&mut self, frame: &mut Frame, area: Rect, theme: &Theme) {
        self.delete_button_areas.clear();

        if self.worktrees.is_empty() {
            let empty = Paragraph::new(Line::from(Span::styled(
                "  No worktrees found. Press 'n' to create one.",
                Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg),
            )));
            frame.render_widget(empty, area);
            return;
        }

        // We need to track the width of each line to position delete buttons,
        // so we compute spans and line widths together.
        let mut items: Vec<ListItem> = Vec::new();
        let mut delete_btn_info: Vec<(usize, u16)> = Vec::new(); // (worktree_idx, line_width_before_btn)

        for (idx, wt) in self.worktrees.iter().enumerate() {
            let is_selected = idx == self.selected;
            let is_deleting = is_selected && self.confirm_delete;

            let status_icon = if wt.is_main {
                "\u{f015}" // home icon
            } else if wt.status_count == 0 {
                "\u{f00c}" // checkmark
            } else {
                "\u{f111}" // circle (modified)
            };

            let status_color = if wt.is_main {
                theme.ui.accent
            } else if wt.status_count == 0 {
                theme.syntax.string // green-ish
            } else {
                theme.syntax.keyword // yellow/orange-ish
            };

            let mut spans = vec![
                Span::styled(
                    if is_selected { " \u{f054} " } else { "   " },
                    Style::default().fg(theme.ui.accent).bg(theme.ui.popup_bg),
                ),
                Span::styled(
                    format!("{} ", status_icon),
                    Style::default().fg(status_color).bg(theme.ui.popup_bg),
                ),
                Span::styled(
                    format!("{:<20}", wt.branch),
                    Style::default()
                        .fg(if is_selected { theme.ui.accent } else { theme.ui.foreground })
                        .bg(theme.ui.popup_bg)
                        .add_modifier(if is_selected { Modifier::BOLD } else { Modifier::empty() }),
                ),
            ];

            if !wt.is_main {
                spans.push(Span::styled(
                    format!("  {}", wt.path),
                    Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg),
                ));
            } else {
                spans.push(Span::styled(
                    "  (main worktree)",
                    Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg),
                ));
            }

            if wt.status_count > 0 {
                spans.push(Span::styled(
                    format!("  ~{}", wt.status_count),
                    Style::default().fg(theme.syntax.keyword).bg(theme.ui.popup_bg),
                ));
            }

            if wt.commits_ahead > 0 {
                spans.push(Span::styled(
                    format!("  \u{f062}{}", wt.commits_ahead),
                    Style::default().fg(theme.syntax.string).bg(theme.ui.popup_bg),
                ));
            }

            if is_deleting {
                spans.push(Span::styled(
                    "  [Delete? y/n]",
                    Style::default()
                        .fg(theme.syntax.keyword)
                        .bg(theme.ui.popup_bg)
                        .add_modifier(Modifier::BOLD),
                ));
            } else if !wt.is_main {
                // Calculate current line width for button positioning
                let line_width: u16 = spans.iter().map(|s| s.content.chars().count() as u16).sum();
                delete_btn_info.push((idx, line_width));

                spans.push(Span::styled(
                    "  [✕]",
                    Style::default()
                        .fg(theme.syntax.keyword)
                        .bg(theme.ui.popup_bg),
                ));
            }

            items.push(ListItem::new(Line::from(spans)));
        }

        // Store delete button areas based on area position + line offsets
        for (wt_idx, line_width) in &delete_btn_info {
            let row = area.y + *wt_idx as u16;
            if row < area.y + area.height {
                // "  [✕]" = 5 chars, but the button label starts after 2 spaces
                let btn_x = area.x + line_width + 2; // skip the 2 leading spaces
                let btn_width = 3; // [✕]
                self.delete_button_areas.push((*wt_idx, Rect::new(btn_x, row, btn_width, 1)));
            }
        }

        let list = List::new(items)
            .style(Style::default().bg(theme.ui.popup_bg));

        frame.render_widget(list, area);
    }

    fn render_create(&self, frame: &mut Frame, area: Rect, theme: &Theme, _popup_area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Length(2),
                Constraint::Length(1),
                Constraint::Min(1),
            ])
            .split(area);

        // Branch name label
        let branch_label_style = if self.create_field == CreateField::Branch {
            Style::default().fg(theme.ui.accent).bg(theme.ui.popup_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.ui.foreground).bg(theme.ui.popup_bg)
        };
        let branch_label = Paragraph::new(Line::from(Span::styled(
            "  Branch name:",
            branch_label_style,
        )));
        frame.render_widget(branch_label, chunks[0]);

        // Branch input
        let input_style = if self.create_field == CreateField::Branch {
            Style::default().fg(theme.ui.foreground).bg(theme.ui.background)
        } else {
            Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg)
        };
        let input_area = Rect::new(area.x + 2, chunks[1].y, area.width.saturating_sub(4), 1);
        let display_text = if self.branch_input.is_empty() && self.create_field != CreateField::Branch {
            "feature-name".to_string()
        } else {
            self.branch_input.clone()
        };
        let branch_input = Paragraph::new(Line::from(Span::styled(
            format!(" {} ", display_text),
            input_style,
        )));
        frame.render_widget(branch_input, input_area);

        if self.create_field == CreateField::Branch {
            let cursor_x = input_area.x + 1 + self.branch_input.len() as u16;
            frame.set_cursor_position((cursor_x, input_area.y));
        }

        // Base branch label
        let base_label_style = if self.create_field == CreateField::BaseBranch {
            Style::default().fg(theme.ui.accent).bg(theme.ui.popup_bg).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.ui.foreground).bg(theme.ui.popup_bg)
        };
        let base_label = Paragraph::new(Line::from(Span::styled(
            "  Base branch:",
            base_label_style,
        )));
        frame.render_widget(base_label, chunks[2]);

        // Base branch input
        let base_input_style = if self.create_field == CreateField::BaseBranch {
            Style::default().fg(theme.ui.foreground).bg(theme.ui.background)
        } else {
            Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg)
        };
        let base_input_area = Rect::new(area.x + 2, chunks[3].y, area.width.saturating_sub(4), 1);
        let base_input = Paragraph::new(Line::from(Span::styled(
            format!(" {} ", self.base_branch_input),
            base_input_style,
        )));
        frame.render_widget(base_input, base_input_area);

        if self.create_field == CreateField::BaseBranch {
            let cursor_x = base_input_area.x + 1 + self.base_branch_input.len() as u16;
            frame.set_cursor_position((cursor_x, base_input_area.y));
        }

        // Info text
        let info_text = "  Worktree will be created as sibling directory: <repo>-<branch>";
        let info = Paragraph::new(Line::from(Span::styled(
            info_text,
            Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg),
        )));
        frame.render_widget(info, chunks[4]);

        // Show available branches for reference
        if !self.branches.is_empty() {
            let branch_lines: Vec<Line> = std::iter::once(
                Line::from(Span::styled(
                    "  Available branches:",
                    Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg),
                ))
            ).chain(
                self.branches.iter()
                    .filter(|b| !b.is_remote)
                    .take(chunks[5].height.saturating_sub(1) as usize)
                    .map(|b| {
                        let icon = if b.is_current { "\u{f00c} " } else { "  " };
                        Line::from(Span::styled(
                            format!("    {}{}", icon, b.name),
                            Style::default()
                                .fg(if b.is_current { theme.ui.accent } else { theme.ui.foreground })
                                .bg(theme.ui.popup_bg),
                        ))
                    })
            ).collect();

            let branches_list = Paragraph::new(branch_lines);
            frame.render_widget(branches_list, chunks[5]);
        }
    }
}
