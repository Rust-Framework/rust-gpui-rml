//! Arc Studio IDE 核心契约
//!
//! 此 crate 定义 Arc Studio IDE 的核心抽象:
//! - [`component::IWorkbenchComponent`] —— 工作台内部呈现组件贡献点(编辑/预览/设计多态)
//! - [`component::TextPosition`] / [`component::TextSpan`] —— 文本位置(点)与文本块(范围)
//! - [`command::IEditorCommand`] —— 编辑器命令,扩展 `ICommand` 添加 `gesture()`
//! - [`worktree::IWorktree`] —— 文件系统抽象,AI 并行编程依赖此能力
//! - [`workspace::IWorkspace`] / [`workspace::IWorkspaceManager`] —— IDE 工作空间管理
//! - [`registry`] —— 工作空间 opener 注册表(`#[ctor::ctor]` 自注册模式)
//!
//! 所有 `*AbilityExt` trait 与 `register_*_ability` 函数集中定义在 [`ability_ext`],
//! 避免分散到多个模块。

// 包名统一为 rust-rml-* 前缀,通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_core as rml_core;

pub mod ability_ext;
pub mod command;
pub mod component;
pub mod registry;
pub mod worktree;
pub mod workspace;

pub use registry::{
    get_workbench_components, open_workspace, register_workbench_component,
    register_workspace_opener,
};
