//! `IViewModel` trait —— ViewModel 层契约
//!
//! ViewModel 持有视图状态、响应命令、暴露计算属性、管理生命周期。
//! ViewModel 自身即 GPUI Entity（通过 `cx.new()` 创建）。
//!
//! 生命周期方法继承自 `ILifecycle`，避免重复定义。

use crate::lifecycle::ILifecycle;
use crate::model::IModel;

/// ViewModel 基础 trait，扩展 IModel 并继承 ILifecycle。
///
/// `#[view]` 标记的结构体实现此 trait（由编译器生成的 `Render` 实现依赖它）。
/// 生命周期回调 `on_loaded` / `on_unloaded` 来自 `ILifecycle`。
pub trait IViewModel: IModel + ILifecycle {}
