//! `#[contributehost]` host marker + framework registration.

use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};
use syn::spanned::Spanned;
use syn::{
    parse::{Parse, ParseStream, Parser},
    punctuated::Punctuated,
    Fields, Item, LitStr, Token,
};

struct ContributeHostArgs {
    id: LitStr,
    bindings: Option<LitStr>,
}

impl Parse for ContributeHostArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut id = None;
        let mut bindings = None;

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
                Some("bindings") => {
                    bindings = Some(syn::parse2(nv.value.into_token_stream())?);
                }
                Some("on_changed") => {
                    return Err(syn::Error::new(
                        nv.path.span(),
                        "on_changed is removed: use bindings = \"refresh_method\" or rely on automatic cx.notify()",
                    ));
                }
                _ => {}
            }
        }

        Ok(ContributeHostArgs {
            id: id.ok_or_else(|| syn::Error::new(input.span(), "missing id"))?,
            bindings,
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

    let struct_name = items.iter_mut().find_map(|item| {
        if let Item::Struct(s) = item {
            if args.bindings.is_some() {
                let has_field = match &s.fields {
                    Fields::Named(fields) => fields.named.iter().any(|f| {
                        f.ident
                            .as_ref()
                            .is_some_and(|i| i == "__rml_contribution_bindings_attached")
                    }),
                    _ => false,
                };
                if !has_field {
                    if let Fields::Named(fields) = &mut s.fields {
                        fields.named.push(syn::parse_quote! {
                            #[doc(hidden)]
                            __rml_contribution_bindings_attached: bool
                        });
                    }
                }
            }
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

    let attach_bindings = if let Some(method) = &args.bindings {
        let method = format_ident!("{}", method.value());
        quote! {
            impl #struct_name {
                #[doc(hidden)]
                fn __rml_attach_contribution_bindings(&mut self, cx: &mut gpui::Context<Self>) {
                    if self.__rml_contribution_bindings_attached {
                        return;
                    }
                    self.__rml_contribution_bindings_attached = true;
                    use rml_app::contribution::subscribe_host_changes;
                    subscribe_host_changes(Self::ID, cx, |this, cx| {
                        this.#method(cx);
                        cx.notify();
                    });
                    self.#method(cx);
                    cx.notify();
                }
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #(#items)*

        impl #struct_name {
            pub const ID: &'static str = #id;
        }

        impl rml_core::contribution::IContributionHost for #struct_name {
            const ID: &'static str = #id;
        }

        #attach_bindings

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
