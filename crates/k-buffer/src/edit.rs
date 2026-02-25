use std::time::Instant;

use crate::cursor::Position;

#[derive(Debug, Clone)]
pub struct EditOp {
    pub kind: EditKind,
    pub position: Position,
    pub text: String,
    pub timestamp: Instant,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditKind {
    Insert,
    Delete,
}

impl EditOp {
    pub fn insert(position: Position, text: String) -> Self {
        Self {
            kind: EditKind::Insert,
            position,
            text,
            timestamp: Instant::now(),
        }
    }

    pub fn delete(position: Position, text: String) -> Self {
        Self {
            kind: EditKind::Delete,
            position,
            text,
            timestamp: Instant::now(),
        }
    }

    pub fn invert(&self) -> Self {
        Self {
            kind: match self.kind {
                EditKind::Insert => EditKind::Delete,
                EditKind::Delete => EditKind::Insert,
            },
            position: self.position,
            text: self.text.clone(),
            timestamp: Instant::now(),
        }
    }
}
