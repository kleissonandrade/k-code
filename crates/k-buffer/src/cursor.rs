#[derive(Debug, Clone, Default)]
pub struct CursorState {
    pub line: usize,
    pub col: usize,
    pub desired_col: usize,
}

impl CursorState {
    pub fn new(line: usize, col: usize) -> Self {
        Self {
            line,
            col,
            desired_col: col,
        }
    }

    pub fn move_to(&mut self, line: usize, col: usize) {
        self.line = line;
        self.col = col;
        self.desired_col = col;
    }

    pub fn move_vertical(&mut self, line: usize, max_col: usize) {
        self.line = line;
        self.col = self.desired_col.min(max_col);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone)]
pub struct Selection {
    pub anchor: Position,
    pub head: Position,
}

impl Selection {
    pub fn ordered(&self) -> (Position, Position) {
        if self.anchor.line < self.head.line
            || (self.anchor.line == self.head.line && self.anchor.col <= self.head.col)
        {
            (self.anchor, self.head)
        } else {
            (self.head, self.anchor)
        }
    }
}
