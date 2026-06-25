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
