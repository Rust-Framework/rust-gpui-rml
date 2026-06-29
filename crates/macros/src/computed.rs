//! `#[computed]` 实现
//!
//! Phase B-2：将原方法重命名为 `__rml_computed_<name>`，由 codegen 生成的
//! 包装方法接管原签名，提供基于版本号的缓存命中。
//!
//! ## 行为
//!
//! - 校验签名是 `&self` 且无参（除 self 外）
//! - 将 `fn <name>` 重命名为 `fn __rml_computed_<name>`，保留可见性、返回类型、方法体
//! - codegen 生成的包装方法 `pub fn <name>(&self) -> <RetType>` 调用
//!   `ComputedCache::get_or_compute` 实现缓存

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn};

pub fn expand(input: TokenStream) -> TokenStream {
    let mut item: ItemFn = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    // 校验：必须是 &self（不能是 &mut self）
    let receiver = item.sig.inputs.first().and_then(|arg| match arg {
        FnArg::Receiver(r) => Some(r),
        _ => None,
    });
    match receiver {
        Some(r) if r.reference.is_some() && r.mutability.is_none() => {
            // OK: &self
        }
        _ => {
            return syn::Error::new_spanned(
                &item.sig,
                "#[computed] methods must take &self (not &mut self)",
            )
            .to_compile_error();
        }
    }

    // 校验：无参数（除 self 外）
    let extra_params = item.sig.inputs.iter().skip(1).count();
    if extra_params > 0 {
        return syn::Error::new_spanned(
            &item.sig,
            "#[computed] methods must have no parameters besides &self",
        )
        .to_compile_error();
    }

    // 重命名：fn <name> → fn __rml_computed_<name>
    // codegen 生成的包装方法将使用原签名调用此重命名后的方法
    let new_name = format_ident!("__rml_computed_{}", item.sig.ident);
    item.sig.ident = new_name;

    quote! { #item }
}
