//! 工作台组件契约 + 文本位置/块类型。

use rml_core::contribution::IVisualContribution;
use rml_core::workbench::Uri;

/// 工作台呈现组件 —— IWorkbench 实现内部的贡献点。
///
/// 继承 `IVisualContribution`(具备 `id`/`name`/`description`/`icon` + `render`)。
/// 此 trait 为空标记 —— 仅用于能力查询区分"此视觉贡献是工作台组件"。
///
/// # 应用场景
///
/// `EditorWorkbench`(IWorkbench 实现 + IContributionHost)受理多个 IWorkbenchComponent:
/// - `CodeEditorComponent`(id="code")—— 代码编辑,默认内置
/// - `RmlDesignComponent`(id="design")—— RML 可视化设计器,仅 .rml
/// - `PreviewComponent`(id="preview")—— 只读预览,仅 Markdown/HTML
///
/// 用户在组件间切换,实现编辑/预览/设计多态呈现。
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
    /// 默认返回 `true` —— 作为默认视图组件(如 CodeWorkbench)。
    /// 特化组件(如 RmlDesignComponent 仅 .rml)应 override 此方法。
    fn matches(&self, _uri: &Uri) -> bool {
        true
    }
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
