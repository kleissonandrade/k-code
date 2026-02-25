use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub struct AppLayout {
    pub file_tree: Option<Rect>,
    pub tab_bar: Rect,
    pub editor: Rect,
    pub status_bar: Rect,
}

pub fn build_layout(area: Rect, show_tree: bool, tree_width: u16) -> AppLayout {
    let main_chunks = if show_tree {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(tree_width),
                Constraint::Min(1),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Min(1)])
            .split(area)
    };

    let tree_area = if show_tree {
        Some(main_chunks[0])
    } else {
        None
    };

    let right = if show_tree {
        main_chunks[1]
    } else {
        main_chunks[0]
    };

    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(right);

    AppLayout {
        file_tree: tree_area,
        tab_bar: right_chunks[0],
        editor: right_chunks[1],
        status_bar: right_chunks[2],
    }
}

pub fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
