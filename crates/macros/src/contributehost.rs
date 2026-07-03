//! `#[contributehost]` —— Host 自动化标记宏
//!
//! 宏负责：
//! 1. 向 struct 注入 `entries: ObservableVec<ContributionEntry>` + `i18n_version: u32` 字段
//! 2. 生成 `pub const ID: &'static str`
//! 3. 自动生成 `impl IContributionHost`（add/remove 操作 entries）
//! 4. 自动生成 `impl ILifecycle`（channel + spawn + take_pending + i18n observe + IHostEntity 委托）
//! 5. 编译期断言目标类型实现 `IHostEntity`
//!
//! 业务代码只需实现 `IHostEntity` trait 提供 host 特有逻辑（`host_on_loaded`/`on_locale_changed`）。
//! 宏展开顺序：`#[component]`/`#[window]`（内层先）→ `#[contributehost]`（注入字段）→ `#[contribute]`（最外层）。

use proc_macro2::{Span, TokenStream};
use quote::{quote, ToTokens};
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
                        "bindings/on_changed is removed: host 自动化由宏生成",
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
    let mut items: Vec<Item> = match parse_items.parse2(input) {
        Ok(items) => items,
        Err(e) => return e.to_compile_error(),
    };

    // 找到 struct 并注入 entries + i18n_version 字段
    let mut struct_name = None;
    for item in items.iter_mut() {
        if let Item::Struct(s) = item {
            if struct_name.is_none() {
                struct_name = Some(s.ident.clone());
            }
            if let syn::Fields::Named(named) = &mut s.fields {
                named.named.push(syn::parse_quote! {
                    #[allow(non_snake_case, dead_code)]
                    entries: rml_core::observable::ObservableVec<rml_core::contribution::ContributionEntry>
                });
                named.named.push(syn::parse_quote! {
                    #[allow(non_snake_case, dead_code)]
                    i18n_version: u32
                });
            }
        }
    }
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

        // 编译期断言：目标类型必须实现 IHostEntity（业务钩子）
        const _: () = {
            fn assert_host_entity<T: rml_core::contribution::IHostEntity>() {}
            fn check() { assert_host_entity::<#struct_name>(); }
        };

        // 自动生成 IContributionHost：add/remove 操作 entries
        impl rml_core::contribution::IContributionHost for #struct_name {
            fn id(&self) -> &'static str {
                Self::ID
            }
            fn add(
                &self,
                contribution: std::sync::Arc<dyn rml_core::contribution::IContribution>,
                options: rml_core::contribution::ContributionOptions,
            ) {
                self.entries.push(rml_core::contribution::ContributionEntry {
                    contribution,
                    options,
                });
            }
            fn remove(&self, contribution_id: &str) {
                self.entries
                    .retain(|e| e.contribution.id() != contribution_id);
            }
        }

        // 自动生成 ILifecycle：框架标准 setup + IHostEntity 钩子委托
        impl rml_core::lifecycle::ILifecycle for #struct_name {
            fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
                use rml_app::contribution::ContributionRegistryExt;

                // 1. channel + spawn：ObservableVec 变更 → cx.notify()
                let (tx, rx) = rml_core::flume::unbounded::<()>();
                self.entries = rml_core::observable::ObservableVec::with_notifier(tx);
                cx.spawn(async move |this, cx| {
                    while rx.recv_async().await.is_ok() {
                        let _ = this.update(cx, |_, cx| cx.notify());
                    }
                })
                .detach();

                // 2. take_pending → self.add 受理
                let pending = cx.get_contribution_registry().take_pending(Self::ID);
                for (c, o) in pending {
                    self.add(c, o);
                }

                // 3. i18n observe：locale 变更 → bump i18n_version + on_locale_changed + cx.notify
                cx.observe_global::<rml_core::i18n::I18nState>(|this, cx| {
                    this.i18n_version = this.i18n_version.wrapping_add(1);
                    rml_core::contribution::IHostEntity::on_locale_changed(this, cx);
                    cx.notify();
                })
                .detach();

                // 4. 委托 IHostEntity 钩子（业务代码的 host 特有逻辑）
                rml_core::contribution::IHostEntity::host_on_loaded(self, _window, cx);
            }
        }
    }
}
