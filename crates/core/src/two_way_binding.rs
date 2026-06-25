//! `ITwoWayBinding` trait —— 组件双向绑定契约
//!
//! 自定义组件实现此 trait 后，可在父视图中通过 `model` 指令双向绑定。
//! 详见文档 §6.2.6 组件的双向绑定。

use gpui::Context;

/// 组件双向绑定 trait。
///
/// ```rust
/// impl ITwoWayBinding for Counter {
///     type Value = i32;
///     fn get_value(&self) -> Self::Value { self.count }
///     fn set_value(&mut self, value: Self::Value, cx: &mut Context<Self>) {
///         self.count = value;
///         cx.notify();
///     }
/// }
/// ```
pub trait ITwoWayBinding {
    /// 绑定的值类型
    type Value: Clone + Send + Sync;

    /// 读取当前值（UI → ViewModel 方向读取）
    fn get_value(&self) -> Self::Value;

    /// 写入新值（UI → ViewModel 方向写入）
    fn set_value(&mut self, value: Self::Value, cx: &mut Context<Self>)
    where
        Self: Sized;
}
