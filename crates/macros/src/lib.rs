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
mod derive_model;
mod lifecycle;
mod window;

use proc_macro::TokenStream;

/// 派生 `IModel` trait，使结构体成为 RML 响应式 Model/ViewModel。
///
/// 所有 `pub` 字段自动成为可绑定字段。
/// 声明 `element` 为 helper attribute，允许在字段上使用 `#[element]`。
#[proc_macro_derive(IModel, attributes(element))]
pub fn derive_i_model(input: TokenStream) -> TokenStream {
    derive_model::derive(input.into()).into()
}

/// 标记结构体为 RML 组件（Code-Behind ViewModel）。
///
/// 编译器会为该结构体生成 `Render` trait 实现。
///
/// # 参数
/// - `template = "path"`：显式指定 `.rml` 模板路径（默认按命名约定）
///
/// 合并自旧 `#[view]` + `#[component]`。
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    component::expand(args.into(), input.into()).into()
}

/// 标记结构体为窗口（顶层 OS 窗口）。
///
/// 在 `#[component]` 基础上额外生成 `IWindow` trait 实现，
/// 包含窗口配置（title/width/height）和窗口操作（open/close/show/hide/activate/state）。
///
/// # 参数
/// - `title = "..."`：窗口标题
/// - `width = N`：窗口宽度（像素）
/// - `height = N`：窗口高度（像素）
/// - `template = "path"`：显式指定 `.rml` 模板路径
///
/// # 示例
///
/// ```rust,ignore
/// #[window(title = "My App", width = 800, height = 600)]
/// #[derive(Default)]
/// pub struct MainWindow {
///     pub count: i32,
/// }
/// ```
#[proc_macro_attribute]
pub fn window(args: TokenStream, input: TokenStream) -> TokenStream {
    window::expand(args.into(), input.into()).into()
}

/// 标记方法为 UI 可调用的命令。
#[proc_macro_attribute]
pub fn command(_args: TokenStream, input: TokenStream) -> TokenStream {
    command::expand(input.into()).into()
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
