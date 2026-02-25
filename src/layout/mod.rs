use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub const ACTIVITY_BAR_WIDTH: u16 = 7;

pub struct AppLayout {
    pub activity_bar: Rect,
    pub file_tree: Option<Rect>,
    pub tab_bar: Rect,
    pub editor: Rect,
    pub status_bar: Rect,
}

pub fn build_layout(area: Rect, show_tree: bool, tree_width: u16) -> AppLayout {
    // First split: status bar at bottom, main content above
    let vert = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main_area = vert[0];
    let status_bar = vert[1];

    // Horizontal split: activity bar | file tree (optional) | editor area
    let horiz = if show_tree {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(ACTIVITY_BAR_WIDTH),
                Constraint::Length(tree_width),
                Constraint::Min(1),
            ])
            .split(main_area)
    } else {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(ACTIVITY_BAR_WIDTH),
                Constraint::Min(1),
            ])
            .split(main_area)
    };

    let activity_bar = horiz[0];

    let (tree_area, right) = if show_tree {
        (Some(horiz[1]), horiz[2])
    } else {
        (None, horiz[1])
    };

    // Right side: tab bar + editor
    let right_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(right);

    AppLayout {
        activity_bar,
        file_tree: tree_area,
        tab_bar: right_chunks[0],
        editor: right_chunks[1],
        status_bar,
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
