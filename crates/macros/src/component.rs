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
                fn fields(&self) -> &'static [rml_core::model::FieldMeta] {
                    &[]
                }
            }
        };
    }

    quote! {
        impl rml_core::model::IModel for #struct_name {
            fn fields(&self) -> &'static [rml_core::model::FieldMeta] {
                static FIELDS: [rml_core::model::FieldMeta; #field_count] = [
                    #(#field_metas),*
                ];
                &FIELDS
            }
        }
    }
}

/// 为结构体注入单一 `__rml_state: rml_ui::RmlState` 字段
///
/// `RmlState` 统一承载框架运行时所需的全部状态：
/// - 字段版本追踪（`HashMap<String, AtomicU64>`，替代旧每字段一个 AtomicU64 的设计）
/// - `#[computed]` 缓存
/// - `<input model={field}>` 双向绑定所需的 `InputState` entity 暂存与正向同步版本
/// - 字段校验错误状态
/// - `on_loaded` 一次性初始化守卫
/// - 窗口句柄（由 `#[window]` 使用）
/// - 具名插槽渲染闭包（`HashMap<&'static str, SlotRenderer>`）
///
/// 设计目标：把原本散落在用户结构体中的 7+ 类 `__rml_*` 仪式字段收敛为单一字段，
/// 让 IDE 自动补全与 rustdoc 只显示一个入口，消除视觉噪声。
///
/// `slots` 参数仅为文档目的保留——插槽名通过 `RmlState::set_slot` 动态注册，
/// 不再生成 per-slot 字段。父视图 codegen 调用 `__rml_set_slot_<name>()` setter，
/// setter 内部调用 `self.__rml_state.set_slot("<name>", renderer)`。
pub fn inject_tracking_fields(fields: &mut Fields, _slots: &[String]) {
    let Fields::Named(named) = fields else {
        return;
    };

    let state_field: Field = parse_quote! {
        #[allow(dead_code)]
        __rml_state: rml_ui::RmlState
    };
    named.named.push(state_field);
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
            fn template() -> &'static str {
                #template_path
            }
            fn tag() -> &'static str {
                #struct_name_str
            }
            #slots_override
        }
    };

    // 为每个声明的 slot 生成 setter 方法（`__rml_set_slot_<name>`）
    // 父视图 codegen 在 clone entity 后调用此方法注入 slot 渲染闭包
    let slot_setters = if slots.is_empty() {
        None
    } else {
        let setter_methods: Vec<TokenStream> = slots.iter().map(|slot_name| {
            let method_name = format_ident!("__rml_set_slot_{}", slot_name);
            quote! {
                pub fn #method_name(&mut self, renderer: rml_core::slot::SlotRenderer) {
                    self.__rml_state.set_slot(#slot_name, renderer);
                }
            }
        }).collect();
        Some(quote! {
            #[allow(non_snake_case)]
            impl #struct_name {
                #(#setter_methods)*
            }
        })
    };

    quote! {
        #impl_i_model
        #impl_i_view_model
        #impl_i_component
        #slot_setters
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

    // 注入追踪字段（AtomicU64 + ComputedCache + Slot 字段）
    let mut item = item;
    inject_tracking_fields(&mut item.fields, &slots);

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
