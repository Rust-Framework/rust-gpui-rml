//! `#[contributehost]` host marker + framework registration.
//!
//! 宏只负责：
//! 1. 生成 `pub const ID: &'static str`
//! 2. 编译期断言目标类型已手动实现 `IContributionHost`
//! 3. 生成隐藏的注册函数（`cx.add(ID)`）
//!
//! 宏**不**自动 impl `IContributionHost`——用户须手动声明，强制区分
//! 「宏的注册职责」与「trait 的契约职责」。

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote, ToTokens};
use syn::parse::Parser;
use syn::{
    parse::{Parse, ParseStream},
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
                        "bindings/on_changed is removed: use subscribe_host_changes + cx.notify() for reactive updates",
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
            proc_macro2::Span::call_site(),
            "#[contributehost] requires a struct definition in its input",
        )
        .to_compile_error();
    };

    let register_fn = format_ident!(
        "__rml_register_{}",
        struct_name.to_string().to_lowercase()
    );
    let hidden_mod = format_ident!(
        "__rml_host_{}",
        struct_name.to_string().to_lowercase()
    );

    let id = &args.id;

    quote! {
        #(#items)*

        impl #struct_name {
            pub const ID: &'static str = #id;
        }

        // 编译期检测：目标对象必须实现 IContributionHost 接口
        // 宏不再自动 impl，用户须手动声明；此处断言确保不遗漏
        const _: () = {
            fn assert_contribution_host<T: rml_core::contribution::IContributionHost>() {}
            fn check() { assert_contribution_host::<#struct_name>(); }
        };

        #[doc(hidden)]
        mod #hidden_mod {
            use super::#struct_name;

            pub(super) fn register(cx: &mut gpui::App) {
                use rml_app::contribution::{ContributionExt, ensure_contribution_registry};
                ensure_contribution_registry(cx);
                cx.add(#struct_name::ID);
            }
        }

        #[doc(hidden)]
        pub fn #register_fn(cx: &mut gpui::App) {
            #hidden_mod::register(cx);
        }
    }
}
