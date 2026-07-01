//! `#[contribute]` —— 为类型生成 `IContribution` 与注册函数（数据贡献，MVVM 绑定）

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::{
    parse::Parse, parse::ParseStream, punctuated::Punctuated, Expr, Ident, ItemStruct, LitStr, Token,
};

struct ContributeArgs {
    host: LitStr,
    id: LitStr,
    name: LitStr,
    description: Option<LitStr>,
    icon: Option<Expr>,
    mode: Option<Ident>,
    order: Option<syn::LitInt>,
    placement: Option<Ident>,
    group: Option<LitStr>,
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

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse2::<ContributeArgs>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };
    let item = match syn::parse2::<ItemStruct>(input) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = &item.ident;
    let register_fn = format_ident!(
        "__rml_register_{}",
        struct_name.to_string().to_lowercase()
    );

    let host = &args.host;
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
    let visual_mode = visual_mode_tokens(&args.mode);
    let placement = placement_tokens(&args.placement);
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

    quote! {
        #item

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

        impl rml_app::contribution::Registerable for #struct_name {
            fn into_entry(
                contribution: std::sync::Arc<Self>,
                options: rml_core::contribution::ContributionOptions,
            ) -> rml_core::contribution::ContributedEntry {
                rml_app::contribution::data_registerable(contribution, options)
            }
        }

        /// 由功能模块在启动时调用，向贡献注册表注册本条目
        pub fn #register_fn(cx: &mut gpui::App) {
            use std::sync::Arc;
            use gpui::BorrowAppContext;
            use rml_app::contribution::ContributionRegistryGlobal;
            let contribution = Arc::new(#struct_name::default());
            let options = rml_core::contribution::ContributionOptions::new()
                .visual_mode(#visual_mode)
                .placement(#placement)
                #order
                #group;
            cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
                global.0.register(#host, contribution, options, cx);
            });
        }
    }
}
