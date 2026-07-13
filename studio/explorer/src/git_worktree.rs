//! GitWorktree —— git worktree 的 `IWorktree` + `IWorkspace` 实现。
//!
//! `IWorktree` trait 本身就是对 git worktree 的抽象,本模块是其唯一具体实现。
//! 文件操作经 `std::fs`,git 操作(branch/status/create_worktree)经 `git2`。
//!
//! # 核心价值
//!
//! git worktree 允许同一仓库的多个工作树并行存在,AI/人类可在不同分支上
//! 互不干扰地工作——"平行宇宙"开发模式。`create_worktree` 为此提供基础设施。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use gpui::SharedString;
use rml_core::contribution::{IContribution, IconSpec};
use rml_core::workbench::Uri;
use studio_core::worktree::{EntryKind, FileStatus, IWorktree, WorktreeChange, WorktreeEntry, WorktreeStat};
use studio_core::workspace::IWorkspace;

/// git worktree 实现 —— `IWorktree` + `IWorkspace` 的唯一具体类型。
///
/// `repo` 使用 `Mutex` 包裹(`git2::Repository` 是 `Send` 但非 `Sync`),
/// 使 `GitWorktree` 满足 `Send + Sync`(IContribution: IValue 的要求)。
///
/// 文件操作(entries/read/write/stat)直接走 `std::fs`(worktree root 即普通目录),
/// git 操作(branch/file_status/create_worktree)走 `git2::Repository`。
pub struct GitWorktree {
    root: PathBuf,
    repo: Mutex<git2::Repository>,
}

/// git worktree 操作错误。
#[derive(Debug)]
pub enum WorktreeError {
    Git(String),
    Io(std::io::Error),
}

impl From<git2::Error> for WorktreeError {
    fn from(e: git2::Error) -> Self {
        WorktreeError::Git(e.message().to_string())
    }
}

impl From<std::io::Error> for WorktreeError {
    fn from(e: std::io::Error) -> Self {
        WorktreeError::Io(e)
    }
}

impl std::fmt::Display for WorktreeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorktreeError::Git(s) => write!(f, "git error: {s}"),
            WorktreeError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for WorktreeError {}

impl GitWorktree {
    /// 打开已存在的 git worktree。
    ///
    /// `root` 必须是 git worktree 的根目录(包含 `.git` 文件或目录)。
    /// 非 git 目录返回错误。
    pub fn open(root: PathBuf) -> Result<Self, WorktreeError> {
        let repo = git2::Repository::open(&root)?;
        Ok(Self {
            root,
            repo: Mutex::new(repo),
        })
    }

    /// 创建新 worktree —— 为 AI/人类创建隔离的"平行宇宙"。
    ///
    /// 在当前 worktree 所属仓库中,于 `path` 创建新工作树并检出 `branch` 分支。
    /// 若 `branch` 已存在则返回错误(用 `open` 打开已有 worktree)。
    pub fn create_worktree(&self, path: &Path, branch: &str) -> Result<Self, WorktreeError> {
        let repo = self.repo.lock().unwrap();
        let _worktree = repo.worktree(branch, path, None)?;
        drop(repo);
        Self::open(path.to_path_buf())
    }

    /// 列出同一仓库下所有 sibling worktrees(含自身)。
    ///
    /// 返回每个 worktree 的路径 + 分支名,供 ExplorerPanel 显示多根文件树。
    pub fn list_sibling_worktrees(&self) -> Vec<(PathBuf, Option<SharedString>)> {
        let repo = self.repo.lock().unwrap();
        let mut result = Vec::new();
        if let Ok(worktrees) = repo.worktrees() {
            for name in worktrees.iter().flatten() {
                if let Ok(wt) = repo.find_worktree(name) {
                    let path = wt.path().to_path_buf();
                    // 打开此 worktree 的 repo 查询分支
                    let branch = git2::Repository::open(&path)
                        .ok()
                        .and_then(|r| {
                            r.head().ok().and_then(|h| {
                                if h.is_branch() {
                                    h.shorthand().map(Into::into)
                                } else {
                                    None
                                }
                            })
                        });
                    result.push((path, branch));
                }
            }
        }
        result
    }

