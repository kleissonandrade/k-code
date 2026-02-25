use serde::{Deserialize, Serialize};

use crate::theme::ThemeName;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_theme")]
    pub theme: ThemeName,
    #[serde(default)]
    pub editor: EditorConfig,
    #[serde(default)]
    pub file_tree: FileTreeConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorConfig {
    #[serde(default = "default_tab_size")]
    pub tab_size: usize,
    #[serde(default = "default_true")]
    pub use_spaces: bool,
    #[serde(default = "default_true")]
    pub show_line_numbers: bool,
    #[serde(default = "default_true")]
    pub highlight_current_line: bool,
    #[serde(default = "default_scroll_padding")]
    pub scroll_padding: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileTreeConfig {
    #[serde(default)]
    pub show_hidden: bool,
    #[serde(default = "default_tree_width")]
    pub width: u16,
    #[serde(default = "default_true")]
    pub show_icons: bool,
}

fn default_theme() -> ThemeName {
    ThemeName::Amethyst
}
fn default_tab_size() -> usize {
    4
}
fn default_true() -> bool {
    true
}
fn default_scroll_padding() -> usize {
    5
}
fn default_tree_width() -> u16 {
    40
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            editor: EditorConfig::default(),
            file_tree: FileTreeConfig::default(),
        }
    }
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: default_tab_size(),
            use_spaces: true,
            show_line_numbers: true,
            highlight_current_line: true,
            scroll_padding: default_scroll_padding(),
        }
    }
}

impl Default for FileTreeConfig {
    fn default() -> Self {
        Self {
            show_hidden: false,
            width: default_tree_width(),
            show_icons: true,
        }
    }
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = dirs::config_dir()
            .map(|d| d.join("k-code").join("config.toml"));

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str(&content) {
                        return config;
                    }
                }
            }
        }
        Self::default()
    }
}
