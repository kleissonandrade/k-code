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
    Dracula,
    Airo,
    Monokai,
}

impl ThemeName {
    pub fn next(&self) -> Self {
        match self {
            Self::Amethyst => Self::Aureum,
            Self::Aureum => Self::Dracula,
            Self::Dracula => Self::Airo,
            Self::Airo => Self::Monokai,
            Self::Monokai => Self::Amethyst,
        }
    }
}

impl std::fmt::Display for ThemeName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Amethyst => write!(f, "Amethyst"),
            Self::Aureum => write!(f, "Aureum"),
            Self::Dracula => write!(f, "Dracula"),
            Self::Airo => write!(f, "Airo"),
            Self::Monokai => write!(f, "Monokai"),
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

pub fn dracula() -> Theme {
    Theme {
        name: "Dracula".to_string(),
        ui: UiColors {
            background: Color::Rgb(40, 42, 54),
            foreground: Color::Rgb(248, 248, 242),
            cursor: Color::Rgb(189, 147, 249),
            selection: Color::Rgb(68, 71, 90),
            line_number: Color::Rgb(98, 114, 164),
            line_number_active: Color::Rgb(248, 248, 242),
            status_bar_bg: Color::Rgb(33, 34, 44),
            status_bar_fg: Color::Rgb(248, 248, 242),
            status_bar_mode_bg: Color::Rgb(189, 147, 249),
            status_bar_mode_fg: Color::Rgb(40, 42, 54),
            tab_active_bg: Color::Rgb(68, 71, 90),
            tab_active_fg: Color::Rgb(248, 248, 242),
            tab_inactive_bg: Color::Rgb(33, 34, 44),
            tab_inactive_fg: Color::Rgb(98, 114, 164),
            file_tree_bg: Color::Rgb(33, 34, 44),
            file_tree_fg: Color::Rgb(248, 248, 242),
            file_tree_selected_bg: Color::Rgb(68, 71, 90),
            file_tree_selected_fg: Color::Rgb(248, 248, 242),
            file_tree_dir: Color::Rgb(139, 233, 253),
            border: Color::Rgb(68, 71, 90),
            search_match: Color::Rgb(241, 250, 140),
            search_current: Color::Rgb(255, 184, 108),
            popup_bg: Color::Rgb(33, 34, 44),
            popup_border: Color::Rgb(189, 147, 249),
            accent: Color::Rgb(189, 147, 249),
        },
        syntax: SyntaxColors {
            keyword: Color::Rgb(255, 121, 198),    // Pink
            string: Color::Rgb(241, 250, 140),     // Yellow
            number: Color::Rgb(189, 147, 249),     // Purple
            comment: Color::Rgb(98, 114, 164),     // Comment
            function: Color::Rgb(80, 250, 123),    // Green
            r#type: Color::Rgb(139, 233, 253),     // Cyan
            variable: Color::Rgb(248, 248, 242),   // Foreground
            constant: Color::Rgb(189, 147, 249),   // Purple
            operator: Color::Rgb(255, 121, 198),   // Pink
            punctuation: Color::Rgb(248, 248, 242),
        },
        git: GitColors {
            added: Color::Rgb(80, 250, 123),       // Green
            modified: Color::Rgb(255, 184, 108),   // Orange
            deleted: Color::Rgb(255, 85, 85),      // Red
            renamed: Color::Rgb(139, 233, 253),    // Cyan
            conflict: Color::Rgb(255, 184, 108),   // Orange
            branch: Color::Rgb(189, 147, 249),     // Purple
        },
    }
}

