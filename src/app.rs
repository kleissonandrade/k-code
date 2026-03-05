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
use crate::components::global_search::GlobalSearchComponent;
use crate::components::git_panel::GitPanelComponent;
use crate::components::popup::PopupComponent;
use crate::components::search::SearchComponent;
use crate::components::status_bar::StatusBarComponent;
use crate::components::tab_bar::TabBarComponent;
use crate::components::terminal_panel::{TerminalPanelComponent, TabClickResult};
use crate::components::tutorial::TutorialComponent;
use crate::components::worktree_panel::WorktreePanelComponent;
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
    Terminal,
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
    pub global_search: GlobalSearchComponent,
    pub tutorial: TutorialComponent,
    pub popup: PopupComponent,
    pub highlighter: Highlighter,
    pub terminal_panel: TerminalPanelComponent,

    pub worktree_panel: WorktreePanelComponent,

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
            fuzzy_finder: FuzzyFinderComponent::new(workspace_root.clone()),
            terminal_panel: TerminalPanelComponent::new(workspace_root.to_string_lossy().to_string()),
            global_search: GlobalSearchComponent::new(workspace_root),
            search: SearchComponent::new(),
            command: CommandPaletteComponent::new(),
            git_panel: GitPanelComponent::new(),
            worktree_panel: WorktreePanelComponent::new(),
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

    fn open_diff_for_pr(&mut self) {
        let pr = match self.git_panel.selected_pull_request() {
            Some(pr) => pr,
            None => return,
        };

        let number = pr.number;
        let title = pr.title.clone();

        let output = std::process::Command::new("gh")
            .args(["pr", "diff", &number.to_string()])
            .current_dir(&self.workspace_root)
            .output();

        let diff_text = match output {
            Ok(out) => {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout).to_string();
                    if text.is_empty() {
                        "No diff available for this PR".to_string()
                    } else {
                        text
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                    format!("Failed to get PR diff: {}", stderr.trim())
                }
            }
            Err(_) => "'gh' CLI not found".to_string(),
        };

        let tab_name = format!("PR #{}: {}", number, title);
        let doc = Document::from_string(&diff_text, &tab_name, Some("diff".to_string()));
        self.documents.push(doc);
        self.active_doc = self.documents.len() - 1;
        self.editor.viewport.top_line = 0;
        self.focus = FocusTarget::Editor;
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
                self.terminal_panel.poll_output();
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
            EditorMode::GlobalSearch => return self.handle_global_search_key(key),
            EditorMode::GitPanel => return self.handle_git_panel_key(key),
            EditorMode::WorktreePanel => return self.handle_worktree_panel_key(key),
            _ => {}
        }

        if self.focus == FocusTarget::Terminal {
            return self.handle_terminal_key(key);
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

    fn handle_terminal_key(&mut self, key: KeyEvent) -> Action {
        // Escape from terminal focus back to editor
        match (key.modifiers, key.code) {
            (KeyModifiers::CONTROL, KeyCode::Char('`')) => {
                self.focus = FocusTarget::Editor;
                return Action::Noop;
            }
            (KeyModifiers::CONTROL, KeyCode::Char('n')) if key.modifiers.contains(KeyModifiers::SHIFT) => {
                return Action::NewTerminal;
            }
            _ => {}
        }
        // Forward everything else to PTY
        self.terminal_panel.handle_key(key);
        Action::Noop
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
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                Action::EnterMode(EditorMode::WorktreePanel)
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
            CommandAction::WorktreePanel => Action::EnterMode(EditorMode::WorktreePanel),
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
                if !self.editor.search_matches.is_empty() {
                    self.editor.next_match();
                    self.search.current_match = self.editor.current_match;
                    if let Some((line, col)) = self.editor.current_match_position() {
                        self.documents[self.active_doc].cursor.move_to(line, col);
                        let scroll_padding = self.config.editor.scroll_padding;
                        let doc = &self.documents[self.active_doc];
                        self.editor.ensure_cursor_visible(doc, scroll_padding);
                    }
                }
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.search.delete_char();
                self.run_live_search();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.search.insert_char(c);
                self.run_live_search();
                Action::Noop
            }
            _ => Action::Noop,
        }
    }

    fn run_live_search(&mut self) {
        let query = self.search.input.clone();
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

    fn handle_global_search_key(&mut self, key: KeyEvent) -> Action {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                self.global_search.reset();
                Action::EnterMode(EditorMode::Normal)
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                if let Some(result) = self.global_search.selected_result() {
                    // Has results — open the selected file
                    let path = result.path.clone();
                    let line = result.matches.first().map(|&(l, _)| l).unwrap_or(0);
                    self.global_search.reset();
                    self.mode = EditorMode::Normal;
                    self.focus = FocusTarget::Editor;
                    self.open_file(path);
                    self.documents[self.active_doc].cursor.move_to(line, 0);
                    let scroll_padding = self.config.editor.scroll_padding;
                    let doc = &self.documents[self.active_doc];
                    self.editor.ensure_cursor_visible(doc, scroll_padding);
                } else {
                    // No results yet — trigger the search
                    self.global_search.trigger_search();
                }
                Action::Noop
            }
            (KeyModifiers::CONTROL, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                self.global_search.move_down();
                Action::Noop
            }
            (KeyModifiers::CONTROL, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                self.global_search.move_up();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                self.global_search.delete_char();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                self.global_search.toggle_collapsed();
                Action::Noop
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                self.global_search.insert_char(c);
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
                    } else if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs {
                        self.git_panel.load_pull_requests();
                    }
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('o')) => {
                    if self.git_panel.active_tab == crate::components::git_panel::GitTab::Actions {
                        self.git_panel.open_selected_action_url();
                    } else if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs {
                        self.git_panel.open_selected_pr_url();
                    }
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('f')) if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs => {
                    self.git_panel.pr_filter = self.git_panel.pr_filter.cycle();
                    self.git_panel.selected = 0;
                    self.git_panel.load_pull_requests();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('1')) if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs => {
                    self.git_panel.pr_filter = crate::components::git_panel::PrFilter::Open;
                    self.git_panel.selected = 0;
                    self.git_panel.load_pull_requests();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('2')) if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs => {
                    self.git_panel.pr_filter = crate::components::git_panel::PrFilter::ReviewRequested;
                    self.git_panel.selected = 0;
                    self.git_panel.load_pull_requests();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('3')) if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs => {
                    self.git_panel.pr_filter = crate::components::git_panel::PrFilter::Reviewed;
                    self.git_panel.selected = 0;
                    self.git_panel.load_pull_requests();
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
                        crate::components::git_panel::GitTab::PRs => {
                            self.open_diff_for_pr();
                            Action::Noop
                        }
                        _ => Action::Noop,
                    }
                }
                _ => Action::Noop,
            }
        }
    }

    fn handle_worktree_panel_key(&mut self, key: KeyEvent) -> Action {
        use crate::components::worktree_panel::{CreateField, WorktreeTab};

        if self.worktree_panel.active_tab == WorktreeTab::Create {
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc) => {
                    if self.worktree_panel.branch_input.is_empty() {
                        self.worktree_panel.reset();
                        Action::EnterMode(EditorMode::Normal)
                    } else {
                        self.worktree_panel.branch_input.clear();
                        self.worktree_panel.active_tab = WorktreeTab::List;
                        Action::Noop
                    }
                }
                (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                    self.worktree_panel.reset();
                    Action::EnterMode(EditorMode::Normal)
                }
                (KeyModifiers::NONE, KeyCode::Tab) => {
                    self.worktree_panel.create_field = match self.worktree_panel.create_field {
                        CreateField::Branch => CreateField::BaseBranch,
                        CreateField::BaseBranch => CreateField::Branch,
                    };
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    let branch = self.worktree_panel.branch_input.clone();
                    let base = self.worktree_panel.base_branch_input.clone();
                    if branch.is_empty() {
                        self.worktree_panel.message = "Branch name cannot be empty".to_string();
                        Action::Noop
                    } else {
                        self.worktree_panel.creating = true;
                        self.worktree_panel.message = format!("Creating worktree for '{}'...", branch);
                        Action::WorktreeCreate { branch, base }
                    }
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    match self.worktree_panel.create_field {
                        CreateField::Branch => { self.worktree_panel.branch_input.pop(); }
                        CreateField::BaseBranch => { self.worktree_panel.base_branch_input.pop(); }
                    }
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char(c))
                | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
                    match self.worktree_panel.create_field {
                        CreateField::Branch => self.worktree_panel.branch_input.push(c),
                        CreateField::BaseBranch => self.worktree_panel.base_branch_input.push(c),
                    }
                    Action::Noop
                }
                _ => Action::Noop,
            }
        } else {
            // List tab
            match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Esc)
                | (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                    self.worktree_panel.reset();
                    Action::EnterMode(EditorMode::Normal)
                }
                (KeyModifiers::NONE, KeyCode::Tab) => {
                    self.worktree_panel.next_tab();
                    Action::Noop
                }
                (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                    self.worktree_panel.prev_tab();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                    self.worktree_panel.move_down();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                    self.worktree_panel.move_up();
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('y')) if self.worktree_panel.confirm_delete => {
                    if let Some(wt) = self.worktree_panel.selected_worktree() {
                        let path = wt.path.clone();
                        self.worktree_panel.confirm_delete = false;
                        Action::WorktreeRemove { path }
                    } else {
                        Action::Noop
                    }
                }
                (KeyModifiers::NONE, KeyCode::Char('n')) => {
                    if self.worktree_panel.confirm_delete {
                        self.worktree_panel.confirm_delete = false;
                    } else {
                        self.worktree_panel.active_tab = WorktreeTab::Create;
                        self.worktree_panel.create_field = CreateField::Branch;
                        if let Some(repo) = &self.git_repo {
                            repo.request_branches();
                        }
                    }
                    Action::Noop
                }
                (KeyModifiers::NONE, KeyCode::Char('d')) => {
                    if let Some(wt) = self.worktree_panel.selected_worktree() {
                        if wt.is_main {
                            self.worktree_panel.message = "Cannot delete the main worktree".to_string();
                        } else if !self.worktree_panel.confirm_delete {
                            self.worktree_panel.confirm_delete = true;
                        }
                    }
                    Action::Noop
                }
                (KeyModifiers::SHIFT, KeyCode::Char('P')) => {
                    Action::WorktreePrune
                }
                (KeyModifiers::NONE, KeyCode::Char('r')) => {
                    Action::WorktreeList
                }
                (KeyModifiers::NONE, KeyCode::Enter) => {
                    if let Some(wt) = self.worktree_panel.selected_worktree() {
                        let path = wt.path.clone();
                        Action::WorktreeOpenTerminal(path)
                    } else {
                        Action::Noop
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
                            EditorMode::WorktreePanel => { self.worktree_panel.reset(); }
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
                            ActivityItem::Worktree => {
                                self.activity_bar.active = ActivityItem::Worktree;
                                self.mode = EditorMode::Normal;
                                return Action::EnterMode(EditorMode::WorktreePanel);
                            }
                            ActivityItem::Terminal => {
                                return Action::ToggleTerminal;
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

                // Global search click (overlay)
                if self.mode == EditorMode::GlobalSearch {
                    use crate::components::global_search::ClickResult;
                    match self.global_search.click_at(x, y, self.last_area) {
                        ClickResult::OpenFile => {
                            if let Some(result) = self.global_search.selected_result() {
                                let path = result.path.clone();
                                let line = result.matches.first().map(|&(l, _)| l).unwrap_or(0);
                                self.global_search.reset();
                                self.mode = EditorMode::Normal;
                                self.focus = FocusTarget::Editor;
                                self.open_file(path);
                                self.documents[self.active_doc].cursor.move_to(line, 0);
                                let scroll_padding = self.config.editor.scroll_padding;
                                let doc = &self.documents[self.active_doc];
                                self.editor.ensure_cursor_visible(doc, scroll_padding);
                            }
                        }
                        _ => {}
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
                    // Click on PR filter buttons
                    if self.git_panel.active_tab == crate::components::git_panel::GitTab::PRs {
                        // Filter bar is at content area top (inner.y + 2: after tabs + separator)
                        let filter_y = inner.y + 2;
                        if y == filter_y {
                            use crate::components::git_panel::PrFilter;
                            let filters = [PrFilter::Open, PrFilter::ReviewRequested, PrFilter::Reviewed];
                            let rel_x = x.saturating_sub(inner.x) as usize;
                            let mut pos = 0usize;
                            for f in &filters {
                                let is_active = *f == self.git_panel.pr_filter;
                                let label_len = if is_active {
                                    f.label().len() + 4 // " [label] "
                                } else {
                                    f.label().len() + 4 // "  label  "
                                };
                                if rel_x < pos + label_len {
                                    if *f != self.git_panel.pr_filter {
                                        self.git_panel.pr_filter = *f;
                                        self.git_panel.selected = 0;
                                        self.git_panel.load_pull_requests();
                                    }
                                    return Action::Noop;
                                }
                                pos += label_len + 1; // +1 for "│" divider
                            }
                            return Action::Noop;
                        }
                    }
                }

                // Worktree panel click (overlay) - delete button
                if self.mode == EditorMode::WorktreePanel
                    && self.worktree_panel.active_tab == crate::components::worktree_panel::WorktreeTab::List
                {
                    if let Some(wt_idx) = self.worktree_panel.click_delete_at(x, y) {
                        if let Some(wt) = self.worktree_panel.worktrees.get(wt_idx) {
                            if !wt.is_main {
                                self.worktree_panel.selected = wt_idx;
                                self.worktree_panel.confirm_delete = true;
                            }
                        }
                        return Action::Noop;
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
                    // Check if click is in terminal panel area
                    let term_h = if self.terminal_panel.visible {
                        Some(self.terminal_panel.height)
                    } else {
                        None
                    };
                    let tmp_layout = layout::build_layout(
                        self.last_area,
                        self.file_tree.visible,
                        self.config.file_tree.width,
                        term_h,
                    );
                    if let Some(term_area) = tmp_layout.terminal_panel {
                        if self.terminal_panel.is_in_area(x, y, term_area) {
                            // Click on terminal tab bar (first row)
                            if y == term_area.y {
                                match self.terminal_panel.tab_click(x, term_area) {
                                    TabClickResult::NewTerminal => {
                                        return Action::NewTerminal;
                                    }
                                    TabClickResult::Closed => {
                                        if self.terminal_panel.terminals_count() == 0 {
                                            self.focus = FocusTarget::Editor;
                                        }
                                        self.maybe_switch_workspace_for_terminal();
                                        return Action::Noop;
                                    }
                                    TabClickResult::Switched => {
                                        self.maybe_switch_workspace_for_terminal();
                                    }
                                    _ => {}
                                }
                            }
                            self.focus = FocusTarget::Terminal;
                            return Action::Noop;
                        }
                    }
                    // Check revert button click on diff toolbar
                    let doc = &self.documents[self.active_doc];
                    let editor_area = tmp_layout.editor;
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
                if self.focus == FocusTarget::Terminal && self.terminal_panel.visible {
                    self.terminal_panel.scroll_up();
                    return Action::Noop;
                }
                if self.mode == EditorMode::GlobalSearch {
                    if self.global_search.is_in_preview(mouse.column, mouse.row) {
                        self.global_search.preview_scroll_up(3);
                    } else {
                        self.global_search.move_up();
                    }
                    Action::Noop
                } else if self.focus == FocusTarget::FileTree {
                    self.file_tree.move_up();
                    Action::Noop
                } else {
                    Action::MoveCursor(CursorMotion::Up)
                }
            }
            MouseEventKind::ScrollDown => {
                if self.focus == FocusTarget::Terminal && self.terminal_panel.visible {
                    self.terminal_panel.scroll_down();
                    return Action::Noop;
                }
                if self.mode == EditorMode::GlobalSearch {
                    if self.global_search.is_in_preview(mouse.column, mouse.row) {
                        self.global_search.preview_scroll_down(3);
                    } else {
                        self.global_search.move_down();
                    }
                    Action::Noop
                } else if self.focus == FocusTarget::FileTree {
                    self.file_tree.move_down();
                    Action::Noop
                } else {
                    Action::MoveCursor(CursorMotion::Down)
                }
            }
            MouseEventKind::Moved => {
                if self.mode == EditorMode::FuzzyFinder {
                    self.fuzzy_finder.hover_at(mouse.column, mouse.row, self.last_area);
                } else if self.mode == EditorMode::GlobalSearch {
                    self.global_search.hover_at(mouse.column, mouse.row, self.last_area);
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
                    EditorMode::WorktreePanel => {
                        self.worktree_panel.reset();
                        self.activity_bar.active = ActivityItem::Worktree;
                        if let Some(repo) = &self.git_repo {
                            repo.request_worktree_list();
                            repo.request_branches();
                        }
                    }
                    EditorMode::GlobalSearch => {
                        self.global_search.reset();
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
            Action::WorktreeList => {
                if let Some(repo) = &self.git_repo {
                    repo.request_worktree_list();
                }
            }
            Action::WorktreeCreate { branch, base } => {
                if let Some(repo) = &self.git_repo {
                    repo.request_worktree_add(branch, base);
                }
            }
            Action::WorktreeRemove { path } => {
                if let Some(repo) = &self.git_repo {
                    repo.request_worktree_remove(path, true);
                }
            }
            Action::WorktreePrune => {
                if let Some(repo) = &self.git_repo {
                    repo.request_worktree_prune();
                }
            }
            Action::WorktreeOpenTerminal(path) => {
                self.worktree_panel.reset();
                self.mode = EditorMode::Normal;
                self.terminal_panel.visible = true;
                let cols = if self.terminal_panel.last_cols > 0 { self.terminal_panel.last_cols } else { self.last_area.width.saturating_sub(crate::layout::ACTIVITY_BAR_WIDTH) };
                let rows = self.terminal_panel.height.saturating_sub(1);
                self.terminal_panel.spawn_terminal_in_dir(cols, rows, &path);
                self.focus = FocusTarget::Terminal;
                self.switch_workspace(PathBuf::from(&path));
            }
            Action::StatusMessage(msg) => {
                self.status_message = msg;
            }
            Action::Error(msg) => {
                self.status_message = format!("Error: {}", msg);
            }
            Action::ToggleTerminal => {
                if self.terminal_panel.visible {
                    if self.focus == FocusTarget::Terminal {
                        self.focus = FocusTarget::Editor;
                    } else {
                        self.focus = FocusTarget::Terminal;
                    }
                } else {
                    self.terminal_panel.visible = true;
                    if !self.terminal_panel.has_terminals() {
                        let cols = if self.terminal_panel.last_cols > 0 { self.terminal_panel.last_cols } else { self.last_area.width.saturating_sub(crate::layout::ACTIVITY_BAR_WIDTH) };
                        let rows = self.terminal_panel.height.saturating_sub(1);
                        self.terminal_panel.spawn_terminal(cols, rows);
                    }
                    self.focus = FocusTarget::Terminal;
                }
            }
            Action::NewTerminal => {
                let cols = if self.terminal_panel.last_cols > 0 { self.terminal_panel.last_cols } else { self.last_area.width.saturating_sub(crate::layout::ACTIVITY_BAR_WIDTH) };
                let rows = self.terminal_panel.height.saturating_sub(1);
                self.terminal_panel.spawn_terminal(cols, rows);
                self.focus = FocusTarget::Terminal;
                if !self.terminal_panel.visible {
                    self.terminal_panel.visible = true;
                }
            }
            Action::CloseTerminal => {
                self.terminal_panel.close_active();
                if !self.terminal_panel.has_terminals() {
                    self.focus = FocusTarget::Editor;
                }
            }
            Action::NextTerminalTab => {
                self.terminal_panel.next_tab();
                self.maybe_switch_workspace_for_terminal();
            }
            Action::PrevTerminalTab => {
                self.terminal_panel.prev_tab();
                self.maybe_switch_workspace_for_terminal();
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

    fn switch_workspace(&mut self, new_root: PathBuf) {
        if self.workspace_root == new_root {
            return;
        }
        self.workspace_root = new_root.clone();

        // Recreate git repo for the new workspace
        match k_git::GitRepository::open(&self.workspace_root) {
            Ok(repo) => {
                self.current_branch = repo.current_branch.clone();
                self.git_repo = Some(repo);
            }
            Err(_) => {
                self.git_repo = None;
                self.current_branch = String::new();
            }
        }

        // Update file tree
        self.file_tree.set_root(self.workspace_root.clone());

        // Invalidate fuzzy finder and global search caches
        self.fuzzy_finder.set_root(self.workspace_root.clone());
        self.global_search.set_root(self.workspace_root.clone());

        // Refresh git panel if open
        if self.mode == EditorMode::GitPanel {
            self.refresh_git_tab();
        }
    }

    fn maybe_switch_workspace_for_terminal(&mut self) {
        if let Some(dir) = self.terminal_panel.active_working_dir() {
            let new_root = PathBuf::from(dir);
            self.switch_workspace(new_root);
        }
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
                crate::components::git_panel::GitTab::PRs => {
                    self.git_panel.load_pull_requests();
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
                    self.git_panel.branches = branches.clone();
                    self.worktree_panel.branches = branches;
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
                k_git::GitResponse::Worktrees(worktrees) => {
                    self.worktree_panel.worktrees = worktrees;
                    if self.worktree_panel.selected >= self.worktree_panel.worktrees.len() {
                        self.worktree_panel.selected = self.worktree_panel.worktrees.len().saturating_sub(1);
                    }
                }
                k_git::GitResponse::WorktreeAdded(result) => {
                    self.worktree_panel.creating = false;
                    match result {
                        Ok(path) => {
                            self.worktree_panel.message = format!("Worktree created: {}", path);
                            self.worktree_panel.branch_input.clear();
                            self.worktree_panel.active_tab = crate::components::worktree_panel::WorktreeTab::List;
                            if let Some(repo) = &self.git_repo {
                                repo.request_worktree_list();
                            }

                            // Auto-open terminal and run setup commands
                            let setup_cmds = detect_setup_commands(&path);
                            self.worktree_panel.reset();
                            self.mode = EditorMode::Normal;
                            self.terminal_panel.visible = true;
                            let cols = if self.terminal_panel.last_cols > 0 {
                                self.terminal_panel.last_cols
                            } else {
                                self.last_area.width.saturating_sub(crate::layout::ACTIVITY_BAR_WIDTH)
                            };
                            let rows = self.terminal_panel.height.saturating_sub(1);
                            self.terminal_panel.spawn_terminal_in_dir(cols, rows, &path);
                            self.focus = FocusTarget::Terminal;

                            if !setup_cmds.is_empty() {
                                let cmd_str = format!("{}\r", setup_cmds.join(" && "));
                                self.terminal_panel.write_to_active(cmd_str.as_bytes());
                            }
                        }
                        Err(e) => {
                            self.worktree_panel.message = format!("Failed: {}", e);
                        }
                    }
                }
                k_git::GitResponse::WorktreeRemoved(result) => match result {
                    Ok(()) => {
                        self.worktree_panel.message = "Worktree removed".to_string();
                        if let Some(repo) = &self.git_repo {
                            repo.request_worktree_list();
                        }
                    }
                    Err(e) => {
                        self.worktree_panel.message = format!("Remove failed: {}", e);
                    }
                },
                k_git::GitResponse::WorktreePruned(result) => match result {
                    Ok(()) => {
                        self.worktree_panel.message = "Worktrees pruned".to_string();
                        if let Some(repo) = &self.git_repo {
                            repo.request_worktree_list();
                        }
                    }
                    Err(e) => {
                        self.worktree_panel.message = format!("Prune failed: {}", e);
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
            .hidden(false)
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
        let terminal_height = if self.terminal_panel.visible {
            Some(self.terminal_panel.height)
        } else {
            None
        };
        let app_layout =
            layout::build_layout(area, self.file_tree.visible, self.config.file_tree.width, terminal_height);

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

        if let Some(term_area) = app_layout.terminal_panel {
            // Resize PTY only when dimensions change
            let content_cols = term_area.width;
            let content_rows = term_area.height.saturating_sub(1);
            if content_cols > 0 && content_rows > 0 {
                self.terminal_panel.resize_if_needed(content_cols, content_rows);
            }
            self.terminal_panel.render(frame, term_area, &self.theme);
        }

        if self.mode == EditorMode::Search {
            self.search
                .render(frame, app_layout.editor, &self.theme);
        }

        if self.mode == EditorMode::FuzzyFinder {
            self.fuzzy_finder.render(frame, area, &self.theme);
        }
        if self.mode == EditorMode::GitPanel {
            self.git_panel.render(frame, area, &self.theme);
        }
        if self.mode == EditorMode::WorktreePanel {
            self.worktree_panel.render(frame, area, &self.theme);
        }
        if self.mode == EditorMode::GlobalSearch {
            self.global_search.render(frame, area, &self.theme, &self.highlighter);
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

fn detect_setup_commands(dir: &str) -> Vec<String> {
    let path = std::path::Path::new(dir);
    let mut cmds = Vec::new();

    if path.join(".tool-versions").exists() {
        cmds.push("mise install 2>/dev/null || asdf install".to_string());
    }
    if path.join("Gemfile").exists() {
        cmds.push("bundle install".to_string());
    }
    if path.join("package.json").exists() {
        if path.join("yarn.lock").exists() {
            cmds.push("yarn install".to_string());
        } else {
            cmds.push("npm install".to_string());
        }
    }
    if path.join("requirements.txt").exists() {
        cmds.push("pip install -r requirements.txt".to_string());
    }
    if path.join("go.mod").exists() {
        cmds.push("go mod download".to_string());
    }
    if path.join("Cargo.toml").exists() {
        cmds.push("cargo fetch".to_string());
    }
    cmds
}
