use unicode_width::UnicodeWidthChar;
use vte::{Params, Perform};

use crate::cell::{AnsiColor, Cell, CellStyle};

pub struct TerminalGrid {
    pub cols: u16,
    pub rows: u16,
    cells: Vec<Vec<Cell>>,
    pub scrollback: Vec<Vec<Cell>>,
    pub scrollback_limit: usize,
    pub cursor_row: u16,
    pub cursor_col: u16,
    pub cursor_visible: bool,
    current_style: CellStyle,
    scroll_region_top: u16,
    scroll_region_bottom: u16,
    saved_cursor: Option<(u16, u16, CellStyle)>,
    alternate_cells: Option<Vec<Vec<Cell>>>,
    alternate_cursor: Option<(u16, u16)>,
    parser: vte::Parser,
    wrap_pending: bool,
    pub application_cursor_keys: bool,
    pub bracketed_paste: bool,
    pub response_bytes: Vec<u8>,
}

impl TerminalGrid {
    pub fn new(cols: u16, rows: u16) -> Self {
        let cells = (0..rows).map(|_| vec![Cell::default(); cols as usize]).collect();
        Self {
            cols,
            rows,
            cells,
            scrollback: Vec::new(),
            scrollback_limit: 10_000,
            cursor_row: 0,
            cursor_col: 0,
            cursor_visible: true,
            current_style: CellStyle::default(),
            scroll_region_top: 0,
            scroll_region_bottom: rows.saturating_sub(1),
            saved_cursor: None,
            alternate_cells: None,
            alternate_cursor: None,
            parser: vte::Parser::new(),
            wrap_pending: false,
            application_cursor_keys: false,
            bracketed_paste: false,
            response_bytes: Vec::new(),
        }
    }

    pub fn process_bytes(&mut self, bytes: &[u8]) {
        let mut parser = std::mem::take(&mut self.parser);
        for &byte in bytes {
            parser.advance(self, byte);
        }
        self.parser = parser;
    }

    pub fn active_cells(&self) -> &[Vec<Cell>] {
        &self.cells
    }

    pub fn resize(&mut self, cols: u16, rows: u16) {
        if cols == self.cols && rows == self.rows {
            return;
        }
        let mut new_cells: Vec<Vec<Cell>> = (0..rows)
            .map(|_| vec![Cell::default(); cols as usize])
            .collect();

        let copy_rows = rows.min(self.rows) as usize;
        let copy_cols = cols.min(self.cols) as usize;
        for r in 0..copy_rows {
            for c in 0..copy_cols {
                new_cells[r][c] = self.cells[r][c].clone();
            }
        }

        self.cells = new_cells;
        self.cols = cols;
        self.rows = rows;
        self.cursor_row = self.cursor_row.min(rows.saturating_sub(1));
        self.cursor_col = self.cursor_col.min(cols.saturating_sub(1));
        self.scroll_region_top = 0;
        self.scroll_region_bottom = rows.saturating_sub(1);
        self.wrap_pending = false;
    }

    fn scroll_up(&mut self) {
        let top = self.scroll_region_top as usize;
        let bottom = self.scroll_region_bottom as usize;

        if top < self.cells.len() && bottom < self.cells.len() && top <= bottom {
            let line = self.cells.remove(top);
            // Only add to scrollback if scrolling the full screen
            if self.scroll_region_top == 0 {
                self.scrollback.push(line);
                if self.scrollback.len() > self.scrollback_limit {
                    self.scrollback.remove(0);
                }
            }
            self.cells.insert(bottom, vec![Cell::default(); self.cols as usize]);
        }
    }

    fn scroll_down(&mut self) {
        let top = self.scroll_region_top as usize;
        let bottom = self.scroll_region_bottom as usize;

        if top < self.cells.len() && bottom < self.cells.len() && top <= bottom {
            self.cells.remove(bottom);
            self.cells.insert(top, vec![Cell::default(); self.cols as usize]);
        }
    }

