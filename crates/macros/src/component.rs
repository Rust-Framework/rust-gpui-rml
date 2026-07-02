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
//!
//! **不接受任何属性参数**。模板路径固定为 `<snake_case>.rml`，
//! RML 根节点必须为 `<component>`（或 `<window>`/`<modern_window>` 用于窗口）。

use crate::derive_model::to_snake_case;
use proc_macro2::TokenStream;
use quote::{quote, format_ident};
use syn::{Field, Fields, Ident, ItemStruct, Visibility, parse_quote};

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

/// 为 pub 字段注入版本追踪字段 + ComputedCache 字段 + InputState 存储 + 订阅 guard
///
/// Phase B-2：每个 pub 字段自动成为 observable 字段，宏注入以下字段（均为私有）：
/// - `__rml_<field>_version: AtomicU64`（每个 pub 字段一个，作为版本计数器）
/// - `__rml_computed_cache: ComputedCache`（每结构体一个，存储 #[computed] 结果）
///
/// Phase B-3：双向绑定所需的状态存储：
/// - `__rml_input_states: HashMap<String, Entity<InputState>>`（每结构体一个，惰性存储
///   每个 `<input model={field}>` 绑定的 InputState entity，按字段名索引）
/// - `__rml_input_state_versions: HashMap<String, u64>`（每结构体一个，记录每个字段上次
///   正向同步到 InputState 的版本号，render 时对比决定是否需 set_value）
///
/// 注意：`cx.subscribe` 返回的 `Subscription` 调用 `.detach()` 后随 entity 生命周期存活，
/// 不存储在结构体中（`Subscription` 非 `Sync`，存储会导致视图不满足 `Send + Sync`）。
///
/// 注入字段为私有，不会进入 `IModel::rml_fields()`（其只收集 pub 字段）。
/// `AtomicU64: Default = 0`，`ComputedCache::default() = 空 map`，
/// `HashMap::default() = 空 map`，`Vec::default() = 空 vec`，`#[derive(Default)]` 兼容。
pub fn inject_tracking_fields(fields: &mut Fields) {
    let Fields::Named(named) = fields else {
        return;
    };

    // 收集所有具名字段（pub + private）——版本追踪需要覆盖全部字段，
    // 因为 #[computed] 方法可能依赖私有字段。IModel 的 pub 过滤在 gen_impl_i_model 中独立处理。
    let field_names: Vec<String> = named
        .named
        .iter()
        .filter_map(|f| f.ident.as_ref().map(|i| i.to_string()))
        .collect();

    // 为每个字段注入 AtomicU64 版本计数器
    for name in &field_names {
        let version_field_name = format_ident!("__rml_{}_version", name);
        let field: Field = parse_quote! {
            #[allow(non_snake_case, dead_code)]
            #version_field_name: std::sync::atomic::AtomicU64
        };
        named.named.push(field);
    }

    // 注入 ComputedCache（供 #[computed] 缓存包装使用）
    let cache_field: Field = parse_quote! {
        #[allow(dead_code)]
        __rml_computed_cache: rml_core::computed_cache::ComputedCache
    };
    named.named.push(cache_field);

    // Phase B-3：注入 InputState 存储（供双向绑定 <input model={field}> 惰性初始化使用）
    let input_states_field: Field = parse_quote! {
        #[allow(dead_code)]
        __rml_input_states: std::collections::HashMap<String, gpui::Entity<rml_ui::InputState>>
    };
    named.named.push(input_states_field);

    // Phase B-3：注入正向同步版本追踪（记录每个字段上次同步到 InputState 的版本号）
    // render 时对比 __rml_get_version(field) 与此值，若不同则调用 InputState::set_value
    //
    // 注意：不注入 Vec<Subscription> 字段——Subscription 非 Sync，会导致视图类型不满足
    // Send + Sync 约束。改用 cx.subscribe(...).detach() 让订阅随 entity 生命周期存活。
    let input_versions_field: Field = parse_quote! {
        #[allow(dead_code)]
        __rml_input_state_versions: std::collections::HashMap<String, u64>
    };
    named.named.push(input_versions_field);

    // Phase B-3.1：注入字段校验错误状态（记录每个字段的校验失败信息）
    // None = 校验通过，Some(msg) = 校验失败（红色边框 + tooltip 显示 msg）
    // 反向闭包 parse 失败时设置 Some，成功时清除为 None；正向同步 set_value 后清除为 None
    let field_errors_field: Field = parse_quote! {
        #[allow(dead_code)]
        __rml_field_errors: std::collections::HashMap<String, Option<gpui::SharedString>>
    };
    named.named.push(field_errors_field);

    let loaded_field: Field = parse_quote! {
        #[allow(dead_code)]
        __rml_loaded: bool
    };
    named.named.push(loaded_field);
}

