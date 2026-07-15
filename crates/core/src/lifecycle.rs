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

    /// 每帧渲染前调用，用于同步状态、初始化等。默认空实现。
    ///
    /// 由框架在 `Render::render` 中自动调用（`on_loaded` 之后、模板渲染之前），
    /// 统一两条渲染路径（RML 模板内嵌 + 贡献点 `IVisual::render`）的状态同步入口。
    ///
    /// 典型用途：从 host document 同步状态、检测 URI 变化、重新初始化子组件等。
    /// 替代手写 `IVisual::render` 中的状态同步逻辑，每个组件自治，父组件无需协调。
    fn before_render(&mut self, _window: &mut Window, _cx: &mut Context<Self>)
    where
        Self: Sized,
    {
    }

    /// IWorkbench 专用：外部实例（Provider 创建）→ 缓存 Entity 的数据同步。
    ///
    /// 仅由 `#[component(workbench)]` 宏生成的 `IVisual::render` 在 `Render::render`
    /// 之前调用一次/帧。`self` 是缓存 Entity（持久化、承载真实 UI 状态），
    /// `external` 是 Provider 每次渲染新建的外部实例（仅携带本次 URI 等元数据）。
    ///
    /// 典型实现：检测 `self.uri != external.uri`，变化时 reload 文件、重置子组件等。
    /// 替代 IWorkbench 手写 `IVisual::render` 中的状态同步逻辑。
    fn sync_from_external(&mut self, _external: &Self, _cx: &mut Context<Self>)
    where
        Self: Sized,
    {
    }
}
