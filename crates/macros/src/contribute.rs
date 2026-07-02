//! `#[contribute]` —— 为类型生成 `IContribution`、`IVisualContribution`（视觉贡献）impl +
//! 视觉提取器注册 + 单行注册函数。build.rs 扫描汇总为 `register_rml_contributions`。

use proc_macro2::TokenStream;

use quote::{format_ident, quote, ToTokens};

use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    Expr, Item, LitStr, Token,
};

struct ContributeArgs {
    host_id: Expr,
    id: LitStr,
    name: LitStr,
    description: Option<LitStr>,
    icon: Option<Expr>,
    order: Option<syn::LitInt>,
    group: Option<LitStr>,
    slot: Option<LitStr>,
    parent_id: Option<LitStr>,
    visual: bool,
    align_right: bool,
}

impl Parse for ContributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut host_id = None;
        let mut id = None;
        let mut name = None;
        let mut description = None;
        let mut icon = None;
        let mut order = None;
        let mut group = None;
        let mut slot = None;
        let mut kind = None;
        let mut parent_id = None;
        let mut visual = false;
        let mut align_right = false;

        let fields: Punctuated<syn::Meta, Token![,]> =
            input.parse_terminated(syn::Meta::parse, Token![,])?;

        for meta in fields {
            match &meta {
                syn::Meta::Path(path) if path.is_ident("visual") => {
                    visual = true;
                }
                syn::Meta::NameValue(nv) => {
                    let key = nv.path.get_ident().map(|i| i.to_string());
                    match key.as_deref() {
                        Some("host_id") => {
                            host_id = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("host") => {
                            return Err(syn::Error::new(
                                nv.path.span(),
                                "parameter renamed: use `host_id = \"...\"` (string literal only)",
                            ));
                        }
                        Some("id") => {
                            id = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("name") => {
                            name = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("description") => {
                            description = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("icon") => {
                            icon = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("order") => {
                            order = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("group") => {
                            group = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("slot") => {
                            slot = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("kind") => {
                            kind = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("parent_id") => {
                            parent_id = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("placement") => {
                            if let Expr::Path(p) = &nv.value {
                                if p.path.is_ident("Right") {
                                    align_right = true;
                                }
                            }
                        }
                        Some("mode") => {
                            if let Expr::Path(p) = &nv.value {
                                if p.path.is_ident("Panel") {
                                    visual = true;
                                }
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }

        Ok(ContributeArgs {
            host_id: host_id.ok_or_else(|| {
                syn::Error::new(input.span(), "missing host_id (e.g. host_id = \"demo.shell\")")
            })?,
            id: id.ok_or_else(|| syn::Error::new(input.span(), "missing id"))?,
            name: name.ok_or_else(|| syn::Error::new(input.span(), "missing name"))?,
            description,
            icon,
            order,
            group,
            slot: slot.or(kind),
            parent_id,
            visual,
            align_right,
        })
    }
}

/// `host_id` 只接受字符串字面量（如 `"demo.shell"`），彻底解耦贡献点与宿主类型
fn host_id_tokens(host_id: &Expr) -> TokenStream {
    match host_id {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => quote! { #s },
        _ => quote! {
            compile_error!(
                "host_id must be a string literal (e.g. host_id = \"demo.shell\"). \
                 The host = Type form is removed to decouple contributions from host types."
            )
        },
    }
}

pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let args = match syn::parse2::<ContributeArgs>(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

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
    let visual_extractor_fn = format_ident!(
        "__rml_register_visual_extractor_{}",
        struct_name.to_string().to_lowercase()
    );

    let host_id = host_id_tokens(&args.host_id);
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

    let slot = args
        .slot
        .as_ref()
        .map(|s| quote! { .slot(#s) })
        .unwrap_or_default();

    let parent_id = args
        .parent_id
        .as_ref()
        .map(|p| quote! { .parent_id(#p) })
        .unwrap_or_default();

    let align = if args.align_right {
        quote! { .property("align", "right") }
    } else {
        TokenStream::new()
    };

    let has_component = items.iter().any(|item| {
        matches!(
            item,
            Item::Struct(s) if s.attrs.iter().any(|a| a.path().is_ident("component"))
        )
    });
    let use_component_visual = args.visual || has_component;

    // 视觉贡献契约：`#[contribute]` + `#[component]` 叠加时自动实现。
    // `render` 通过框架实体缓存复用 Entity——避免每次渲染创建新实例导致状态丢失。
    // host 通过 `extract_visual` 获取 `Arc<dyn IVisualContribution>` 后直接调 `render(window, cx)`。
    let visual_impl = if use_component_visual {
        quote! {
            impl rml_core::contribution::IVisualContribution for #struct_name {
                fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
                    let entity = rml_app::contribution::get_or_create_entity::<#struct_name>(cx);
                    entity.update(cx, |this, ctx| {
                        this.render(window, ctx).into_any_element()
                    })
                }
            }
        }
    } else {
        quote! {}
    };

    // 视觉提取器注册：`#[ctor::ctor]` 在进程启动期将 `TypeId::of::<T>()` → 提取器
    // 写入进程级静态表。利用 `Any` supertrait + trait upcasting coercion 实现
    // `Arc<dyn IContribution>` → `Arc<dyn Any + Send + Sync>` → `Arc::downcast::<T>()` → `Arc<dyn IVisualContribution>`。
    let visual_extractor = if use_component_visual {
        quote! {
            #[rml_core::ctor::ctor]
            fn #visual_extractor_fn() {
                rml_app::contribution::register_visual_extractor(
                    std::any::TypeId::of::<#struct_name>(),
                    |contrib: &std::sync::Arc<dyn rml_core::contribution::IContribution>|
                        -> Option<std::sync::Arc<dyn rml_core::contribution::IVisualContribution>>
                    {
                        let any: std::sync::Arc<dyn std::any::Any + Send + Sync> = contrib.clone();
                        any.downcast::<#struct_name>()
                            .ok()
                            .map(|a| a as std::sync::Arc<dyn rml_core::contribution::IVisualContribution>)
                    },
                );
            }
        }
    } else {
        quote! {}
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

        #visual_impl

        #visual_extractor

        /// 由 build.rs 生成的 `register_rml_contributions` 统一调用；用户无需手写清单。
        pub fn #register_fn(cx: &mut gpui::App) {
            use rml_app::contribution::ContributionRegistryExt;
            cx.get_contribution_registry().register(
                #host_id,
                std::sync::Arc::new(#struct_name::default()),
                rml_core::contribution::ContributionOptions::new()
                    #slot
                    #parent_id
                    #order
                    #group
                    #align,
            );
        }
    }
}