    fn do_wrap(&mut self) {
        self.cursor_col = 0;
        if self.cursor_row >= self.scroll_region_bottom {
            self.scroll_up();
        } else {
            self.cursor_row += 1;
        }
        self.wrap_pending = false;
    }

    fn put_char(&mut self, c: char) {
        let width = c.width().unwrap_or(0);
        if width == 0 {
            return;
        }

        // Deferred wrap: if wrap was pending, execute it now before placing the char
        if self.wrap_pending {
            self.do_wrap();
        }

        let col = self.cursor_col as usize;
        let row = self.cursor_row as usize;

        if col < self.cols as usize && row < self.rows as usize {
            self.cells[row][col] = Cell {
                ch: c,
                style: self.current_style,
                width: width as u8,
            };

            // Wide character: mark next cell as continuation
            if width == 2 && col + 1 < self.cols as usize {
                self.cells[row][col + 1] = Cell {
                    ch: ' ',
                    style: self.current_style,
                    width: 0,
                };
            }
        }

        let new_col = self.cursor_col + width as u16;
        if new_col >= self.cols {
            // Cursor stays at last column, set wrap pending
            self.cursor_col = self.cols.saturating_sub(1);
            self.wrap_pending = true;
        } else {
            self.cursor_col = new_col;
        }
    }

    fn newline(&mut self) {
        self.wrap_pending = false;
        if self.cursor_row >= self.scroll_region_bottom {
            self.scroll_up();
        } else {
            self.cursor_row += 1;
        }
    }

    fn make_erase_cell(&self) -> Cell {
        // Erase fills with current background color
        Cell {
            ch: ' ',
            style: CellStyle {
                bg: self.current_style.bg,
                ..CellStyle::default()
            },
            width: 1,
        }
    }

    fn erase_in_display(&mut self, mode: u16) {
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        let erase = self.make_erase_cell();
        match mode {
            0 => {
                // Erase from cursor to end of screen
                if row < self.cells.len() {
                    for c in col..self.cols as usize {
                        if c < self.cells[row].len() {
                            self.cells[row][c] = erase.clone();
                        }
                    }
                    for r in (row + 1)..self.rows as usize {
                        if r < self.cells.len() {
                            self.cells[r] = vec![erase.clone(); self.cols as usize];
                        }
                    }
                }
            }
            1 => {
                // Erase from start to cursor
                for r in 0..row {
                    if r < self.cells.len() {
                        self.cells[r] = vec![erase.clone(); self.cols as usize];
                    }
                }
                if row < self.cells.len() {
                    for c in 0..=col.min(self.cols as usize - 1) {
                        self.cells[row][c] = erase.clone();
                    }
                }
            }
            2 | 3 => {
                // Erase entire screen
                for r in 0..self.rows as usize {
                    if r < self.cells.len() {
                        self.cells[r] = vec![erase.clone(); self.cols as usize];
                    }
                }
            }
            _ => {}
        }
    }

    fn erase_in_line(&mut self, mode: u16) {
        let row = self.cursor_row as usize;
        let col = self.cursor_col as usize;
        if row >= self.cells.len() {
            return;
        }
        let erase = self.make_erase_cell();
        match mode {
            0 => {
                for c in col..self.cols as usize {
                    if c < self.cells[row].len() {
                        self.cells[row][c] = erase.clone();
                    }
                }
            }
            1 => {
                for c in 0..=col.min(self.cols as usize - 1) {
                    self.cells[row][c] = erase.clone();
                }
            }
            2 => {
                self.cells[row] = vec![erase; self.cols as usize];
            }
            _ => {}
        }
    }

