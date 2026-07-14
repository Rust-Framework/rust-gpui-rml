//! WPF TabItem 风格的 Tab 子项：title (header) + body (闭包模板)
//!
//! 与 [`super::Tab`] 的关键差异：
//! - `Tab` 仅有 header 内容（children 作为 header 渲染），无 body 概念
//! - `TabItem` 同时承载 title (header) 与 body (选中时渲染的内容)，对应 WPF TabControl/TabItem 模式
//!
//! `TabItem` 是纯数据载体，由 [`super::Tabs`] 在 `render` 内部消费：
//! - title 部分转换为 `Tab` 进行 header 渲染（保留 6 种 variant 动画/状态）
//! - body 部分作为闭包模板惰性渲染（仅选中 tab 的 body 被调用）
//!
//! 因此 `TabItem` 不实现 `IntoElement`，避免被误用为顶层独立元素。

use std::rc::Rc;
use std::sync::Arc;

use gpui::{AnyElement, App, ClickEvent, IntoElement, ParentElement, SharedString, Window};
use gpui_component::Icon;

pub type TabBodyRenderer = Arc<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync + 'static>;
type TabClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

/// WPF TabItem 风格的 Tab 子项：title (header) + body (闭包模板)。
///
/// 详见模块级文档。
#[derive(Default)]
pub struct TabItem {
    pub(super) ix: usize,
    pub(super) title_label: Option<SharedString>,
    pub(super) title_icon: Option<Icon>,
    pub(super) title_children: Vec<AnyElement>,
    pub(super) body: Option<TabBodyRenderer>,
    pub(super) disabled: bool,
    pub(super) tab_bar_prefix: Option<bool>,
    pub(super) on_click: Option<TabClickHandler>,
    /// 透传到 [`super::Tab::closable`]，控制是否在 header 末尾渲染关闭按钮。
    pub(super) closable: bool,
    /// 透传到 [`super::Tab::preview`]，控制是否以 italic 标题渲染（VSCode 预览 tab）。
    pub(super) preview: bool,
}

impl TabItem {
    /// 创建一个空 TabItem。
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置 header 文字。
    ///
    /// 与 [`Self::title_icon`] 互斥（优先级：title_child > title > title_icon）。
    pub fn title(mut self, label: impl Into<SharedString>) -> Self {
        self.title_label = Some(label.into());
        self
    }

    /// 设置 header 图标。
    ///
    /// 与 [`Self::title`] 互斥（优先级：title_child > title > title_icon）。
    pub fn title_icon(mut self, icon: impl Into<Icon>) -> Self {
        self.title_icon = Some(icon.into());
        self
    }

    /// 追加 header 自定义内容（最高优先级，覆盖 title/title_icon）。
    ///
    /// 可多次调用以追加多个子元素。
    pub fn title_child(mut self, child: impl IntoElement) -> Self {
        self.title_children.push(child.into_any_element());
        self
    }

    /// 设置 body 闭包模板（WPF TabItem.Content 的惰性渲染版）。
    ///
    /// 仅当选中此 tab 时，Tabs 才调用闭包渲染 body 内容。
    /// 闭包签名与 [`crate::components::table::CellTemplate`] 一致（但不传索引/数据，
    /// 索引和数据在闭包外捕获）。
    pub fn body<F>(mut self, body: F) -> Self
    where
        F: Fn(&mut Window, &mut App) -> AnyElement + Send + Sync + 'static,
    {
        self.body = Some(Arc::new(body));
        self
    }

    /// 返回 body renderer 的 clone（Arc 共享），供外部在内容区渲染选中 tab 的 body。
    pub fn body_renderer(&self) -> Option<TabBodyRenderer> {
        self.body.clone()
    }

    /// 移除并返回 body renderer，使此 TabItem 仅保留 header。
    ///
    /// 用于 TabWindowShell 场景：TabWindowShell 自行在内容区渲染选中 tab 的 body，
    /// 需在将 TabItem 传入 Tabs（tab bar）前剥离 body，避免 Tabs 的 WPF TabControl
    /// 模式重复渲染 body 导致 deferred 弹出层（Select/ComboBox 下拉框）出现双份。
    pub fn take_body(&mut self) -> Option<TabBodyRenderer> {
        self.body.take()
    }

