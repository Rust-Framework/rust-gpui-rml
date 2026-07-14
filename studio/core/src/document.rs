//! 工作台共享文档与状态模型 —— IWorkbenchComponent 间数据同步的媒介。
//!
//! 两大共享 Entity:
//! - [`WorkbenchDocument`] —— 文档内容单一真相源,组件读它(渲染)或写它(编辑)
//! - [`WorkbenchState`] —— 跨组件统一管理 dirty/saving/last_error 等状态
//!
//! # 设计理由
//!
//! 不让每个组件各自持有 content 副本与 dirty 标记,避免:
//! 1. "design 编辑后切换到 code 看不到最新数据"问题
//! 2. "切换组件丢失修改标记"问题
//!
//! 任何组件修改 `WorkbenchDocument::content` → GPUI Entity 通知 →
//! 其他组件 observe 触发重新同步;`EditorWorkbench` observe document →
//! 更新 `WorkbenchState::dirty` → Tab 标题联动。
//!
//! # 扩展性
//!
//! 文档类型([`WorkbenchDocument::kind`])采用**开放字符串**而非封闭枚举,
//! 框架在 [`document_kind`] 模块提供常用类型常量,插件可自由定义新类型
//! (如 `"pdf"` / `"svg"` / `"json-tree"`)。组件优先经
//! `IWorkbenchComponent::matches(uri)` 判断适配性,`kind` 仅作辅助元数据。

use gpui::SharedString;

/// 文档类型标识常量 —— 开放扩展,插件可自由定义新类型。
///
/// 框架内置常用类型,插件可用任意字符串作为 [`WorkbenchDocument::kind`]。
/// 组件优先经 `IWorkbenchComponent::matches(uri)` 判断适配性,
/// `kind` 作为辅助元数据供组件条件渲染参考。
///
/// # 插件扩展示例
///
/// ```ignore
/// use studio_core::document::WorkbenchDocument;
///
/// // 插件自定义文档类型
/// const PDF: &str = "pdf";
/// let doc = WorkbenchDocument::new(uri, content, PDF);
/// ```
///
/// 对应的预览组件经 `matches(uri)` 匹配 `.pdf` 扩展名即可,
/// 无需修改框架代码。
pub mod document_kind {
    /// 纯文本(默认)
    pub const TEXT: &str = "text";
    /// Markdown —— PreviewComponent 经 `<Markdown>` 渲染为 GFM 富文本
    pub const MARKDOWN: &str = "markdown";
    /// HTML —— GPUI 无原生 HTML 渲染器,降级为 `<pre>` 纯文本展示源码
    pub const HTML: &str = "html";
    /// RML —— 预留 RmlDesignComponent 可视化设计器
    pub const RML: &str = "rml";
}

/// 共享文档模型 —— IWorkbenchComponent 间数据同步的单一真相源。
///
/// 持有当前文本内容 + 加载时原始内容(用于 dirty 判断)。
/// 由 `IWorkbenchComponentHost::document()` 返回,所有受该 host 管理的组件共享同一 Entity。
///
/// # 同步链路
///
/// 1. 组件编辑(如 CodeComponent 的 InputState 变更)→ `set_content(new_text)`
/// 2. GPUI Entity 通知所有 observers
/// 3. EditorWorkbench observe → 比对 `original` → 更新 `WorkbenchState::dirty`
/// 4. 其他组件(如 PreviewComponent)render 时读 `content` → 显示最新
///
/// # 文档类型开放性
///
/// `kind` 是 [`SharedString`] 而非枚举,插件可自由定义新类型。
/// 框架在 [`document_kind`] 模块提供常用类型常量(如 [`document_kind::MARKDOWN`])。
/// 组件用 `document.kind() == document_kind::MARKDOWN` 判断,或直接经
/// `IWorkbenchComponent::matches(uri)` 基于扩展名判断(不依赖 kind)。
pub struct WorkbenchDocument {
    uri: SharedString,
    content: SharedString,
    original: SharedString,
    kind: SharedString,
}

impl Default for WorkbenchDocument {
    fn default() -> Self {
        Self {
            uri: SharedString::default(),
            content: SharedString::default(),
            original: SharedString::default(),
            kind: document_kind::TEXT.into(),
        }
    }
}

impl WorkbenchDocument {
    /// 构造新文档。`original` 与 `content` 同时初始化为传入内容。
    ///
    /// `kind` 接受任意字符串,推荐使用 [`document_kind`] 模块的常量,
    /// 插件也可传入自定义类型标识。
    pub fn new(uri: SharedString, content: SharedString, kind: impl Into<SharedString>) -> Self {
        Self {
            original: content.clone(),
            content,
            uri,
            kind: kind.into(),
        }
    }

    pub fn uri(&self) -> SharedString {
        self.uri.clone()
    }

    pub fn content(&self) -> SharedString {
        self.content.clone()
    }

    /// 文档类型标识(开放字符串)。
    ///
    /// 框架内置类型见 [`document_kind`] 模块;插件可自由定义新类型。
    pub fn kind(&self) -> SharedString {
        self.kind.clone()
    }

    pub fn original(&self) -> SharedString {
        self.original.clone()
    }

    /// 更新当前内容(组件编辑时调用)。
    ///
    /// 不修改 `original` —— dirty 判断基于 `content != original`。
    pub fn set_content(&mut self, content: SharedString) {
        self.content = content;
    }

    /// 重新加载文件内容(Tab 切换或外部变更时调用)。
    ///
    /// 重置 `original` 与 `content` 为传入内容,清除 dirty 状态。
    pub fn reload(
        &mut self,
        uri: SharedString,
        content: SharedString,
        kind: impl Into<SharedString>,
    ) {
        self.uri = uri;
        self.original = content.clone();
        self.content = content;
        self.kind = kind.into();
    }

    /// 是否有未保存修改。
    pub fn is_dirty(&self) -> bool {
        self.content != self.original
    }

    /// 标记为已保存(写盘后调用)。
    ///
    /// 将 `original` 同步为当前 `content`,清除 dirty。
    pub fn mark_saved(&mut self) {
        self.original = self.content.clone();
    }
}

/// 工作台共享状态 —— 跨组件统一管理。
///
/// 不让每个组件各自管 dirty 标记,避免"切换组件丢失修改标记"问题。
/// `EditorWorkbench` observe `WorkbenchDocument` 变化 → 更新此状态 → Tab 标题联动。
#[derive(Default)]
pub struct WorkbenchState {
    dirty: bool,
    saving: bool,
    last_error: Option<SharedString>,
}

impl WorkbenchState {
    pub fn dirty(&self) -> bool {
        self.dirty
    }

    pub fn saving(&self) -> bool {
        self.saving
    }

    pub fn last_error(&self) -> Option<SharedString> {
        self.last_error.clone()
    }

    pub fn set_dirty(&mut self, dirty: bool) {
        self.dirty = dirty;
    }

    pub fn set_saving(&mut self, saving: bool) {
        self.saving = saving;
    }

    pub fn set_error(&mut self, error: Option<SharedString>) {
        self.last_error = error;
    }
}
