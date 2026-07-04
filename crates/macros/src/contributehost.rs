//! `#[contributehost]` —— Host 标记宏（精简版）
//!
//! 宏仅负责生成 `pub const ID: &'static str`，作为 host_id 的单一来源。
//!
//! 用户职责：
//! - 手写一个 host handle struct（实现 `IContributionHost`），持有 `Arc<RwLock<Vec<...>>>` 共享存储
//! - 手写 `impl ILifecycle`（在 `on_loaded` 中创建 handle + `cx.get_contribution_registry().add(Arc::new(handle))` + `bootstrap_host_contributions(cx, Self::ID)`）
//! - 自管贡献存储（如 `Arc<RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>>`，handle 持有 clone）
//!
//! 宏展开顺序：`#[component]`/`#[window]`（内层先）→ `#[contributehost]`（外层）→ `#[contribute]`（最外层）。

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    Item, LitStr, Token,
};

struct ContributeHostArgs {
    id: LitStr,
}

impl Parse for ContributeHostArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;

        let fields: Punctuated<syn::Meta, Token![,]> =
            input.parse_terminated(syn::Meta::parse, Token![,])?;
        for meta in fields {
            let syn::Meta::NameValue(nv) = meta else {
                continue;
            };
            let key = nv.path.get_ident().map(|i| i.to_string());
            match key.as_deref() {
                Some("id") => {
                    id = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("bindings") | Some("on_changed") => {
                    return Err(syn::Error::new(
                        Span::call_site(),
                        "bindings/on_changed is removed: host 直接实现 IContributionHost",
                    ));
                }
                _ => {}
            }
        }

        Ok(ContributeHostArgs {
            id: id.ok_or_else(|| syn::Error::new(input.span(), "missing id"))?,
        })
    }
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse2::<ContributeHostArgs>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

    let parse_items = |input: ParseStream| {
        let mut items = Vec::new();
        while !input.is_empty() {
            items.push(input.parse::<Item>()?);
        }
        Ok(items)
    };
    let items: Vec<Item> = match parse_items.parse2(input) {
        Ok(items) => items,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = items.iter().find_map(|item| {
        if let Item::Struct(s) = item {
            Some(s.ident.clone())
        } else {
            None
        }
    });
    let Some(struct_name) = struct_name else {
        return syn::Error::new(
            Span::call_site(),
            "#[contributehost] requires a struct definition in its input",
        )
        .to_compile_error();
    };

    let id = &args.id;

    quote! {
        #(#items)*

        impl #struct_name {
            pub const ID: &'static str = #id;
        }
    }
}
