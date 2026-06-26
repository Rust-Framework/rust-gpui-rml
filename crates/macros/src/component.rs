//! `#[component]` 实现
//!
//! 为标注的结构体生成：
//! - `impl IModel`（字段元信息）
//! - `impl ILifecycle`（默认空，由 `#[on_loaded]`/`#[on_unloaded]` 覆写）
//! - `impl IViewModel`
//! - `impl IComponent`（声明模板路径 + 标签名）
//! - `include!(concat!(env!("OUT_DIR"), "/rml_generated/<name>.rs"))`
//!
//! 合并自旧 `#[view]` + `#[component]`。

use crate::derive_model::to_snake_case;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, ItemStruct, Meta, Visibility};

/// 解析 `#[component]` 或 `#[component(template = "path")]` 的参数
pub fn parse_template_arg(args: TokenStream) -> Option<String> {
    if args.is_empty() {
        return None;
    }
    // 尝试解析为 `template = "path"`
    if let Ok(meta) = syn::parse2::<Meta>(args.clone()) {
        if let Meta::NameValue(nv) = meta {
            if nv.path.is_ident("template") {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = nv.value {
                    return Some(s.value());
                }
            }
        }
    }
    None
}

/// 生成 `impl IModel`（同 derive_model 的核心逻辑）
fn gen_impl_i_model(struct_name: &Ident, fields: &Fields) -> TokenStream {
    let field_metas: Vec<TokenStream> = fields.iter().filter_map(|f| {
        let is_pub = matches!(f.vis, Visibility::Public(_));
        if !is_pub {
            return None;
        }
        let name = match f.ident.as_ref() {
            Some(i) => i,
            None => return None,
        };
        let name_str = name.to_string();
        let ty = &f.ty;
        let ty_str = quote!(#ty).to_string().replace(' ', "");
        Some(quote! {
            rml_core::model::FieldMeta { name: #name_str, ty: #ty_str }
        })
    }).collect();

    let field_count = field_metas.len();

    if field_count == 0 {
        return quote! {
            impl rml_core::model::IModel for #struct_name {
                fn rml_fields(&self) -> &'static [rml_core::model::FieldMeta] {
                    &[]
                }
            }
        };
    }

    quote! {
        impl rml_core::model::IModel for #struct_name {
            fn rml_fields(&self) -> &'static [rml_core::model::FieldMeta] {
                static FIELDS: [rml_core::model::FieldMeta; #field_count] = [
                    #(#field_metas),*
                ];
                &FIELDS
            }
        }
    }
}

/// 生成组件所需的全部 trait 实现（IModel + ILifecycle + IViewModel + IComponent）
///
/// 供 `#[component]` 和 `#[window]` 共用。
pub fn expand_component_impls(
    struct_name: &Ident,
    fields: &Fields,
    template_path: &str,
    struct_name_str: &str,
) -> TokenStream {
    let impl_i_model = gen_impl_i_model(struct_name, fields);

    let impl_i_lifecycle = quote! {
        impl rml_core::lifecycle::ILifecycle for #struct_name {
            fn rml_on_loaded(&mut self, _cx: &mut gpui::Context<Self>) {}
            fn rml_on_unloaded(&mut self, _cx: &mut gpui::Context<Self>) {}
        }
    };

    let impl_i_view_model = quote! {
        impl rml_core::view_model::IViewModel for #struct_name {}
    };

    let impl_i_component = quote! {
        impl rml_core::component::IComponent for #struct_name {
            fn rml_template() -> &'static str {
                #template_path
            }
            fn rml_tag() -> &'static str {
                #struct_name_str
            }
        }
    };

    quote! {
        #impl_i_model
        #impl_i_lifecycle
        #impl_i_view_model
        #impl_i_component
    }
}

/// `#[component]` 入口
pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let item: ItemStruct = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };
    let struct_name = item.ident.clone();
    let struct_name_str = struct_name.to_string();
    let snake = to_snake_case(&struct_name);

    // 默认模板路径：<snake_case>.rml
    let template_path = parse_template_arg(args).unwrap_or_else(|| format!("{}.rml", snake));

    // 生成文件名（不含扩展名）：<snake_case>
    let generated_file = format!("{}.rs", snake);

    // 生成全部 trait 实现
    let trait_impls = expand_component_impls(
        &struct_name,
        &item.fields,
        &template_path,
        &struct_name_str,
    );

    // include! 生成代码
    let include_stmt = quote! {
        #[allow(non_snake_case, unused_imports, unused_variables, dead_code)]
        include!(concat!(env!("OUT_DIR"), "/rml_generated/", #generated_file));
    };

    // 重新构造 struct（移除 #[element] 等内部属性以免告警）
    let mut item_clean = item.clone();
    for f in item_clean.fields.iter_mut() {
        f.attrs.retain(|a| !a.path().is_ident("element"));
    }

    let expanded = quote! {
        #item_clean

        #trait_impls

        #include_stmt
    };

    expanded
}
