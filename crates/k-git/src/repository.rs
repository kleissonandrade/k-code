use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;

use crossbeam_channel::{Receiver, Sender};
use git2::{Repository, Signature};
use tokio::sync::mpsc;

use crate::error::GitError;

#[derive(Debug, Clone)]
pub struct StatusEntry {
    pub path: String,
    pub kind: StatusKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    New,
    Modified,
    Deleted,
    Renamed,
    Typechange,
    Conflicted,
    Untracked,
}

#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub id: String,
    pub short_id: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_current: bool,
    pub is_remote: bool,
}

#[derive(Debug, Clone)]
pub struct StashEntry {
    pub index: usize,
    pub message: String,
}

#[derive(Debug, Clone)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub kind: DiffLineKind,
    pub content: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiffLineKind {
    Context,
    Addition,
    Deletion,
    Header,
}

#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    pub name: String,
    pub path: String,
    pub branch: String,
    pub is_main: bool,
    pub status_count: usize,
    pub commits_ahead: usize,
}

#[derive(Debug)]
pub enum GitRequest {
    Status,
    Log { count: usize },
    Diff,
    Commit { message: String },
    Push,
    Pull,
    Checkout { branch: String },
    CreateBranch { name: String },
    Stash { message: String },
    StashPop,
    StashList,
    BranchList,
    CurrentBranch,
    StageFile { path: String },
    StageAll,
    WorktreeList,
    WorktreeAdd { branch: String, base_branch: String },
    WorktreeRemove { path: String, force: bool },
    WorktreePrune,
    Shutdown,
}

#[derive(Debug, Clone)]
pub enum GitResponse {
    Status(Vec<StatusEntry>),
    Log(Vec<CommitInfo>),
    Diff(Vec<DiffHunk>),
    CommitDone(Result<String, String>),
    PushDone(Result<(), String>),
    PullDone(Result<(), String>),
    CheckoutDone(Result<(), String>),
    BranchCreated(Result<(), String>),
    Branches(Vec<BranchInfo>),
    Stashes(Vec<StashEntry>),
    StashDone(Result<(), String>),
    StashPopDone(Result<(), String>),
    CurrentBranch(String),
    StageDone(Result<(), String>),
    Worktrees(Vec<WorktreeInfo>),
    WorktreeAdded(Result<String, String>),
    WorktreeRemoved(Result<(), String>),
    WorktreePruned(Result<(), String>),
    Error(String),
}

pub struct GitRepository {
    request_tx: Sender<GitRequest>,
    pub response_rx: mpsc::UnboundedReceiver<GitResponse>,
    _worker: thread::JoinHandle<()>,
    pub current_branch: String,
}

impl GitRepository {
    pub fn open(path: &Path) -> Result<Self, GitError> {
        let repo = Repository::discover(path).map_err(|_| GitError::NotARepo)?;
        let repo_path = repo
            .workdir()
            .unwrap_or_else(|| repo.path())
            .to_path_buf();

        let current_branch = repo
            .head()
            .ok()
            .and_then(|h| h.shorthand().map(String::from))
            .unwrap_or_else(|| "HEAD".to_string());

        let (req_tx, req_rx): (Sender<GitRequest>, Receiver<GitRequest>) =
            crossbeam_channel::unbounded();
        let (resp_tx, resp_rx) = mpsc::unbounded_channel();

        let worker = thread::spawn(move || {
            git_worker(repo_path, req_rx, resp_tx);
        });

        Ok(Self {
            request_tx: req_tx,
            response_rx: resp_rx,
            _worker: worker,
            current_branch,
        })
    }

    pub fn send(&self, req: GitRequest) {
        let _ = self.request_tx.send(req);
    }

    pub fn request_status(&self) {
        self.send(GitRequest::Status);
    }

    pub fn request_log(&self, count: usize) {
        self.send(GitRequest::Log { count });
    }

    pub fn request_diff(&self) {
        self.send(GitRequest::Diff);
    }

    pub fn request_commit(&self, message: String) {
        self.send(GitRequest::Commit { message });
    }

    pub fn request_push(&self) {
        self.send(GitRequest::Push);
    }

    pub fn request_branches(&self) {
        self.send(GitRequest::BranchList);
    }

    pub fn request_checkout(&self, branch: String) {
        self.send(GitRequest::Checkout { branch });
    }

    pub fn request_stash(&self, message: String) {
        self.send(GitRequest::Stash { message });
    }

    pub fn request_stash_pop(&self) {
        self.send(GitRequest::StashPop);
    }

