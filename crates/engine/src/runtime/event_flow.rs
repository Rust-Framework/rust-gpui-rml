//! 事件流调度器与 GPUI→RML 事件转换
//!
//! Phase B-1：
//! - 提供 `from_gpui_*` 转换函数，将 GPUI 原生事件转为 RML 事件对象
//! - Phase B-4 会补全三阶段调度（捕获 → 目标 → 冒泡）
//!
//! GPUI 事件类型清单（已核对源码 crates/gpui/src/interactive.rs 与 window.rs）：
//! - `ClickEvent` 为枚举（Mouse/Keyboard），含 `position()` / `modifiers()` / `click_count()` 方法
//! - `MouseDownEvent` / `MouseUpEvent` / `MouseMoveEvent` / `ScrollWheelEvent` 为结构体，含 `position` / `modifiers` 字段
//! - `KeyDownEvent` / `KeyUpEvent` 含 `keystroke: Keystroke` 字段
//! - `FocusOutEvent`（window.rs）含 `blurred: WeakFocusHandle` 字段
//! - `ScrollDelta` 为枚举（Pixels/Lines），含 `pixel_delta()` 方法
//!
//! GPUI 不直接提供 `InputEvent` / `ChangeEvent` / `FocusInEvent` / `SubmitEvent` / `LoadEvent` / `ResizeEvent` / `ScrollEvent`
//! 这些结构体 —— RML 自行定义，由命令方法直接构造或运行时补全。
//!
//! 注：RML 事件对象的 `flags` 字段为私有，跨 crate 不能用结构体字面量构造，
//! 必须先 `Default::default()` 再赋值公开字段。

/// 事件阶段（文档 §5.3）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventPhase {
    Capture,
    Target,
    Bubble,
}

/// GPUI 事件 → RML 事件转换
pub mod convert {
    /// 转换 GPUI 点击事件
    ///
    /// `ClickEvent` 为枚举（Mouse/Keyboard），通过方法访问 position/modifiers/click_count。
    pub fn from_gpui_click(ev: &gpui::ClickEvent) -> rml_core::events::ClickEvent {
        let mut out = rml_core::events::ClickEvent::default();
        out.position = ev.position();
        out.modifiers = ev.modifiers();
        out.click_count = ev.click_count() as u32;
        out
    }

    /// 转换 GPUI 鼠标按下事件
    pub fn from_gpui_mouse_down(ev: &gpui::MouseDownEvent) -> rml_core::events::MouseEvent {
        let mut out = rml_core::events::MouseEvent::default();
        out.position = ev.position;
        out.modifiers = ev.modifiers;
        out
    }

    /// 转换 GPUI 鼠标释放事件
    pub fn from_gpui_mouse_up(ev: &gpui::MouseUpEvent) -> rml_core::events::MouseEvent {
        let mut out = rml_core::events::MouseEvent::default();
        out.position = ev.position;
        out.modifiers = ev.modifiers;
        out
    }

    /// 转换 GPUI 鼠标移动事件
    pub fn from_gpui_mouse_move(ev: &gpui::MouseMoveEvent) -> rml_core::events::MouseEvent {
        let mut out = rml_core::events::MouseEvent::default();
        out.position = ev.position;
        out.modifiers = ev.modifiers;
        out
    }

    /// 转换 GPUI 滚轮事件
    ///
    /// `ScrollDelta` 为枚举（Pixels/Lines），统一调用 `pixel_delta()` 折算为像素。
    pub fn from_gpui_scroll_wheel(ev: &gpui::ScrollWheelEvent) -> rml_core::events::WheelEvent {
        let delta = ev.delta.pixel_delta(gpui::px(20.));
        let mut out = rml_core::events::WheelEvent::default();
        out.delta_x = delta.x;
        out.delta_y = delta.y;
        out.position = ev.position;
        out.modifiers = ev.modifiers;
        out
    }

