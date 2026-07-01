//! RML 过程宏集合
//!
//! 提供 `#[derive(IModel)]`、`#[component]`、`#[window]`、`#[command]`、
//! `#[computed]`、`#[on_loaded]`、`#[on_unloaded]` 等宏。
//!
//! 注：`#[element]` 作为字段属性，通过 `#[derive(IModel)]` 的 helper attribute
//! 声明（`attributes(element)`），由 `#[component]`/`#[window]` 在展开时剥离并解析。

#![forbid(unsafe_code)]

// 包名统一为 rust-rml-* 前缀，通过 extern crate 别名保留源码中的短名引用
extern crate rust_rml_core as rml_core;

mod command;
mod component;
mod computed;
mod contribute;
mod contributehost;
mod derive_model;
mod lifecycle;
mod main_attr;
mod validate;
mod window;

use proc_macro::TokenStream;

/// 派生 `IModel` trait，使结构体成为 RML 响应式 Model/ViewModel。
///
/// 所有 `pub` 字段自动成为可绑定字段。
/// 声明 `element` 与 `validate` 为 helper attribute，允许在字段上使用 `#[element]` 与 `#[validate]`。
#[proc_macro_derive(IModel, attributes(element, validate))]
pub fn derive_i_model(input: TokenStream) -> TokenStream {
    derive_model::derive(input.into()).into()
}

/// 标记结构体为可视化贡献点（配合 `#[component]` 使用）。
///
/// 生成 `IContribution` 实现及 `__rml_register_<Type>` 注册函数。
/// 若同 struct 带 `#[component]`，自动走组件 visual 注册路径。
#[proc_macro_attribute]
pub fn contribute(args: TokenStream, input: TokenStream) -> TokenStream {
    contribute::expand(args.into(), input.into()).into()
}

/// 声明贡献点主机标记类型（通常标注主窗口 ViewModel）。
///
/// - 自动实现 [`IContributionHost`] 并在启动时 `App::add` 注册 host slot
/// - 若指定 `bindings = "method"`：宏生成 `__rml_attach_contribution_bindings`，
///   在首次 render 时 `subscribe_host_changes` 并调用该方法（应用自行决定如何刷新 UI）
///
/// ```rust,ignore
/// #[contributehost(id = "my.app", bindings = "refresh_bindings")]
/// #[window]
/// pub struct MainWindow { ... }
/// ```
#[proc_macro_attribute]
pub fn contributehost(args: TokenStream, input: TokenStream) -> TokenStream {
    contributehost::expand(args.into(), input.into()).into()
}

/// 标记结构体为 RML 组件（Code-Behind ViewModel）。
///
/// 编译器会为该结构体生成 `Render` trait 实现（使用根节点的子节点作为渲染树）。
///
/// **不接受任何属性参数**。模板路径固定为 `<snake_case>.rml`，
/// 对应的 `.rml` 根节点必须为 `<component>`。
///
/// # 示例
///
/// ```rust,ignore
/// #[component]
/// #[derive(Default)]
/// pub struct MyWidget {
///     pub label: SharedString,
/// }
/// ```
///
/// 对应 `my_widget.rml`：
/// ```text
/// <component>
///     <!-- 子元素 -->
/// </component>
/// ```
///
/// 合并自旧 `#[view]` + `#[component]`。
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args.into(), input.into()).into()
}

/// 标记结构体为窗口（顶层 OS 窗口）。
///
/// 在 `#[component]` 基础上额外生成窗口句柄字段（`__rml_window_handle`）。
/// `IWindow` trait 实现由 RML 编译器从 `<window>` 根节点属性提取并生成。
///
/// **不接受任何属性参数**。窗口配置（`title`/`width`/`height`）在 `.rml` 根节点上声明式配置：
/// ```text
/// <window title="..." width="N" height="N">...</window>
/// ```
///
/// 也可使用 `<modern_window>` 根节点获得原生标题栏样式（`WindowChrome::Native`）。
///
/// # 示例
///
/// ```rust,ignore
/// #[window]
/// #[derive(Default)]
/// pub struct MainWindow {
///     pub count: i32,
/// }
/// ```
///
/// 对应 `main_window.rml`：
/// ```text
/// <window title="MainWindow" width="800" height="450">
///     <!-- 子元素 -->
/// </window>
/// ```
#[proc_macro_attribute]
pub fn window(args: TokenStream, input: TokenStream) -> TokenStream {
    window::expand(args.into(), input.into()).into()
}

/// 标记方法为 UI 可调用的命令。
///
/// 自动注入字段版本号 bump（`self.__rml_bump_version("<field>")`）和 `cx.notify()`，
/// 用户无需手动调用。通过参数可控制 notify 行为。
///
/// # 参数
///
/// - 无参数：默认行为，方法末尾自动注入 `cx.notify()`
/// - `no_notify`：不注入 `cx.notify()`（仍注入 `bump_version`），用于批量操作前或手动控制更新时机
/// - `debounce = "100ms"`：预留参数，本版本不实现 debounce 逻辑
///
/// # 示例
///
/// ```rust,ignore
/// // 默认：自动 notify
/// #[command]
/// pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
///     self.count += 1;
/// }
///
/// // 不自动 notify（用户手动控制）
/// #[command(no_notify)]
/// pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
///     self.a = 1;
///     self.b = 2;
///     cx.notify(); // 批量操作完成后手动 notify 一次
/// }
/// ```
#[proc_macro_attribute]
pub fn command(args: TokenStream, input: TokenStream) -> TokenStream {
    command::expand(args.into(), input.into()).into()
}

/// 标记方法为计算属性（依赖追踪 + 缓存）。
#[proc_macro_attribute]
pub fn computed(_args: TokenStream, input: TokenStream) -> TokenStream {
    computed::expand(input.into()).into()
}

/// 视图首次渲染完成后触发。
#[proc_macro_attribute]
pub fn on_loaded(_args: TokenStream, input: TokenStream) -> TokenStream {
    lifecycle::expand_on_loaded(input.into()).into()
}

/// 视图卸载前触发。
#[proc_macro_attribute]
pub fn on_unloaded(_args: TokenStream, input: TokenStream) -> TokenStream {
    lifecycle::expand_on_unloaded(input.into()).into()
}

/// 应用入口属性宏
///
/// 在 `fn main` 之前注入 `rml::embed_assets!()`,等价于
/// `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))`。
///
/// 生成文件内含 `#[ctor::ctor]` 自动注册函数,在 `main` 之前完成
/// `rml_core::assets::init(...)` 调用,因此 main.rs 无需手写资源相关代码
/// （无需 `rml::embed_assets!()` 或 `RmlApplication::assets()`）。
///
/// 模式（嵌入 vs 文件系统）由 `build.rs` 的 `.assets(path, embed)` 决定,
/// main.rs 不感知差异。
///
/// # 示例
///
/// ```rust,ignore
/// #[rml::main]
/// fn main() {
///     rml_app::RmlApplication::new()
///         .main_window::<MainWindow>()
///         .run::<Startup>();
/// }
/// ```
#[proc_macro_attribute]
pub fn main(attr: TokenStream, input: TokenStream) -> TokenStream {
    main_attr::expand(attr, input)
}
