//! Arc Studio IDE 文件资源管理器。
//!
//! 此 crate 实现:
//! - [`git_worktree`] —— `GitWorktree`(`IWorktree` + `IWorkspace` 的 git worktree 实现)
//! - [`explorer_panel`] —— `ExplorerPanel`(`#[component]` 文件树面板,`IContribution` + `IVisual`)

extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;
extern crate studio_core as studio_core;

pub mod git_worktree;
#[path = "explorer_panel.rml.rs"]
pub mod explorer_panel;