/// 生成组件所需的全部 trait 实现（IModel + ILifecycle + IViewModel + IComponent）
///
/// 供 `#[component]` 和 `#[window]` 共用。
/// `slots` 为组件声明的具名插槽列表（来自 `#[component(slots = [...])]`），
/// 空切片表示不接受任何插槽（IComponent::slots 默认实现返回 &[]，无需覆写）。
pub fn expand_component_impls(
    struct_name: &Ident,
    fields: &Fields,
    template_path: &str,
    struct_name_str: &str,
    slots: &[String],
) -> TokenStream {
    let impl_i_model = gen_impl_i_model(struct_name, fields);

    let impl_i_view_model = quote! {
        impl rml_core::view_model::IViewModel for #struct_name {}
    };

    // 仅当 slots 非空时覆写 IComponent::slots()，避免与默认实现重复
    let slots_override = if slots.is_empty() {
        None
    } else {
        let slot_literals: Vec<&str> = slots.iter().map(|s| s.as_str()).collect();
        Some(quote! {
            fn slots() -> &'static [&'static str] {
                &[#(#slot_literals),*]
            }
        })
    };

    let impl_i_component = quote! {
        impl rml_core::component::IComponent for #struct_name {
            fn rml_template() -> &'static str {
                #template_path
            }
            fn rml_tag() -> &'static str {
                #struct_name_str
            }
            #slots_override
        }
    };

    quote! {
        #impl_i_model
        #impl_i_view_model
        #impl_i_component
    }
}

/// `#[component]` 入口
///
/// 可选参数 `slots = ["header", "footer", "default"]` 声明具名插槽列表。
/// 模板路径固定为 `<snake_case>.rml`。
pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let slots = match parse_component_args(&args) {
        Ok(s) => s,
        Err(e) => return e.to_compile_error(),
    };

    let item: ItemStruct = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };
    let struct_name = item.ident.clone();
    let struct_name_str = struct_name.to_string();
    let snake = to_snake_case(&struct_name);

    // 模板路径固定为 <snake_case>.rml
    let template_path = format!("{}.rml", snake);

    // 生成文件名（不含扩展名）：<snake_case>
    let generated_file = format!("{}.rs", snake);

    // 注入追踪字段（AtomicU64 + ComputedCache）
    let mut item = item;
    inject_tracking_fields(&mut item.fields);

    // 生成全部 trait 实现
    let trait_impls = expand_component_impls(
        &struct_name,
        &item.fields,
        &template_path,
        &struct_name_str,
        &slots,
    );

    // include! 生成代码
    let include_stmt = quote! {
        #[allow(non_snake_case, unused_imports, unused_variables, dead_code)]
        include!(concat!(env!("OUT_DIR"), "/rml_generated/", #generated_file));
    };

    // 重新构造 struct（移除 #[element]/#[validate] 等内部属性以免告警）
    let mut item_clean = item.clone();
    crate::validate::strip_internal_attributes(&mut item_clean.fields);

    let expanded = quote! {
        #item_clean

        #trait_impls

        #include_stmt
    };

    expanded
}

/// 解析 `#[component(...)]` 参数
///
/// 当前支持：
/// - `slots = ["name1", "name2", ...]`：声明具名插槽列表
///
/// 未来可扩展更多参数（如 `template = "..."`）。
fn parse_component_args(args: &TokenStream) -> syn::Result<Vec<String>> {
    if args.is_empty() {
        return Ok(Vec::new());
    }

    // 尝试解析为 `slots = [...]` 形式
    let parsed: syn::Result<ComponentArgs> = syn::parse2(args.clone());
    match parsed {
        Ok(ComponentArgs { slots }) => Ok(slots),
        Err(_) => Err(syn::Error::new(
            proc_macro2::Span::call_site(),
            "expected #[component(slots = [\"name1\", \"name2\"])]",
        )),
    }
}

/// `#[component(...)]` 参数结构
struct ComponentArgs {
    slots: Vec<String>,
}

impl syn::parse::Parse for ComponentArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut slots = Vec::new();

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "slots" {
                let _eq: syn::Token![=] = input.parse()?;
                let arr: syn::ExprArray = input.parse()?;
                for expr in arr.elems {
                    let lit: syn::LitStr = syn::parse2(quote! { #expr })?;
                    slots.push(lit.value());
                }
            } else {
                return Err(syn::Error::new(ident.span(), "unknown argument, expected `slots`"));
            }

            // 允许逗号分隔多个参数（为未来扩展预留）
            if !input.is_empty() {
                let _comma: syn::Token![,] = input.parse()?;
            }
        }

        Ok(ComponentArgs { slots })
    }
}
