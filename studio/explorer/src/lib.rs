//! Arc Studio IDE 文件资源管理器。
//!
//! 此 crate 实现:
//! - [`git_worktree`] —— `GitWorktree`(`IWorktree` + `IWorkspace` 的 git worktree 实现)
//! - [`explorer_panel`] —— `ExplorerPanel`(`#[component]` 文件树面板,`IContribution` + `IVisual`)
//!
//! 自注册: `#[ctor::ctor]` 在 `main` 之前执行,
//! 注册 `ExplorerPanel` 的能力 cast + ActivityBar 面板工厂 + GitWorktree 工作空间 opener。

extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;
extern crate studio_core as studio_core;

pub mod git_worktree;
#[path = "explorer_panel.rml.rs"]
pub mod explorer_panel;

/// 自动注册 —— `#[ctor::ctor]` 在 `main` 之前执行:
/// 1. 注册能力 cast（IContribution + IVisual）
/// 2. 注册 `ExplorerPanel` 为 ActivityBar 面板工厂
/// 3. 注册 `GitWorktree` 为工作空间 opener
#[rml_core::ctor::ctor]
fn register_explorer_services() {
    use std::sync::Arc;
    use rml_core::contribution::IContribution;
    use rml_ui::register_activity_panel;
    use studio_core::workspace::IWorkspace;
    use studio_core::register_workspace_opener;

    crate::explorer_panel::register_explorer_abilities();
    register_activity_panel(|| {
        Arc::new(crate::explorer_panel::ExplorerPanel::default()) as Arc<dyn IContribution>
    });
    register_workspace_opener(|path| {
        crate::git_worktree::GitWorktree::open(path.to_path_buf())
            .ok()
            .map(|wt| Arc::new(wt) as Arc<dyn IWorkspace>)
    });
}
