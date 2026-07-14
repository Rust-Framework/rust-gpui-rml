//! CodeWorkbench —— 默认代码编辑视图组件(IWorkbenchComponent)。
//!
//! 注册为全局 `IWorkbenchComponent`,`matches(uri)` 默认返回 `true`(所有资源均可用代码视图)。
//! 提供视图切换按钮所需的元数据(id/name/icon)。
//!
//! # 当前阶段(Phase 1)
//!
//! 实际代码编辑由 `EditorWorkbench` 的 RML 模板直接渲染(`<CodeEditor>`)。
//! 此结构的 `render()` 为占位实现 —— 当 EditorWorkbench 检测到仅有 1 个匹配组件时
//! 不显示切换按钮,直接渲染内置 CodeEditor。
//!
//! # 后续阶段(Phase 2)
//!
//! 将代码编辑逻辑(InputState/LSP/文件读取)从 `EditorWorkbench` 提取到此组件,
//! `EditorWorkbench` 变为纯壳(Header + Body),Body 经 `IWorkbenchComponent::render()`
//! 动态渲染选中组件。需 RML 框架支持参数化模板(当前已知限制)。

use std::sync::Arc;

use gpui::{div, AnyElement, App, IntoElement, SharedString, Window};
use rml_core::contribution::{IContribution, IconSpec, IVisual};
use studio_core::ability_ext::register_workbench_component_ability;
use studio_core::component::IWorkbenchComponent;
use studio_core::register_workbench_component;

// 注意:此文件为 Phase 1 占位实现,Phase 3 将重写为 CodeComponent
// (改名 code_workbench.rs → code_component.rml.rs,改造为 #[component] + .rml 模板)。

/// 默认代码编辑视图组件。
///
/// `matches(uri)` 使用默认实现(返回 `true`)—— 所有资源均可使用代码视图。
pub struct CodeWorkbench;

impl IContribution for CodeWorkbench {
    fn id(&self) -> &str {
        "code"
    }
    fn name(&self) -> SharedString {
        "Code".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("FileCode"))
    }
}

impl IVisual for CodeWorkbench {
    fn render(&self, _window: &mut Window, _cx: &mut App) -> AnyElement {
        // Phase 1 占位:实际代码编辑由 EditorWorkbench 模板渲染。
        div().into_any_element()
    }
}

impl IWorkbenchComponent for CodeWorkbench {
    // matches(uri) 使用默认实现(返回 true)—— CodeWorkbench 是默认文本视图
}

/// 注册 CodeWorkbench 能力 cast + 工厂。
///
/// 在 `#[ctor::ctor]` 中调用:
/// 1. `register_workbench_component_ability::<CodeWorkbench>()` —— 注册能力 cast
/// 2. `register_workbench_component(factory)` —— 注册工厂到全局注册表
pub fn register_code_workbench() {
    register_workbench_component_ability::<CodeWorkbench>();
    register_workbench_component(|| {
        Arc::new(CodeWorkbench) as Arc<dyn IWorkbenchComponent>
    });
}
