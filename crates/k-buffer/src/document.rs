use std::path::PathBuf;

use ropey::Rope;

use crate::cursor::CursorState;
use crate::edit::{EditKind, EditOp};
use crate::cursor::Position;

pub type BufferId = u64;

static NEXT_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

pub struct Document {
    pub id: BufferId,
    pub path: Option<PathBuf>,
    pub rope: Rope,
    pub cursor: CursorState,
    pub modified: bool,
    pub language: Option<String>,
    undo_stack: Vec<EditOp>,
    redo_stack: Vec<EditOp>,
}

impl Document {
    pub fn new() -> Self {
        Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            path: None,
            rope: Rope::new(),
            cursor: CursorState::default(),
            modified: false,
            language: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }

    pub fn from_file(path: PathBuf) -> Result<Self, std::io::Error> {
        let content = std::fs::read_to_string(&path)?;
        let lang = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_string());
        Ok(Self {
            id: NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            path: Some(path),
            rope: Rope::from_str(&content),
            cursor: CursorState::default(),
            modified: false,
            language: lang,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        })
    }

    pub fn save(&mut self) -> Result<(), std::io::Error> {
        if let Some(ref path) = self.path {
            let content = self.rope.to_string();
            std::fs::write(path, content)?;
            self.modified = false;
            Ok(())
        } else {
            Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "No file path set",
            ))
        }
    }

    pub fn save_as(&mut self, path: PathBuf) -> Result<(), std::io::Error> {
        self.path = Some(path);
        self.save()
    }

    pub fn line_count(&self) -> usize {
        self.rope.len_lines()
    }

    pub fn line_len(&self, line: usize) -> usize {
        if line >= self.rope.len_lines() {
            return 0;
        }
        let line_slice = self.rope.line(line);
        let len = line_slice.len_chars();
        if len > 0 && line_slice.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    pub fn get_line(&self, line: usize) -> Option<String> {
        if line >= self.rope.len_lines() {
            return None;
        }
        let line_slice = self.rope.line(line);
        let mut s = line_slice.to_string();
        if s.ends_with('\n') {
            s.pop();
        }
        if s.ends_with('\r') {
            s.pop();
        }
        Some(s)
    }

    pub fn insert_char(&mut self, ch: char) {
        let line = self.cursor.line;
        let col = self.cursor.col;
        let char_idx = self.line_col_to_char(line, col);
        self.rope.insert_char(char_idx, ch);

        let op = EditOp::insert(Position { line, col }, ch.to_string());
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.modified = true;

        if ch == '\n' {
            self.cursor.line += 1;
            self.cursor.col = 0;
            self.cursor.desired_col = 0;
        } else {
            self.cursor.col += 1;
            self.cursor.desired_col = self.cursor.col;
        }
    }

    pub fn insert_str(&mut self, s: &str) {
        let line = self.cursor.line;
        let col = self.cursor.col;
        let char_idx = self.line_col_to_char(line, col);
        self.rope.insert(char_idx, s);

        let op = EditOp::insert(Position { line, col }, s.to_string());
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.modified = true;

        let newlines = s.chars().filter(|&c| c == '\n').count();
        if newlines > 0 {
            self.cursor.line += newlines;
            let last_line = s.rsplit('\n').next().unwrap_or("");
            self.cursor.col = last_line.len();
        } else {
            self.cursor.col += s.len();
        }
        self.cursor.desired_col = self.cursor.col;
    }

    pub fn delete_char_backward(&mut self) {
        if self.cursor.col == 0 && self.cursor.line == 0 {
            return;
        }

        if self.cursor.col == 0 {
            let prev_line = self.cursor.line - 1;
            let prev_len = self.line_len(prev_line);
            let char_idx = self.line_col_to_char(self.cursor.line, 0);
            let deleted = self.rope.char(char_idx - 1).to_string();
            self.rope.remove(char_idx - 1..char_idx);

            let op = EditOp::delete(
                Position {
                    line: prev_line,
                    col: prev_len,
                },
                deleted,
            );
            self.undo_stack.push(op);
            self.redo_stack.clear();
            self.modified = true;

            self.cursor.line = prev_line;
            self.cursor.col = prev_len;
            self.cursor.desired_col = self.cursor.col;
        } else {
            let char_idx = self.line_col_to_char(self.cursor.line, self.cursor.col);
            let deleted = self.rope.char(char_idx - 1).to_string();
            self.rope.remove(char_idx - 1..char_idx);

            let op = EditOp::delete(
                Position {
                    line: self.cursor.line,
                    col: self.cursor.col - 1,
                },
                deleted,
            );
            self.undo_stack.push(op);
            self.redo_stack.clear();
            self.modified = true;

            self.cursor.col -= 1;
            self.cursor.desired_col = self.cursor.col;
        }
    }

    pub fn delete_char_forward(&mut self) {
        let char_idx = self.line_col_to_char(self.cursor.line, self.cursor.col);
        if char_idx >= self.rope.len_chars() {
            return;
        }
        let deleted = self.rope.char(char_idx).to_string();
        self.rope.remove(char_idx..char_idx + 1);

        let op = EditOp::delete(
            Position {
                line: self.cursor.line,
                col: self.cursor.col,
            },
            deleted,
        );
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.modified = true;
    }

    pub fn delete_line(&mut self) -> String {
        if self.rope.len_lines() == 0 {
            return String::new();
        }
        let line = self.cursor.line;
        let start = self.rope.line_to_char(line);
        let end = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            self.rope.len_chars()
        };
        let deleted = self.rope.slice(start..end).to_string();
        self.rope.remove(start..end);

        let op = EditOp::delete(Position { line, col: 0 }, deleted.clone());
        self.undo_stack.push(op);
        self.redo_stack.clear();
        self.modified = true;

        let max_line = self.rope.len_lines().saturating_sub(1);
        if self.cursor.line > max_line {
            self.cursor.line = max_line;
        }
        let max_col = self.line_len(self.cursor.line);
        if self.cursor.col > max_col {
            self.cursor.col = max_col;
        }
        deleted
    }

    pub fn yank_line(&self) -> String {
        self.get_line(self.cursor.line)
            .map(|l| l + "\n")
            .unwrap_or_default()
    }

    pub fn undo(&mut self) -> bool {
        if let Some(op) = self.undo_stack.pop() {
            let inverse = op.invert();
            self.apply_edit(&inverse);
            self.cursor.move_to(op.position.line, op.position.col);
            self.redo_stack.push(op);
            self.modified = true;
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self) -> bool {
        if let Some(op) = self.redo_stack.pop() {
            self.apply_edit(&op);
            self.cursor.move_to(op.position.line, op.position.col);
            self.undo_stack.push(op);
            self.modified = true;
            true
        } else {
            false
        }
    }

    fn apply_edit(&mut self, op: &EditOp) {
        let char_idx = self.line_col_to_char(op.position.line, op.position.col);
        match op.kind {
            EditKind::Insert => {
                self.rope.insert(char_idx, &op.text);
            }
            EditKind::Delete => {
                let end = char_idx + op.text.len();
                let end = end.min(self.rope.len_chars());
                self.rope.remove(char_idx..end);
            }
        }
    }

    fn line_col_to_char(&self, line: usize, col: usize) -> usize {
        if line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }
        let line_start = self.rope.line_to_char(line);
        let line_len = self.line_len(line);
        line_start + col.min(line_len)
    }

    pub fn filename(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|n| n.to_str())
            .unwrap_or("[untitled]")
    }

    pub fn display_path(&self) -> &str {
        self.path
            .as_ref()
            .and_then(|p| p.to_str())
            .unwrap_or("[untitled]")
    }
}

impl Default for Document {
    fn default() -> Self {
        Self::new()
    }
}
