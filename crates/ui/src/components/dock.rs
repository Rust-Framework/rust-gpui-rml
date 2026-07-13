//! Dock 系统集成 —— gpui-component Dock 的 RML 适配层
//!
//! gpui-component 提供完整的 Dock 系统（`DockArea` / `DockItem` / `Panel`），
//! 内置拖拽、标签页、可伸缩布局。本模块提供：
//!
//! - [`SimplePanel`]：将任意 `AnyView` 包装为 `Panel` trait 实现的适配器
//! - dock 类型 re-export：`DockArea` / `DockItem` / `DockPlacement` / `Panel` /
//!   `PanelEvent` / `PanelView` / `PanelControl` / `PanelStyle` / `DockEvent` /
//!   `TabPanel` / `StackPanel` / `register_panel`
//!
//! ## 声明式用法
//!
//! Dock 系统本质上是命令式的（面板设置需要 `&mut Window` + `&mut Context`），
//! RML 集成通过 `<component content={dock_area} />` 透明容器渲染 `Entity<DockArea>`：
//!
//! ```rml
//! <component content={dock_area} />
//! ```
//!
//! ViewModel 侧在 `on_loaded` 中创建 `DockArea` 并设置面板：
//!
//! ```rust,ignore
//! use rml_ui::{DockArea, DockItem, DockPlacement, SimplePanel};
//! use std::sync::Arc;
//!
//! impl ILifecycle for MyCase {
//!     fn on_loaded(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
//!         let dock_area = cx.new(|cx| DockArea::new("my-dock", None, window, cx));
//!
//!         // 创建面板：文件树
//!         let tree_panel = cx.new(|cx| {
//!             SimplePanel::new("file-tree", "文件树", tree_view.clone(), window, cx)
//!         });
//!         let tree_item = DockItem::tab(tree_panel, &dock_area, window, cx);
//!
//!         // 设置左侧 dock
//!         dock_area.update(cx, |da, cx| {
//!             da.set_left_dock(tree_item, Some(gpui::px(240.)), true, window, cx);
//!         });
//!
//!         self.dock_area = Some(dock_area);
//!     }
//! }
//! ```

use gpui::{
    AnyView, App, Context, EventEmitter, FocusHandle, Focusable, IntoElement, Render, SharedString,
    Window,
};

// re-export gpui-component dock 类型，使 RML 用户可通过 rml_ui::DockArea 等直接引用
pub use gpui_component::dock::{
    register_panel, DockArea, DockEvent, DockItem, DockPlacement, Panel, PanelControl, PanelEvent,
    PanelStyle, PanelView, StackPanel, TabPanel,
};

/// 简单面板适配器 —— 将任意 `AnyView` 包装为 `Panel` trait 实现。
///
/// 用于在 `DockArea` 中添加自定义视图面板。实现 `Panel` trait 的所有必需方法，
/// 包括 `panel_name` / `title` / `closable` / `zoomable` / `Render` / `Focusable`。
///
/// ## 构造
///
/// ```rust,ignore
/// let panel = cx.new(|cx| {
///     SimplePanel::new("my-panel", "面板标题", any_view, window, cx)
///         .closable(false)
/// });
/// ```
pub struct SimplePanel {
    focus_handle: FocusHandle,
    title: SharedString,
    name: &'static str,
    closable: bool,
    zoomable: Option<PanelControl>,
    content: AnyView,
}

impl SimplePanel {
    /// 创建简单面板。
    ///
    /// - `name`：面板唯一标识（用于序列化/反序列化，不可变）
    /// - `title`：面板标题（显示在标签页上）
    /// - `content`：面板内容视图
    pub fn new(
        name: &'static str,
        title: impl Into<SharedString>,
        content: AnyView,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            title: title.into(),
            name,
            closable: true,
            zoomable: Some(PanelControl::Menu),
            content,
        }
    }

    /// 设置面板是否可关闭，默认 `true`。
    pub fn closable(mut self, closable: bool) -> Self {
        self.closable = closable;
        self
    }

    /// 设置面板缩放控制，默认 `Some(PanelControl::Menu)`。
    pub fn zoomable(mut self, zoomable: Option<PanelControl>) -> Self {
        self.zoomable = zoomable;
        self
    }

    /// 设置面板标题。
    pub fn title(mut self, title: impl Into<SharedString>) -> Self {
        self.title = title.into();
        self
    }
}

impl EventEmitter<PanelEvent> for SimplePanel {}

impl Focusable for SimplePanel {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for SimplePanel {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.content.clone()
    }
}

#[allow(unused_variables)]
impl Panel for SimplePanel {
    fn panel_name(&self) -> &'static str {
        self.name
    }

    fn title(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        self.title.clone()
    }

    fn closable(&self, _cx: &App) -> bool {
        self.closable
    }

    fn zoomable(&self, _cx: &App) -> Option<PanelControl> {
        self.zoomable
    }
}
