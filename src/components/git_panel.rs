use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Tabs};
use ratatui::Frame;

use k_git::{BranchInfo, CommitInfo, StashEntry, StatusEntry, StatusKind};

use crate::layout::centered_rect;
use crate::theme::Theme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitTab {
    Status,
    Diff,
    Branches,
    Stash,
    Log,
    Actions,
}

impl GitTab {
    pub fn next(&self) -> Self {
        match self {
            Self::Status => Self::Diff,
            Self::Diff => Self::Branches,
            Self::Branches => Self::Stash,
            Self::Stash => Self::Log,
            Self::Log => Self::Actions,
            Self::Actions => Self::Status,
        }
    }

    pub fn prev(&self) -> Self {
        match self {
            Self::Status => Self::Actions,
            Self::Diff => Self::Status,
            Self::Branches => Self::Diff,
            Self::Stash => Self::Branches,
            Self::Log => Self::Stash,
            Self::Actions => Self::Log,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionRun {
    pub name: String,
    pub status: ActionStatus,
    pub conclusion: String,
    pub branch: String,
    pub event: String,
    pub elapsed: String,
    pub url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionStatus {
    Success,
    Failure,
    InProgress,
    Queued,
    Cancelled,
    Skipped,
    Unknown,
}

impl ActionStatus {
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Success => "\u{f00c}",     // ✓
            Self::Failure => "\u{f00d}",     // ✗
            Self::InProgress => "\u{f110}",  // spinner
            Self::Queued => "\u{f017}",      // clock
            Self::Cancelled => "\u{f05e}",   // ban
            Self::Skipped => "\u{f04e}",     // forward
            Self::Unknown => "\u{f128}",     // ?
        }
    }
}

pub struct GitPanelComponent {
    pub active_tab: GitTab,
    pub status_entries: Vec<StatusEntry>,
    pub changes_files: Vec<ChangedFile>,
    pub hovered_change: Option<usize>,
    pub branches: Vec<BranchInfo>,
    pub stashes: Vec<StashEntry>,
    pub log_entries: Vec<CommitInfo>,
    pub action_runs: Vec<ActionRun>,
    pub actions_loading: bool,
    pub actions_error: Option<String>,
    pub selected: usize,
    pub commit_input: String,
    pub editing_commit: bool,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct ChangedFile {
    pub path: String,
    pub kind: StatusKind,
    pub additions: usize,
    pub deletions: usize,
}

impl GitPanelComponent {
    pub fn new() -> Self {
        Self {
            active_tab: GitTab::Status,
            status_entries: Vec::new(),
            changes_files: Vec::new(),
            hovered_change: None,
            branches: Vec::new(),
            stashes: Vec::new(),
            log_entries: Vec::new(),
            action_runs: Vec::new(),
            actions_loading: false,
            actions_error: None,
            selected: 0,
            commit_input: String::new(),
            editing_commit: false,
            message: String::new(),
        }
    }

    pub fn reset(&mut self) {
        self.selected = 0;
        self.commit_input.clear();
        self.editing_commit = false;
        self.message.clear();
    }

    pub fn next_tab(&mut self) {
        self.active_tab = self.active_tab.next();
        self.selected = 0;
    }

    pub fn prev_tab(&mut self) {
        self.active_tab = self.active_tab.prev();
        self.selected = 0;
    }

    /// Given a click at absolute (x, y), determine if it hit a git panel tab.
    /// `tabs_area` is the Rect where the Tabs widget is rendered.
    pub fn tab_at_position(&self, x: u16, y: u16, tabs_area: Rect) -> Option<GitTab> {
        if y != tabs_area.y || x < tabs_area.x || x >= tabs_area.x + tabs_area.width {
            return None;
        }
        let rel_x = (x - tabs_area.x) as usize;
        // Tabs widget renders: " Title │ Title │ Title "
        // Each tab has 1 padding left + title + 1 padding right, then "│" divider (3 bytes but 1 col)
        let tab_titles = ["Status", "Diff", "Branches", "Stash", "Log", "Actions"];
        let mut pos = 0usize;
        for (idx, title) in tab_titles.iter().enumerate() {
            let tab_width = 1 + title.len() + 1; // " Title "
            if rel_x < pos + tab_width {
                return Some(match idx {
                    0 => GitTab::Status,
                    1 => GitTab::Diff,
                    2 => GitTab::Branches,
                    3 => GitTab::Stash,
                    4 => GitTab::Log,
                    5 => GitTab::Actions,
                    _ => return None,
                });
            }
            pos += tab_width;
            // divider "│" takes 1 column
            if idx < tab_titles.len() - 1 {
                if rel_x < pos + 1 {
                    return None; // clicked on divider
                }
                pos += 1;
            }
        }
        None
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max = match self.active_tab {
            GitTab::Status => self.status_entries.len(),
            GitTab::Diff => self.changes_files.len(),
            GitTab::Branches => self.branches.len(),
            GitTab::Stash => self.stashes.len(),
            GitTab::Log => self.log_entries.len(),
            GitTab::Actions => self.action_runs.len(),
        };
        if self.selected + 1 < max {
            self.selected += 1;
        }
    }

    pub fn load_actions(&mut self) {
        self.actions_loading = true;
        self.actions_error = None;
        self.action_runs.clear();

        // Use `gh` CLI to fetch workflow runs
        let output = std::process::Command::new("gh")
            .args(["run", "list", "--limit", "20", "--json", "name,status,conclusion,headBranch,event,updatedAt,url"])
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let json_str = String::from_utf8_lossy(&out.stdout);
                    self.parse_actions_json(&json_str);
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    if stderr.contains("not a git repository") || stderr.contains("could not determine") {
                        self.actions_error = Some("Not a GitHub repository".to_string());
                    } else if stderr.contains("gh auth") {
                        self.actions_error = Some("Run 'gh auth login' to authenticate".to_string());
                    } else {
                        self.actions_error = Some(format!("gh error: {}", stderr.trim()));
                    }
                }
            }
            Err(_) => {
                self.actions_error = Some("'gh' CLI not found. Install: https://cli.github.com".to_string());
            }
        }
        self.actions_loading = false;
    }

