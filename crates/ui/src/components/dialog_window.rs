//! Dialog 拖动支持 —— 可拖拽标题栏组件 + 拖动偏移状态
//!
//! `DialogTitleBar` 是标准 gpui-component 组件（`RenderOnce` + `#[derive(IntoElement)]`），
//! 作为 Dialog 的 title 槽位，内置拖动追踪与关闭按钮。
//!
//! `DialogDragState` 持久化于 `Entity`，builder 闭包每帧读取 `offset` 并通过
//! `Styled::style().margin` 叠加到 Dialog 定位上（Dialog 的 `.left(x)` / `.top(y)`
//! 设置 position，CSS margin 与之独立累加）。

use gpui::{
    div, point, px, App, Context, CursorStyle, Entity, InteractiveElement, IntoElement,
    MouseDownEvent, MouseButton, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels, Point,
    Render, RenderOnce, SharedString, Styled, Window,
};
use gpui_component::{
    IconName, Sizable as _, WindowExt as _,
    button::{Button, ButtonVariants as _},
};

/// Dialog 拖动状态：偏移量 + 当前拖动起点
#[derive(Default)]
pub struct DialogDragState {
    pub offset: Point<Pixels>,
    dragging: Option<Point<Pixels>>,
}

impl Render for DialogDragState {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

impl DialogDragState {
    pub fn on_drag_start(&mut self, pos: Point<Pixels>) {
        self.dragging = Some(pos);
    }

    pub fn on_drag_move(&mut self, pos: Point<Pixels>) -> bool {
        if let Some(start) = self.dragging {
            self.offset = point(
                self.offset.x + pos.x - start.x,
                self.offset.y + pos.y - start.y,
            );
            self.dragging = Some(pos);
            return true;
        }
        false
    }

    pub fn on_drag_end(&mut self) {
        self.dragging = None;
    }
}

/// 可拖拽标题栏组件（作为 Dialog 的 title 槽位）。
///
/// 内含标题文本 + 关闭按钮，拖动通过 `window.listener_for` 实现窗口级追踪。
#[derive(IntoElement)]
pub struct DialogTitleBar {
    title: SharedString,
    drag_state: Entity<DialogDragState>,
}

impl DialogTitleBar {
    pub fn new(title: impl Into<SharedString>, drag_state: Entity<DialogDragState>) -> Self {
        Self {
            title: title.into(),
            drag_state,
        }
    }
}

impl RenderOnce for DialogTitleBar {
    fn render(self, window: &mut Window, _cx: &mut App) -> impl IntoElement {
        div()
            .id("dialog-title-bar")
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .h(px(36.))
            .w_full()
            .cursor(CursorStyle::OpenHand)
            .on_mouse_down(
                MouseButton::Left,
                window.listener_for(&self.drag_state, |s, e: &MouseDownEvent, _, _| {
                    s.on_drag_start(e.position)
                }),
            )
            .on_mouse_move(window.listener_for(&self.drag_state, |s, e: &MouseMoveEvent, _, cx| {
                if s.on_drag_move(e.position) {
                    cx.notify();
                }
            }))
            .on_mouse_up(
                MouseButton::Left,
                window.listener_for(&self.drag_state, |s, _e: &MouseUpEvent, _, _| {
                    s.on_drag_end()
                }),
            )
            .child(div().child(self.title))
            .child(
                Button::new("dialog-close")
                    .ghost()
                    .small()
                    .icon(IconName::Close)
                    .on_click(|_, window, cx| window.close_dialog(cx)),
            )
    }
}