    pub fn request_stash_list(&self) {
        self.send(GitRequest::StashList);
    }

    pub fn request_stage_file(&self, path: String) {
        self.send(GitRequest::StageFile { path });
    }

    pub fn request_stage_all(&self) {
        self.send(GitRequest::StageAll);
    }

    pub fn request_current_branch(&self) {
        self.send(GitRequest::CurrentBranch);
    }

    pub fn request_worktree_list(&self) {
        self.send(GitRequest::WorktreeList);
    }

    pub fn request_worktree_add(&self, branch: String, base_branch: String) {
        self.send(GitRequest::WorktreeAdd { branch, base_branch });
    }

    pub fn request_worktree_remove(&self, path: String, force: bool) {
        self.send(GitRequest::WorktreeRemove { path, force });
    }

    pub fn request_worktree_prune(&self) {
        self.send(GitRequest::WorktreePrune);
    }

    pub fn shutdown(&self) {
        let _ = self.request_tx.send(GitRequest::Shutdown);
    }
}

impl Drop for GitRepository {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn git_worker(
    repo_path: PathBuf,
    req_rx: Receiver<GitRequest>,
    resp_tx: mpsc::UnboundedSender<GitResponse>,
) {
    let mut repo = match Repository::open(&repo_path) {
        Ok(r) => r,
        Err(e) => {
            let _ = resp_tx.send(GitResponse::Error(format!("Failed to open repo: {}", e)));
            return;
        }
    };

    while let Ok(req) = req_rx.recv() {
        let response = match req {
            GitRequest::Shutdown => break,
            GitRequest::Status => handle_status(&repo),
            GitRequest::Log { count } => handle_log(&repo, count),
            GitRequest::Diff => handle_diff(&repo),
            GitRequest::Commit { message } => handle_commit(&repo, &message),
            GitRequest::Push => handle_push(&repo),
            GitRequest::Pull => GitResponse::PullDone(Err("Pull not yet implemented".into())),
            GitRequest::Checkout { branch } => handle_checkout(&repo, &branch),
            GitRequest::CreateBranch { name } => handle_create_branch(&repo, &name),
            GitRequest::Stash { message } => handle_stash(&mut repo, &message),
            GitRequest::StashPop => handle_stash_pop(&mut repo),
            GitRequest::StashList => handle_stash_list(&mut repo),
            GitRequest::BranchList => handle_branch_list(&repo),
            GitRequest::CurrentBranch => handle_current_branch(&repo),
            GitRequest::StageFile { path } => handle_stage_file(&repo, &path),
            GitRequest::StageAll => handle_stage_all(&repo),
            GitRequest::WorktreeList => handle_worktree_list(&repo_path),
            GitRequest::WorktreeAdd { branch, base_branch } => handle_worktree_add(&repo_path, &branch, &base_branch),
            GitRequest::WorktreeRemove { path, force } => handle_worktree_remove(&repo_path, &path, force),
            GitRequest::WorktreePrune => handle_worktree_prune(&repo_path),
        };
        if resp_tx.send(response).is_err() {
            break;
        }
    }
}

fn handle_status(repo: &Repository) -> GitResponse {
    let mut opts = git2::StatusOptions::new();
    opts.include_untracked(true)
        .recurse_untracked_dirs(true)
        .exclude_submodules(true)
        .include_ignored(false);

    match repo.statuses(Some(&mut opts)) {
        Ok(statuses) => {
            let entries: Vec<StatusEntry> = statuses
                .iter()
                .filter(|s| !s.status().is_ignored())
                .map(|s| {
                    let kind = if s.status().is_wt_new() || s.status().is_index_new() {
                        StatusKind::New
                    } else if s.status().is_wt_modified() || s.status().is_index_modified() {
                        StatusKind::Modified
                    } else if s.status().is_wt_deleted() || s.status().is_index_deleted() {
                        StatusKind::Deleted
                    } else if s.status().is_wt_renamed() || s.status().is_index_renamed() {
                        StatusKind::Renamed
                    } else if s.status().is_conflicted() {
                        StatusKind::Conflicted
                    } else {
                        StatusKind::Untracked
                    };
                    StatusEntry {
                        path: s.path().unwrap_or("").to_string(),
                        kind,
                    }
                })
                .collect();
            GitResponse::Status(entries)
        }
        Err(e) => GitResponse::Error(e.to_string()),
    }
}

fn handle_log(repo: &Repository, count: usize) -> GitResponse {
    let mut revwalk = match repo.revwalk() {
        Ok(r) => r,
        Err(e) => return GitResponse::Error(e.to_string()),
    };
    if revwalk.push_head().is_err() {
        return GitResponse::Log(vec![]);
    }

    let commits: Vec<CommitInfo> = revwalk
        .take(count)
        .filter_map(|oid| oid.ok())
        .filter_map(|oid| repo.find_commit(oid).ok())
        .map(|commit| {
            let id = commit.id().to_string();
            let short_id = id[..7.min(id.len())].to_string();
            CommitInfo {
                id,
                short_id,
                message: commit.message().unwrap_or("").trim().to_string(),
                author: commit.author().name().unwrap_or("").to_string(),
                date: {
                    let time = commit.time();
                    let secs = time.seconds();
                    format_timestamp(secs)
                },
            }
        })
        .collect();
    GitResponse::Log(commits)
}

fn handle_diff(repo: &Repository) -> GitResponse {
    let diff = match repo.diff_index_to_workdir(None, None) {
        Ok(d) => d,
        Err(e) => return GitResponse::Error(e.to_string()),
    };

    let mut hunks: Vec<DiffHunk> = Vec::new();

    let _ = diff.print(git2::DiffFormat::Patch, |_delta, maybe_hunk, line| {
        if let Some(hunk) = maybe_hunk {
            let header = String::from_utf8_lossy(hunk.header()).to_string();
            // Check if we already have this hunk
            if hunks.last().map(|h| h.header != header).unwrap_or(true) {
                hunks.push(DiffHunk {
                    header,
                    lines: Vec::new(),
                });
            }
        }

        let kind = match line.origin() {
            '+' => DiffLineKind::Addition,
            '-' => DiffLineKind::Deletion,
            'H' => DiffLineKind::Header,
            _ => DiffLineKind::Context,
        };
        let content = String::from_utf8_lossy(line.content()).to_string();

        if let Some(last_hunk) = hunks.last_mut() {
            last_hunk.lines.push(DiffLine { kind, content });
        } else {
            hunks.push(DiffHunk {
                header: String::new(),
                lines: vec![DiffLine { kind, content }],
            });
        }
        true
    });

    GitResponse::Diff(hunks)
}

fn handle_commit(repo: &Repository, message: &str) -> GitResponse {
    let result = (|| -> Result<String, git2::Error> {
        let sig = repo.signature().or_else(|_| {
            Signature::now("k-code user", "user@k-code.local")
        })?;
        let mut index = repo.index()?;
        let tree_oid = index.write_tree()?;
        let tree = repo.find_tree(tree_oid)?;

        let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();

        let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;
        Ok(oid.to_string()[..7].to_string())
    })();

    match result {
        Ok(id) => GitResponse::CommitDone(Ok(id)),
        Err(e) => GitResponse::CommitDone(Err(e.to_string())),
    }
}

fn handle_push(repo: &Repository) -> GitResponse {
    let result = (|| -> Result<(), git2::Error> {
        let mut remote = repo.find_remote("origin")?;
        let head = repo.head()?;
        let refspec = head.name().unwrap_or("refs/heads/main");
        remote.push(&[refspec], None)?;
        Ok(())
    })();
    match result {
        Ok(()) => GitResponse::PushDone(Ok(())),
        Err(e) => GitResponse::PushDone(Err(e.to_string())),
    }
}

fn handle_checkout(repo: &Repository, branch: &str) -> GitResponse {
    let result = (|| -> Result<(), git2::Error> {
        let (object, reference) = repo.revparse_ext(branch)?;
        repo.checkout_tree(&object, None)?;
        match reference {
            Some(gref) => repo.set_head(gref.name().unwrap_or(""))?,
            None => repo.set_head_detached(object.id())?,
        }
        Ok(())
    })();
    match result {
        Ok(()) => GitResponse::CheckoutDone(Ok(())),
        Err(e) => GitResponse::CheckoutDone(Err(e.to_string())),
    }
}

fn handle_create_branch(repo: &Repository, name: &str) -> GitResponse {
    let result = (|| -> Result<(), git2::Error> {
        let head = repo.head()?.peel_to_commit()?;
        repo.branch(name, &head, false)?;
        Ok(())
    })();
    match result {
        Ok(()) => GitResponse::BranchCreated(Ok(())),
        Err(e) => GitResponse::BranchCreated(Err(e.to_string())),
    }
}

fn handle_stash(repo: &mut Repository, message: &str) -> GitResponse {
    let result = (|| -> Result<(), git2::Error> {
        let sig = repo.signature().or_else(|_| {
            Signature::now("k-code user", "user@k-code.local")
        })?;
        let msg = if message.is_empty() {
            "WIP"
        } else {
            message
        };
        repo.stash_save(&sig, msg, None)?;
        Ok(())
    })();
    match result {
        Ok(()) => GitResponse::StashDone(Ok(())),
        Err(e) => GitResponse::StashDone(Err(e.to_string())),
    }
}

fn handle_stash_pop(repo: &mut Repository) -> GitResponse {
    let result = repo.stash_pop(0, None);
    match result {
        Ok(()) => GitResponse::StashPopDone(Ok(())),
        Err(e) => GitResponse::StashPopDone(Err(e.to_string())),
    }
}

fn handle_stash_list(repo: &mut Repository) -> GitResponse {
    let mut stashes = Vec::new();
    let _ = repo.stash_foreach(|index, message, _oid| {
        stashes.push(StashEntry {
            index,
            message: message.to_string(),
        });
        true
    });
    GitResponse::Stashes(stashes)
}

fn handle_branch_list(repo: &Repository) -> GitResponse {
    match repo.branches(None) {
        Ok(branches) => {
            let list: Vec<BranchInfo> = branches
                .filter_map(|b| b.ok())
                .map(|(branch, branch_type)| {
                    let name = branch.name().ok().flatten().unwrap_or("").to_string();
                    BranchInfo {
                        name,
                        is_current: branch.is_head(),
                        is_remote: branch_type == git2::BranchType::Remote,
                    }
                })
                .collect();
            GitResponse::Branches(list)
        }
        Err(e) => GitResponse::Error(e.to_string()),
    }
}

fn handle_current_branch(repo: &Repository) -> GitResponse {
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(String::from))
        .unwrap_or_else(|| "HEAD".to_string());
    GitResponse::CurrentBranch(branch)
}

