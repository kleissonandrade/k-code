use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use k_terminal::cell::AnsiColor;
use k_terminal::{PtyHandle, TerminalGrid};

use crate::theme::Theme;

struct TerminalInstance {
    id: usize,
    title: String,
    grid: TerminalGrid,
    pty: PtyHandle,
    scroll_offset: usize,
    exited: bool,
}

pub struct TerminalPanelComponent {
    pub visible: bool,
    pub height: u16,
    terminals: Vec<TerminalInstance>,
    active_terminal: usize,
    next_id: usize,
    workspace_root: String,
    pub last_cols: u16,
    pub last_rows: u16,
}

impl TerminalPanelComponent {
    pub fn new(workspace_root: String) -> Self {
        Self {
            visible: false,
            height: 20,
            terminals: Vec::new(),
            active_terminal: 0,
            next_id: 1,
            workspace_root,
            last_cols: 0,
            last_rows: 0,
        }
    }

    pub fn spawn_terminal(&mut self, cols: u16, rows: u16) {
        let grid = TerminalGrid::new(cols, rows);
        match PtyHandle::spawn(cols, rows, &self.workspace_root) {
            Ok(pty) => {
                let id = self.next_id;
                self.next_id += 1;
                let title = format!("Terminal {}", id);
                self.terminals.push(TerminalInstance {
                    id,
                    title,
                    grid,
                    pty,
                    scroll_offset: 0,
                    exited: false,
                });
                self.active_terminal = self.terminals.len() - 1;
            }
            Err(_) => {}
        }
    }

    pub fn has_terminals(&self) -> bool {
        !self.terminals.is_empty()
    }

    pub fn toggle(&mut self) {
        self.visible = !self.visible;
    }

    pub fn increase_height(&mut self) {
        self.height = (self.height + 3).min(50);
        self.last_rows = 0; // force resize on next render
    }

    pub fn decrease_height(&mut self) {
        self.height = self.height.saturating_sub(3).max(5);
        self.last_rows = 0; // force resize on next render
    }

    pub fn close_active(&mut self) {
        if self.terminals.is_empty() {
            return;
        }
        self.terminals.remove(self.active_terminal);
        if self.terminals.is_empty() {
            self.visible = false;
            self.active_terminal = 0;
        } else if self.active_terminal >= self.terminals.len() {
            self.active_terminal = self.terminals.len() - 1;
        }
    }

    pub fn next_tab(&mut self) {
        if self.terminals.len() > 1 {
            self.active_terminal = (self.active_terminal + 1) % self.terminals.len();
        }
    }

    pub fn prev_tab(&mut self) {
        if self.terminals.len() > 1 {
            self.active_terminal = if self.active_terminal == 0 {
                self.terminals.len() - 1
            } else {
                self.active_terminal - 1
            };
        }
    }

    pub fn poll_output(&mut self) {
        for term in &mut self.terminals {
            if let Some(data) = term.pty.try_read_output() {
                term.grid.process_bytes(&data);
                term.scroll_offset = 0;
                // Send DSR responses back to PTY
                if !term.grid.response_bytes.is_empty() {
                    let response: Vec<u8> = term.grid.response_bytes.drain(..).collect();
                    term.pty.write_input(&response);
                }
            }
        }
    }

