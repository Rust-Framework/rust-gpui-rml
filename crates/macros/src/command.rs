//! `#[command]` 实现
//!
//! Phase B-2：从方法签名提取元信息（命令名、事件类型、参数）用于编译期校验。
//! 方法本体保持不变（pass-through）。
//!
//! 限制：`#[command]` 作用于 impl 块内的方法，无法获取所属结构体名，
//! 且 const item 不能在 impl 块内生成，因此无法在此生成 `impl ICommand`。
//! 元信息提取逻辑保留供未来 build.rs 扫描或 `#[view]` 宏配合使用。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{FnArg, ItemFn, Pat, Type};

pub fn expand(input: TokenStream) -> TokenStream {
    let item: ItemFn = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    // 校验：必须是方法（&mut self 或 &self 作为第一个参数）
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

    // 编译期校验：提取事件类型和参数，确认签名合法
    // （提取结果暂不使用，仅为未来元信息生成预留）
    let _event_type = extract_event_type(&item.sig.inputs);
    let _params = extract_params(&item.sig.inputs);

    // Pass-through：原样返回方法
    quote! { #item }
}

/// 从方法参数中提取事件类型名
///
/// 约定：最后一个非 Context 引用参数是事件对象（如 `&ClickEvent`）
fn extract_event_type(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> String {
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = arg {
            let ty_str = quote!(#pat_type.ty).to_string();
            // 跳过 Context 参数
            if ty_str.contains("Context") {
                continue;
            }
            // 提取引用类型的内层类型名
            if let Type::Reference(type_ref) = pat_type.ty.as_ref() {
                let inner = &type_ref.elem;
                let inner_str = quote!(#inner).to_string();
                // 去掉路径前缀，只保留类型名（如 rml_core::events::ClickEvent → ClickEvent）
                return inner_str
                    .split("::")
                    .last()
                    .unwrap_or(&inner_str)
                    .trim()
                    .to_string();
            }
        }
    }
    String::new()
}

/// 提取命令参数（除 self、事件、Context 外的参数）
fn extract_params(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Vec<(String, String)> {
    let mut params = Vec::new();
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = arg {
            let ty_str = quote!(#pat_type.ty).to_string();
            // 跳过 Context 和事件参数（引用类型）
            if ty_str.contains("Context") {
                continue;
            }
            if let Type::Reference(_) = pat_type.ty.as_ref() {
                continue;
            }
            // 提取参数名
            if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                let name = pat_ident.ident.to_string();
                let ty = ty_str.trim().to_string();
                params.push((name, ty));
            }
        }
    }
    params
}