fn handle_stage_file(repo: &Repository, path: &str) -> GitResponse {
    let result = (|| -> Result<(), git2::Error> {
        let mut index = repo.index()?;
        index.add_path(Path::new(path))?;
        index.write()?;
        Ok(())
    })();
    match result {
        Ok(()) => GitResponse::StageDone(Ok(())),
        Err(e) => GitResponse::StageDone(Err(e.to_string())),
    }
}

fn handle_stage_all(repo: &Repository) -> GitResponse {
    let result = (|| -> Result<(), git2::Error> {
        let mut index = repo.index()?;
        index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)?;
        index.write()?;
        Ok(())
    })();
    match result {
        Ok(()) => GitResponse::StageDone(Ok(())),
        Err(e) => GitResponse::StageDone(Err(e.to_string())),
    }
}

fn handle_worktree_list(repo_path: &Path) -> GitResponse {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_path)
        .output();

    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => return GitResponse::Error(String::from_utf8_lossy(&o.stderr).to_string()),
        Err(e) => return GitResponse::Error(e.to_string()),
    };

    let mut worktrees = Vec::new();
    let mut current_path = String::new();
    let mut current_branch = String::new();
    let mut is_bare = false;
    let repo_path_str = repo_path.to_string_lossy().to_string();

    for line in output.lines() {
        if line.starts_with("worktree ") {
            if !current_path.is_empty() {
                let is_main = current_path == repo_path_str;
                let (status_count, commits_ahead) = worktree_details(&current_path);
                let name = Path::new(&current_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| current_path.clone());
                worktrees.push(WorktreeInfo {
                    name,
                    path: current_path.clone(),
                    branch: current_branch.clone(),
                    is_main,
                    status_count,
                    commits_ahead,
                });
            }
            current_path = line[9..].to_string();
            current_branch.clear();
            is_bare = false;
        } else if line.starts_with("branch ") {
            current_branch = line[7..].split('/').last().unwrap_or("").to_string();
        } else if line == "bare" {
            is_bare = true;
        }
    }

    // Push last entry
    if !current_path.is_empty() && !is_bare {
        let is_main = current_path == repo_path_str;
        let (status_count, commits_ahead) = worktree_details(&current_path);
        let name = Path::new(&current_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| current_path.clone());
        worktrees.push(WorktreeInfo {
            name,
            path: current_path,
            branch: current_branch,
            is_main,
            status_count,
            commits_ahead,
        });
    }

    GitResponse::Worktrees(worktrees)
}

