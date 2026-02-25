use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;
use ratatui::Frame;

use k_buffer::Document;
use k_git::GitRepository;
use k_syntax::Highlighter;

use crate::action::{Action, CursorMotion};
use crate::components::activity_bar::{ActivityBarComponent, ActivityItem};
use crate::components::command_palette::{CommandAction, CommandPaletteComponent};
use crate::components::editor::EditorComponent;
use crate::components::file_tree::FileTreeComponent;
use crate::components::fuzzy_finder::FuzzyFinderComponent;
use crate::components::git_panel::GitPanelComponent;
use crate::components::popup::PopupComponent;
use crate::components::search::SearchComponent;
use crate::components::status_bar::StatusBarComponent;
use crate::components::tab_bar::TabBarComponent;
use crate::components::tutorial::TutorialComponent;
use crate::config::AppConfig;
use crate::event::Event;
use crate::keymap::{KeyMap, KeyResult};
use crate::layout;
use crate::mode::EditorMode;
use crate::theme::{self, Theme, ThemeName};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FocusTarget {
    Editor,
    FileTree,
}

pub struct App {
    pub running: bool,
    pub mode: EditorMode,
    pub focus: FocusTarget,

    pub documents: Vec<Document>,
    pub active_doc: usize,
    pub clipboard: String,
    pub workspace_root: PathBuf,
    pub status_message: String,
    last_revealed_path: Option<PathBuf>,

    pub config: AppConfig,
    pub theme_name: ThemeName,
    pub theme: Theme,

    pub keymap: KeyMap,
    pub activity_bar: ActivityBarComponent,
    pub editor: EditorComponent,
    pub file_tree: FileTreeComponent,
    pub fuzzy_finder: FuzzyFinderComponent,
    pub search: SearchComponent,
    pub command: CommandPaletteComponent,
    pub git_panel: GitPanelComponent,
    pub tutorial: TutorialComponent,
    pub popup: PopupComponent,
    pub highlighter: Highlighter,

    pub git_repo: Option<GitRepository>,
    pub current_branch: String,
    pub last_area: Rect,
}

impl App {
    pub fn new(workspace_root: PathBuf) -> Self {
        let config = AppConfig::load();
        let theme_name = config.theme;
        let theme = theme::get_theme(theme_name);

        let git_repo = GitRepository::open(&workspace_root).ok();
        let current_branch = git_repo
            .as_ref()
            .map(|r| r.current_branch.clone())
            .unwrap_or_default();

        Self {
            running: true,
            mode: EditorMode::Normal,
            focus: FocusTarget::Editor,
            documents: vec![Document::new()],
            active_doc: 0,
            clipboard: String::new(),
            workspace_root: workspace_root.clone(),
            status_message: String::new(),
            last_revealed_path: None,
            config,
            theme_name,
            theme,
            keymap: KeyMap::new(),
            activity_bar: ActivityBarComponent::new(),
            editor: EditorComponent::new(),
            file_tree: FileTreeComponent::new(workspace_root.clone()),
            fuzzy_finder: FuzzyFinderComponent::new(workspace_root),
            search: SearchComponent::new(),
            command: CommandPaletteComponent::new(),
            git_panel: GitPanelComponent::new(),
            tutorial: TutorialComponent::new(),
            popup: PopupComponent::new(),
            highlighter: Highlighter::new(),
            git_repo,
            current_branch,
            last_area: Rect::default(),
        }
    }

