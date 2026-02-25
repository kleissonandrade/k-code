#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Terminal error: {0}")]
    Terminal(String),
    #[error("Git error: {0}")]
    Git(#[from] k_git::GitError),
    #[error("{0}")]
    Other(String),
}