fn worktree_details(worktree_path: &str) -> (usize, usize) {
    let status_count = Command::new("git")
        .args(["status", "-s"])
        .current_dir(worktree_path)
        .output()
        .ok()
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter(|l| !l.is_empty())
                .count()
        })
        .unwrap_or(0);

    let commits_ahead = Command::new("git")
        .args(["rev-list", "--count", "@{u}..HEAD"])
        .current_dir(worktree_path)
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8_lossy(&o.stdout)
                    .trim()
                    .parse::<usize>()
                    .ok()
            } else {
                None
            }
        })
        .unwrap_or(0);

    (status_count, commits_ahead)
}

fn handle_worktree_add(repo_path: &Path, branch: &str, base_branch: &str) -> GitResponse {
    let repo_name = repo_path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();

    let parent_dir = repo_path.parent().unwrap_or(repo_path);
    let worktree_dir = parent_dir.join(format!("{}-{}", repo_name, branch));
    let worktree_path = worktree_dir.to_string_lossy().to_string();

    // Check if branch already exists
    let branch_exists = Command::new("git")
        .args(["show-ref", "--verify", "--quiet", &format!("refs/heads/{}", branch)])
        .current_dir(repo_path)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let output = if branch_exists {
        Command::new("git")
            .args(["worktree", "add", &worktree_path, branch])
            .current_dir(repo_path)
            .output()
    } else {
        Command::new("git")
            .args(["worktree", "add", &worktree_path, "-b", branch, base_branch])
            .current_dir(repo_path)
            .output()
    };

    match output {
        Ok(o) if o.status.success() => {
            // Find the main worktree root to copy config files from.
            // repo_path might itself be a worktree, so use git to find the real root.
            let main_root = Command::new("git")
                .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
                .current_dir(repo_path)
                .output()
                .ok()
                .filter(|o| o.status.success())
                .and_then(|o| {
                    let git_common = String::from_utf8_lossy(&o.stdout).trim().to_string();
                    // git-common-dir returns the .git dir; parent is the worktree root
                    Path::new(&git_common).parent().map(|p| p.to_path_buf())
                })
                .unwrap_or_else(|| repo_path.to_path_buf());

            // Copy untracked config files that tools depend on (e.g. .tool-versions, .node-version)
            for file in &[".tool-versions", ".node-version", ".ruby-version", ".nvmrc"] {
                let src = main_root.join(file);
                if src.exists() {
                    let _ = std::fs::copy(&src, worktree_dir.join(file));
                }
            }
            GitResponse::WorktreeAdded(Ok(worktree_path))
        }
        Ok(o) => GitResponse::WorktreeAdded(Err(String::from_utf8_lossy(&o.stderr).trim().to_string())),
        Err(e) => GitResponse::WorktreeAdded(Err(e.to_string())),
    }
}

