use ratatui::style::Color;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Theme {
    pub name: String,
    pub ui: UiColors,
    pub syntax: SyntaxColors,
    pub git: GitColors,
}

#[derive(Debug, Clone)]
pub struct UiColors {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,
    pub line_number: Color,
    pub line_number_active: Color,
    pub status_bar_bg: Color,
    pub status_bar_fg: Color,
    pub status_bar_mode_bg: Color,
    pub status_bar_mode_fg: Color,
    pub tab_active_bg: Color,
    pub tab_active_fg: Color,
    pub tab_inactive_bg: Color,
    pub tab_inactive_fg: Color,
    pub file_tree_bg: Color,
    pub file_tree_fg: Color,
    pub file_tree_selected_bg: Color,
    pub file_tree_selected_fg: Color,
    pub file_tree_dir: Color,
    pub border: Color,
    pub search_match: Color,
    pub search_current: Color,
    pub popup_bg: Color,
    pub popup_border: Color,
    pub accent: Color,
}

#[derive(Debug, Clone)]
pub struct SyntaxColors {
    pub keyword: Color,
    pub string: Color,
    pub number: Color,
    pub comment: Color,
    pub function: Color,
    pub r#type: Color,
    pub variable: Color,
    pub constant: Color,
    pub operator: Color,
    pub punctuation: Color,
}

#[derive(Debug, Clone)]
pub struct GitColors {
    pub added: Color,
    pub modified: Color,
    pub deleted: Color,
    pub renamed: Color,
    pub conflict: Color,
    pub branch: Color,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeName {
    Amethyst,
    Aureum,
}

impl ThemeName {
    pub fn next(&self) -> Self {
        match self {
            Self::Amethyst => Self::Aureum,
            Self::Aureum => Self::Amethyst,
        }
    }
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Amethyst => write!(f, "Amethyst"),
            Self::Aureum => write!(f, "Aureum"),
        }
    }
}

pub fn amethyst() -> Theme {
    Theme {
        name: "Amethyst".to_string(),
        ui: UiColors {
            background: Color::Rgb(13, 13, 20),
            foreground: Color::Rgb(220, 215, 235),
            cursor: Color::Rgb(157, 78, 221),
            selection: Color::Rgb(80, 50, 120),
            line_number: Color::Rgb(70, 60, 90),
            line_number_active: Color::Rgb(157, 78, 221),
            status_bar_bg: Color::Rgb(26, 20, 40),
            status_bar_fg: Color::Rgb(208, 161, 255),
            status_bar_mode_bg: Color::Rgb(157, 78, 221),
            status_bar_mode_fg: Color::Rgb(13, 13, 20),
            tab_active_bg: Color::Rgb(40, 30, 60),
            tab_active_fg: Color::Rgb(208, 161, 255),
            tab_inactive_bg: Color::Rgb(20, 16, 30),
            tab_inactive_fg: Color::Rgb(100, 85, 130),
            file_tree_bg: Color::Rgb(16, 14, 24),
            file_tree_fg: Color::Rgb(180, 170, 200),
            file_tree_selected_bg: Color::Rgb(50, 35, 75),
            file_tree_selected_fg: Color::Rgb(220, 200, 255),
            file_tree_dir: Color::Rgb(157, 78, 221),
            border: Color::Rgb(60, 45, 85),
            search_match: Color::Rgb(120, 80, 180),
            search_current: Color::Rgb(180, 120, 255),
            popup_bg: Color::Rgb(20, 16, 30),
            popup_border: Color::Rgb(157, 78, 221),
            accent: Color::Rgb(157, 78, 221),
        },
        syntax: SyntaxColors {
            keyword: Color::Rgb(199, 125, 255),
            string: Color::Rgb(224, 170, 255),
            number: Color::Rgb(180, 140, 255),
            comment: Color::Rgb(90, 78, 122),
            function: Color::Rgb(179, 136, 255),
            r#type: Color::Rgb(160, 120, 220),
            variable: Color::Rgb(200, 190, 230),
            constant: Color::Rgb(230, 180, 255),
            operator: Color::Rgb(170, 150, 210),
            punctuation: Color::Rgb(130, 115, 170),
        },
        git: GitColors {
            added: Color::Rgb(120, 220, 120),
            modified: Color::Rgb(180, 160, 255),
            deleted: Color::Rgb(255, 100, 100),
            renamed: Color::Rgb(150, 200, 255),
            conflict: Color::Rgb(255, 180, 80),
            branch: Color::Rgb(157, 78, 221),
        },
    }
}

pub fn aureum() -> Theme {
    Theme {
        name: "Aureum".to_string(),
        ui: UiColors {
            background: Color::Rgb(13, 11, 7),
            foreground: Color::Rgb(235, 225, 200),
            cursor: Color::Rgb(255, 183, 0),
            selection: Color::Rgb(100, 80, 30),
            line_number: Color::Rgb(90, 75, 50),
            line_number_active: Color::Rgb(255, 183, 0),
            status_bar_bg: Color::Rgb(26, 21, 8),
            status_bar_fg: Color::Rgb(255, 215, 0),
            status_bar_mode_bg: Color::Rgb(255, 183, 0),
            status_bar_mode_fg: Color::Rgb(13, 11, 7),
            tab_active_bg: Color::Rgb(50, 40, 15),
            tab_active_fg: Color::Rgb(255, 215, 0),
            tab_inactive_bg: Color::Rgb(20, 17, 8),
            tab_inactive_fg: Color::Rgb(130, 110, 70),
            file_tree_bg: Color::Rgb(16, 13, 8),
            file_tree_fg: Color::Rgb(200, 185, 150),
            file_tree_selected_bg: Color::Rgb(60, 48, 18),
            file_tree_selected_fg: Color::Rgb(255, 230, 150),
            file_tree_dir: Color::Rgb(255, 183, 0),
            border: Color::Rgb(85, 68, 30),
            search_match: Color::Rgb(150, 120, 40),
            search_current: Color::Rgb(255, 200, 50),
            popup_bg: Color::Rgb(20, 17, 8),
            popup_border: Color::Rgb(255, 183, 0),
            accent: Color::Rgb(255, 183, 0),
        },
        syntax: SyntaxColors {
            keyword: Color::Rgb(255, 213, 79),
            string: Color::Rgb(255, 224, 130),
            number: Color::Rgb(255, 200, 80),
            comment: Color::Rgb(107, 93, 62),
            function: Color::Rgb(255, 202, 40),
            r#type: Color::Rgb(220, 180, 60),
            variable: Color::Rgb(230, 215, 170),
            constant: Color::Rgb(255, 235, 130),
            operator: Color::Rgb(210, 190, 130),
            punctuation: Color::Rgb(170, 150, 100),
        },
        git: GitColors {
            added: Color::Rgb(120, 220, 120),
            modified: Color::Rgb(255, 200, 80),
            deleted: Color::Rgb(255, 100, 100),
            renamed: Color::Rgb(150, 200, 255),
            conflict: Color::Rgb(255, 150, 50),
            branch: Color::Rgb(255, 183, 0),
        },
    }
}

pub fn get_theme(name: ThemeName) -> Theme {
    match name {
        ThemeName::Amethyst => amethyst(),
        ThemeName::Aureum => aureum(),
    }
}