    pub fn resize_if_needed(&mut self, cols: u16, rows: u16) {
        if cols == self.last_cols && rows == self.last_rows {
            return;
        }
        self.last_cols = cols;
        self.last_rows = rows;
        for term in &mut self.terminals {
            term.pty.resize(cols, rows);
            term.grid.resize(cols, rows);
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if self.terminals.is_empty() {
            return;
        }
        let term = &mut self.terminals[self.active_terminal];
        let app_cursor = term.grid.application_cursor_keys;
        if let Some(bytes) = key_to_bytes(key, app_cursor) {
            term.pty.write_input(&bytes);
        }
    }

    pub fn scroll_up(&mut self) {
        if let Some(term) = self.terminals.get_mut(self.active_terminal) {
            let max = term.grid.scrollback.len();
            if term.scroll_offset < max {
                term.scroll_offset += 3;
                if term.scroll_offset > max {
                    term.scroll_offset = max;
                }
            }
        }
    }

    pub fn scroll_down(&mut self) {
        if let Some(term) = self.terminals.get_mut(self.active_terminal) {
            term.scroll_offset = term.scroll_offset.saturating_sub(3);
        }
    }

    pub fn is_in_area(&self, x: u16, y: u16, area: Rect) -> bool {
        x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height
    }

    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &Theme) {
        if area.height < 3 || area.width < 4 {
            return;
        }

        let bg = theme.ui.background;
        let fg = theme.ui.foreground;

        // Tab bar (1 line at top)
        let tab_area = Rect::new(area.x, area.y, area.width, 1);
        let content_area = Rect::new(area.x, area.y + 1, area.width, area.height - 1);

        // Render separator + tabs
        let mut tab_spans: Vec<Span> = Vec::new();
        let sep_style = Style::default().fg(theme.ui.border).bg(theme.ui.tab_inactive_bg);
        tab_spans.push(Span::styled("─", sep_style));

        for (idx, term) in self.terminals.iter().enumerate() {
            let is_active = idx == self.active_terminal;
            let style = if is_active {
                Style::default()
                    .fg(theme.ui.accent)
                    .bg(theme.ui.tab_inactive_bg)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(theme.ui.tab_inactive_fg)
                    .bg(theme.ui.tab_inactive_bg)
            };
            tab_spans.push(Span::styled(
                format!(" {} ", term.title),
                style,
            ));
            tab_spans.push(Span::styled("│", sep_style));
        }

        // + button for new terminal
        let plus_style = Style::default()
            .fg(theme.ui.line_number)
            .bg(theme.ui.tab_inactive_bg);
        tab_spans.push(Span::styled(" + ", plus_style));

        // Resize buttons at right edge: ─ + ─ ─
        let btn_style = Style::default()
            .fg(theme.ui.accent)
            .bg(theme.ui.tab_inactive_bg)
            .add_modifier(Modifier::BOLD);
        let resize_buttons = vec![
            Span::styled(" + ", btn_style),
            Span::styled("│", sep_style),
            Span::styled(" - ", btn_style),
            Span::styled("─", sep_style),
        ];
        let btn_width: usize = resize_buttons.iter().map(|s| s.width()).sum();

        // Fill rest with separator (leaving room for buttons)
        let used: usize = tab_spans.iter().map(|s| s.width()).sum();
        let remaining = (area.width as usize).saturating_sub(used + btn_width);
        if remaining > 0 {
            tab_spans.push(Span::styled(
                "─".repeat(remaining),
                sep_style,
            ));
        }
        tab_spans.extend(resize_buttons);

        frame.render_widget(Paragraph::new(Line::from(tab_spans)), tab_area);

        // Render terminal content
        if let Some(term) = self.terminals.get(self.active_terminal) {
            let rows = content_area.height as usize;
            let cols = content_area.width as usize;
            let cells = term.grid.active_cells();

            let mut lines: Vec<Line> = Vec::new();
            for row_idx in 0..rows {
                let grid_row = if term.scroll_offset > 0 {
                    // Reading from scrollback
                    let sb_len = term.grid.scrollback.len();
                    let sb_start = sb_len.saturating_sub(term.scroll_offset);
                    let actual_row = sb_start + row_idx;
                    if actual_row < sb_len {
                        Some(&term.grid.scrollback[actual_row])
                    } else {
                        let grid_idx = actual_row - sb_len;
                        cells.get(grid_idx)
                    }
                } else {
                    cells.get(row_idx)
                };

                if let Some(row_cells) = grid_row {
                    let mut spans: Vec<Span> = Vec::new();
                    let mut current_text = String::new();
                    let mut current_style = Style::default().fg(fg).bg(bg);
                    let mut col = 0;

                    for cell in row_cells.iter().take(cols) {
                        if cell.width == 0 {
                            continue; // continuation of wide char
                        }
                        let style = cell_to_ratatui_style(&cell.style, bg, fg);
                        if style != current_style {
                            if !current_text.is_empty() {
                                spans.push(Span::styled(current_text.clone(), current_style));
                                current_text.clear();
                            }
                            current_style = style;
                        }
                        current_text.push(cell.ch);
                        col += cell.width as usize;
                    }
                    // Pad remaining cols
                    let default_style = Style::default().bg(bg).fg(fg);
                    if col < cols {
                        if current_style == default_style {
                            current_text.push_str(&" ".repeat(cols - col));
                        } else {
                            if !current_text.is_empty() {
                                spans.push(Span::styled(current_text.clone(), current_style));
                                current_text.clear();
                            }
                            current_style = default_style;
                            current_text = " ".repeat(cols - col);
                        }
                    }
                    if !current_text.is_empty() {
                        spans.push(Span::styled(current_text, current_style));
                    }
                    lines.push(Line::from(spans));
                } else {
                    lines.push(Line::from(Span::styled(
                        " ".repeat(cols),
                        Style::default().bg(bg).fg(fg),
                    )));
                }
            }

            frame.render_widget(Paragraph::new(lines), content_area);

            // Cursor
            if term.grid.cursor_visible && term.scroll_offset == 0 {
                let cx = content_area.x + term.grid.cursor_col;
                let cy = content_area.y + term.grid.cursor_row;
                if cx < content_area.x + content_area.width
                    && cy < content_area.y + content_area.height
                {
                    frame.set_cursor_position((cx, cy));
                }
            }
        }
    }

