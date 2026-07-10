//! `IEvent` trait —— RML 事件基础契约
//!
//! 所有 RML 事件对象实现此 trait，支持事件流控制（阻止默认行为、停止冒泡）。
//! 详见文档 §5.2.9 事件对象。

/// RML 事件基础 trait。
///
/// 实现此 trait 的事件对象可在事件流中被调度，
/// 通过 `prevent_default` / `stop_propagation` 控制事件传播。
pub trait IEvent: std::fmt::Debug + Clone + Send + Sync + 'static {
    /// 阻止默认行为（如阻止表单提交、阻止输入）
    fn prevent_default(&mut self);

    /// 停止事件冒泡
    fn stop_propagation(&mut self);

    /// 是否已调用 `prevent_default`
    fn is_default_prevented(&self) -> bool;

    /// 是否已调用 `stop_propagation`
    fn is_propagation_stopped(&self) -> bool;
}

// ──────────────────────────────────────────────────────────────────────────
//  用户组件事件处理器类型别名
//
// 用户组件在 .rml.rs 中声明对应字段（如
// `pub on_click: Option<rml_core::event::ClickHandler>`），父视图通过
// `on-click={handler}` 绑定时，codegen 把父视图的 handler 方法包装为闭包
// 并注入到子组件字段。
//
// 闭包签名与 GPUI `.on_click()` 等回调一致，便于在 .rml 模板中通过
// `on-click={self.on_click}` 应用（apply_event 识别 `self.<field>` 为
// `EventHandler::ClosureField`，生成 `.on_click(cx.listener(move |this, ev, _w, cx| {
//     if let Some(h) = &this.on_click { h(ev, _w, cx.deref_mut()); }
// }))`）。
//
// `Context<Self>: DerefMut<Target = App>`，故 `cx.deref_mut()` 将
// `&mut Context<Self>` 转换为 `&mut App`。
// ──────────────────────────────────────────────────────────────────────────

/// 点击事件处理器（on_click / on_aux_click）
pub type ClickHandler =
    Box<dyn Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static>;

/// 鼠标按下事件处理器（on_mouse_down）
pub type MouseDownHandler = Box<
    dyn Fn(&gpui::MouseDownEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static,
>;

/// 鼠标抬起事件处理器（on_mouse_up）
pub type MouseUpHandler = Box<
    dyn Fn(&gpui::MouseUpEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static,
>;

/// 鼠标移动事件处理器（on_mouse_move）
pub type MouseMoveHandler = Box<
    dyn Fn(&gpui::MouseMoveEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static,
>;

/// 滚轮事件处理器（on_wheel）
pub type WheelHandler = Box<
    dyn Fn(&gpui::ScrollWheelEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static,
>;

/// 键盘按下事件处理器（on_key_down）
pub type KeyDownHandler = Box<
    dyn Fn(&gpui::KeyDownEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static,
>;

/// 键盘抬起事件处理器（on_key_up）
pub type KeyUpHandler = Box<
    dyn Fn(&gpui::KeyUpEvent, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static,
>;

/// 悬停事件处理器（on_hover / on_mouse_enter / on_mouse_leave）
///
/// 回调参数 `&bool` 为 true 表示鼠标进入，false 表示离开。
pub type HoverHandler =
    Box<dyn Fn(&bool, &mut gpui::Window, &mut gpui::App) + Send + Sync + 'static>;

/// 根据 GPUI 事件类型名获取对应的 handler 类型名
///
/// 用于 scanner.rs 扫描字段类型时识别事件回调字段。
/// 返回 None 表示不是事件处理器类型。
pub fn handler_type_name(gpui_event_type: &str) -> Option<&'static str> {
    match gpui_event_type {
        "gpui::ClickEvent" => Some("ClickHandler"),
        "gpui::MouseDownEvent" => Some("MouseDownHandler"),
        "gpui::MouseUpEvent" => Some("MouseUpHandler"),
        "gpui::MouseMoveEvent" => Some("MouseMoveHandler"),
        "gpui::ScrollWheelEvent" => Some("WheelHandler"),
        "gpui::KeyDownEvent" => Some("KeyDownHandler"),
        "gpui::KeyUpEvent" => Some("KeyUpHandler"),
        _ => None,
    }
}
