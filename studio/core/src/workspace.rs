//! IDE 工作空间管理契约。

use std::path::Path;
use std::sync::Arc;

use crate::worktree::IWorktree;

/// IDE 工作空间 —— 一个已注册的根目录。
///
/// 继承 `IWorktree`(空标记 trait)—— 工作空间即是一个 worktree,
/// 额外标记"此 worktree 已被 IDE 注册为工作空间"。
///
/// # 应用场景
///
/// 用户在资源管理器中"打开文件夹" → 创建 `LocalWorktree` → 注册为 `IWorkspace` →
/// `ExplorerPanel` 经 `IWorkspaceManager::list()` 渲染多根文件树。
/// 每个根目录独立显示为文件树顶级节点(VSCode 多根工作空间模式)。
pub trait IWorkspace: IWorktree {}

/// 工作空间管理器 —— 管理多个已打开的工作空间。
///
/// 镜像 `IWorkbenchManager` 模式。`StudioShellManager` 直接 impl 此 trait。
/// 注册为 DI singleton,经 `provider.get::<dyn IWorkspaceManager>()` 解析。
pub trait IWorkspaceManager: Send + Sync + 'static {
    /// 添加工作空间;若根路径已存在则忽略。
    fn add(&self, workspace: Arc<dyn IWorkspace>);

    /// 移除工作空间(按根路径)。
    fn remove(&self, root: &Path);

    /// 当前所有工作空间。
    fn list(&self) -> Vec<Arc<dyn IWorkspace>>;

    /// 按根路径获取工作空间。
    fn get(&self, root: &Path) -> Option<Arc<dyn IWorkspace>>;
}