    fn parse_actions_json(&mut self, json: &str) {
        // Simple JSON parsing without serde_json dependency
        // Format: [{"name":"...","status":"...","conclusion":"...","headBranch":"...","event":"...","updatedAt":"...","url":"..."},...]
        let json = json.trim();
        if json == "[]" || json.is_empty() {
            return;
        }

        // Split by },{ to get individual objects
        let inner = json.trim_start_matches('[').trim_end_matches(']');
        let objects: Vec<&str> = split_json_objects(inner);

        for obj in objects {
            let name = extract_json_field(obj, "name");
            let status_str = extract_json_field(obj, "status");
            let conclusion = extract_json_field(obj, "conclusion");
            let branch = extract_json_field(obj, "headBranch");
            let event = extract_json_field(obj, "event");
            let updated = extract_json_field(obj, "updatedAt");
            let url = extract_json_field(obj, "url");

            let status = match (status_str.as_str(), conclusion.as_str()) {
                ("completed", "success") => ActionStatus::Success,
                ("completed", "failure") => ActionStatus::Failure,
                ("completed", "cancelled") => ActionStatus::Cancelled,
                ("completed", "skipped") => ActionStatus::Skipped,
                ("in_progress", _) => ActionStatus::InProgress,
                ("queued", _) => ActionStatus::Queued,
                _ => ActionStatus::Unknown,
            };

            // Format elapsed time from updatedAt
            let elapsed = format_relative_time(&updated);

            self.action_runs.push(ActionRun {
                name,
                status,
                conclusion,
                branch,
                event,
                elapsed,
                url,
            });
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let popup_area = centered_rect(75, 75, area);
        frame.render_widget(Clear, popup_area);

        let block = Block::default()
            .title(" Git (Ctrl+G) ")
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
        let tab_titles = vec!["Status", "Diff", "Branches", "Stash", "Log", "Actions"];
        let selected_idx = match self.active_tab {
            GitTab::Status => 0,
            GitTab::Diff => 1,
            GitTab::Branches => 2,
            GitTab::Stash => 3,
            GitTab::Log => 4,
            GitTab::Actions => 5,
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
        match self.active_tab {
            GitTab::Status => self.render_status(frame, chunks[2], theme),
            GitTab::Diff => self.render_changes(frame, chunks[2], theme),
            GitTab::Branches => self.render_branches(frame, chunks[2], theme),
            GitTab::Stash => self.render_stashes(frame, chunks[2], theme),
            GitTab::Log => self.render_log(frame, chunks[2], theme),
            GitTab::Actions => self.render_actions(frame, chunks[2], theme),
        }

        // Footer help
        let help_text = match self.active_tab {
            GitTab::Status => "Tab: switch │ a: stage all │ s: stage file │ c: commit │ p: push │ Esc: close",
            GitTab::Diff => "Tab: switch │ Enter/click: view diff │ j/k: navigate │ Esc: close",
            GitTab::Branches => "Tab: switch │ Enter: checkout │ Esc: close",
            GitTab::Stash => "Tab: switch │ s: stash │ Enter: pop │ Esc: close",
            GitTab::Log => "Tab: switch │ j/k: navigate │ Esc: close",
            GitTab::Actions => "Tab: switch │ r: refresh │ o: open in browser │ j/k: navigate │ Esc: close",
        };
        let footer = Paragraph::new(Line::from(Span::styled(
            help_text,
            Style::default()
                .fg(theme.ui.line_number)
                .bg(theme.ui.popup_bg),
        )));
        frame.render_widget(footer, chunks[3]);

        // Commit input overlay
        if self.editing_commit {
            let commit_area = centered_rect(50, 20, area);
            frame.render_widget(Clear, commit_area);
            let commit_block = Block::default()
                .title(" Commit Message ")
                .title_style(Style::default().fg(theme.ui.accent).add_modifier(Modifier::BOLD))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.ui.accent))
                .style(Style::default().bg(theme.ui.popup_bg));
            let commit_inner = commit_block.inner(commit_area);
            frame.render_widget(commit_block, commit_area);

            let input = Paragraph::new(self.commit_input.as_str())
                .style(Style::default().fg(theme.ui.foreground).bg(theme.ui.popup_bg));
            frame.render_widget(input, commit_inner);

            let cursor_x = commit_inner.x + self.commit_input.len() as u16;
            frame.set_cursor_position((cursor_x, commit_inner.y));
        }

        // Message
        if !self.message.is_empty() {
            let msg_area = Rect::new(
                popup_area.x + 1,
                popup_area.y + popup_area.height,
                popup_area.width.saturating_sub(2),
                1,
            );
            if msg_area.y < area.height {
                let msg = Paragraph::new(Line::from(Span::styled(
                    &self.message,
                    Style::default().fg(theme.ui.accent),
                )));
                frame.render_widget(msg, msg_area);
            }
        }
    }