    fn repo(&self) -> std::sync::MutexGuard<'_, git2::Repository> {
        self.repo.lock().unwrap()
    }
}

impl IContribution for GitWorktree {
    fn id(&self) -> &str {
        // root 路径作为唯一 ID(同一仓库的不同 worktree 路径不同)
        self.root.to_str().unwrap_or("git-worktree")
    }

    fn name(&self) -> SharedString {
        self.root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("worktree")
            .into()
    }

    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("Folder"))
    }
}

impl IWorktree for GitWorktree {
    fn root(&self) -> &Path {
        &self.root
    }

    fn entries(&self, dir: Option<&Path>) -> Vec<WorktreeEntry> {
        let dir = dir
            .map(|d| self.root.join(d))
            .unwrap_or_else(|| self.root.clone());

        let Ok(rd) = std::fs::read_dir(&dir) else {
            return Vec::new();
        };

        rd.filter_map(|e| e.ok())
            .filter_map(|e| {
                let path = e.path();
                let name = path.file_name().and_then(|n| n.to_str())?;
                // 跳过 .git(worktree 根级)与常见无关隐藏目录
                if name == ".git" {
                    return None;
                }
                let kind = if path.is_symlink() {
                    EntryKind::Symlink
                } else if path.is_dir() {
                    EntryKind::Directory
                } else {
                    EntryKind::File
                };
                Some(WorktreeEntry {
                    path: path.strip_prefix(&self.root).unwrap_or(&path).to_path_buf(),
                    kind,
                    name: name.into(),
                })
            })
            .collect()
    }

    fn resolve(&self, path: &Path) -> Uri {
        url::Url::from_file_path(self.root.join(path)).unwrap_or_else(|_| {
            url::Url::parse("file:///invalid").expect("fallback URL")
        })
    }

    fn relativize(&self, uri: &Uri) -> Option<PathBuf> {
        uri.to_file_path()
            .ok()
            .and_then(|p| p.strip_prefix(&self.root).ok().map(|p| p.to_path_buf()))
    }

    fn read(&self, path: &Path) -> std::io::Result<String> {
        std::fs::read_to_string(self.root.join(path))
    }

    fn write(&self, path: &Path, content: &str) -> std::io::Result<()> {
        std::fs::write(self.root.join(path), content)
    }

    fn stat(&self, path: &Path) -> std::io::Result<WorktreeStat> {
        let meta = std::fs::metadata(self.root.join(path))?;
        Ok(WorktreeStat {
            kind: if meta.is_dir() {
                EntryKind::Directory
            } else {
                EntryKind::File
            },
            size: meta.len(),
            modified: meta.modified().ok(),
        })
    }

    fn watch(&self, _on_change: Arc<dyn Fn(&WorktreeChange) + Send + Sync>) -> Box<dyn FnOnce()> {
        // Phase 3 暂不实现文件监听(notify crate 待后续引入)
        Box::new(|| {})
    }

    fn branch(&self) -> Option<SharedString> {
        let repo = self.repo();
        let head = repo.head().ok()?;
        if head.is_branch() {
            head.shorthand().map(Into::into)
        } else {
            None // detached HEAD
        }
    }

    fn file_status(&self, path: &Path) -> FileStatus {
        let repo = self.repo();
        match repo.status_file(path) {
            Ok(status) => {
                use git2::Status;
                // 优先检查暂存区(INDEX_*),再检查工作树(WT_*)
                if status.intersects(
                    Status::INDEX_NEW
                        | Status::INDEX_MODIFIED
                        | Status::INDEX_DELETED
                        | Status::INDEX_RENAMED
                        | Status::INDEX_TYPECHANGE,
                ) {
                    FileStatus::Staged
                } else if status.contains(Status::WT_NEW) {
                    FileStatus::Untracked
                } else if status.intersects(
                    Status::WT_MODIFIED | Status::WT_DELETED | Status::WT_RENAMED | Status::WT_TYPECHANGE,
                ) {
                    FileStatus::Modified
                } else if status.contains(Status::IGNORED) {
                    FileStatus::Ignored
                } else {
                    FileStatus::Clean
                }
            }
            Err(_) => FileStatus::Clean,
        }
    }
}

impl IWorkspace for GitWorktree {}
