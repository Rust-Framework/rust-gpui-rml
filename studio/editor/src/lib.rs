//! Arc Studio IDE 代码编辑器
//!
//! 此 crate 实现:
//! - [`editor_workbench`] —— `EditorWorkbench`(IWorkbench + IWorkbenchComponentHost,RML 组件,纯壳)
//! - [`editor_provider`] —— `EditorProvider`(IWorkbenchProvider，schema="file")
//! - [`code_component`] —— `CodeComponent`(IWorkbenchComponent，默认代码视图,接管 InputState + LSP)
//! - [`preview_component`] —— `PreviewComponent`(IWorkbenchComponent,只读预览,匹配 .md/.markdown/.html)
//!
//! 服务自注册: `#[ctor::ctor]` 自动注册 `EditorProvider` 为 `IWorkbenchProvider("file")`，
//! 并注册 `EditorWorkbench` 的能力 cast（IContribution + IVisual + IWorkbench + IWorkbenchComponentHost），
//! 以及 `CodeComponent` / `PreviewComponent` 为 IWorkbenchComponent。

extern crate rust_rml_engine as rml;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_ui as rml_ui;

#[path = "editor_workbench.rml.rs"]
pub mod editor_workbench;
#[path = "code_component.rml.rs"]
pub mod code_component;
#[path = "preview_component.rml.rs"]
pub mod preview_component;
pub mod editor_provider;

/// 自动注册 —— `#[ctor::ctor]` 在 `main` 之前执行:
/// 1. 注册 EditorWorkbench 能力 cast(IContribution + IVisual + IWorkbench + IWorkbenchComponentHost)
/// 2. 注册 CodeComponent 能力 cast + 工厂(默认 IWorkbenchComponent)
/// 3. 注册 PreviewComponent 能力 cast + 工厂(只读预览 IWorkbenchComponent)
/// 4. 注册 `EditorProvider` 为 keyed `IWorkbenchProvider("file")`
#[rml_core::ctor::ctor]
fn register_editor_services() {
    use std::sync::Arc;
    use rml_core::workbench::IWorkbenchProvider;
    use studio_core::di::{auto_register, ServiceCollection, ServiceCollectionExt};

    crate::editor_workbench::register_editor_abilities();
    crate::code_component::register_code_component();
    crate::preview_component::register_preview_component();
    auto_register(|s: &mut ServiceCollection| {
        s.add_keyed_singleton::<dyn IWorkbenchProvider>("file", || {
            Arc::new(crate::editor_provider::EditorProvider) as Arc<dyn IWorkbenchProvider>
        });
    });
}