    pub fn load_branch_changes(&mut self, workspace_root: &std::path::Path) {
        // Find git toplevel to ensure commands run from repo root
        let git_root = std::process::Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(workspace_root)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        let root = if git_root.is_empty() {
            workspace_root.to_path_buf()
        } else {
            std::path::PathBuf::from(&git_root)
        };

        // git status --porcelain --no-untracked-files=no --ignored=no
        // Shows only tracked changes, strictly respecting .gitignore
        let status_output = std::process::Command::new("git")
            .args(["status", "--porcelain", "-uno"])
            .current_dir(&root)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Also get unstaged changes on tracked files
        let status_unstaged = std::process::Command::new("git")
            .args(["diff", "--name-status"])
            .current_dir(&root)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Get numstat for line counts (staged + unstaged vs HEAD)
        let numstat = std::process::Command::new("git")
            .args(["diff", "HEAD", "--numstat"])
            .current_dir(&root)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Also get staged numstat
        let numstat_cached = std::process::Command::new("git")
            .args(["diff", "--cached", "--numstat"])
            .current_dir(&root)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Build stats map: path -> (additions, deletions)
        let mut stats: std::collections::HashMap<String, (usize, usize)> =
            std::collections::HashMap::new();
        for src in [&numstat, &numstat_cached] {
            for line in src.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    let add = parts[0].parse::<usize>().unwrap_or(0);
                    let del = parts[1].parse::<usize>().unwrap_or(0);
                    let entry = stats.entry(parts[2].to_string()).or_insert((0, 0));
                    entry.0 = entry.0.max(add);
                    entry.1 = entry.1.max(del);
                }
            }
        }

        let mut files: Vec<ChangedFile> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // Parse staged changes from git status --porcelain -uno
        for line in status_output.lines() {
            if line.len() < 3 {
                continue;
            }
            let path = line[3..].trim_matches('"').to_string();
            if path.ends_with('/') || seen.contains(&path) {
                continue;
            }
            seen.insert(path.clone());

            let xy = &line[..2];
            let kind = match xy.chars().next().unwrap_or(' ') {
                'M' => StatusKind::Modified,
                'A' => StatusKind::New,
                'D' => StatusKind::Deleted,
                'R' => StatusKind::Renamed,
                'C' => StatusKind::Modified,
                _ => {
                    // Check second char (working tree)
                    match xy.chars().nth(1).unwrap_or(' ') {
                        'M' => StatusKind::Modified,
                        'D' => StatusKind::Deleted,
                        _ => StatusKind::Modified,
                    }
                }
            };

            let (additions, deletions) = stats.get(&path).copied().unwrap_or((0, 0));
            files.push(ChangedFile { path, kind, additions, deletions });
        }

        // Add unstaged tracked changes not yet seen
        for line in status_unstaged.lines() {
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() < 2 {
                continue;
            }
            let path = parts[1].trim().to_string();
            if path.ends_with('/') || seen.contains(&path) {
                continue;
            }
            seen.insert(path.clone());

            let kind = match parts[0].trim() {
                "M" => StatusKind::Modified,
                "A" => StatusKind::New,
                "D" => StatusKind::Deleted,
                s if s.starts_with('R') => StatusKind::Renamed,
                _ => StatusKind::Modified,
            };

            let (additions, deletions) = stats.get(&path).copied().unwrap_or((0, 0));
            files.push(ChangedFile { path, kind, additions, deletions });
        }

        self.changes_files = files;
        self.hovered_change = None;
        self.selected = 0;
    }

    pub fn selected_changed_file(&self) -> Option<&ChangedFile> {
        self.changes_files.get(self.selected)
    }

    /// Update hover state for Changes tab. Returns true if inside the content area.
    pub fn hover_change_at(&mut self, x: u16, y: u16, area: Rect) -> bool {
        let popup_area = centered_rect(75, 75, area);
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(popup_area);

        if inner.height < 4 {
            self.hovered_change = None;
            return false;
        }

        // Content starts at inner.y + 2 (tabs row + separator)
        let content_y = inner.y + 2;
        let content_bottom = inner.y + inner.height.saturating_sub(1); // minus footer

        if x < inner.x || x >= inner.x + inner.width || y < content_y || y >= content_bottom {
            self.hovered_change = None;
            return false;
        }

        let idx = (y - content_y) as usize;
        if idx < self.changes_files.len() {
            self.hovered_change = Some(idx);
            true
        } else {
            self.hovered_change = None;
            false
        }
    }

    /// Handle click on Changes tab content area. Returns selected file index if clicked.
    pub fn click_change_at(&mut self, x: u16, y: u16, area: Rect) -> Option<usize> {
        let popup_area = centered_rect(75, 75, area);
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(popup_area);

        if inner.height < 4 {
            return None;
        }

        let content_y = inner.y + 2;
        let content_bottom = inner.y + inner.height.saturating_sub(1);

        if x < inner.x || x >= inner.x + inner.width || y < content_y || y >= content_bottom {
            return None;
        }

        let idx = (y - content_y) as usize;
        if idx < self.changes_files.len() {
            self.selected = idx;
            Some(idx)
        } else {
            None
        }
    }

    fn render_changes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.changes_files.is_empty() {
            let msg = Paragraph::new(Line::from(Span::styled(
                " No changed files",
                Style::default().fg(theme.ui.line_number).bg(theme.ui.popup_bg),
            )));
            frame.render_widget(msg, area);
            return;
        }

        let items: Vec<ListItem> = self
            .changes_files
            .iter()
            .enumerate()
            .map(|(idx, file)| {
                let is_selected = idx == self.selected;
                let is_hovered = self.hovered_change == Some(idx) && !is_selected;
                let (indicator, color) = match file.kind {
                    StatusKind::New => ("A", theme.git.added),
                    StatusKind::Modified => ("M", theme.git.modified),
                    StatusKind::Deleted => ("D", theme.git.deleted),
                    StatusKind::Renamed => ("R", theme.git.renamed),
                    StatusKind::Conflicted => ("C", theme.git.conflict),
                    StatusKind::Untracked => ("?", theme.git.modified),
                    _ => (" ", theme.ui.foreground),
                };
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
                let mut spans = vec![
                    Span::styled(
                        format!(" {} ", indicator),
                        Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(&file.path, Style::default().fg(fg).bg(bg)),
                ];
                // Show additions/deletions counts
                if file.additions > 0 || file.deletions > 0 {
                    spans.push(Span::styled("  ", Style::default().bg(bg)));
                    if file.additions > 0 {
                        spans.push(Span::styled(
                            format!("++{}", file.additions),
                            Style::default().fg(theme.git.added).bg(bg),
                        ));
                        spans.push(Span::styled(" ", Style::default().bg(bg)));
                    }
                    if file.deletions > 0 {
                        spans.push(Span::styled(
                            format!("--{}", file.deletions),
                            Style::default().fg(theme.git.deleted).bg(bg),
                        ));
                    }
                }
                ListItem::new(Line::from(spans))
            })
            .collect();

        let list = List::new(items).style(Style::default().bg(theme.ui.popup_bg));
        frame.render_widget(list, area);
    }

    fn render_status(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .status_entries
            .iter()
            .enumerate()
            .map(|(idx, entry)| {
                let is_selected = idx == self.selected;
                let (indicator, color) = match entry.kind {
                    StatusKind::New => ("A", theme.git.added),
                    StatusKind::Modified => ("M", theme.git.modified),
                    StatusKind::Deleted => ("D", theme.git.deleted),
                    StatusKind::Renamed => ("R", theme.git.renamed),
                    StatusKind::Conflicted => ("C", theme.git.conflict),
                    StatusKind::Untracked => ("?", theme.git.modified),
                    _ => (" ", theme.ui.foreground),
                };
                let bg = if is_selected {
                    theme.ui.file_tree_selected_bg
                } else {
                    theme.ui.popup_bg
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {} ", indicator),
                        Style::default().fg(color).bg(bg).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &entry.path,
                        Style::default().fg(theme.ui.foreground).bg(bg),
                    ),
                ]))
            })
            .collect();

        if items.is_empty() {
            let msg = Paragraph::new(Line::from(Span::styled(
                " Nothing to commit, working tree clean",
                Style::default()
                    .fg(theme.git.added)
                    .bg(theme.ui.popup_bg),
            )));
            frame.render_widget(msg, area);
        } else {
            let list = List::new(items).style(Style::default().bg(theme.ui.popup_bg));
            frame.render_widget(list, area);
        }
    }

    fn render_branches(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .branches
            .iter()
            .enumerate()
            .map(|(idx, branch)| {
                let is_selected = idx == self.selected;
                let bg = if is_selected {
                    theme.ui.file_tree_selected_bg
                } else {
                    theme.ui.popup_bg
                };
                let prefix = if branch.is_current { "* " } else { "  " };
                let color = if branch.is_current {
                    theme.git.branch
                } else if branch.is_remote {
                    theme.git.renamed
                } else {
                    theme.ui.foreground
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" {}{}", prefix, branch.name),
                    Style::default().fg(color).bg(bg),
                )))
            })
            .collect();

        let list = List::new(items).style(Style::default().bg(theme.ui.popup_bg));
        frame.render_widget(list, area);
    }

    fn render_stashes(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .stashes
            .iter()
            .enumerate()
            .map(|(idx, stash)| {
                let is_selected = idx == self.selected;
                let bg = if is_selected {
                    theme.ui.file_tree_selected_bg
                } else {
                    theme.ui.popup_bg
                };
                ListItem::new(Line::from(Span::styled(
                    format!(" stash@{{{}}}: {}", stash.index, stash.message),
                    Style::default().fg(theme.ui.foreground).bg(bg),
                )))
            })
            .collect();

        if items.is_empty() {
            let msg = Paragraph::new(Line::from(Span::styled(
                " No stashes",
                Style::default()
                    .fg(theme.ui.line_number)
                    .bg(theme.ui.popup_bg),
            )));
            frame.render_widget(msg, area);
        } else {
            let list = List::new(items).style(Style::default().bg(theme.ui.popup_bg));
            frame.render_widget(list, area);
        }
    }

    fn render_log(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let items: Vec<ListItem> = self
            .log_entries
            .iter()
            .enumerate()
            .map(|(idx, commit)| {
                let is_selected = idx == self.selected;
                let bg = if is_selected {
                    theme.ui.file_tree_selected_bg
                } else {
                    theme.ui.popup_bg
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {} ", commit.short_id),
                        Style::default()
                            .fg(theme.ui.accent)
                            .bg(bg)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &commit.message,
                        Style::default().fg(theme.ui.foreground).bg(bg),
                    ),
                    Span::styled(
                        format!("  {} - {}", commit.author, commit.date),
                        Style::default().fg(theme.ui.line_number).bg(bg),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items).style(Style::default().bg(theme.ui.popup_bg));
        frame.render_widget(list, area);
    }

    fn render_actions(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if self.actions_loading {
            let msg = Paragraph::new(Line::from(Span::styled(
                " \u{f110} Loading GitHub Actions...",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.popup_bg),
            )));
            frame.render_widget(msg, area);
            return;
        }

        if let Some(ref err) = self.actions_error {
            let msg = Paragraph::new(Line::from(Span::styled(
                format!(" \u{f071} {}", err),
                Style::default()
                    .fg(theme.git.deleted)
                    .bg(theme.ui.popup_bg),
            )));
            frame.render_widget(msg, area);
            return;
        }

        if self.action_runs.is_empty() {
            let msg = Paragraph::new(Line::from(Span::styled(
                " No workflow runs found",
                Style::default()
                    .fg(theme.ui.line_number)
                    .bg(theme.ui.popup_bg),
            )));
            frame.render_widget(msg, area);
            return;
        }

        let items: Vec<ListItem> = self
            .action_runs
            .iter()
            .enumerate()
            .map(|(idx, run)| {
                let is_selected = idx == self.selected;
                let bg = if is_selected {
                    theme.ui.file_tree_selected_bg
                } else {
                    theme.ui.popup_bg
                };

                let status_color = match run.status {
                    ActionStatus::Success => theme.git.added,
                    ActionStatus::Failure => theme.git.deleted,
                    ActionStatus::InProgress => theme.ui.accent,
                    ActionStatus::Queued => theme.git.modified,
                    ActionStatus::Cancelled => theme.ui.line_number,
                    ActionStatus::Skipped => theme.ui.line_number,
                    ActionStatus::Unknown => theme.ui.foreground,
                };

                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!(" {} ", run.status.icon()),
                        Style::default().fg(status_color).bg(bg).add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        &run.name,
                        Style::default().fg(theme.ui.foreground).bg(bg),
                    ),
                    Span::styled(
                        format!("  \u{e702} {}", run.branch),
                        Style::default().fg(theme.git.branch).bg(bg),
                    ),
                    Span::styled(
                        format!("  {} ", run.elapsed),
                        Style::default().fg(theme.ui.line_number).bg(bg),
                    ),
                ]))
            })
            .collect();

        let list = List::new(items).style(Style::default().bg(theme.ui.popup_bg));
        frame.render_widget(list, area);
    }

    pub fn open_selected_action_url(&self) {
        if self.active_tab != GitTab::Actions {
            return;
        }
        if let Some(run) = self.action_runs.get(self.selected) {
            if !run.url.is_empty() {
                let _ = open_url(&run.url);
            }
        }
    }
}

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/c", "start", url]).spawn();
}