    pub fn tab_click(&mut self, x: u16, area: Rect) -> TabClickResult {
        let rel_x = (x - area.x) as usize;
        let w = area.width as usize;

        // Resize buttons at right edge: " + " "│" " - " "─" = 8 chars
        // " - " starts at w-4, " + " starts at w-8
        let minus_start = w.saturating_sub(4);
        let plus_start = w.saturating_sub(8);
        if rel_x >= minus_start && rel_x < minus_start + 3 {
            self.decrease_height();
            return TabClickResult::Resized;
        }
        if rel_x >= plus_start && rel_x < plus_start + 3 {
            self.increase_height();
            return TabClickResult::Resized;
        }

        // Check clicks on terminal tabs
        let mut pos = 1; // skip first separator char
        for (idx, term) in self.terminals.iter().enumerate() {
            let tab_width = term.title.len() + 2; // " title "
            if rel_x >= pos && rel_x < pos + tab_width {
                self.active_terminal = idx;
                return TabClickResult::Switched;
            }
            pos += tab_width + 1; // +1 for separator
        }
        // Check + (new terminal) button
        if rel_x >= pos && rel_x < pos + 3 {
            return TabClickResult::NewTerminal;
        }
        TabClickResult::None
    }
}

pub enum TabClickResult {
    None,
    Switched,
    NewTerminal,
    Resized,
}

fn key_to_bytes(key: KeyEvent, app_cursor: bool) -> Option<Vec<u8>> {
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char(c)) => {
            let byte = (c as u8).wrapping_sub(b'a').wrapping_add(1);
            Some(vec![byte])
        }
        (KeyModifiers::NONE, KeyCode::Char(c)) | (KeyModifiers::SHIFT, KeyCode::Char(c)) => {
            let mut buf = [0u8; 4];
            let s = c.encode_utf8(&mut buf);
            Some(s.as_bytes().to_vec())
        }
        (_, KeyCode::Enter) => Some(vec![b'\r']),
        (_, KeyCode::Backspace) => Some(vec![0x7f]),
        (_, KeyCode::Tab) => Some(vec![b'\t']),
        (_, KeyCode::Esc) => Some(vec![0x1b]),
        (_, KeyCode::Up) => {
            if app_cursor { Some(b"\x1bOA".to_vec()) } else { Some(b"\x1b[A".to_vec()) }
        }
        (_, KeyCode::Down) => {
            if app_cursor { Some(b"\x1bOB".to_vec()) } else { Some(b"\x1b[B".to_vec()) }
        }
        (_, KeyCode::Right) => {
            if app_cursor { Some(b"\x1bOC".to_vec()) } else { Some(b"\x1b[C".to_vec()) }
        }
        (_, KeyCode::Left) => {
            if app_cursor { Some(b"\x1bOD".to_vec()) } else { Some(b"\x1b[D".to_vec()) }
        }
        (_, KeyCode::Home) => {
            if app_cursor { Some(b"\x1bOH".to_vec()) } else { Some(b"\x1b[H".to_vec()) }
        }
        (_, KeyCode::End) => {
            if app_cursor { Some(b"\x1bOF".to_vec()) } else { Some(b"\x1b[F".to_vec()) }
        }
        (_, KeyCode::PageUp) => Some(b"\x1b[5~".to_vec()),
        (_, KeyCode::PageDown) => Some(b"\x1b[6~".to_vec()),
        (_, KeyCode::Delete) => Some(b"\x1b[3~".to_vec()),
        (_, KeyCode::Insert) => Some(b"\x1b[2~".to_vec()),
        _ => None,
    }
}

fn ansi_to_color(ansi: &AnsiColor) -> Color {
    match ansi {
        AnsiColor::Named(n) => match n {
            0 => Color::Black,
            1 => Color::Red,
            2 => Color::Green,
            3 => Color::Yellow,
            4 => Color::Blue,
            5 => Color::Magenta,
            6 => Color::Cyan,
            7 => Color::White,
            8 => Color::DarkGray,
            9 => Color::LightRed,
            10 => Color::LightGreen,
            11 => Color::LightYellow,
            12 => Color::LightBlue,
            13 => Color::LightMagenta,
            14 => Color::LightCyan,
            15 => Color::White,
            _ => Color::White,
        },
        AnsiColor::Indexed(i) => Color::Indexed(*i),
        AnsiColor::Rgb(r, g, b) => Color::Rgb(*r, *g, *b),
    }
}

fn cell_to_ratatui_style(
    style: &k_terminal::cell::CellStyle,
    default_bg: Color,
    default_fg: Color,
) -> Style {
    let mut fg = style.fg.as_ref().map(ansi_to_color).unwrap_or(default_fg);
    let mut bg = style.bg.as_ref().map(ansi_to_color).unwrap_or(default_bg);
    if style.reverse {
        std::mem::swap(&mut fg, &mut bg);
    }
    let mut s = Style::default().fg(fg).bg(bg);
    if style.bold {
        s = s.add_modifier(Modifier::BOLD);
    }
    if style.italic {
        s = s.add_modifier(Modifier::ITALIC);
    }
    if style.underline {
        s = s.add_modifier(Modifier::UNDERLINED);
    }
    if style.dim {
        s = s.add_modifier(Modifier::DIM);
    }
    if style.strikethrough {
        s = s.add_modifier(Modifier::CROSSED_OUT);
    }
    s
}