fn handle_worktree_remove(repo_path: &Path, path: &str, force: bool) -> GitResponse {
    let mut args = vec!["worktree", "remove", path];
    if force {
        args.push("--force");
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(repo_path)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            // Try to delete the branch too
            let branch = Path::new(path)
                .file_name()
                .and_then(|n| n.to_str())
                .and_then(|name| {
                    let repo_name = repo_path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("");
                    name.strip_prefix(&format!("{}-", repo_name))
                })
                .map(|s| s.to_string());

            if let Some(branch) = branch {
                let _ = Command::new("git")
                    .args(["branch", "-D", &branch])
                    .current_dir(repo_path)
                    .output();
            }

            // Ensure the worktree directory is fully removed from disk
            let wt_path = Path::new(path);
            if wt_path.exists() {
                let _ = std::fs::remove_dir_all(wt_path);
            }

            GitResponse::WorktreeRemoved(Ok(()))
        }
        Ok(o) => GitResponse::WorktreeRemoved(Err(String::from_utf8_lossy(&o.stderr).trim().to_string())),
        Err(e) => GitResponse::WorktreeRemoved(Err(e.to_string())),
    }
}

fn handle_worktree_prune(repo_path: &Path) -> GitResponse {
    let output = Command::new("git")
        .args(["worktree", "prune"])
        .current_dir(repo_path)
        .output();

    match output {
        Ok(o) if o.status.success() => GitResponse::WorktreePruned(Ok(())),
        Ok(o) => GitResponse::WorktreePruned(Err(String::from_utf8_lossy(&o.stderr).trim().to_string())),
        Err(e) => GitResponse::WorktreePruned(Err(e.to_string())),
    }
}

fn format_timestamp(secs: i64) -> String {
    let days = secs / 86400;
    let remaining = secs % 86400;
    let hours = remaining / 3600;
    let minutes = (remaining % 3600) / 60;

    let year = 1970 + (days / 365);
    let day_of_year = days % 365;
    let month = day_of_year / 30 + 1;
    let day = day_of_year % 30 + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year, month, day, hours, minutes
    )
}
