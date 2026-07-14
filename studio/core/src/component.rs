//! 工作台组件契约 + 文本位置/块类型 + 组件宿主契约。

use std::sync::Arc;

use gpui::{App, Entity, SharedString};
use rml_core::contribution::IVisualContribution;
use rml_core::workbench::Uri;

use crate::document::{WorkbenchDocument, WorkbenchState};

/// 工作台呈现组件 —— IWorkbench 实现内部的贡献点。
///
/// 继承 `IVisualContribution`(具备 `id`/`name`/`description`/`icon` + `render`)。
/// 此 trait 为空标记 —— 仅用于能力查询区分"此视觉贡献是工作台组件"。
///
/// # 命名规范
///
/// `IWorkbenchComponent` 实现类统一命名 `XXXComponent`(如 `CodeComponent`、
/// `PreviewComponent`、`RmlDesignComponent`),与 `IWorkbench` 实现类
/// (`XXXWorkbench`,如 `EditorWorkbench`)区分,避免混淆。
///
/// # 应用场景
///
/// `EditorWorkbench`(IWorkbench 实现 + `IWorkbenchComponentHost`)受理多个 IWorkbenchComponent:
/// - `CodeComponent`(id="code")—— 代码编辑,默认内置(matches=true)
/// - `PreviewComponent`(id="preview")—— 只读预览,仅 Markdown/HTML
/// - `RmlDesignComponent`(id="design")—— RML 可视化设计器,仅 .rml(后续计划)
///
/// 用户在组件间切换,实现编辑/预览/设计多态呈现。组件间经 `WorkbenchDocument`
/// 共享 Entity 同步数据,经 `WorkbenchState` 共享 Entity 统一状态。
///
/// # 元数据来源(无冗余)
///
/// - `IContribution::id()` → 组件标识("code"/"design"/"preview")
/// - `IContribution::name()` → 切换按钮标签
/// - `IContribution::icon()` → 切换按钮图标
/// - `IVisual::render()` → 组件视图内容
/// - `IWorkbenchComponent::matches()` → 判断是否能处理指定 URI
pub trait IWorkbenchComponent: IVisualContribution {
    /// 判断此组件是否能处理指定 URI 的资源。
    ///
    /// 工作台在渲染时查询所有已注册组件,按 `matches(uri)` 过滤。
    /// 匹配多个组件时 Header 显示视图切换按钮;仅匹配一个时直接渲染。
    ///
    /// 默认返回 `true` —— 作为默认视图组件(如 CodeComponent)。
    /// 特化组件(如 PreviewComponent 仅 .md/.html)应 override 此方法。
    fn matches(&self, _uri: &Uri) -> bool {
        true
    }
}

/// 工作台组件宿主 —— IWorkbench 实现按需 impl,统筹管理多个 IWorkbenchComponent。
///
/// 提供三大能力:
/// 1. **组件枚举与激活**:[`components`] / [`active_component_id`] / [`switch_component`]
/// 2. **共享文档访问**:[`document`] —— 组件间数据同步的媒介(单一真相源)
/// 3. **共享状态访问**:[`state`] —— 跨组件统一管理 dirty/saving 等
///
/// # 不强制所有 IWorkbench impl
///
/// 不受理子组件的工作台(如 demo 的 `CaseWorkbench`)不 impl 即可。这与
/// project_memory 硬约束一致:"IWorkbench super trait 仅含 IContribution + IVisual,
/// Host 状态由实现决定"。
///
/// # 实现示例
///
/// `EditorWorkbench` impl 此 trait,受理 `CodeComponent` / `PreviewComponent` 等。
/// 组件经 `get_or_create_entity::<EditorWorkbench>(cx)` 取 host,再读 document/state。
///
/// [`components`]: IWorkbenchComponentHost::components
/// [`active_component_id`]: IWorkbenchComponentHost::active_component_id
/// [`switch_component`]: IWorkbenchComponentHost::switch_component
/// [`document`]: IWorkbenchComponentHost::document
/// [`state`]: IWorkbenchComponentHost::state
pub trait IWorkbenchComponentHost {
    /// 此工作台受理的所有组件(经 `matches(uri)` 过滤)。
    ///
    /// 返回顺序即 Header 切换按钮的展示顺序。
    fn components(&self) -> Vec<Arc<dyn IWorkbenchComponent>>;

    /// 当前激活的组件 id。
    fn active_component_id(&self) -> SharedString;

    /// 切换激活组件。id 不在 `components()` 中时为 no-op。
    ///
    /// 实现通常经 `entity.update(cx, |this, _| { this.active_component_id = id.into() })`
    /// 更新字段,RML 模板经条件分支重新渲染 Body。
    fn switch_component(&self, id: &str, cx: &mut App);

    /// 共享文档模型 —— 组件间数据同步的媒介。
    ///
    /// 组件 observe 此 Entity,任何组件修改 `content` → 通知所有 observers。
    /// 详见 [`WorkbenchDocument`] 的同步链路说明。
    fn document(&self) -> Entity<WorkbenchDocument>;

    /// 共享工作台状态 —— 跨组件统一管理 dirty/saving 等。
    ///
    /// `EditorWorkbench` observe `document` 变化 → 更新此状态 → Tab 标题联动。
    fn state(&self) -> Entity<WorkbenchState>;
}

/// 文本位置 —— 行/列从 0 开始。
///
/// 用于 `EditorContext::cursor` 与 `TextSpan` 的起止点。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextPosition {
    pub line: usize,
    pub character: usize,
}

/// 文本块 —— 位置范围,表示一段连续文本。
///
/// 用于 `EditorContext::selection`、LSP Range、AI 上下文选区。
/// `start` 必须 ≤ `end`。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextSpan {
    pub start: TextPosition,
    pub end: TextPosition,
}

/// 编辑器命令上下文 —— 经 `CallContext::parameter` 传入 `ICommand::execute` / `can_execute`。
///
/// `can_execute` 据此判断命令可用性(替代 v1 的 `when()` 方法)。
#[derive(Debug, Clone)]
pub struct EditorContext {
    pub uri: Uri,
    pub cursor: Option<TextPosition>,
    pub selection: Option<TextSpan>,
}
