//! `#[contribute]` —— 编译期校验 + 统一注册 + 能力注册函数生成
//!
//! 宏职责（精简后）：
//! 1. 生成 `pub const CONTRIBUTION_ID: &str`
//! 2. 编译期断言目标实现 `IContribution`（用户手写 impl）
//! 3. 生成 `__rml_register_*` 函数：
//!    - 按 `command`/`visual` flag 调用 `ability::register` 注册能力 cast 函数
//!    - 统一调用 `registry.register(host_id, c, Some(opts))`
//! 4. 视觉能力（`visual` flag 或 `#[component]` 叠加）额外生成 `impl IVisual`（仅 `render`）
//!    blanket impl 自动获得 `IVisualContribution: IContribution + IVisual` 标记
//!
//! 宏不再自动生成 `impl IContribution`——用户必须手写。
//!
//! 参数：
//! - 固定：`host_id`/`id`/`parent_id`/`order`/`group`
//! - flag：`command`/`visual`
//! - 任意 `key = "string"` → `ContributionOptions.properties`
//! - `name`/`description`/`icon` 被拒绝（compile_error，提示手写 impl）

use proc_macro2::TokenStream;

use quote::{format_ident, quote, ToTokens};

use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    spanned::Spanned,
    Item, LitStr, Token,
};

struct ContributeArgs {
    host_id: LitStr,
    id: LitStr,
    parent_id: Option<LitStr>,
    order: Option<syn::LitInt>,
    group: Option<LitStr>,
    command: bool,
    visual: bool,
    properties: Vec<(String, LitStr)>,
}

impl Parse for ContributeArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut host_id = None;
        let mut id = None;
        let mut parent_id = None;
        let mut order = None;
        let mut group = None;
        let mut command = false;
        let mut visual = false;
        let mut properties: Vec<(String, LitStr)> = Vec::new();

        let fields: Punctuated<syn::Meta, Token![,]> =
            input.parse_terminated(syn::Meta::parse, Token![,])?;

        for meta in fields {
            match &meta {
                syn::Meta::Path(path) => {
                    if path.is_ident("command") {
                        command = true;
                    } else if path.is_ident("visual") {
                        visual = true;
                    }
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
                        Some("parent_id") => {
                            parent_id = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("order") => {
                            order = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("group") => {
                            group = Some(syn::parse2(nv.value.clone().into_token_stream())?);
                        }
                        Some("name") => {
                            return Err(syn::Error::new(
                                nv.path.span(),
                                "`name` must be hand-written in `impl IContribution` (dynamic i18n supported)",
                            ));
                        }
                        Some("description") => {
                            return Err(syn::Error::new(
                                nv.path.span(),
                                "`description` must be hand-written in `impl IContribution`",
                            ));
                        }
                        Some("icon") => {
                            return Err(syn::Error::new(
                                nv.path.span(),
                                "`icon` must be hand-written in `impl IContribution::icon()`",
                            ));
                        }
                        Some("slot") | Some("kind") => {
                            // `slot`/`kind` 统一进 properties["kind"]
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(s),
                                ..
                            }) = &nv.value
                            {
                                properties.push(("kind".to_string(), s.clone()));
                            } else {
                                return Err(syn::Error::new(
                                    nv.value.span(),
                                    "`kind`/`slot` must be a string literal",
                                ));
                            }
                        }
                        Some("placement") | Some("mode") => {
                            return Err(syn::Error::new(
                                nv.path.span(),
                                "`placement`/`mode` removed: use `visual` flag or `align = \"right\"`",
                            ));
                        }
                        Some(other) => {
                            // 任意扩展属性：必须是字符串字面量
                            if let syn::Expr::Lit(syn::ExprLit {
                                lit: syn::Lit::Str(s),
                                ..
                            }) = &nv.value
                            {
                                properties.push((other.to_string(), s.clone()));
                            } else {
                                return Err(syn::Error::new(
                                    nv.value.span(),
                                    "extra properties must be string literals",
                                ));
                            }
                        }
                        None => {}
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
            parent_id,
            order,
            group,
            command,
            visual,
            properties,
        })
    }
}

