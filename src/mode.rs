#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EditorMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
    Command,
    Search,
    FuzzyFinder,
    GitPanel,
    Tutorial,
}

impl EditorMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Normal => "NORMAL",
            Self::Insert => "INSERT",
            Self::Visual => "VISUAL",
            Self::VisualLine => "V-LINE",
            Self::Command => "COMMAND",
            Self::Search => "SEARCH",
            Self::FuzzyFinder => "FILES",
            Self::GitPanel => "GIT",
            Self::Tutorial => "HELP",
        }
    }
}

impl std::fmt::Display for EditorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}
