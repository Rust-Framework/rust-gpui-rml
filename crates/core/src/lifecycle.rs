//! `ILifecycle` trait —— 视图生命周期契约
//!
//! RML 视图生命周期分四阶段：创建 → 加载 → 更新 → 卸载。
//! `#[on_loaded]` / `#[on_unloaded]` 宏标记的方法由用户手动接入此 trait。
//! 详见文档 §8.1 生命周期总览。

use gpui::{Context, Window};

/// 生命周期回调 trait。
///
/// `#[on_loaded]` 标记的方法应在 `on_loaded` 中调用；
/// `#[on_unloaded]` 标记的方法应在 `on_unloaded` 中调用。
pub trait ILifecycle {
    /// 视图首次渲染完成后触发（仅一次）。
    ///
    /// 典型用途：加载初始数据、启动定时器、获取焦点、订阅外部事件、
    /// 打开 Dialog / Sheet（拥有 `&mut Window`）。
    fn on_loaded(&mut self, _window: &mut Window, _cx: &mut Context<Self>)
    where
        Self: Sized,
    {
    }

    /// 视图卸载前触发（仅一次）。
    ///
    /// 典型用途：取消异步任务、取消订阅、保存状态、释放资源。
    fn on_unloaded(&mut self, _cx: &mut Context<Self>)
    where
        Self: Sized,
    {
    }
}
