use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

pub struct CommandPaletteComponent {
    pub input: String,
    pub cursor_pos: usize,
}

impl CommandPaletteComponent {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            cursor_pos: 0,
        }
    }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor_pos = 0;
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

    pub fn execute(&self) -> Option<CommandAction> {
        let cmd = self.input.trim();
        if cmd.is_empty() {
            return None;
        }

        let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
        match parts[0] {
            "w" | "write" => Some(CommandAction::Save),
            "q" | "quit" => Some(CommandAction::Quit),
            "wq" => Some(CommandAction::SaveAndQuit),
            "q!" => Some(CommandAction::ForceQuit),
            "e" | "edit" => {
                if parts.len() > 1 {
                    Some(CommandAction::OpenFile(parts[1].to_string()))
                } else {
                    None
                }
            }
            "theme" => {
                Some(CommandAction::SwitchTheme)
            }
            "git" => {
                if parts.len() > 1 {
                    match parts[1] {
                        "commit" => Some(CommandAction::GitCommit),
                        "push" => Some(CommandAction::GitPush),
                        "pull" => Some(CommandAction::GitPull),
                        "log" => Some(CommandAction::GitLog),
                        "diff" => Some(CommandAction::GitDiff),
                        "stash" => Some(CommandAction::GitStash),
                        "stash pop" => Some(CommandAction::GitStashPop),
                        _ => None,
                    }
                } else {
                    Some(CommandAction::GitPanel)
                }
            }
            "help" | "tutorial" => Some(CommandAction::ShowTutorial),
            _ => {
                if let Ok(line) = cmd.parse::<usize>() {
                    Some(CommandAction::GoToLine(line))
                } else {
                    None
                }
            }
        }
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        let spans = vec![
            Span::styled(
                " : ",
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.status_bar_bg)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                &self.input,
                Style::default()
                    .fg(theme.ui.foreground)
                    .bg(theme.ui.status_bar_bg),
            ),
        ];

        let content_width: usize = spans.iter().map(|s| s.content.len()).sum();
        let remaining = (area.width as usize).saturating_sub(content_width);

        let mut all_spans = spans;
        all_spans.push(Span::styled(
            " ".repeat(remaining),
            Style::default().bg(theme.ui.status_bar_bg),
        ));

        let line = Line::from(all_spans);
        let paragraph = Paragraph::new(vec![line]);
        frame.render_widget(paragraph, area);

        let cursor_x = area.x + 3 + self.cursor_pos as u16;
        let cursor_y = area.y;
        if cursor_x < area.x + area.width {
            frame.set_cursor_position((cursor_x, cursor_y));
        }
    }
}

#[derive(Debug, Clone)]
pub enum CommandAction {
    Save,
    Quit,
    SaveAndQuit,
    ForceQuit,
    OpenFile(String),
    SwitchTheme,
    GitCommit,
    GitPush,
    GitPull,
    GitLog,
    GitDiff,
    GitStash,
    GitStashPop,
    GitPanel,
    ShowTutorial,
    GoToLine(usize),
}

impl Default for CommandPaletteComponent {
    fn default() -> Self {
        Self::new()
    }
}
