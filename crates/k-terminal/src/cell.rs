#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AnsiColor {
    Named(u8),
    Indexed(u8),
    Rgb(u8, u8, u8),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellStyle {
    pub fg: Option<AnsiColor>,
    pub bg: Option<AnsiColor>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub reverse: bool,
    pub strikethrough: bool,
}

impl Default for CellStyle {
    fn default() -> Self {
        Self {
            fg: None,
            bg: None,
            bold: false,
            italic: false,
            underline: false,
            dim: false,
            reverse: false,
            strikethrough: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Cell {
    pub ch: char,
    pub style: CellStyle,
    pub width: u8, // 1 for normal, 2 for wide chars, 0 for continuation
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            ch: ' ',
            style: CellStyle::default(),
            width: 1,
        }
    }
}
