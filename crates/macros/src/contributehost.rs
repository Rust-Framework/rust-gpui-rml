//! `#[contributehost]` —— Host 标记宏（精简版）
//!
//! 宏仅负责：
//! 1. 生成 `pub const ID: &'static str`
//! 2. 生成 `pub fn __rml_install_host(this: &Entity<Self>, cx: &mut App) -> flume::Receiver<HostOp>`
//!    （内部调 `rml_app::contribution::install_entity_host`：注册 handle + 触发 host_id 的贡献注册）
//! 3. 编译期断言 `T: IContributionHost`（用户必须手写 impl）
//!
//! 用户职责：
//! - 手写 `impl IContributionHost`（override `add`/`remove` 中需要的）
//! - 手写 `impl ILifecycle`（在 `on_loaded` 中调 `Self::__rml_install_host` + `drain_host_ops`）
//! - 自管贡献存储（如 `RwLock<Vec<(Arc<dyn IContribution>, ContributionOptions)>>`）
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

            /// 注册 host handle + 触发该 host_id 的所有贡献注册。
            ///
            /// 由用户在 `impl ILifecycle::on_loaded` 中调用：
            /// ```rust,ignore
            /// fn on_loaded(&mut self, _: &mut Window, cx: &mut Context<Self>) {
            ///     let rx = Self::__rml_install_host(cx.entity(), cx);
            ///     self.host_rx = Some(rx);
            ///     if let Some(rx) = &self.host_rx {
            ///         rml_app::contribution::drain_host_ops(rx, self);
            ///     }
            ///     cx.notify();
            /// }
            /// ```
            pub fn __rml_install_host(
                this: gpui::Entity<Self>,
                cx: &mut gpui::App,
            ) -> rml_core::flume::Receiver<rml_app::contribution::HostOp> {
                rml_app::contribution::install_entity_host(Self::ID, this, cx)
            }
        }

        // 编译期断言：目标类型必须实现 IContributionHost（用户手写）
        const _: () = {
            fn assert_host<T: rml_core::contribution::IContributionHost>() {}
            fn check() { assert_host::<#struct_name>(); }
        };
    }
}
