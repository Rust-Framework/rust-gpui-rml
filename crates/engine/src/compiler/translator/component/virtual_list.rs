//! VirtualList 专用 translator —— v_virtual_list/h_virtual_list 函数构造 + slot="render" 闭包注入
//!
//! VirtualList 构造器为函数 `v_virtual_list(view, id, item_sizes, f)`，非 `VirtualList::new(id)`。
//! 核心难点是渲染闭包 `Fn(&mut V, Range<usize>, &mut Window, &mut Context<V>) -> Vec<R>` 的注入：
//! 通过 `<template slot="render" each={i in range}>` 声明循环变量，translator 生成闭包体。
//!
//! ## 生成代码示例
//!
//! ```ignore
//! rml_ui::v_virtual_list(
//!     cx.entity(),
//!     ("rml_vlist", 0usize),
//!     self.item_sizes.clone(),
//!     move |this: &mut Self, range: std::ops::Range<usize>, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>| {
//!         range.into_iter().map(|i| {
//!             gpui::div().child(rml_ui::Label::new(&this.items[i].name))
//!         }).collect::<Vec<_>>()
//!     }
//! )
//! ```
//!
//! ## self_alias 机制
//!
//! 闭包内 `this: &mut Self` 是 ViewModel 引用（非 `self`）。
//! 使用 `with_self_alias("this", ...)` 让模板体中的字段访问生成 `this.field` 而非 `self.field`。
//! `range`/`_window`/`cx`/`i` 作为 scope_vars 加入 loop_vars，避免被加 `this.` 前缀。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::expr::with_self_alias;
use crate::compiler::setters::{component_bind_rust_expr, component_static_setter};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};
use crate::tags;

#[derive(Debug)]
pub struct VirtualListTranslator;

impl IRmlTranslator for VirtualListTranslator {
    fn tag(&self) -> &'static str {
        "VirtualList"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "VirtualList"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let id_val = *id_counter;
        *id_counter += 1;

        // 1. direction 属性 → 选择 v/h 构造器（默认 vertical）
        let direction = extract_static_attr(elem, "direction").unwrap_or_else(|| "vertical".to_string());
        let ctor_fn = match direction.as_str() {
            "horizontal" | "h" => "rml_ui::h_virtual_list",
            _ => "rml_ui::v_virtual_list",
        };

        // 2. item-sizes 绑定表达式 → self.item_sizes（或 slot 上下文 __rml_self_ref.item_sizes）
        let item_sizes_expr = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "item_sizes" {
                    return Some(expr.clone());
                }
            }
            None
        }).ok_or_else(|| CodegenError {
            message: "<virtual-list> 必须提供 item-sizes={expr} 绑定属性（Rc<Vec<Size<Pixels>>>）".to_string(),
            span: Some(elem.span),
        })?;

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        let item_sizes_rust = component_bind_rust_expr(&item_sizes_expr, &lv, &computed);

        // 3. 查找 <template slot="render" each={i in range}> 子节点
        let render_template = find_render_template(elem).ok_or_else(|| CodegenError {
            message: "<virtual-list> 必须包含 <template slot=\"render\" each={i in range}> 子节点".to_string(),
            span: Some(elem.span),
        })?;

        // 4. 提取 each 子句的循环变量
        let each_clause = render_template.directives.iter().find_map(|d| {
            if let Directive::Each { clause, .. } = d {
                Some(clause.clone())
            } else {
                None
            }
        }).ok_or_else(|| CodegenError {
            message: "<template slot=\"render\"> 必须使用 each={i in range} 声明循环变量".to_string(),
            span: Some(render_template.span),
        })?;

        let loop_var = each_clause.item.clone();

        // 5. 生成模板体代码（self_alias="this"，loop_var + range/_window/cx 作为 scope vars）
        let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
        child_loop_vars.push(loop_var.clone());
        child_loop_vars.push("range".to_string());
        child_loop_vars.push("_window".to_string());
        child_loop_vars.push("cx".to_string());

        let body_result = with_self_alias("this", || {
            gen_template_body(render_template, ctx, id_counter, &child_loop_vars)
        });
        let body_code = body_result?;

        // 6. 组装闭包 + 函数调用
        let closure = format!(
            "move |this: &mut Self, range: std::ops::Range<usize>, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>| {{\n    \
             range.into_iter().map(|{}| {{\n        \
             {}\n    \
             }}).collect::<Vec<_>>()\n\
             }}",
            loop_var, body_code
        );

        let mut code = format!(
            "{}(cx.entity(), (\"rml_vlist\", {}usize), std::rc::Rc::new({}.clone()), {})",
            ctor_fn, id_val, item_sizes_rust, closure
        );

        // 7. CSS class 样式
        append_css_class_styles(&mut code, elem, &elem.tag, ctx.stylesheet.as_ref(), parents);

        // 8. 应用剩余 static setter（跳过 direction，item_sizes 是 bind 属性在循环中跳过）
        let resolved = tags::normalize_component_tag(&elem.tag);
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    if name == "direction" {
                        continue;
                    }
                    if let Some(setter) = component_static_setter(name, value, &resolved) {
                        code.push_str(&setter);
                    }
                }
                Attribute::Bind { name, .. } => {
                    // item_sizes 已作为函数参数处理，跳过
                    if name == "item_sizes" {
                        continue;
                    }
                }
                Attribute::Event { .. } => {}
            }
        }

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("VirtualList", "VirtualList", ComponentCategory::Layout)
    }
}

/// 从元素属性中提取 static 字符串值
fn extract_static_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            if n == name {
                return Some(value.clone());
            }
        }
        None
    })
}

/// 查找 <template slot="render"> 子节点
///
/// `slot="render"` 由 parser 解析到 `element.slot_name` 字段（不进入 attributes），
/// 因此此处检查 `slot_name == Some("render")` 而非遍历 attributes。
fn find_render_template(elem: &Element) -> Option<&Element> {
    elem.children.iter().find_map(|child| {
        if let Node::Element(e) = child {
            if e.tag == "template" && e.slot_name.as_deref() == Some("render") {
                return Some(e);
            }
        }
        None
    })
}

/// 生成模板体代码：单子节点直接生成，多子节点包裹在 div 中
fn gen_template_body(
    template: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    if template.children.is_empty() {
        return Err(CodegenError {
            message: "<template slot=\"render\"> 不能为空".to_string(),
            span: Some(template.span),
        });
    }

    if template.children.len() == 1 {
        let (code, _) = gen_node(&template.children[0], ctx, 0, id_counter, loop_vars)?;
        return Ok(code);
    }

    // 多子节点：包裹在 gpui::div() 中
    let mut code = String::from("gpui::div()");
    for child in &template.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", child_code));
        } else {
            code.push_str(&format!(".child({})", child_code));
        }
    }
    Ok(code)
}

/// 注册 VirtualList translator
pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(VirtualListTranslator);
}