    fn parse_sgr(&mut self, params: &Params) {
        let mut iter = params.iter();
        while let Some(param) = iter.next() {
            let code = param[0];
            match code {
                0 => self.current_style = CellStyle::default(),
                1 => self.current_style.bold = true,
                2 => self.current_style.dim = true,
                3 => self.current_style.italic = true,
                4 => self.current_style.underline = true,
                7 => self.current_style.reverse = true,
                8 => {} // hidden - ignore for now
                9 => self.current_style.strikethrough = true,
                22 => {
                    self.current_style.bold = false;
                    self.current_style.dim = false;
                }
                23 => self.current_style.italic = false,
                24 => self.current_style.underline = false,
                27 => self.current_style.reverse = false,
                29 => self.current_style.strikethrough = false,
                30..=37 => {
                    self.current_style.fg = Some(AnsiColor::Named((code - 30) as u8));
                }
                38 => {
                    if let Some(next) = iter.next() {
                        match next[0] {
                            5 => {
                                if let Some(idx) = iter.next() {
                                    self.current_style.fg =
                                        Some(AnsiColor::Indexed(idx[0] as u8));
                                }
                            }
                            2 => {
                                let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                                let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                                let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                                self.current_style.fg = Some(AnsiColor::Rgb(r, g, b));
                            }
                            _ => {}
                        }
                    }
                }
                39 => self.current_style.fg = None,
                40..=47 => {
                    self.current_style.bg = Some(AnsiColor::Named((code - 40) as u8));
                }
                48 => {
                    if let Some(next) = iter.next() {
                        match next[0] {
                            5 => {
                                if let Some(idx) = iter.next() {
                                    self.current_style.bg =
                                        Some(AnsiColor::Indexed(idx[0] as u8));
                                }
                            }
                            2 => {
                                let r = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                                let g = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                                let b = iter.next().map(|p| p[0] as u8).unwrap_or(0);
                                self.current_style.bg = Some(AnsiColor::Rgb(r, g, b));
                            }
                            _ => {}
                        }
                    }
                }
                49 => self.current_style.bg = None,
                90..=97 => {
                    self.current_style.fg = Some(AnsiColor::Named((code - 90 + 8) as u8));
                }
                100..=107 => {
                    self.current_style.bg = Some(AnsiColor::Named((code - 100 + 8) as u8));
                }
                _ => {}
            }
        }
    }

    fn enter_alternate_screen(&mut self) {
        if self.alternate_cells.is_some() {
            return;
        }
        self.alternate_cells = Some(self.cells.clone());
        self.alternate_cursor = Some((self.cursor_row, self.cursor_col));
        // Clear screen for alternate buffer
        self.cells = (0..self.rows)
            .map(|_| vec![Cell::default(); self.cols as usize])
            .collect();
        self.cursor_row = 0;
        self.cursor_col = 0;
        self.wrap_pending = false;
    }

    fn leave_alternate_screen(&mut self) {
        if let Some(cells) = self.alternate_cells.take() {
            self.cells = cells;
            if let Some((row, col)) = self.alternate_cursor.take() {
                self.cursor_row = row;
                self.cursor_col = col;
            }
            self.wrap_pending = false;
        }
    }

    fn insert_lines(&mut self, count: u16) {
        let row = self.cursor_row as usize;
        let bottom = self.scroll_region_bottom as usize;
        for _ in 0..count {
            if row <= bottom && bottom < self.cells.len() {
                self.cells.remove(bottom);
                self.cells.insert(row, vec![Cell::default(); self.cols as usize]);
            }
        }
    }

    fn delete_lines(&mut self, count: u16) {
        let row = self.cursor_row as usize;
        let bottom = self.scroll_region_bottom as usize;
        for _ in 0..count {
            if row <= bottom && row < self.cells.len() {
                self.cells.remove(row);
                self.cells.insert(bottom, vec![Cell::default(); self.cols as usize]);
            }
        }
    }
}

impl Perform for TerminalGrid {
    fn print(&mut self, c: char) {
        self.put_char(c);
    }

