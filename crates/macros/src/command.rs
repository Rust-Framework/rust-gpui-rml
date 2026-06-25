//! `#[command]` 实现
//!
//! Phase A：pass-through（不修改方法），仅校验方法签名合法。
//! Phase B：生成 `ICommand` 实现 + 参数元信息 + 事件类型元信息。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn};

pub fn expand(input: TokenStream) -> TokenStream {
    let item: ItemFn = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    // Phase A 校验：必须是方法（&mut self 或 &self 作为第一个参数）
    let has_self = item.sig.inputs.iter().any(|arg| {
        matches!(arg, FnArg::Receiver(_))
    });
    if !has_self {
        return syn::Error::new_spanned(
            &item.sig,
            "#[command] methods must take &self or &mut self as first parameter",
        )
        .to_compile_error();
    }

    // Pass-through：原样返回方法
    quote! { #item }
}
