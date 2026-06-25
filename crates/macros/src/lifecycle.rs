//! `#[on_loaded]` 与 `#[on_unloaded]` 实现
//!
//! Phase A：pass-through，仅校验方法签名。
//! Phase B：通过 inventory 或重命名机制让 `#[view]` 在生成 `impl ILifecycle` 时调用此方法。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn};

pub fn expand_on_loaded(input: TokenStream) -> TokenStream {
    expand_lifecycle_hook(input, "on_loaded")
}

pub fn expand_on_unloaded(input: TokenStream) -> TokenStream {
    expand_lifecycle_hook(input, "on_unloaded")
}

fn expand_lifecycle_hook(input: TokenStream, kind: &str) -> TokenStream {
    let item: ItemFn = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    // 校验：必须是 &mut self
    let receiver = item.sig.inputs.first().and_then(|arg| match arg {
        FnArg::Receiver(r) => Some(r),
        _ => None,
    });
    match receiver {
        Some(r) if r.reference.is_some() && r.mutability.is_some() => {
            // OK: &mut self
        }
        _ => {
            return syn::Error::new_spanned(
                &item.sig,
                format!("#[{}] methods must take &mut self", kind),
            )
            .to_compile_error();
        }
    }

    // Phase A pass-through：原样返回方法
    // Phase B 会生成 `rml_on_loaded_impl`/`rml_on_unloaded_impl` 别名
    // 让 #[view] 在生成 impl ILifecycle 时调用此方法
    quote! { #item }
}
