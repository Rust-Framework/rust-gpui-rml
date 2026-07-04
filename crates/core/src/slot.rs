//! Slot 渲染器类型
//!
//! 用户组件（`#[component]`）声明的具名插槽通过 `SlotRenderer` 存储渲染闭包，
//! 而非直接存储 `gpui::AnyElement`。原因：
//!
//! - `IModel: 'static + Send + Sync` 要求组件类型满足线程安全（contribution 实体缓存需要）
//! - `gpui::AnyElement` 内部含 `Rc`，不满足 `Send`
//! - 闭包 `Fn(&mut Window, &mut App) -> AnyElement` 把 `cx` 作为参数传入，
//!   不捕获 `cx` 引用，可满足 `Send + Sync`
//!
//! 父视图 codegen 把 slot 内容表达式包装为 `Box::new(move |window, cx| { ... })`，
//! 通过 setter 注入；子组件 render 时调用闭包即时生成 element。
//!
//! # 限制
//!
//! slot 内容表达式不应直接引用父视图的 `self` 字段（生命周期不允许）。
//! 需要引用父视图数据时，应在 RML 中通过子组件自身的 props 传递。

/// Slot 渲染器：每次调用生成新的 `AnyElement`。
///
/// 子组件 render 时通过 `self.__rml_state.slot(<name>).map(|f| f(window, cx))`
/// 调用闭包生成 slot 内容。
pub type SlotRenderer = Box<
    dyn Fn(&mut gpui::Window, &mut gpui::App) -> gpui::AnyElement + Send + Sync + 'static,
>;