    /// 设置 disabled 状态。
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// 设置点击处理器。
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }

    /// 透传到 [`super::Tab::closable`]，控制是否在 header 末尾渲染关闭按钮。
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// 透传到 [`super::Tab::preview`]，控制是否以 italic 标题渲染（VSCode 预览 tab）。
    pub fn preview(mut self, preview: bool) -> Self {
        self.preview = preview;
        self
    }

    /// 由 Tabs 在 render 时透传索引。
    pub(crate) fn ix(mut self, ix: usize) -> Self {
        self.ix = ix;
        self
    }

    /// 由 Tabs 在 render 时透传 prefix 标志。
    pub(crate) fn tab_bar_prefix(mut self, tab_bar_prefix: bool) -> Self {
        self.tab_bar_prefix = Some(tab_bar_prefix);
        self
    }

    /// 浅克隆供 TabWindow 跨帧缓存复用（body/title 为 Arc/SharedString，可共享）。
    /// `title_children` 无法克隆 `AnyElement`，TabWindow 简单绑定模式不使用该字段。
    pub(crate) fn clone_for_cache(&self) -> Self {
        Self {
            ix: self.ix,
            title_label: self.title_label.clone(),
            title_icon: self.title_icon.clone(),
            title_children: Vec::new(),
            body: self.body.clone(),
            disabled: self.disabled,
            tab_bar_prefix: self.tab_bar_prefix,
            on_click: self.on_click.clone(),
            closable: self.closable,
            preview: self.preview,
        }
    }

    /// 把 TabItem 的 title 部分转换为 [`super::Tab`] 进行 header 渲染。
    ///
    /// variant/size/selected/indicator_* 由 Tabs 在调用方设置。
    pub(super) fn into_header_tab(self) -> super::Tab {
        let mut tab = super::Tab::new()
            .ix(self.ix)
            .tab_bar_prefix(self.tab_bar_prefix.unwrap_or(true))
            .disabled(self.disabled)
            .closable(self.closable)
            .preview(self.preview);
        if let Some(label) = self.title_label {
            tab = tab.label(label);
        }
        if let Some(icon) = self.title_icon {
            tab = tab.icon(icon);
        }
        if let Some(on_click) = self.on_click {
            tab = tab.on_click(move |e, w, c| (on_click)(e, w, c));
        }
        for child in self.title_children {
            tab = tab.child(child);
        }
        tab
    }
}

impl From<&'static str> for TabItem {
    fn from(label: &'static str) -> Self {
        Self::new().title(label)
    }
}

impl From<String> for TabItem {
    fn from(label: String) -> Self {
        Self::new().title(label)
    }
}

impl From<SharedString> for TabItem {
    fn from(label: SharedString) -> Self {
        Self::new().title(label)
    }
}

impl From<Icon> for TabItem {
    fn from(icon: Icon) -> Self {
        Self::new().title_icon(icon)
    }
}

impl From<gpui_component::IconName> for TabItem {
    fn from(icon_name: gpui_component::IconName) -> Self {
        Self::new().title_icon(Icon::new(icon_name))
    }
}

/// 从 [`super::Tab`] 转换为 [`TabItem`]（body=None），保留 Tab 的所有 header 字段。
///
/// 这使得现有 `Tabs::child(Tab::new()...)` 调用方式仍然有效。
impl From<super::Tab> for TabItem {
    fn from(tab: super::Tab) -> Self {
        Self {
            ix: tab.ix,
            title_label: tab.label,
            title_icon: tab.icon,
            title_children: tab.children,
            body: None,
            disabled: tab.disabled,
            tab_bar_prefix: tab.tab_bar_prefix,
            on_click: tab.on_click,
            closable: tab.closable,
            preview: tab.preview,
        }
    }
}
