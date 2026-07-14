//! Arc Studio IDE 代码编辑器
//!
//! 此 crate 实现:
//! - [`editor_workbench`] —— `EditorWorkbench`(IWorkbench + RML 组件，代码编辑 + LSP 集成)
//! - [`editor_provider`] —— `EditorProvider`(IWorkbenchProvider，schema="file")
//!
//! 服务自注册: `#[ctor::ctor]` 自动注册 `EditorProvider` 为 `IWorkbenchProvider("file")`，
//! 并注册 `EditorWorkbench` 的能力 cast（IContribution + IVisual + IWorkbench）。

extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;

#[path = "editor_workbench.rml.rs"]
pub mod editor_workbench;
pub mod editor_provider;

/// 自动注册 —— `#[ctor::ctor]` 在 `main` 之前执行:
/// 1. 注册能力 cast（IContribution + IVisual + IWorkbench）
/// 2. 注册 `EditorProvider` 为 keyed `IWorkbenchProvider("file")`
#[rml_core::ctor::ctor]
fn register_editor_services() {
    use std::sync::Arc;
    use rml_core::workbench::IWorkbenchProvider;
    use rust_rml_di::{auto_register, ServiceCollection};

    crate::editor_workbench::register_editor_abilities();
    auto_register(|s: &mut ServiceCollection| {
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", |_| {
            Arc::new(crate::editor_provider::EditorProvider) as Arc<dyn IWorkbenchProvider>
        });
    });
}