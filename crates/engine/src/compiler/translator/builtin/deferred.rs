//! `<deferred>` translator —— 映射到 GPUI 原生 `gpui::deferred(child)`
//!
//! Deferred 延迟子元素绘制（用于 z-order 控制 / overlay 渲染）。
//! 与其他 builtin 元素不同，Deferred 非 ParentElement —— child 必须作为
//! `gpui::deferred(child)` 构造参数传入，而非 `.child()` 链式调用。
//! 因此本 translator 不复用 `BuiltinTranslator`，自行生成构造代码。
//!
//! `priority` 属性映射到 `.with_priority(N)`，控制 z-order（越大越上层）。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::node::gen_node_impl;
use crate::compiler::codegen::text::gen_expr_code;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element};

const TAG: &str = "deferred";
const DISPLAY_NAME: &str = "Deferred";

#[derive(Debug)]
pub struct DeferredTranslator;

impl IRmlTranslator for DeferredTranslator {
    fn tag(&self) -> &'static str {
        TAG
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        // 1. 提取 priority 属性（默认 0）
        let priority: usize = elem
            .attributes
            .iter()
            .find_map(|attr| match attr {
                Attribute::Static { name, value, .. } if name == "priority" => {
                    value.parse::<usize>().ok()
                }
                _ => None,
            })
            .unwrap_or(0);

        // 2. 生成唯一子元素代码
        if elem.children.is_empty() {
            return Err(CodegenError {
                message: "`<deferred>` 必须包含且仅包含一个子元素".to_string(),
                span: Some(elem.span),
            });
        }
        let child_node = &elem.children[0];
        let (child_code, is_iter) =
            gen_node_impl(child_node, ctx, 0, id_counter, loop_vars, parents)?;
        if is_iter {
            return Err(CodegenError {
                message: "`<deferred>` 的子元素不支持 `each` 指令".to_string(),
                span: Some(elem.span),
            });
        }

        // 3. 构造 deferred(child).with_priority(N)
        let mut code = format!("gpui::deferred({{{}}})", child_code);
        if priority > 0 {
            code.push_str(&format!(".with_priority({})", priority));
        }

        // 4. 处理 if / show 指令（不支持 each）
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        let if_cond: Option<String> = elem.directives.iter().find_map(|d| match d {
            Directive::If { expr: c, .. } => Some(c.clone()),
            _ => None,
        });
        let show_cond: Option<String> = if if_cond.is_some() {
            None
        } else {
            elem.directives.iter().find_map(|d| match d {
                Directive::Show { expr: c, .. } => Some(c.clone()),
                _ => None,
            })
        };

        if let Some(cond) = if_cond {
            let cond_code = gen_expr_code(&cond, &lv, &computed);
            let cond_code = cond_code
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.to_string())
                .unwrap_or(cond_code);
            code = format!(
                "if {} {{ {}.into_any_element() }} else {{ gpui::Empty.into_any_element() }}",
                cond_code, code
            );
        } else if let Some(cond) = show_cond {
            let cond_code = gen_expr_code(&cond, &lv, &computed);
            let cond_code = cond_code
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.to_string())
                .unwrap_or(cond_code);
            code = format!("{}.when(!{}, |d| d.invisible())", code, cond_code);
        }

        Ok((code, false))
    }

    fn to_rml(
        &self,
        elem: &Element,
        ctx: &PrinterCtx,
    ) -> Result<String, super::PrintError> {
        super::meta::builtin_engine::print(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new(TAG, DISPLAY_NAME, ComponentCategory::Layout).container(true)
    }
}
