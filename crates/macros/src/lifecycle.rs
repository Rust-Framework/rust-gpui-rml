//! `#[on_loaded]` 与 `#[on_unloaded]` 实现
//!
//! Phase B-2：pass-through + 签名校验。
//!
//! ## 生命周期联动限制
//!
//! 由于 Rust 过程宏架构约束，`#[on_loaded]`/`#[on_unloaded]` 作用于 impl 块内的方法，
//! 无法获取所属结构体名，且无法从 impl 块内生成 trait 实现。
//! 因此当前阶段采用以下策略：
//!
//! 1. `#[view]` 宏生成 `impl ILifecycle` 使用 trait 默认空实现
//! 2. 用户如需生命周期回调，在 `.rml.rs` 中手动实现 `ILifecycle` trait
//! 3. `#[on_loaded]`/`#[on_unloaded]` 作为标记和校验，不自动生成代码
//!
//! 未来可通过 build.rs 扫描 `.rml.rs` 文件中的 `#[on_loaded]` 标记，
//! 生成注册文件实现自动联动（Phase B-3）。

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

    // Pass-through：原样返回方法
    // 用户可在手动 impl ILifecycle 时调用此方法
    quote! { #item }
}
