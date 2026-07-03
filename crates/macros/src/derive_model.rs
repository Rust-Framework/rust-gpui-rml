//! `#[derive(IModel)]` 实现
//!
//! 为结构体的所有 `pub` 字段生成 `FieldMeta`，
//! 实现 `IModel::fields()` 返回字段元信息。

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Ident};

pub fn derive(input: TokenStream) -> TokenStream {
    let input: DeriveInput = match syn::parse2(input) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = &input.ident;

    // 仅支持 struct，不支持 enum / union
    let fields = match &input.data {
        Data::Struct(s) => &s.fields,
        _ => {
            return syn::Error::new_spanned(
                &input,
                "#[derive(IModel)] only supports structs",
            )
            .to_compile_error();
        }
    };

    // 收集所有 pub 字段（具名或元组）
    let field_metas: Vec<TokenStream> = fields.iter().filter_map(|f| {
        let is_pub = matches!(f.vis, syn::Visibility::Public(_));
        if !is_pub {
            return None;
        }
        let name = match f.ident.as_ref() {
            Some(i) => i,
            None => return None, // 元组字段跳过（无名字无法绑定）
        };
        let name_str = name.to_string();
        let ty = &f.ty;
        let ty_str = quote!(#ty).to_string().replace(' ', "");
        Some(quote! {
            rml_core::model::FieldMeta { name: #name_str, ty: #ty_str }
        })
    }).collect();

    let field_count = field_metas.len();

    let expanded = quote! {
        impl rml_core::model::IModel for #struct_name {
            fn fields(&self) -> &'static [rml_core::model::FieldMeta] {
                static FIELDS: [rml_core::model::FieldMeta; #field_count] = [
                    #(#field_metas),*
                ];
                &FIELDS
            }
        }
    };

    // 处理 0 字段情况（static 数组大小为 0）
    if field_count == 0 {
        return quote! {
            impl rml_core::model::IModel for #struct_name {
                fn fields(&self) -> &'static [rml_core::model::FieldMeta] {
                    &[]
                }
            }
        };
    }

    expanded
}

/// 将 PascalCase 标识符转为 snake_case（如 `Counter` → `counter`）
pub fn to_snake_case(ident: &Ident) -> String {
    let s = ident.to_string();
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}
