#[derive(Debug, thiserror::Error)]
pub enum GitError {
    #[error("Git error: {0}")]
    Git2(#[from] git2::Error),
    #[error("Not a git repository")]
    NotARepo,
    #[error("Operation failed: {0}")]
    Operation(String),
    #[error("Channel error")]
    Channel,
}
