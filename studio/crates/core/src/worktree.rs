//! 工作区文件系统抽象契约。

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;

use gpui::SharedString;
use rml_core::contribution::IContribution;
use rml_core::workbench::Uri;

/// 文件树条目类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

/// 文件树条目。
#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// 相对于工作区根的路径。
    pub path: PathBuf,
    pub kind: EntryKind,
    pub name: SharedString,
}

/// 文件元数据。
#[derive(Debug, Clone)]
pub struct WorktreeStat {
    pub kind: EntryKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// 文件变更事件。
#[derive(Debug, Clone)]
pub enum WorktreeChange {
    Created(PathBuf),
    Modified(PathBuf),
    Removed(PathBuf),
    Renamed { from: PathBuf, to: PathBuf },
}

/// 文件系统抽象 —— 文件枚举、路径解析、读写、元数据、变更监听。
///
/// 继承 `IContribution`(`id`/`name` 标识 fs 类型)。
/// 一个 `IWorkspace` 拥有一个 `IWorktree` 实例。
///
/// # 职责清单(AI 并行编程依赖)
///
/// | 方法 | UI 场景 | AI 场景 |
/// |------|---------|---------|
/// | `root()` | 文件树根节点 | 工作空间定位 |
/// | `entries(dir)` | ExplorerPanel 渲染文件树 | AI 扫描代码库结构 |
/// | `resolve(path)` | 点击文件 → Uri → 打开 Tab | AI 引用文件 |
/// | `relativize(uri)` | Tab 标题显示相对路径 | AI 输出相对路径 |
/// | `read(path)` | EditorWorkbench 打开文件 | AI 理解代码内容 |
/// | `write(path, content)` | FormatCommand 保存 | **AI 应用代码修改** |
/// | `stat(path)` | ExplorerPanel 显示大小/时间 | AI 检查文件大小 |
/// | `watch(on_change)` | ExplorerPanel 自动刷新 | AI 检测外部修改 |
pub trait IWorktree: IContribution {
    /// 工作区根路径。
    fn root(&self) -> &Path;

    /// 枚举指定目录下的条目(None = 根目录)。
    fn entries(&self, dir: Option<&Path>) -> Vec<WorktreeEntry>;

    /// 将相对路径解析为 Uri。
    fn resolve(&self, path: &Path) -> Uri;

    /// 将 Uri 还原为相对路径(`resolve` 的逆运算)。
    fn relativize(&self, uri: &Uri) -> Option<PathBuf>;

    /// 读取文件内容。
    fn read(&self, path: &Path) -> std::io::Result<String>;

    /// 写入文件内容(AI 应用修改、FormatCommand 保存)。
    fn write(&self, path: &Path, content: &str) -> std::io::Result<()>;

    /// 获取文件元数据。
    fn stat(&self, path: &Path) -> std::io::Result<WorktreeStat>;

    /// 订阅文件变更。返回取消订阅句柄(调用即停止监听)。
    fn watch(&self, on_change: Arc<dyn Fn(&WorktreeChange) + Send + Sync>) -> Box<dyn FnOnce()>;
}