/// `host_id` 只接受字符串字面量（如 `"demo.shell"`），彻底解耦贡献点与宿主类型
fn host_id_tokens(host_id: &LitStr) -> TokenStream {
    quote! { #host_id }
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

    let host_id = host_id_tokens(&args.host_id);
    let id = &args.id;

    let parent_id = args
        .parent_id
        .as_ref()
        .map(|p| quote! { .parent_id(#p) })
        .unwrap_or_default();

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

    let properties = args.properties.iter().map(|(k, v)| {
        quote! { .property(#k, #v) }
    }).collect::<Vec<_>>();
    let properties_tokens = quote! { #(#properties)* };

    let has_component = items.iter().any(|item| {
        matches!(
            item,
            Item::Struct(s) if s.attrs.iter().any(|a| a.path().is_ident("component"))
        )
    });
    let use_visual = args.visual || has_component;
    let use_command = args.command;

    // 视觉能力契约:`#[contribute]` + `#[component]` 叠加时自动实现 `IVisual::render`。
    // 用户仍需手写 `impl IContribution`。blanket impl 自动获得 `IVisualContribution` 标记。
    let visual_impl = if use_visual {
        quote! {
            impl rml_core::contribution::IVisual for #struct_name {
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

    // 能力注册：按 flag 注册到 ability registry（幂等）。
    // cast_fn 内部用 trait upcast（&dyn IContribution → &dyn Any）+ downcast_ref::<Self>()
    // 再 trait upcast 到目标能力 trait，最后 erase 为 ErasedAbility fat pointer。
    let command_ability_registration = if use_command {
        quote! {
            rml_core::ability::register::<#struct_name, dyn rml_core::command::ICommand>(
                |c| {
                    let any: &dyn std::any::Any = c;
                    any.downcast_ref::<#struct_name>().map(|s| {
                        let cmd: &dyn rml_core::command::ICommand = s;
                        unsafe { rml_core::ability::erase(cmd) }
                    })
                },
            );
        }
    } else {
        quote! {}
    };

    let visual_ability_registration = if use_visual {
        quote! {
            rml_core::ability::register::<#struct_name, dyn rml_core::contribution::IVisual>(
                |c| {
                    let any: &dyn std::any::Any = c;
                    any.downcast_ref::<#struct_name>().map(|s| {
                        let v: &dyn rml_core::contribution::IVisual = s;
                        unsafe { rml_core::ability::erase(v) }
                    })
                },
            );
        }
    } else {
        quote! {}
    };

    // 无条件注册 `dyn IContribution` 能力——使 `dyn IValue` 可经 `as_contribution()`
    // 查询到贡献元数据（id/name/description/icon）。
    let contribution_ability_registration = quote! {
        rml_core::ability::register::<#struct_name, dyn rml_core::contribution::IContribution>(
            |c| {
                let any: &dyn std::any::Any = c;
                any.downcast_ref::<#struct_name>().map(|s| {
                    let c: &dyn rml_core::contribution::IContribution = s;
                    unsafe { rml_core::ability::erase(c) }
                })
            },
        );
    };

    // 统一注册调用：始终 register（host 自行用 as_command()/as_visual() 分类）
    let register_call = quote! {
        cx.get_contribution_registry().register(
            #host_id,
            std::sync::Arc::new(#struct_name::default()),
            Some(
                rml_core::contribution::ContributionOptions::new()
                    #parent_id
                    #order
                    #group
                    #properties_tokens,
            ),
        );
    };

    // 命令贡献额外断言：目标必须实现 ICommand（用户手写）
    let command_assert = if use_command {
        quote! {
            const _: () = {
                fn assert_command<T: rml_core::command::ICommand>() {}
                fn check_command() { assert_command::<#struct_name>(); }
            };
        }
    } else {
        quote! {}
    };

    quote! {
        #(#items)*

        impl #struct_name {
            pub const CONTRIBUTION_ID: &'static str = #id;
        }

        // 编译期断言：目标必须实现 IContribution（用户手写）
        const _: () = {
            fn assert_contribution<T: rml_core::contribution::IContribution>() {}
            fn check() { assert_contribution::<#struct_name>(); }
        };

        #command_assert

        #visual_impl

        /// 由 build.rs 生成的 `register_rml_contributions_for(cx, host_id)` 按 host_id 分组调用。
        pub fn #register_fn(cx: &mut gpui::App) {
            use rml_app::extensions::IAppContextExt;
            #contribution_ability_registration
            #command_ability_registration
            #visual_ability_registration
            #register_call
        }
    }
}
