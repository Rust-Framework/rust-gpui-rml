//! Slot 渲染器类型与插槽作用域
//!
//! 用户组件（`#[component]`）声明的具名插槽通过 `SlotRenderer` 存储渲染闭包，
//! 而非直接存储 `gpui::AnyElement`。原因：
//!
//! - `IModel: 'static + Send + Sync` 要求组件类型满足线程安全（contribution 实体缓存需要）
//! - `gpui::AnyElement` 内部含 `Rc`，不满足 `Send`
//! - 闭包 `Fn(&dyn ISlotScope, &mut Window, &mut App) -> AnyElement` 把 `cx` 作为参数传入，
//!   不捕获 `cx` 引用，可满足 `Send + Sync`
//!
//! 父视图 codegen 把 slot 内容表达式包装为 `Box::new(move |scope, window, cx| { ... })`，
//! 通过 setter 注入；子组件 render 时调用闭包即时生成 element。
//!
//! # 作用域插槽
//!
//! 闭包首参 `&dyn ISlotScope` 由插槽宿主（slot host）构造并传入：
//! - `TabWindowShell`：暴露 left/right/bottom 插槽的 resizable 操控权
//! - 自定义组件默认传 `NullSlotScope`（无操控权）
//!
//! RML 端通过 `<template slot="bottom" scope={panel}>` 声明接收作用域变量，
//! codegen 注入为 `let panel: &dyn ISlotScope = <closure_param>;`。
//! 不写 `scope={...}` 时闭包首参以 `_scope` 忽略，向后兼容。
//!
//! # 限制
//!
//! slot 内容表达式不应直接引用父视图的 `self` 字段（生命周期不允许）。
//! 需要引用父视图数据时，应在 RML 中通过子组件自身的 props 传递。

use gpui::{App, Pixels, Window};

/// 插槽作用域：由插槽宿主（slot host）实现，向 slot 内容暴露父容器操控权。
///
/// 实现方：
/// - [`NullSlotScope`]：默认空作用域，所有方法返回 `None` / no-op
/// - `TabWindowSlotScope`（在 `rml_ui` 中）：暴露包裹 left/right/bottom 的 resizable 操控
///
/// RML 中通过 `<template slot="bottom" scope={panel}>` 接收，调用 `panel.maximize(window, cx)`
/// 等便捷方法即可驱动父容器行为，无需了解底层 `ResizableState` API。
pub trait ISlotScope: Send + Sync {
    /// 插槽名（`"left"` / `"right"` / `"bottom"` / `"header"` / ...）
    fn slot_name(&self) -> &str;

    /// 此 slot 当前尺寸（width 或 height，依 axis 而定）。
    ///
    /// TabWindow 的 left/right 返回宽度，bottom 返回高度。无 resizable 包裹时返回 `None`。
    fn current_size(&self) -> Option<Pixels> {
        None
    }

    /// 容器总尺寸（用于 maximize 计算）。
    ///
    /// 通常等于 resizable group 在对应 axis 上的可用空间。
    fn container_size(&self) -> Option<Pixels> {
        None
    }

    /// 是否支持 resizable 操控。
    ///
    /// 返回 `false` 时 `maximize` / `restore` / `close` 为 no-op。
    /// 默认 `false`，由 `TabWindowSlotScope` 等实现覆盖为 `true`。
    fn has_resizable(&self) -> bool {
        false
    }

    /// 最大化此面板（默认 no-op）。
    ///
    /// 行为约定：记录当前尺寸供 [`restore`](Self::restore) 还原，并将面板尺寸
    /// 调整到 [`container_size`](Self::container_size)。
    fn maximize(&self, _window: &mut Window, _cx: &mut App) {}

    /// 还原此面板到 maximize 之前的尺寸（默认 no-op）。
    ///
    /// 若未经过 [`maximize`](Self::maximize)，调用此方法无效。
    fn restore(&self, _window: &mut Window, _cx: &mut App) {}

    /// 关闭/折叠此面板（默认 no-op）。
    ///
    /// 行为约定：将面板尺寸调整为 0 或最小阈值，触发宿主的折叠逻辑。
    fn close(&self, _window: &mut Window, _cx: &mut App) {}
}

/// 默认空作用域：所有方法返回 `None` / no-op。
///
/// 用于不支持上下文注入的 slot：
/// - 自定义组件的 `<slot>` 占位符默认传此类型
/// - 折叠状态下的 TabWindow 插槽（已移出 resizable group）
pub struct NullSlotScope {
    slot_name: &'static str,
}

impl NullSlotScope {
    pub fn new(slot_name: &'static str) -> Self {
        Self { slot_name }
    }
}

impl ISlotScope for NullSlotScope {
    fn slot_name(&self) -> &str {
        self.slot_name
    }
}

/// Slot 渲染器：每次调用生成新的 `AnyElement`。
///
/// 子组件 render 时通过 `self.__rml_state.slot(<name>).map(|f| f(&scope, window, cx))`
/// 调用闭包生成 slot 内容，其中 `scope` 由宿主构造：
/// - 自定义组件默认 `NullSlotScope::new(<name>)`
/// - TabWindowShell 构造 `TabWindowSlotScope` 暴露 resizable 操控
pub type SlotRenderer = Box<
    dyn Fn(&dyn ISlotScope, &mut gpui::Window, &mut gpui::App) -> gpui::AnyElement + Send + Sync + 'static,
>;