fn split_json_objects(s: &str) -> Vec<&str> {
    let mut objects = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    let mut in_string = false;
    let mut escape = false;

    for (i, c) in s.char_indices() {
        if escape {
            escape = false;
            continue;
        }
        if c == '\\' && in_string {
            escape = true;
            continue;
        }
        if c == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match c {
            '{' => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            '}' => {
                depth -= 1;
                if depth == 0 {
                    objects.push(&s[start..=i]);
                }
            }
            _ => {}
        }
    }
    objects
}

fn extract_json_field<'a>(json: &'a str, field: &str) -> String {
    let key = format!("\"{}\":", field);
    if let Some(pos) = json.find(&key) {
        let rest = &json[pos + key.len()..];
        let rest = rest.trim_start();
        if rest.starts_with('"') {
            // String value
            let inner = &rest[1..];
            if let Some(end) = inner.find('"') {
                return inner[..end].to_string();
            }
        } else if rest.starts_with("null") {
            return String::new();
        } else {
            // Number or other
            let end = rest.find(|c: char| c == ',' || c == '}').unwrap_or(rest.len());
            return rest[..end].trim().to_string();
        }
    }
    String::new()
}

fn format_relative_time(iso: &str) -> String {
    // Simple parsing of ISO 8601 date like "2026-02-25T10:30:00Z"
    // Just show the date part for simplicity
    if iso.len() >= 10 {
        iso[..10].to_string()
    } else if iso.is_empty() {
        "unknown".to_string()
    } else {
        iso.to_string()
    }
}

impl Default for GitPanelComponent {
    fn default() -> Self {
        Self::new()
    }
}
