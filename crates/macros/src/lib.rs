//! RML 过程宏集合
//!
//! 提供 `#[derive(IModel)]`、`#[view]`、`#[component]`、`#[command]`、
//! `#[computed]`、`#[on_loaded]`、`#[on_unloaded]` 等宏。
//!
//! 注：`#[element]` 作为字段属性，通过 `#[derive(IModel)]` 的 helper attribute
//! 声明（`attributes(element)`），由 `#[view]` 在展开时剥离并解析。

#![forbid(unsafe_code)]

mod command;
mod computed;
mod derive_model;
mod lifecycle;
mod view;

use proc_macro::TokenStream;

/// 派生 `IModel` trait，使结构体成为 RML 响应式 Model/ViewModel。
///
/// 所有 `pub` 字段自动成为可绑定字段。
/// 声明 `element` 为 helper attribute，允许在字段上使用 `#[element]`。
#[proc_macro_derive(IModel, attributes(element))]
pub fn derive_i_model(input: TokenStream) -> TokenStream {
    derive_model::derive(input.into()).into()
}

/// 标记结构体为 RML 视图的 Code-Behind（ViewModel）。
///
/// 编译器会为该结构体生成 `Render` trait 实现。
///
/// # 参数
/// - `template = "path"`：显式指定 `.rml` 模板路径（默认按命名约定）
#[proc_macro_attribute]
pub fn view(args: TokenStream, input: TokenStream) -> TokenStream {
    view::expand(args.into(), input.into()).into()
}

/// 标记结构体为可复用的自定义组件。
#[proc_macro_attribute]
pub fn component(args: TokenStream, input: TokenStream) -> TokenStream {
    view::expand_component(args.into(), input.into()).into()
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