    /// 转换 GPUI 键盘按下事件
    pub fn from_gpui_key_down(ev: &gpui::KeyDownEvent) -> rml_core::events::KeyDownEvent {
        let mut out = rml_core::events::KeyDownEvent::default();
        out.key = ev.keystroke.clone();
        out.modifiers = ev.keystroke.modifiers;
        out
    }

    /// 转换 GPUI 键盘释放事件
    pub fn from_gpui_key_up(ev: &gpui::KeyUpEvent) -> rml_core::events::KeyUpEvent {
        let mut out = rml_core::events::KeyUpEvent::default();
        out.key = ev.keystroke.clone();
        out.modifiers = ev.keystroke.modifiers;
        out
    }

    /// 转换 GPUI 焦点丢失事件
    ///
    /// GPUI 仅有 `FocusOutEvent`（on_focus_out 监听器），不直接提供 `FocusInEvent`。
    /// RML `FocusEvent` 的 target 字段留空，由运行时补全。
    ///
    /// 注：`on_focus_out` 是 `Window`/`Context` 级方法，不是元素级方法。
    /// 元素级事件绑定（`.on_focus_out(...)`）在 GPUI 中不存在。
    pub fn from_gpui_focus_out(_ev: &gpui::FocusOutEvent) -> rml_core::events::FocusEvent {
        rml_core::events::FocusEvent::default()
    }

    /// 转换 GPUI 悬停事件
    ///
    /// GPUI `on_hover` 回调接收 `&bool`（true = 进入，false = 离开），
    /// RML 将其封装为 `HoverEvent`。
    pub fn from_gpui_hover(is_hovering: &bool) -> rml_core::events::HoverEvent {
        let mut out = rml_core::events::HoverEvent::default();
        out.is_hovering = *is_hovering;
        out
    }

    // —— 以下事件类型 GPUI 不直接提供 ——
    // RML 在命令方法 / 运行时直接构造对应 RML 事件对象。
    // 这些函数保留作为 codegen 统一入口（Phase B-2 数据绑定补全）。

    /// 占位：RML `InputEvent` 由 codegen 直接构造（GPUI 无 `InputEvent` 结构体，仅有同名 trait）。
    pub fn rml_input(
        value: gpui::SharedString,
        prev: gpui::SharedString,
    ) -> rml_core::events::InputEvent {
        let mut out = rml_core::events::InputEvent::default();
        out.value = value;
        out.prev_value = prev;
        out
    }

    /// 占位：RML `ChangeEvent` 由 codegen 直接构造。
    pub fn rml_change(value: gpui::SharedString) -> rml_core::events::ChangeEvent {
        let mut out = rml_core::events::ChangeEvent::default();
        out.value = value;
        out
    }

    /// 占位：RML `FocusEvent`（获得焦点）由 codegen 直接构造。
    pub fn rml_focus_in() -> rml_core::events::FocusEvent {
        rml_core::events::FocusEvent::default()
    }

    /// 占位：RML `SubmitEvent` 由 codegen 直接构造。
    pub fn rml_submit() -> rml_core::events::SubmitEvent {
        rml_core::events::SubmitEvent::default()
    }

    /// 占位：RML `LoadEvent` 由 codegen 直接构造。
    pub fn rml_load() -> rml_core::events::LoadEvent {
        rml_core::events::LoadEvent::default()
    }

    /// 占位：RML `ResizeEvent` 由 codegen 直接构造。
    pub fn rml_resize(width: gpui::Pixels, height: gpui::Pixels) -> rml_core::events::ResizeEvent {
        let mut out = rml_core::events::ResizeEvent::default();
        out.width = width;
        out.height = height;
        out
    }

    /// 占位：RML `ScrollEvent` 由 codegen 直接构造。
    pub fn rml_scroll(
        scroll_x: gpui::Pixels,
        scroll_y: gpui::Pixels,
    ) -> rml_core::events::ScrollEvent {
        let mut out = rml_core::events::ScrollEvent::default();
        out.scroll_x = scroll_x;
        out.scroll_y = scroll_y;
        out
    }
}
