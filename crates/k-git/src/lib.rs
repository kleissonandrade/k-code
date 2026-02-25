pub mod error;
pub mod repository;

pub use error::GitError;
pub use repository::{
    BranchInfo, CommitInfo, DiffHunk, DiffLine, GitRepository, GitResponse, StashEntry,
    StatusEntry, StatusKind,
};
