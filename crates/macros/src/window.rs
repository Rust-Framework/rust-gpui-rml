//! `#[window]` 实现
//!
//! 在 `#[component]` 基础上额外生成：
//! - 窗口句柄字段（`__rml_window_handle: Option<AnyWindowHandle>`）
//! - `impl IWindow`（含 title/width/height/open/handle/set_handle）
//!
//! 窗口操作（close/show/hide/activate/state）由 `IWindow` trait 默认实现提供，
//! 基于 `handle()` 调用 GPUI API，无需宏重复生成。
//!
//! 参考 WPF `Window` 类：窗口 IS 组件，额外拥有窗口生命周期操作。

use crate::component::expand_component_impls;
use crate::derive_model::to_snake_case;
use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use syn::{Field, Fields, Ident, ItemStruct};

/// `#[window]` 属性参数
struct WindowArgs {
    title: Option<String>,
    width: Option<f32>,
    height: Option<f32>,
    template: Option<String>,
}

/// 解析 `#[window(title = "...", width = 800, height = 600)]` 参数
fn parse_window_args(args: TokenStream) -> WindowArgs {
    let mut result = WindowArgs {
        title: None,
        width: None,
        height: None,
        template: None,
    };

    if args.is_empty() {
        return result;
    }

    // 解析逗号分隔的 name = value 列表
    use syn::parse::Parser;
    let parser = syn::punctuated::Punctuated::<syn::MetaNameValue, syn::Token![,]>::parse_terminated;
    if let Ok(nv_list) = parser.parse2(args.clone()) {
        for nv in nv_list {
            let key = nv.path;
            if key.is_ident("title") {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = nv.value {
                    result.title = Some(s.value());
                }
            } else if key.is_ident("width") {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Float(f), .. }) = nv.value {
                    result.width = f.base10_parse().ok();
                } else if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) = nv.value {
                    result.width = i.base10_parse().ok();
                }
            } else if key.is_ident("height") {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Float(f), .. }) = nv.value {
                    result.height = f.base10_parse().ok();
                } else if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Int(i), .. }) = nv.value {
                    result.height = i.base10_parse().ok();
                }
            } else if key.is_ident("template") {
                if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = nv.value {
                    result.template = Some(s.value());
                }
            }
        }
    }

    result
}

/// 生成 `impl IWindow` 实现
///
/// 仅生成核心方法（title/width/height/open/handle/set_handle），
/// 窗口操作（close/show/hide/activate/state）由 trait 默认实现提供。
fn gen_impl_iwindow(struct_name: &Ident, args: &WindowArgs) -> TokenStream {
    let title = args.title.as_deref().unwrap_or("RML Window");
    let width = args.width.unwrap_or(800.0);
    let height = args.height.unwrap_or(600.0);

    quote! {
        impl rml_core::window::IWindow for #struct_name {
            fn title(&self) -> &str {
                #title
            }

            fn width(&self) -> gpui::Pixels {
                gpui::px(#width)
            }

            fn height(&self) -> gpui::Pixels {
                gpui::px(#height)
            }

            fn open(&mut self, cx: &mut gpui::App) {
                let options = self.window_options();
                let handle = cx.open_window(options, |window, cx| {
                    let view = cx.new(|_cx| Self::default());
                    cx.new(|cx| rml_ui::Root::new(view, window, cx))
                }).expect("failed to open window");
                self.__rml_window_handle = Some(handle.into());
            }

            fn handle(&self) -> Option<gpui::AnyWindowHandle> {
                self.__rml_window_handle
            }

            fn set_handle(&mut self, handle: gpui::AnyWindowHandle) {
                self.__rml_window_handle = Some(handle);
            }
        }
    }
}

/// `#[window]` 入口
pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut item: ItemStruct = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    let struct_name = item.ident.clone();
    let struct_name_str = struct_name.to_string();
    let snake = to_snake_case(&struct_name);

    // 解析 #[window(...)] 参数
    let window_args = parse_window_args(args);

    // 默认模板路径：<snake_case>.rml
    let template_path = window_args.template.clone()
        .unwrap_or_else(|| format!("{}.rml", snake));

    // 生成文件名（不含扩展名）：<snake_case>
    let generated_file = format!("{}.rs", snake);

    // 添加窗口句柄字段
    match &mut item.fields {
        Fields::Named(named) => {
            let handle_field: Field = parse_quote! {
                #[allow(dead_code, non_snake_case)]
                __rml_window_handle: Option<gpui::AnyWindowHandle>
            };
            named.named.push(handle_field);
        }
        _ => {
            return syn::Error::new(item.ident.span(), "#[window] requires named fields")
                .to_compile_error();
        }
    }

    // 生成组件 trait 实现（IModel + ILifecycle + IViewModel + IComponent）
    let component_impls = expand_component_impls(
        &struct_name,
        &item.fields,
        &template_path,
        &struct_name_str,
    );

    // 生成 IWindow 实现
    let iwindow_impl = gen_impl_iwindow(&struct_name, &window_args);

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

        #component_impls

        #iwindow_impl

        #include_stmt
    };

    expanded
}