    fn execute(&mut self, byte: u8) {
        match byte {
            b'\n' | 0x0b | 0x0c => self.newline(),
            b'\r' => {
                self.cursor_col = 0;
                self.wrap_pending = false;
            }
            b'\t' => {
                self.wrap_pending = false;
                let next_tab = ((self.cursor_col / 8) + 1) * 8;
                self.cursor_col = next_tab.min(self.cols.saturating_sub(1));
            }
            0x08 => {
                // Backspace
                self.wrap_pending = false;
                self.cursor_col = self.cursor_col.saturating_sub(1);
            }
            0x07 => {} // Bell — ignore
            _ => {}
        }
    }

    fn hook(&mut self, _params: &Params, _intermediates: &[u8], _ignore: bool, _action: char) {}

    fn put(&mut self, _byte: u8) {}

    fn unhook(&mut self) {}

    fn osc_dispatch(&mut self, _params: &[&[u8]], _bell_terminated: bool) {
        // OSC sequences (window title, etc.) — ignore for now
    }

    fn csi_dispatch(&mut self, params: &Params, intermediates: &[u8], _ignore: bool, action: char) {
        let p = |idx: usize, default: u16| -> u16 {
            params
                .iter()
                .nth(idx)
                .and_then(|s| s.first().copied())
                .map(|v| if v == 0 { default } else { v })
                .unwrap_or(default)
        };

        // Most CSI sequences clear wrap_pending
        match action {
            'm' => {} // SGR doesn't clear wrap
            _ => self.wrap_pending = false,
        }

        match (action, intermediates) {
            // Cursor Up
            ('A', []) => {
                let n = p(0, 1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
            }
            // Cursor Down
            ('B', []) => {
                let n = p(0, 1);
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
            }
            // Cursor Forward
            ('C', []) => {
                let n = p(0, 1);
                self.cursor_col = (self.cursor_col + n).min(self.cols.saturating_sub(1));
            }
            // Cursor Back
            ('D', []) => {
                let n = p(0, 1);
                self.cursor_col = self.cursor_col.saturating_sub(n);
            }
            // Cursor Next Line (CNL)
            ('E', []) => {
                let n = p(0, 1);
                self.cursor_row = (self.cursor_row + n).min(self.rows.saturating_sub(1));
                self.cursor_col = 0;
            }
            // Cursor Previous Line (CPL)
            ('F', []) => {
                let n = p(0, 1);
                self.cursor_row = self.cursor_row.saturating_sub(n);
                self.cursor_col = 0;
            }
            // Cursor Position (CUP)
            ('H', []) | ('f', []) => {
                let row = p(0, 1).saturating_sub(1);
                let col = p(1, 1).saturating_sub(1);
                self.cursor_row = row.min(self.rows.saturating_sub(1));
                self.cursor_col = col.min(self.cols.saturating_sub(1));
            }
            // Erase in Display
            ('J', []) => {
                let mode = p(0, 0);
                self.erase_in_display(mode);
            }
            // Erase in Line
            ('K', []) => {
                let mode = p(0, 0);
                self.erase_in_line(mode);
            }
            // Insert Lines
            ('L', []) => {
                let n = p(0, 1);
                self.insert_lines(n);
            }
            // Delete Lines
            ('M', []) => {
                let n = p(0, 1);
                self.delete_lines(n);
            }
            // Scroll Up
            ('S', []) => {
                let n = p(0, 1);
                for _ in 0..n {
                    self.scroll_up();
                }
            }
            // Scroll Down
            ('T', []) => {
                let n = p(0, 1);
                for _ in 0..n {
                    self.scroll_down();
                }
            }
            // SGR (Select Graphic Rendition)
            ('m', []) => {
                self.parse_sgr(params);
            }
            // Device Status Report
            ('n', []) => {
                let mode = p(0, 0);
                if mode == 6 {
                    // Cursor Position Report: respond with ESC [ row ; col R
                    let response = format!(
                        "\x1b[{};{}R",
                        self.cursor_row + 1,
                        self.cursor_col + 1
                    );
                    self.response_bytes.extend(response.as_bytes());
                }
            }
            // Set Scroll Region (DECSTBM)
            ('r', []) => {
                let top = p(0, 1).saturating_sub(1);
                let bottom = p(1, self.rows).saturating_sub(1);
                self.scroll_region_top = top.min(self.rows.saturating_sub(1));
                self.scroll_region_bottom = bottom.min(self.rows.saturating_sub(1));
                self.cursor_row = 0;
                self.cursor_col = 0;
            }
            // Cursor column (CHA)
            ('G', []) => {
                let col = p(0, 1).saturating_sub(1);
                self.cursor_col = col.min(self.cols.saturating_sub(1));
            }
            // Cursor to line (VPA)
            ('d', []) => {
                let row = p(0, 1).saturating_sub(1);
                self.cursor_row = row.min(self.rows.saturating_sub(1));
            }
            // Erase chars (ECH)
            ('X', []) => {
                let n = p(0, 1) as usize;
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                let erase = self.make_erase_cell();
                if row < self.cells.len() {
                    for c in col..(col + n).min(self.cols as usize) {
                        self.cells[row][c] = erase.clone();
                    }
                }
            }
            // Delete chars (DCH)
            ('P', []) => {
                let n = p(0, 1) as usize;
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                if row < self.cells.len() {
                    for _ in 0..n {
                        if col < self.cells[row].len() {
                            self.cells[row].remove(col);
                            self.cells[row].push(Cell::default());
                        }
                    }
                }
            }
            // Insert blank chars (ICH)
            ('@', []) => {
                let n = p(0, 1) as usize;
                let row = self.cursor_row as usize;
                let col = self.cursor_col as usize;
                if row < self.cells.len() {
                    for _ in 0..n {
                        self.cells[row].insert(col, Cell::default());
                        self.cells[row].truncate(self.cols as usize);
                    }
                }
            }
            // DEC private modes SET (? prefix)
            ('h', [b'?']) => {
                for param in params.iter() {
                    match param[0] {
                        1 => self.application_cursor_keys = true,
                        7 => {} // auto-wrap — always on
                        25 => self.cursor_visible = true,
                        1049 => self.enter_alternate_screen(),
                        2004 => self.bracketed_paste = true,
                        _ => {}
                    }
                }
            }
            // DEC private modes RESET (? prefix)
            ('l', [b'?']) => {
                for param in params.iter() {
                    match param[0] {
                        1 => self.application_cursor_keys = false,
                        7 => {} // auto-wrap — always on
                        25 => self.cursor_visible = false,
                        1049 => self.leave_alternate_screen(),
                        2004 => self.bracketed_paste = false,
                        _ => {}
                    }
                }
            }
            // Save cursor (ANSI)
            ('s', []) => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col, self.current_style));
            }
            // Restore cursor (ANSI)
            ('u', []) => {
                if let Some((row, col, style)) = self.saved_cursor {
                    self.cursor_row = row;
                    self.cursor_col = col;
                    self.current_style = style;
                }
            }
            // Tab clear, etc. — ignore
            _ => {}
        }
    }

    fn esc_dispatch(&mut self, intermediates: &[u8], _ignore: bool, byte: u8) {
        self.wrap_pending = false;
        match (byte, intermediates) {
            // RIS - Full reset
            (b'c', []) => {
                *self = Self::new(self.cols, self.rows);
            }
            // Save cursor (DECSC)
            (b'7', []) => {
                self.saved_cursor = Some((self.cursor_row, self.cursor_col, self.current_style));
            }
            // Restore cursor (DECRC)
            (b'8', []) => {
                if let Some((row, col, style)) = self.saved_cursor {
                    self.cursor_row = row;
                    self.cursor_col = col;
                    self.current_style = style;
                }
            }
            // Reverse Index (move up, scroll if at top)
            (b'M', []) => {
                if self.cursor_row == self.scroll_region_top {
                    self.scroll_down();
                } else {
                    self.cursor_row = self.cursor_row.saturating_sub(1);
                }
            }
            _ => {}
        }
    }
}
