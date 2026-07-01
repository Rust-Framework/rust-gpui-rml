//! `#[contribute]` —— 为类型生成 `IContribution`、注册函数，并由 build.rs 扫描汇总为 `register_rml_contributions`



use proc_macro2::TokenStream;

use quote::{format_ident, quote, ToTokens};

use syn::{

    parse::{Parse, ParseStream, Parser},

    punctuated::Punctuated,

    Expr, Item, LitStr, Token,

};



struct ContributeArgs {

    host: Expr,

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

        let mut host = None;

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

                        Some("host") => {

                            host = Some(syn::parse2(nv.value.clone().into_token_stream())?);

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

            host: host.ok_or_else(|| syn::Error::new(input.span(), "missing host"))?,

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



/// `host = "id"` 或 `host = MyHost`（须实现 [`IContributionHost`]）

fn host_id_tokens(host: &Expr) -> TokenStream {

    match host {

        Expr::Lit(syn::ExprLit {

            lit: syn::Lit::Str(s),

            ..

        }) => quote! { #s },

        Expr::Path(_) => quote! { #host::ID },

        _ => quote! {

            compile_error!("host must be a string literal or a type implementing IContributionHost")

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

    let registerable_impl = if args.visual {

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
                #slot
                #parent_id
                #order
                #group
                #align;

            register_contribution::<#struct_name>(cx, #host_id, contribution, options);

        }

    }

}


