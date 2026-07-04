//! `#[window]` 实现
//!
//! 为标注的结构体生成：
//! - `__rml_state: rml_ui::RmlState` 字段（统一承载窗口句柄 + 组件运行时状态）
//! - `impl IModel` + `impl ILifecycle` + `impl IViewModel` + `impl IComponent`
//! - `include!(OUT_DIR/rml_generated/<snake>.rs)` 注入编译器生成的
//!   `impl IWindow` + `impl Render`
//!
//! **不再生成 `impl IWindow`**——由 RML 编译器从 `<window>` 根节点属性提取并生成。
//! 窗口属性（`title`/`width`/`height`）在 `.rml` 文件的 `<window>` 根节点上声明式配置。
//!
//! 参考 WPF `Window` 类：窗口 IS 组件，额外拥有窗口生命周期操作。

use crate::component::{expand_component_impls, inject_tracking_fields};
use crate::derive_model::to_snake_case;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, ItemStruct};

/// `#[window]` 入口
///
/// 不接受任何属性参数。窗口配置在 `.rml` 根节点 `<window title="..." width="N" height="N">` 上。
pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    // 拒绝任何属性参数
    if !args.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[window] takes no arguments; configure window properties in .rml root element (<window title=\"...\" width=\"N\" height=\"N\">)",
        )
        .to_compile_error();
    }

    let mut item: ItemStruct = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = item.ident.clone();
    let struct_name_str = struct_name.to_string();
    let snake = to_snake_case(&struct_name);

    // 默认模板路径：<snake_case>.rml
    let template_path = format!("{}.rml", snake);

    // 生成文件名（不含扩展名）：<snake_case>
    let generated_file = format!("{}.rs", snake);

    // 校验具名字段（inject_tracking_fields 仅处理 Named）
    if !matches!(item.fields, Fields::Named(_)) {
        return syn::Error::new(item.ident.span(), "#[window] requires named fields")
            .to_compile_error();
    }

    // 注入单一 `__rml_state: rml_ui::RmlState` 字段
    // （含窗口句柄 + 全部组件运行时状态，替代旧的 `__rml_window_handle` + 7+ 类仪式字段）
    inject_tracking_fields(&mut item.fields, &[]);

    // 生成组件 trait 实现（IModel + ILifecycle + IViewModel + IComponent）
    // 注意：不生成 impl IWindow —— 由 RML 编译器从 <window> 根节点生成
    let component_impls = expand_component_impls(
        &struct_name,
        &item.fields,
        &template_path,
        &struct_name_str,
        &[],
    );

    // include! 生成代码（包含编译器生成的 impl IWindow + impl Render）
    let include_stmt = quote! {
        #[allow(non_snake_case, unused_imports, unused_variables, dead_code)]
        include!(concat!(env!("OUT_DIR"), "/rml_generated/", #generated_file));
    };

    // 重新构造 struct（移除 #[element]/#[validate] 等内部属性以免告警）
    let mut item_clean = item.clone();
    crate::validate::strip_internal_attributes(&mut item_clean.fields);

    let expanded = quote! {
        #item_clean

        #component_impls

        #include_stmt
    };

    expanded
}
