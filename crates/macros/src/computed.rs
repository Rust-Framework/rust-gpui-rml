//! `#[computed]` 实现
//!
//! Phase A：pass-through，仅校验签名是 `&self` 且无参。
//! Phase B：分析方法体中 `self.field` 访问，生成缓存代码。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn};

pub fn expand(input: TokenStream) -> TokenStream {
    let item: ItemFn = match syn::parse2(input.clone()) {
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

    // Pass-through：原样返回方法
    quote! { #item }
}
