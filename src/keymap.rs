use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::{Action, CursorMotion};
use crate::mode::EditorMode;


pub struct KeyMap {
    pending: Vec<KeyEvent>,
}

pub enum KeyResult {
    Action(Action),
    Pending,
    Unmatched(KeyEvent),
}

impl KeyMap {
    pub fn new() -> Self {
        Self {
            pending: Vec::new(),
        }
    }

    pub fn resolve(&mut self, mode: EditorMode, key: KeyEvent) -> KeyResult {
        match mode {
            EditorMode::Normal => self.resolve_normal(key),
            EditorMode::Insert => self.resolve_insert(key),
            EditorMode::Visual | EditorMode::VisualLine => self.resolve_visual(key),
            EditorMode::Command => KeyResult::Unmatched(key),
            EditorMode::Search => KeyResult::Unmatched(key),
            EditorMode::FuzzyFinder => KeyResult::Unmatched(key),
            EditorMode::GitPanel => KeyResult::Unmatched(key),
            EditorMode::WorktreePanel => KeyResult::Unmatched(key),
            EditorMode::GlobalSearch => KeyResult::Unmatched(key),
            EditorMode::Tutorial => self.resolve_tutorial(key),
        }
    }

    fn resolve_normal(&mut self, key: KeyEvent) -> KeyResult {
        if !self.pending.is_empty() {
            return self.resolve_multi_key(key);
        }

        match (key.modifiers, key.code) {
            // Quit
            (KeyModifiers::NONE, KeyCode::Char('q')) => KeyResult::Action(Action::Quit),

            // Mode transitions
            (KeyModifiers::NONE, KeyCode::Char('i')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Insert))
            }
            (KeyModifiers::NONE, KeyCode::Char('v')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Visual))
            }
            (KeyModifiers::SHIFT, KeyCode::Char('V')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::VisualLine))
            }
            (KeyModifiers::NONE, KeyCode::Char(':')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Command))
            }
            (KeyModifiers::NONE, KeyCode::Char('/')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Search))
            }
            (KeyModifiers::NONE, KeyCode::Char('?')) => KeyResult::Action(Action::ShowTutorial),

            // Navigation
            (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Left))
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Down))
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Up))
            }
            (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Right))
            }
            (KeyModifiers::NONE, KeyCode::Char('0')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::LineStart))
            }
            (KeyModifiers::NONE, KeyCode::Char('$')) | (KeyModifiers::SHIFT, KeyCode::Char('$')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::LineEnd))
            }
            (KeyModifiers::NONE, KeyCode::Char('w')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::WordForward))
            }
            (KeyModifiers::NONE, KeyCode::Char('b')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::WordBackward))
            }
            (KeyModifiers::SHIFT, KeyCode::Char('G')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::FileEnd))
            }
            (KeyModifiers::NONE, KeyCode::PageUp) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::PageUp))
            }
            (KeyModifiers::NONE, KeyCode::PageDown) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::PageDown))
            }

            // Insert mode variants
            (KeyModifiers::SHIFT, KeyCode::Char('A')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::LineEnd))
                // Note: will need to also enter insert - handled in app
            }
            (KeyModifiers::SHIFT, KeyCode::Char('I')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::LineStart))
            }
            (KeyModifiers::NONE, KeyCode::Char('o')) => {
                KeyResult::Action(Action::InsertNewline)
            }

            // Edit operations
            (KeyModifiers::NONE, KeyCode::Char('x')) => {
                KeyResult::Action(Action::DeleteCharForward)
            }
            (KeyModifiers::NONE, KeyCode::Char('p')) => KeyResult::Action(Action::Paste),
            (KeyModifiers::NONE, KeyCode::Char('u')) => KeyResult::Action(Action::Undo),
            (KeyModifiers::CONTROL, KeyCode::Char('r')) => KeyResult::Action(Action::Redo),

            // Multi-key sequences
            (KeyModifiers::NONE, KeyCode::Char('g')) => {
                self.pending.push(key);
                KeyResult::Pending
            }
            (KeyModifiers::NONE, KeyCode::Char('d')) => {
                self.pending.push(key);
                KeyResult::Pending
            }
            (KeyModifiers::NONE, KeyCode::Char('y')) => {
                self.pending.push(key);
                KeyResult::Pending
            }

            // File operations
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => KeyResult::Action(Action::SaveFile),
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::FuzzyFinder))
            }
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => KeyResult::Action(Action::NewFile),
            (KeyModifiers::CONTROL, KeyCode::Char('b')) => {
                KeyResult::Action(Action::ToggleFileTree)
            }
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Search))
            }
            (KeyModifiers::CONTROL, KeyCode::Char('g')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::GitPanel))
            }
            (KeyModifiers::CONTROL, KeyCode::Char('w')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::WorktreePanel))
            }
            // Terminal toggle
            (KeyModifiers::CONTROL, KeyCode::Char('`')) => {
                KeyResult::Action(Action::ToggleTerminal)
            }

            // Tab/buffer navigation
            (KeyModifiers::NONE, KeyCode::Tab) => KeyResult::Action(Action::NextBuffer),
            (KeyModifiers::SHIFT, KeyCode::BackTab) => KeyResult::Action(Action::PrevBuffer),

            // Search
            (KeyModifiers::NONE, KeyCode::Char('n')) => KeyResult::Action(Action::SearchNext),
            (KeyModifiers::SHIFT, KeyCode::Char('N')) => KeyResult::Action(Action::SearchPrev),

            // Theme
            (KeyModifiers::NONE, KeyCode::Char(' ')) => {
                self.pending.push(key);
                KeyResult::Pending
            }

            _ => KeyResult::Action(Action::Noop),
        }
    }

    fn resolve_multi_key(&mut self, key: KeyEvent) -> KeyResult {
        let first = self.pending[0];
        self.pending.clear();

        match (first.code, key.code) {
            // gg -> go to file start
            (KeyCode::Char('g'), KeyCode::Char('g')) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::FileStart))
            }
            // gd -> go to definition
            (KeyCode::Char('g'), KeyCode::Char('d')) => {
                KeyResult::Action(Action::GoToDefinition)
            }
            // dd -> delete line
            (KeyCode::Char('d'), KeyCode::Char('d')) => KeyResult::Action(Action::DeleteLine),
            // yy -> yank line
            (KeyCode::Char('y'), KeyCode::Char('y')) => KeyResult::Action(Action::YankLine),
            // space+t -> switch theme
            (KeyCode::Char(' '), KeyCode::Char('t')) => {
                KeyResult::Action(Action::SwitchTheme)
            }
            // space+f -> global search in files
            (KeyCode::Char(' '), KeyCode::Char('f')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::GlobalSearch))
            }
            // space+j -> toggle terminal
            (KeyCode::Char(' '), KeyCode::Char('j')) => {
                KeyResult::Action(Action::ToggleTerminal)
            }
            _ => KeyResult::Action(Action::Noop),
        }
    }

    fn resolve_insert(&mut self, key: KeyEvent) -> KeyResult {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Normal))
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => KeyResult::Action(Action::InsertChar(c)),
            (KeyModifiers::SHIFT, KeyCode::Char(c)) => KeyResult::Action(Action::InsertChar(c)),
            (KeyModifiers::NONE, KeyCode::Enter) => KeyResult::Action(Action::InsertChar('\n')),
            (KeyModifiers::NONE, KeyCode::Tab) => KeyResult::Action(Action::InsertChar('\t')),
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                KeyResult::Action(Action::DeleteCharBackward)
            }
            (KeyModifiers::NONE, KeyCode::Delete) => {
                KeyResult::Action(Action::DeleteCharForward)
            }
            (KeyModifiers::NONE, KeyCode::Left) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Left))
            }
            (KeyModifiers::NONE, KeyCode::Right) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Right))
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Up))
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Down))
            }
            (KeyModifiers::CONTROL, KeyCode::Char('s')) => KeyResult::Action(Action::SaveFile),
            (KeyModifiers::CONTROL, KeyCode::Char('t')) => KeyResult::Action(Action::NewFile),
            (KeyModifiers::CONTROL, KeyCode::Char('f')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Search))
            }
            (KeyModifiers::CONTROL, KeyCode::Char('`')) => {
                KeyResult::Action(Action::ToggleTerminal)
            }
            _ => KeyResult::Action(Action::Noop),
        }
    }

    fn resolve_visual(&mut self, key: KeyEvent) -> KeyResult {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Normal))
            }
            (KeyModifiers::NONE, KeyCode::Char('h')) | (KeyModifiers::NONE, KeyCode::Left) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Left))
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Down))
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Up))
            }
            (KeyModifiers::NONE, KeyCode::Char('l')) | (KeyModifiers::NONE, KeyCode::Right) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Right))
            }
            (KeyModifiers::NONE, KeyCode::Char('y')) => KeyResult::Action(Action::YankLine),
            (KeyModifiers::NONE, KeyCode::Char('d')) => KeyResult::Action(Action::DeleteLine),
            _ => KeyResult::Action(Action::Noop),
        }
    }

    fn resolve_tutorial(&mut self, key: KeyEvent) -> KeyResult {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc)
            | (KeyModifiers::NONE, KeyCode::Char('q'))
            | (KeyModifiers::NONE, KeyCode::Char('?')) => {
                KeyResult::Action(Action::EnterMode(EditorMode::Normal))
            }
            (KeyModifiers::NONE, KeyCode::Char('j')) | (KeyModifiers::NONE, KeyCode::Down) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Down))
            }
            (KeyModifiers::NONE, KeyCode::Char('k')) | (KeyModifiers::NONE, KeyCode::Up) => {
                KeyResult::Action(Action::MoveCursor(CursorMotion::Up))
            }
            _ => KeyResult::Action(Action::Noop),
        }
    }
}

impl Default for KeyMap {
    fn default() -> Self {
        Self::new()
    }
}
