//! 值对象空接口 —— 顶层抽象，供 UI 组件存储业务数据而不耦合框架贡献概念。
//!
//! `IContribution: IValue` 使所有贡献可作为 `IValue` 传递。UI 组件（如 `TabWindowShell`）
//! 仅依赖 `IValue`，通过能力查询（`as_contribution()`/`as_visual()`）按需提取元数据与视图。

use std::any::Any;

/// 值对象空接口 —— 框架顶层抽象。
///
/// `IContribution: IValue`，使 `Arc<dyn IContribution>` 可 trait upcast 为 `Arc<dyn IValue>`，
/// UI 组件只需依赖 `IValue` 而非贡献体系。具体能力（元数据、渲染、命令）通过
/// `ability::query` 按需查询。
///
/// 空接口 + blanket impl：所有 `Send + Sync + 'static` 类型自动实现 `IValue`，
/// 业务类型无需手动声明。`dyn IValue` 作为 trait object 仍是独立类型，
/// 提供 `as_contribution()`/`as_visual()`/`as_command()` 能力查询入口。
pub trait IValue: Send + Sync + Any {}

/// blanket impl —— 所有 `Send + Sync + 'static` 类型自动实现 `IValue`。
impl<T: Send + Sync + Any> IValue for T {}
