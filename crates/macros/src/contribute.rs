//! `#[contribute]` —— 为类型生成 `IContribution`、注册函数，并由 build.rs 扫描汇总为 `register_rml_contributions`

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    Expr, Ident, Item, LitStr, Token,
};

struct ContributeArgs {
    host: Expr,
    id: LitStr,
    name: LitStr,
    description: Option<LitStr>,
    icon: Option<Expr>,
    mode: Option<Ident>,
    order: Option<syn::LitInt>,
    placement: Option<Ident>,
    group: Option<LitStr>,
    kind: Option<LitStr>,
    parent_id: Option<LitStr>,
}

impl Parse for ContributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut host = None;
        let mut id = None;
        let mut name = None;
        let mut description = None;
        let mut icon = None;
        let mut mode = None;
        let mut order = None;
        let mut placement = None;
        let mut group = None;
        let mut kind = None;
        let mut parent_id = None;

        let fields: Punctuated<syn::Meta, Token![,]> =
            input.parse_terminated(syn::Meta::parse, Token![,])?;
        for meta in fields {
            let syn::Meta::NameValue(nv) = meta else {
                continue;
            };
            let key = nv.path.get_ident().map(|i| i.to_string());
            match key.as_deref() {
                Some("host") => {
                    host = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("id") => {
                    id = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("name") => {
                    name = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("description") => {
                    description = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("icon") => {
                    icon = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("mode") => {
                    if let Expr::Path(p) = nv.value {
                        if let Some(seg) = p.path.get_ident() {
                            mode = Some(seg.clone());
                        }
                    }
                }
                Some("order") => {
                    order = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("placement") => {
                    if let Expr::Path(p) = nv.value {
                        if let Some(seg) = p.path.get_ident() {
                            placement = Some(seg.clone());
                        }
                    }
                }
                Some("group") => {
                    group = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("kind") => {
                    kind = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("parent_id") => {
                    parent_id = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                _ => {}
            }
        }

        Ok(ContributeArgs {
            host: host.ok_or_else(|| syn::Error::new(input.span(), "missing host"))?,
            id: id.ok_or_else(|| syn::Error::new(input.span(), "missing id"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "missing name"))?,
            description,
            icon,
            mode,
            order,
            placement,
            group,
            kind,
            parent_id,
        })
    }
}

fn visual_mode_tokens(mode: &Option<Ident>) -> TokenStream {
    match mode.as_ref().map(|i| i.to_string()) {
        Some(m) if m == "Inline" => quote! { rml_core::contribution::VisualMode::Inline },
        Some(m) if m == "Chrome" => quote! { rml_core::contribution::VisualMode::Chrome },
        Some(m) if m == "Overlay" => quote! { rml_core::contribution::VisualMode::Overlay },
        _ => quote! { rml_core::contribution::VisualMode::Panel },
    }
}

fn placement_tokens(placement: &Option<Ident>) -> TokenStream {
    match placement.as_ref().map(|i| i.to_string()) {
        Some(p) if p == "Right" => quote! { rml_core::contribution::VisualPlacement::Right },
        _ => quote! { rml_core::contribution::VisualPlacement::Left },
    }
}

/// `host = "id"` 或 `host = MyHost`（须实现 [`IContributionHostId`]）
fn host_id_tokens(host: &Expr) -> TokenStream {
    match host {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => quote! { #s },
        Expr::Path(_) => quote! { #host::ID },
        _ => quote! {
            compile_error!("host must be a string literal or a type implementing IContributionHostId")
        },
    }
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse2::<ContributeArgs>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

    // 支持多 item 输入：当与 #[component]/#[window] 叠加时，下层宏已展开为
    // `struct + impls + include!` 多个 item。解析所有 item，找到 struct 提取名称，
    // 原样透传所有 item，再追加生成的 impl + 注册函数。
    let parse_items = |input: syn::parse::ParseStream| {
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
            "#[contribute] requires a struct definition in its input",
        )
        .to_compile_error();
    };

    let register_fn = format_ident!(
        "__rml_register_{}",
        struct_name.to_string().to_lowercase()
    );

    let host_id = host_id_tokens(&args.host);
    let id = &args.id;
    let name_key = &args.name;
    let description_impl = if let Some(desc) = &args.description {
        quote! { rml_core::i18n::t_static(#desc).into() }
    } else {
        quote! { gpui::SharedString::default() }
    };
    let icon_impl = if let Some(icon_expr) = &args.icon {
        if let Expr::Path(path) = icon_expr {
            if let Some(seg) = path.path.segments.last() {
                let icon_name = seg.ident.to_string();
                quote! { Some(gpui::SharedString::from(#icon_name)) }
            } else {
                quote! { None }
            }
        } else {
            quote! { None }
        }
    } else {
        quote! { None }
    };
    let visual_mode = args
        .mode
        .as_ref()
        .map(|_| visual_mode_tokens(&args.mode))
        .map(|t| quote! { .visual_mode(#t) });
    let placement = args
        .placement
        .as_ref()
        .map(|_| placement_tokens(&args.placement))
        .map(|p| quote! { .placement(#p) });
    let order = args
        .order
        .as_ref()
        .map(|o| quote! { .order(#o) })
        .unwrap_or_default();
    let group = args
        .group
        .as_ref()
        .map(|g| quote! { .group(#g) })
        .unwrap_or_default();
    let kind = args
        .kind
        .as_ref()
        .map(|k| quote! { .property("kind", #k) })
        .unwrap_or_default();
    let parent_id = args
        .parent_id
        .as_ref()
        .map(|p| quote! { .parent_id(#p) })
        .unwrap_or_default();

    let is_visual_panel = args.mode.as_ref().is_some_and(|m| m == "Panel");

    let registerable_impl = if is_visual_panel {
        quote! {
            impl rml_core::contribution::IVisualContribution for #struct_name {
                type View = #struct_name;

                fn render(&self) -> Self {
                    #struct_name::default()
                }
            }

            impl rml_app::contribution::Registerable for #struct_name {
                fn into_entry(
                    contribution: std::sync::Arc<Self>,
                    options: rml_core::contribution::ContributionOptions,
                ) -> rml_core::contribution::ContributedEntry {
                    rml_app::contribution::visual_registerable(contribution, options)
                }
            }
        }
    } else {
        quote! {
            impl rml_app::contribution::Registerable for #struct_name {
                fn into_entry(
                    contribution: std::sync::Arc<Self>,
                    options: rml_core::contribution::ContributionOptions,
                ) -> rml_core::contribution::ContributedEntry {
                    rml_app::contribution::data_registerable(contribution, options)
                }
            }
        }
    };

    quote! {
        #(#items)*

        impl rml_core::contribution::IContribution for #struct_name {
            fn id(&self) -> &str {
                #id
            }

            fn name(&self) -> gpui::SharedString {
                rml_core::i18n::t_static(#name_key).into()
            }

            fn description(&self) -> gpui::SharedString {
                #description_impl
            }

            fn icon(&self) -> Option<gpui::SharedString> {
                #icon_impl
            }
        }

        #registerable_impl

        /// 由 build.rs 生成的 `register_rml_contributions` 统一调用；用户无需手写清单。
        pub fn #register_fn(cx: &mut gpui::App) {
            use std::sync::Arc;
            use rml_app::contribution::register_contribution;
            let contribution = Arc::new(#struct_name::default());
            let options = rml_core::contribution::ContributionOptions::new()
                #visual_mode
                #placement
                #kind
                #parent_id
                #order
                #group;
            register_contribution::<#struct_name>(cx, #host_id, contribution, options);
        }
    }
}