pub fn airo() -> Theme {
    Theme {
        name: "Airo".to_string(),
        ui: UiColors {
            background: Color::Rgb(15, 20, 25),
            foreground: Color::Rgb(200, 210, 220),
            cursor: Color::Rgb(0, 180, 216),
            selection: Color::Rgb(30, 60, 80),
            line_number: Color::Rgb(55, 70, 85),
            line_number_active: Color::Rgb(0, 180, 216),
            status_bar_bg: Color::Rgb(10, 15, 20),
            status_bar_fg: Color::Rgb(140, 200, 230),
            status_bar_mode_bg: Color::Rgb(0, 180, 216),
            status_bar_mode_fg: Color::Rgb(15, 20, 25),
            tab_active_bg: Color::Rgb(25, 40, 55),
            tab_active_fg: Color::Rgb(140, 200, 230),
            tab_inactive_bg: Color::Rgb(12, 17, 22),
            tab_inactive_fg: Color::Rgb(70, 90, 110),
            file_tree_bg: Color::Rgb(12, 17, 22),
            file_tree_fg: Color::Rgb(170, 185, 200),
            file_tree_selected_bg: Color::Rgb(25, 45, 65),
            file_tree_selected_fg: Color::Rgb(200, 225, 245),
            file_tree_dir: Color::Rgb(0, 180, 216),
            border: Color::Rgb(40, 55, 70),
            search_match: Color::Rgb(60, 120, 160),
            search_current: Color::Rgb(0, 210, 250),
            popup_bg: Color::Rgb(12, 17, 22),
            popup_border: Color::Rgb(0, 180, 216),
            accent: Color::Rgb(0, 180, 216),
        },
        syntax: SyntaxColors {
            keyword: Color::Rgb(255, 121, 198),
            string: Color::Rgb(130, 224, 170),
            number: Color::Rgb(255, 183, 77),
            comment: Color::Rgb(70, 90, 110),
            function: Color::Rgb(100, 200, 255),
            r#type: Color::Rgb(0, 210, 250),
            variable: Color::Rgb(200, 210, 220),
            constant: Color::Rgb(255, 183, 77),
            operator: Color::Rgb(255, 121, 198),
            punctuation: Color::Rgb(140, 160, 180),
        },
        git: GitColors {
            added: Color::Rgb(80, 250, 123),
            modified: Color::Rgb(0, 180, 216),
            deleted: Color::Rgb(255, 85, 85),
            renamed: Color::Rgb(130, 224, 170),
            conflict: Color::Rgb(255, 183, 77),
            branch: Color::Rgb(0, 180, 216),
        },
    }
}

pub fn monokai() -> Theme {
    Theme {
        name: "Monokai".to_string(),
        ui: UiColors {
            background: Color::Rgb(39, 40, 34),
            foreground: Color::Rgb(248, 248, 242),
            cursor: Color::Rgb(248, 248, 240),
            selection: Color::Rgb(73, 72, 62),
            line_number: Color::Rgb(90, 90, 80),
            line_number_active: Color::Rgb(248, 248, 242),
            status_bar_bg: Color::Rgb(30, 31, 26),
            status_bar_fg: Color::Rgb(248, 248, 242),
            status_bar_mode_bg: Color::Rgb(166, 226, 46),
            status_bar_mode_fg: Color::Rgb(39, 40, 34),
            tab_active_bg: Color::Rgb(73, 72, 62),
            tab_active_fg: Color::Rgb(248, 248, 242),
            tab_inactive_bg: Color::Rgb(30, 31, 26),
            tab_inactive_fg: Color::Rgb(117, 113, 94),
            file_tree_bg: Color::Rgb(30, 31, 26),
            file_tree_fg: Color::Rgb(248, 248, 242),
            file_tree_selected_bg: Color::Rgb(73, 72, 62),
            file_tree_selected_fg: Color::Rgb(248, 248, 242),
            file_tree_dir: Color::Rgb(102, 217, 239),
            border: Color::Rgb(73, 72, 62),
            search_match: Color::Rgb(230, 219, 116),
            search_current: Color::Rgb(249, 38, 114),
            popup_bg: Color::Rgb(30, 31, 26),
            popup_border: Color::Rgb(166, 226, 46),
            accent: Color::Rgb(166, 226, 46),
        },
        syntax: SyntaxColors {
            keyword: Color::Rgb(249, 38, 114),      // Monokai Pink/Red
            string: Color::Rgb(230, 219, 116),      // Monokai Yellow
            number: Color::Rgb(174, 129, 255),      // Monokai Purple
            comment: Color::Rgb(117, 113, 94),      // Monokai Comment
            function: Color::Rgb(166, 226, 46),     // Monokai Green
            r#type: Color::Rgb(102, 217, 239),      // Monokai Cyan
            variable: Color::Rgb(248, 248, 242),    // Monokai Foreground
            constant: Color::Rgb(174, 129, 255),    // Monokai Purple
            operator: Color::Rgb(249, 38, 114),     // Monokai Pink/Red
            punctuation: Color::Rgb(248, 248, 242),
        },
        git: GitColors {
            added: Color::Rgb(166, 226, 46),        // Green
            modified: Color::Rgb(230, 219, 116),    // Yellow
            deleted: Color::Rgb(249, 38, 114),      // Pink/Red
            renamed: Color::Rgb(102, 217, 239),     // Cyan
            conflict: Color::Rgb(253, 151, 31),     // Orange
            branch: Color::Rgb(174, 129, 255),      // Purple
        },
    }
}

pub fn get_theme(name: ThemeName) -> Theme {
    match name {
        ThemeName::Amethyst => amethyst(),
        ThemeName::Aureum => aureum(),
        ThemeName::Dracula => dracula(),
        ThemeName::Airo => airo(),
        ThemeName::Monokai => monokai(),
    }
}