    pub fn open_file(&mut self, path: PathBuf) {
        for (idx, doc) in self.documents.iter().enumerate() {
            if doc.path.as_ref() == Some(&path) {
                self.active_doc = idx;
                return;
            }
        }

        match Document::from_file(path.clone()) {
            Ok(doc) => {
                if self.documents.len() == 1
                    && self.documents[0].path.is_none()
                    && !self.documents[0].modified
                {
                    self.documents[0] = doc;
                } else {
                    self.documents.push(doc);
                    self.active_doc = self.documents.len() - 1;
                }
                self.editor.viewport.top_line = 0;
                self.status_message = format!("Opened: {}", path.display());
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
    }

    fn revert_current_diff(&mut self) {
        let doc = &self.documents[self.active_doc];
        if doc.language.as_deref() != Some("diff") {
            return;
        }
        // Extract original file path from tab name "diff: <path>"
        let file_path = doc
            .path
            .as_ref()
            .and_then(|p| p.to_str())
            .and_then(|s| s.strip_prefix("diff: "))
            .map(|s| s.to_string());

        let Some(file_path) = file_path else { return };

        // git checkout -- <file> to revert changes
        let result = std::process::Command::new("git")
            .args(["checkout", "--", &file_path])
            .current_dir(&self.workspace_root)
            .output();

        match result {
            Ok(out) if out.status.success() => {
                // Close the diff tab
                if self.documents.len() > 1 {
                    self.documents.remove(self.active_doc);
                    if self.active_doc >= self.documents.len() {
                        self.active_doc = self.documents.len() - 1;
                    }
                    self.editor.viewport.top_line = 0;
                }
                self.status_message = format!("Reverted: {}", file_path);
            }
            Ok(out) => {
                let err = String::from_utf8_lossy(&out.stderr).to_string();
                self.status_message = format!("Revert failed: {}", err.trim());
            }
            Err(e) => {
                self.status_message = format!("Revert failed: {}", e);
            }
        }
    }

    fn open_diff_for_file(&mut self, file_path: &str) {
        let output = std::process::Command::new("git")
            .args(["diff", "HEAD", "--", file_path])
            .current_dir(&self.workspace_root)
            .output();

        let diff_text = match output {
            Ok(out) => {
                let text = String::from_utf8_lossy(&out.stdout).to_string();
                if text.is_empty() {
                    // Try unstaged diff
                    let out2 = std::process::Command::new("git")
                        .args(["diff", "--", file_path])
                        .current_dir(&self.workspace_root)
                        .output();
                    match out2 {
                        Ok(o) => {
                            let t = String::from_utf8_lossy(&o.stdout).to_string();
                            if t.is_empty() {
                                // Untracked file - show full content
                                let full_path = self.workspace_root.join(file_path);
                                std::fs::read_to_string(&full_path)
                                    .map(|content| {
                                        content
                                            .lines()
                                            .map(|l| format!("+{}", l))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    })
                                    .unwrap_or_else(|_| "No diff available".to_string())
                            } else {
                                t
                            }
                        }
                        Err(_) => "Failed to get diff".to_string(),
                    }
                } else {
                    text
                }
            }
            Err(_) => "Failed to run git diff".to_string(),
        };

        let tab_name = format!("diff: {}", file_path);
        let doc = Document::from_string(&diff_text, &tab_name, Some("diff".to_string()));
        self.documents.push(doc);
        self.active_doc = self.documents.len() - 1;
        self.editor.viewport.top_line = 0;
        self.focus = FocusTarget::Editor;
        // Close git panel
        self.git_panel.reset();
        self.mode = EditorMode::Normal;
        self.activity_bar.active = ActivityItem::FileTree;
    }

    pub fn handle_event(&mut self, event: Event) -> Action {
        match event {
            Event::Key(key) => self.handle_key(key),
            Event::Mouse(mouse) => self.handle_mouse(mouse),
            Event::Resize(w, h) => Action::Resize(w, h),
            Event::Tick => {
                self.poll_git_responses();
                Action::Tick
            }
            _ => Action::Noop,
        }
    }

    fn handle_key(&mut self, key: KeyEvent) -> Action {
        match self.mode {
            EditorMode::Command => return self.handle_command_key(key),
            EditorMode::Search => return self.handle_search_key(key),
            EditorMode::FuzzyFinder => return self.handle_fuzzy_key(key),
            EditorMode::GitPanel => return self.handle_git_panel_key(key),
            _ => {}
        }

        if self.focus == FocusTarget::FileTree && self.mode == EditorMode::Normal {
            return self.handle_file_tree_key(key);
        }

        match self.keymap.resolve(self.mode, key) {
            KeyResult::Action(action) => action,
            KeyResult::Pending => Action::Noop,
            KeyResult::Unmatched(_) => Action::Noop,
        }
    }

    fn handle_file_tree_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.file_tree.move_down();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.file_tree.move_up();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Enter)
            | (KeyModifiers::NONE, KeyCode::Char('l'))
            | (KeyModifiers::NONE, KeyCode::Right) => {
                if let Some(entry) = self.file_tree.selected_entry() {
                    if entry.is_dir {
                        self.file_tree.toggle_expand();
                    } else {
                        let path = entry.path.clone();
                        self.open_file(path);
                        self.focus = FocusTarget::Editor;
                    }
                }
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
                if let Some(entry) = self.file_tree.selected_entry() {
                    if entry.is_dir && self.file_tree.expanded.contains(&entry.path) {
                        self.file_tree.toggle_expand();
                    }
                }
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Esc) | (KeyModifiers::NONE, KeyCode::Tab) => {
                self.focus = FocusTarget::Editor;
                Action::Noop
            }
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => Action::ToggleFileTree,
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                Action::EnterMode(EditorMode::FuzzyFinder)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                Action::EnterMode(EditorMode::GitPanel)
            }
            (KeyModifiers::NONE, KeyCode::Char('?')) => Action::ShowTutorial,
            _ => Action::Noop,
        }
    }

    fn handle_command_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.command.clear();
                Action::EnterMode(EditorMode::Normal)
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let action = self.command.execute();
                self.command.clear();
                self.mode = EditorMode::Normal;
                if let Some(cmd_action) = action {
                    self.execute_command_action(cmd_action)
                } else {
                    self.status_message = "Unknown command".to_string();
                    Action::Noop
                }
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.command.delete_char();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.command.insert_char(c);
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn execute_command_action(&mut self, cmd_action: CommandAction) -> Action {
        match cmd_action {
            CommandAction::Save => Action::SaveFile,
            CommandAction::Quit => Action::Quit,
            CommandAction::SaveAndQuit => {
                self.process_action(Action::SaveFile);
                Action::Quit
            }
            CommandAction::ForceQuit => Action::Quit,
            CommandAction::OpenFile(path) => Action::OpenFile(PathBuf::from(path)),
            CommandAction::SwitchTheme => Action::SwitchTheme,
            CommandAction::GitPanel => Action::EnterMode(EditorMode::GitPanel),
            CommandAction::GitCommit => {
                self.mode = EditorMode::GitPanel;
                self.git_panel.editing_commit = true;
                Action::Noop
            }
            CommandAction::GitPush => Action::GitPush,
            CommandAction::GitPull => Action::GitPull,
            CommandAction::GitLog => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_log(50);
                }
                self.git_panel.active_tab = crate::components::git_panel::GitTab::Log;
                Action::EnterMode(EditorMode::GitPanel)
            }
            CommandAction::GitDiff => Action::Noop,
            CommandAction::GitStash => Action::GitStash(String::new()),
            CommandAction::GitStashPop => Action::GitStashPop,
            CommandAction::ShowTutorial => Action::ShowTutorial,
            CommandAction::GoToLine(line) => Action::GoToLine(line),
        }
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.search.clear();
                self.editor.search_query = None;
                self.editor.search_matches.clear();
                Action::EnterMode(EditorMode::Normal)
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                let query = self.search.input.clone();
                if !query.is_empty() {
                    let doc = &self.documents[self.active_doc];
                    self.editor.search(&query, doc);
                    self.search.match_count = self.editor.search_matches.len();
                    self.search.current_match = self.editor.current_match;
                    if let Some((line, col)) = self.editor.current_match_position() {
                        self.documents[self.active_doc].cursor.move_to(line, col);
                        let scroll_padding = self.config.editor.scroll_padding;
                        let doc = &self.documents[self.active_doc];
                        self.editor.ensure_cursor_visible(doc, scroll_padding);
                    }
                }
                Action::EnterMode(EditorMode::Normal)
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.search.delete_char();
                let query = self.search.input.clone();
                let doc = &self.documents[self.active_doc];
                self.editor.search(&query, doc);
                self.search.match_count = self.editor.search_matches.len();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.search.insert_char(c);
                let query = self.search.input.clone();
                let doc = &self.documents[self.active_doc];
                self.editor.search(&query, doc);
                self.search.match_count = self.editor.search_matches.len();
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn handle_fuzzy_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.fuzzy_finder.reset();
                Action::EnterMode(EditorMode::Normal)
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some(path) = self.fuzzy_finder.selected_path() {
                    let path = path.to_path_buf();
                    self.fuzzy_finder.reset();
                    self.mode = EditorMode::Normal;
                    self.focus = FocusTarget::Editor;
                    Action::OpenFile(path)
                } else {
                    Action::Noop
                }
            }
            (KeyModifiers::CONTROL, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.fuzzy_finder.move_down();
                Action::Noop
            }
            (KeyModifiers::CONTROL, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.fuzzy_finder.move_up();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.fuzzy_finder.delete_char();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.fuzzy_finder.insert_char(c);
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn handle_git_panel_key(&mut self, key: KeyEvent) -> Action {
        if self.git_panel.editing_commit {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    self.git_panel.editing_commit = false;
                    self.git_panel.commit_input.clear();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    let msg = self.git_panel.commit_input.clone();
                    self.git_panel.editing_commit = false;
                    self.git_panel.commit_input.clear();
                    if !msg.is_empty() {
                        Action::GitCommit(msg)
                    } else {
                        Action::Noop
                    }
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    self.git_panel.commit_input.pop();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char(c))
                | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    self.git_panel.commit_input.push(c);
                    Action::Noop
                }
                _ => Action::Noop,
            }
        } else {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc)
                | (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                    self.git_panel.reset();
                    Action::EnterMode(EditorMode::Normal)
                }
                (KeyModifiers::NONE, KeyCode::Tab) => {
                    self.git_panel.next_tab();
                    self.refresh_git_tab();
                    Action::Noop
                }
                (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    self.git_panel.prev_tab();
                    self.refresh_git_tab();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                    self.git_panel.move_down();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                    self.git_panel.move_up();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('a')) => Action::GitStageAll,
                (KeyModifiers::NONE, KeyCode::Char('s')) => {
                    if let Some(entry) =
                        self.git_panel.status_entries.get(self.git_panel.selected)
                    {
                        Action::GitStageFile(entry.path.clone())
                    } else {
                        Action::Noop
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('c')) => {
                    self.git_panel.editing_commit = true;
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('p')) => Action::GitPush,
                (KeyModifiers::NONE, KeyCode::Char('r')) => {
                    if self.git_panel.active_tab == crate::components::git_panel::GitTab::Actions {
                        self.git_panel.load_actions();
                    }
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('o')) => {
                    if self.git_panel.active_tab == crate::components::git_panel::GitTab::Actions {
                        self.git_panel.open_selected_action_url();
                    }
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    match self.git_panel.active_tab {
                        crate::components::git_panel::GitTab::Diff => {
                            if let Some(file) = self.git_panel.selected_changed_file() {
                                let path = file.path.clone();
                                self.open_diff_for_file(&path);
                            }
                            Action::Noop
                        }
                        crate::components::git_panel::GitTab::Branches => {
                            if let Some(branch) =
                                self.git_panel.branches.get(self.git_panel.selected)
                            {
                                Action::GitCheckout(branch.name.clone())
                            } else {
                                Action::Noop
                            }
                        }
                        crate::components::git_panel::GitTab::Stash => Action::GitStashPop,
                        _ => Action::Noop,
                    }
                }
                _ => Action::Noop,
            }
        }
    }

    fn handle_mouse(&mut self, mouse: MouseEvent) -> Action {
        match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let x = mouse.column;
                let y = mouse.row;

                let ab_width = crate::layout::ACTIVITY_BAR_WIDTH;

                // Activity bar click - ALWAYS check first (must work from any mode)
                if x < ab_width {
                    let ab_area = Rect::new(0, 0, ab_width, self.last_area.height.saturating_sub(1));
                    if let Some(item) = self.activity_bar.item_at_y(x, y, ab_area) {
                        // Close any active overlay first
                        match self.mode {
                            EditorMode::FuzzyFinder => { self.fuzzy_finder.reset(); }
                            EditorMode::GitPanel => { self.git_panel.reset(); }
                            EditorMode::Tutorial => { self.tutorial.reset(); }
                            _ => {}
                        }
                        match item {
                            ActivityItem::FileTree => {
                                self.activity_bar.active = ActivityItem::FileTree;
                                self.mode = EditorMode::Normal;
                                return Action::ToggleFileTree;
                            }
                            ActivityItem::Git => {
                                self.activity_bar.active = ActivityItem::Git;
                                self.mode = EditorMode::Normal;
                                return Action::EnterMode(EditorMode::GitPanel);
                            }
                            ActivityItem::Search => {
                                self.activity_bar.active = ActivityItem::Search;
                                self.mode = EditorMode::Normal;
                                return Action::EnterMode(EditorMode::FuzzyFinder);
                            }
                        }
                    }
                }

                // Fuzzy finder click (overlay)
                if self.mode == EditorMode::FuzzyFinder {
                    if self.fuzzy_finder.click_at(x, y, self.last_area) {
                        if let Some(path) = self.fuzzy_finder.selected_path() {
                            let path = path.to_path_buf();
                            self.fuzzy_finder.reset();
                            self.mode = EditorMode::Normal;
                            self.activity_bar.active = ActivityItem::FileTree;
                            return Action::OpenFile(path);
                        }
                    }
                    return Action::Noop;
                }

                // Git panel click (overlay)
                if self.mode == EditorMode::GitPanel && !self.git_panel.editing_commit {
                    let popup_area = crate::layout::centered_rect(75, 75, self.last_area);
                    let block = ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL);
                    let inner = block.inner(popup_area);
                    let tabs_area = Rect::new(inner.x, inner.y, inner.width, 1);
                    if let Some(tab) = self.git_panel.tab_at_position(x, y, tabs_area) {
                        if tab != self.git_panel.active_tab {
                            self.git_panel.active_tab = tab;
                            self.git_panel.selected = 0;
                            self.refresh_git_tab();
                        }
                        return Action::Noop;
                    }
                    // Click on Changes tab content - open diff
                    if self.git_panel.active_tab == crate::components::git_panel::GitTab::Diff {
                        if let Some(_idx) = self.git_panel.click_change_at(x, y, self.last_area) {
                            if let Some(file) = self.git_panel.selected_changed_file() {
                                let path = file.path.clone();
                                self.open_diff_for_file(&path);
                                return Action::Noop;
                            }
                        }
                    }
                }
                let tree_width = if self.file_tree.visible {
                    self.config.file_tree.width
                } else {
                    0
                };
                let tree_end = ab_width + tree_width;

                if x < tree_end && self.file_tree.visible {
                    // File tree click
                    self.focus = FocusTarget::FileTree;
                    let row = y as usize;
                    if row > 0 {
                        let entry_idx = self.file_tree.scroll_offset + row.saturating_sub(1);
                        if entry_idx < self.file_tree.entries.len() {
                            self.file_tree.selected = entry_idx;
                            let entry = self.file_tree.entries[entry_idx].clone();
                            if entry.is_dir {
                                self.file_tree.toggle_expand();
                            } else {
                                self.open_file(entry.path);
                                self.focus = FocusTarget::Editor;
                            }
                        }
                    }
                } else if y == 0 && x >= tree_end {
                    // Tab bar click
                    if let Some((tab_idx, is_close)) = TabBarComponent::hit_test(
                        &self.documents,
                        x,
                        tree_end,
                    ) {
                        if tab_idx < self.documents.len() {
                            if is_close {
                                // Close tab
                                if self.documents.len() > 1 {
                                    self.documents.remove(tab_idx);
                                    if self.active_doc >= self.documents.len() {
                                        self.active_doc = self.documents.len() - 1;
                                    } else if self.active_doc > tab_idx {
                                        self.active_doc -= 1;
                                    }
                                    self.editor.viewport.top_line = 0;
                                }
                            } else {
                                // Switch to tab
                                self.active_doc = tab_idx;
                                self.editor.viewport.top_line = 0;
                            }
                            self.focus = FocusTarget::Editor;
                        }
                    }
                } else {
                    // Check revert button click on diff toolbar
                    let doc = &self.documents[self.active_doc];
                    let editor_area = {
                        let app_layout = layout::build_layout(
                            self.last_area,
                            self.file_tree.visible,
                            self.config.file_tree.width,
                        );
                        app_layout.editor
                    };
                    if let Some(btn) = self.editor.diff_revert_button_area(editor_area, doc) {
                        if x >= btn.x && x < btn.x + btn.width && y == btn.y {
                            self.revert_current_diff();
                            return Action::Noop;
                        }
                    }
                    self.focus = FocusTarget::Editor;
                }
                Action::Noop
            }
            MouseEventKind::ScrollUp => {
                if self.focus == FocusTarget::FileTree {
                    self.file_tree.move_up();
                    Action::Noop
                } else {
                    Action::MoveCursor(CursorMotion::Up)
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focus == FocusTarget::FileTree {
                    self.file_tree.move_down();
                    Action::Noop
                } else {
                    Action::MoveCursor(CursorMotion::Down)
                }
            }
            MouseEventKind::Moved => {
                if self.mode == EditorMode::FuzzyFinder {
                    self.fuzzy_finder.hover_at(mouse.column, mouse.row, self.last_area);
                } else if self.mode == EditorMode::GitPanel
                    && self.git_panel.active_tab == crate::components::git_panel::GitTab::Diff
                {
                    self.git_panel.hover_change_at(mouse.column, mouse.row, self.last_area);
                }
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    pub fn process_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.running = false,
            Action::EnterMode(mode) => {
                self.mode = mode;
                match mode {
                    EditorMode::FuzzyFinder => {
                        self.fuzzy_finder.load_files();
                        self.fuzzy_finder.input.clear();
                        self.fuzzy_finder.cursor_pos = 0;
                        self.fuzzy_finder.selected = 0;
                        self.fuzzy_finder.hovered = None;
                    }
                    EditorMode::GitPanel => {
                        self.refresh_git_tab();
                    }
                    EditorMode::Tutorial => {
                        self.tutorial.reset();
                    }
                    EditorMode::Normal | EditorMode::Insert => {
                        self.activity_bar.active = ActivityItem::FileTree;
                    }
                    _ => {}
                }
            }
            Action::ExitMode => {
                self.mode = EditorMode::Normal;
            }
            Action::ShowTutorial => {
                self.mode = EditorMode::Tutorial;
                self.tutorial.reset();
            }
            Action::NewFile => {
                let doc = Document::new();
                self.documents.push(doc);
                self.active_doc = self.documents.len() - 1;
                self.editor.viewport.top_line = 0;
                self.focus = FocusTarget::Editor;
            }
            Action::OpenFile(path) => {
                self.open_file(path);
            }
            Action::SaveFile => {
                match self.documents[self.active_doc].save() {
                    Ok(()) => self.status_message = "File saved".to_string(),
                    Err(e) => self.status_message = format!("Save error: {}", e),
                }
            }
            Action::CloseBuffer => {
                if self.documents.len() > 1 {
                    self.documents.remove(self.active_doc);
                    if self.active_doc >= self.documents.len() {
                        self.active_doc = self.documents.len() - 1;
                    }
                }
            }
            Action::NextBuffer => {
                if self.documents.len() > 1 {
                    self.active_doc = (self.active_doc + 1) % self.documents.len();
                    self.editor.viewport.top_line = 0;
                }
            }
            Action::PrevBuffer => {
                if self.documents.len() > 1 {
                    if self.active_doc == 0 {
                        self.active_doc = self.documents.len() - 1;
                    } else {
                        self.active_doc -= 1;
                    }
                    self.editor.viewport.top_line = 0;
                }
            }
            Action::MoveCursor(motion) => {
                self.move_cursor(motion);
            }
            Action::InsertChar(c) => {
                self.documents[self.active_doc].insert_char(c);
                let scroll_padding = self.config.editor.scroll_padding;
                let doc = &self.documents[self.active_doc];
                self.editor.ensure_cursor_visible(doc, scroll_padding);
            }
            Action::InsertNewline => {
                let line_len =
                    self.documents[self.active_doc].line_len(self.documents[self.active_doc].cursor.line);
                self.documents[self.active_doc].cursor.col = line_len;
                self.documents[self.active_doc].insert_char('\n');
                self.mode = EditorMode::Insert;
                let scroll_padding = self.config.editor.scroll_padding;
                let doc = &self.documents[self.active_doc];
                self.editor.ensure_cursor_visible(doc, scroll_padding);
            }
            Action::DeleteCharBackward => {
                self.documents[self.active_doc].delete_char_backward();
                let scroll_padding = self.config.editor.scroll_padding;
                let doc = &self.documents[self.active_doc];
                self.editor.ensure_cursor_visible(doc, scroll_padding);
            }
            Action::DeleteCharForward => {
                self.documents[self.active_doc].delete_char_forward();
            }
            Action::DeleteLine => {
                let deleted = self.documents[self.active_doc].delete_line();
                self.clipboard = deleted;
            }
            Action::YankLine => {
                self.clipboard = self.documents[self.active_doc].yank_line();
                self.status_message = "Line yanked".to_string();
            }
            Action::Paste => {
                if !self.clipboard.is_empty() {
                    let text = self.clipboard.clone();
                    self.documents[self.active_doc].insert_str(&text);
                    let scroll_padding = self.config.editor.scroll_padding;
                    let doc = &self.documents[self.active_doc];
                    self.editor.ensure_cursor_visible(doc, scroll_padding);
                }
            }
            Action::Undo => {
                if !self.documents[self.active_doc].undo() {
                    self.status_message = "Nothing to undo".to_string();
                }
                let scroll_padding = self.config.editor.scroll_padding;
                let doc = &self.documents[self.active_doc];
                self.editor.ensure_cursor_visible(doc, scroll_padding);
            }
            Action::Redo => {
                if !self.documents[self.active_doc].redo() {
                    self.status_message = "Nothing to redo".to_string();
                }
                let scroll_padding = self.config.editor.scroll_padding;
                let doc = &self.documents[self.active_doc];
                self.editor.ensure_cursor_visible(doc, scroll_padding);
            }
            Action::ToggleFileTree => {
                self.file_tree.visible = !self.file_tree.visible;
                if self.file_tree.visible {
                    self.focus = FocusTarget::FileTree;
                } else {
                    self.focus = FocusTarget::Editor;
                }
            }
            Action::SearchNext => {
                self.editor.next_match();
                self.search.current_match = self.editor.current_match;
                if let Some((line, col)) = self.editor.current_match_position() {
                    self.documents[self.active_doc].cursor.move_to(line, col);
                    let scroll_padding = self.config.editor.scroll_padding;
                    let doc = &self.documents[self.active_doc];
                    self.editor.ensure_cursor_visible(doc, scroll_padding);
                }
            }
            Action::SearchPrev => {
                self.editor.prev_match();
                self.search.current_match = self.editor.current_match;
                if let Some((line, col)) = self.editor.current_match_position() {
                    self.documents[self.active_doc].cursor.move_to(line, col);
                    let scroll_padding = self.config.editor.scroll_padding;
                    let doc = &self.documents[self.active_doc];
                    self.editor.ensure_cursor_visible(doc, scroll_padding);
                }
            }
            Action::SwitchTheme => {
                self.theme_name = self.theme_name.next();
                self.theme = theme::get_theme(self.theme_name);
                self.status_message = format!("Theme: {}", self.theme_name);
            }
            Action::GoToLine(line) => {
                let target = line.saturating_sub(1);
                let max = self.documents[self.active_doc]
                    .line_count()
                    .saturating_sub(1);
                self.documents[self.active_doc]
                    .cursor
                    .move_to(target.min(max), 0);
                let scroll_padding = self.config.editor.scroll_padding;
                let doc = &self.documents[self.active_doc];
                self.editor.ensure_cursor_visible(doc, scroll_padding);
            }
            Action::GoToDefinition => {
                let doc = &self.documents[self.active_doc];
                let word = doc
                    .get_line(doc.cursor.line)
                    .map(|l| extract_word_at(&l, doc.cursor.col))
                    .unwrap_or_default();
                if !word.is_empty() {
                    self.search_definition(&word);
                }
            }
            Action::GitCommit(msg) => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_commit(msg);
                }
            }
            Action::GitPush => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_push();
                    self.status_message = "Pushing...".to_string();
                }
            }
            Action::GitPull => {
                self.status_message = "Pull not yet implemented".to_string();
            }
            Action::GitCheckout(branch) => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_checkout(branch);
                }
            }
            Action::GitStash(msg) => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_stash(msg);
                }
            }
            Action::GitStashPop => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_stash_pop();
                }
            }
            Action::GitStageFile(path) => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_stage_file(path);
                }
            }
            Action::GitStageAll => {
                if let Some(ref repo) = self.git_repo {
                    repo.request_stage_all();
                }
            }
            Action::GitRefresh => {
                self.refresh_git_tab();
            }
            Action::StatusMessage(msg) => {
                self.status_message = msg;
            }
            Action::Error(msg) => {
                self.status_message = format!("Error: {}", msg);
            }
            _ => {}
        }
    }

    fn move_cursor(&mut self, motion: CursorMotion) {
        if self.mode == EditorMode::Tutorial {
            match motion {
                CursorMotion::Down => self.tutorial.scroll_down(),
                CursorMotion::Up => self.tutorial.scroll_up(),
                _ => {}
            }
            return;
        }

        let doc = &mut self.documents[self.active_doc];
        let line_count = doc.line_count();
        let current_line_len = doc.line_len(doc.cursor.line);
        let mode = self.mode;

        match motion {
            CursorMotion::Up => {
                if doc.cursor.line > 0 {
                    let new_line = doc.cursor.line - 1;
                    let max_col = doc.line_len(new_line);
                    doc.cursor.move_vertical(new_line, max_col);
                }
            }
            CursorMotion::Down => {
                if doc.cursor.line + 1 < line_count {
                    let new_line = doc.cursor.line + 1;
                    let max_col = doc.line_len(new_line);
                    doc.cursor.move_vertical(new_line, max_col);
                }
            }
            CursorMotion::Left => {
                if doc.cursor.col > 0 {
                    doc.cursor.col -= 1;
                    doc.cursor.desired_col = doc.cursor.col;
                }
            }
            CursorMotion::Right => {
                let max = if mode == EditorMode::Insert {
                    current_line_len
                } else {
                    current_line_len.saturating_sub(1)
                };
                if doc.cursor.col < max {
                    doc.cursor.col += 1;
                    doc.cursor.desired_col = doc.cursor.col;
                }
            }
            CursorMotion::LineStart => {
                doc.cursor.col = 0;
                doc.cursor.desired_col = 0;
            }
            CursorMotion::LineEnd => {
                doc.cursor.col = current_line_len.saturating_sub(1);
                doc.cursor.desired_col = doc.cursor.col;
            }
            CursorMotion::WordForward => {
                if let Some(line) = doc.get_line(doc.cursor.line) {
                    let chars: Vec<char> = line.chars().collect();
                    let mut pos = doc.cursor.col;
                    while pos < chars.len() && !chars[pos].is_whitespace() {
                        pos += 1;
                    }
                    while pos < chars.len() && chars[pos].is_whitespace() {
                        pos += 1;
                    }
                    doc.cursor.col = pos.min(chars.len().saturating_sub(1));
                    doc.cursor.desired_col = doc.cursor.col;
                }
            }
            CursorMotion::WordBackward => {
                if let Some(line) = doc.get_line(doc.cursor.line) {
                    let chars: Vec<char> = line.chars().collect();
                    let mut pos = doc.cursor.col;
                    if pos > 0 {
                        pos -= 1;
                    }
                    while pos > 0 && chars[pos].is_whitespace() {
                        pos -= 1;
                    }
                    while pos > 0 && !chars[pos - 1].is_whitespace() {
                        pos -= 1;
                    }
                    doc.cursor.col = pos;
                    doc.cursor.desired_col = doc.cursor.col;
                }
            }
            CursorMotion::FileStart => {
                doc.cursor.move_to(0, 0);
            }
            CursorMotion::FileEnd => {
                let last = line_count.saturating_sub(1);
                doc.cursor.move_to(last, 0);
            }
            CursorMotion::PageUp => {
                let page = self.editor.viewport.height.saturating_sub(2);
                let new_line = doc.cursor.line.saturating_sub(page);
                let max_col = doc.line_len(new_line);
                doc.cursor.move_vertical(new_line, max_col);
            }
            CursorMotion::PageDown => {
                let page = self.editor.viewport.height.saturating_sub(2);
                let new_line = (doc.cursor.line + page).min(line_count.saturating_sub(1));
                let max_col = doc.line_len(new_line);
                doc.cursor.move_vertical(new_line, max_col);
            }
        }

        // Ensure cursor visible after movement - drop mutable borrow first
        let scroll_padding = self.config.editor.scroll_padding;
        let doc = &self.documents[self.active_doc];
        self.editor.ensure_cursor_visible(doc, scroll_padding);
    }

    fn refresh_git_tab(&mut self) {
        if let Some(ref repo) = self.git_repo {
            match self.git_panel.active_tab {
                crate::components::git_panel::GitTab::Status => repo.request_status(),
                crate::components::git_panel::GitTab::Diff => {
                    self.git_panel.load_branch_changes(&self.workspace_root);
                }
                crate::components::git_panel::GitTab::Branches => repo.request_branches(),
                crate::components::git_panel::GitTab::Stash => repo.request_stash_list(),
                crate::components::git_panel::GitTab::Log => repo.request_log(100),
                crate::components::git_panel::GitTab::Actions => {
                    self.git_panel.load_actions();
                }
            }
        }
    }

    fn poll_git_responses(&mut self) {
        let mut responses = Vec::new();
        if let Some(ref mut repo) = self.git_repo {
            while let Ok(response) = repo.response_rx.try_recv() {
                responses.push(response);
            }
        }

        for response in responses {
            match response {
                k_git::GitResponse::Status(entries) => {
                    self.git_panel.status_entries = entries;
                }
                k_git::GitResponse::Log(commits) => {
                    self.git_panel.log_entries = commits;
                }
                k_git::GitResponse::Branches(branches) => {
                    self.git_panel.branches = branches;
                }
                k_git::GitResponse::Stashes(stashes) => {
                    self.git_panel.stashes = stashes;
                }
                k_git::GitResponse::CommitDone(result) => match result {
                    Ok(id) => {
                        self.git_panel.message = format!("Committed: {}", id);
                        self.status_message = format!("Committed: {}", id);
                        if let Some(ref repo) = self.git_repo {
                            repo.request_status();
                            repo.request_current_branch();
                        }
                    }
                    Err(e) => {
                        self.git_panel.message = format!("Commit failed: {}", e);
                        self.status_message = format!("Commit failed: {}", e);
                    }
                },
                k_git::GitResponse::PushDone(result) => match result {
                    Ok(()) => {
                        self.git_panel.message = "Push successful".to_string();
                        self.status_message = "Push successful".to_string();
                    }
                    Err(e) => {
                        self.git_panel.message = format!("Push failed: {}", e);
                        self.status_message = format!("Push failed: {}", e);
                    }
                },
                k_git::GitResponse::CheckoutDone(result) => match result {
                    Ok(()) => {
                        self.git_panel.message = "Checkout successful".to_string();
                        self.status_message = "Checkout successful".to_string();
                        if let Some(ref repo) = self.git_repo {
                            repo.request_current_branch();
                            repo.request_branches();
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Checkout failed: {}", e);
                    }
                },
                k_git::GitResponse::CurrentBranch(branch) => {
                    self.current_branch = branch;
                }
                k_git::GitResponse::StageDone(result) => match result {
                    Ok(()) => {
                        self.git_panel.message = "Staged".to_string();
                        if let Some(ref repo) = self.git_repo {
                            repo.request_status();
                        }
                    }
                    Err(e) => {
                        self.git_panel.message = format!("Stage failed: {}", e);
                    }
                },
                k_git::GitResponse::StashDone(result) => match result {
                    Ok(()) => {
                        self.status_message = "Stashed".to_string();
                        if let Some(ref repo) = self.git_repo {
                            repo.request_stash_list();
                            repo.request_status();
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Stash failed: {}", e);
                    }
                },
                k_git::GitResponse::StashPopDone(result) => match result {
                    Ok(()) => {
                        self.status_message = "Stash popped".to_string();
                        if let Some(ref repo) = self.git_repo {
                            repo.request_stash_list();
                            repo.request_status();
                        }
                    }
                    Err(e) => {
                        self.status_message = format!("Stash pop failed: {}", e);
                    }
                },
                _ => {}
            }
        }
    }

    fn search_definition(&mut self, word: &str) {
        let patterns = vec![
            format!("fn {}(", word),
            format!("struct {} ", word),
            format!("struct {}{{", word),
            format!("enum {} ", word),
            format!("trait {} ", word),
            format!("class {} ", word),
            format!("def {}(", word),
            format!("function {}(", word),
            format!("const {} ", word),
            format!("type {} ", word),
            format!("interface {} ", word),
        ];

        let walker = ignore::WalkBuilder::new(&self.workspace_root)
            .hidden(true)
            .git_ignore(true)
            .build();

        for entry in walker.flatten() {
            if !entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                continue;
            }
            let path = entry.path();
            if let Ok(content) = std::fs::read_to_string(path) {
                for (line_idx, line) in content.lines().enumerate() {
                    for pattern in &patterns {
                        if line.contains(pattern.as_str()) {
                            let file_path = path.to_path_buf();
                            self.open_file(file_path);
                            self.documents[self.active_doc]
                                .cursor
                                .move_to(line_idx, 0);
                            let scroll_padding = self.config.editor.scroll_padding;
                            let doc = &self.documents[self.active_doc];
                            self.editor.ensure_cursor_visible(doc, scroll_padding);
                            self.status_message = format!(
                                "Definition found: {}:{}",
                                path.display(),
                                line_idx + 1
                            );
                            return;
                        }
                    }
                }
            }
        }
        self.status_message = format!("Definition not found: {}", word);
    }

    fn sync_tree_to_active_doc(&mut self) {
        let current_path = self.documents[self.active_doc].path.clone();
        if current_path.is_some() && current_path != self.last_revealed_path {
            self.last_revealed_path = current_path.clone();
            if let Some(ref path) = current_path {
                if self.file_tree.visible {
                    self.file_tree.reveal_path(path);
                }
            }
        }
    }

    pub fn render(&mut self, frame: &mut Frame) {
        self.sync_tree_to_active_doc();
        let area = frame.area();
        self.last_area = area;
        let app_layout =
            layout::build_layout(area, self.file_tree.visible, self.config.file_tree.width);

        let line_count = self.documents[self.active_doc].line_count();
        self.editor.update_viewport_size(app_layout.editor, line_count);

        self.activity_bar.render(frame, app_layout.activity_bar, &self.theme);

        if let Some(tree_area) = app_layout.file_tree {
            self.file_tree.render(frame, tree_area, &self.theme);
        }

        TabBarComponent::render(
            frame,
            app_layout.tab_bar,
            &self.documents,
            self.active_doc,
            &self.theme,
        );

        let doc = &self.documents[self.active_doc];
        let mode = self.mode;
        let theme = &self.theme;
        self.editor
            .render(frame, app_layout.editor, doc, mode, theme, &self.highlighter);

        match self.mode {
            EditorMode::Search => {
                self.search.render(frame, app_layout.status_bar, &self.theme);
            }
            EditorMode::Command => {
                self.command
                    .render(frame, app_layout.status_bar, &self.theme);
            }
            _ => {
                StatusBarComponent::render(
                    frame,
                    app_layout.status_bar,
                    self.mode,
                    Some(&self.documents[self.active_doc]),
                    &self.current_branch,
                    &self.theme,
                    &self.status_message,
                );
            }
        }

        if self.mode == EditorMode::FuzzyFinder {
            self.fuzzy_finder.render(frame, area, &self.theme);
        }
        if self.mode == EditorMode::GitPanel {
            self.git_panel.render(frame, area, &self.theme);
        }
        if self.mode == EditorMode::Tutorial {
            self.tutorial.render(frame, area, &self.theme);
        }
        self.popup.render(frame, area, &self.theme);
    }
}

fn extract_word_at(line: &str, col: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    if col >= chars.len() {
        return String::new();
    }

    let mut start = col;
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    let mut end = col;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    chars[start..end].iter().collect()
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}
