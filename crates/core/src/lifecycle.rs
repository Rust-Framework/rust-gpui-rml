//! `ILifecycle` trait —— 视图生命周期契约
//!
//! RML 视图生命周期分四阶段：创建 → 加载 → 更新 → 卸载。
//! `#[on_loaded]` / `#[on_unloaded]` 宏注册的方法通过此 trait 触发。
//! 详见文档 §8.1 生命周期总览。

use gpui::Context;

/// 生命周期回调 trait。
///
/// `#[on_loaded]` 标记的方法会被注册到 `rml_on_loaded`，
/// `#[on_unloaded]` 标记的方法会被注册到 `rml_on_unloaded`。
pub trait ILifecycle {
    /// 视图首次渲染完成后触发（仅一次）。
    ///
    /// 典型用途：加载初始数据、启动定时器、获取焦点、订阅外部事件。
    fn rml_on_loaded(&mut self, _cx: &mut Context<Self>)
    where
        Self: Sized,
    {
    }

    /// 视图卸载前触发（仅一次）。
    ///
    /// 典型用途：取消异步任务、取消订阅、保存状态、释放资源。
    fn rml_on_unloaded(&mut self, _cx: &mut Context<Self>)
    where
        Self: Sized,
    {
    }
}
