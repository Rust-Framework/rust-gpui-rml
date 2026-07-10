//! `<component>` 透明容器 translator
//!
//! `<component content={expr} />` 通过 `IntoContent` trait 将表达式值转为 `AnyElement` 嵌入，
//! 支持 IntoElement/ToString/IVisual 三类输入。不创建元素包装。
//! 支持 `each` 指令：`<component each={s in status} content={s.render(_window, cx)} />`。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::gen_expr_code;
use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element};

/// `<component>` 透明容器 translator
#[derive(Debug)]
pub struct ComponentTranslator;

impl IRmlTranslator for ComponentTranslator {
    /// 注册表键名，与实际标签 `component` 解耦，避免与 `<component>` 根节点 translator 冲突。
    fn tag(&self) -> &'static str {
        "*component-transparent"
    }

    /// 只匹配带 `content={...}` 的 `<component>` 透明容器；
    /// 根节点 `<component>`（无 content）由 `ComponentRootTranslator` 处理。
    fn matches(&self, elem: &Element) -> bool {
        elem.tag == "component"
            && elem.attributes.iter().any(|a| {
                matches!(a, Attribute::Bind { name, .. } if name == "content")
            })
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let content_expr = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "content" {
                    return Some(expr.clone());
                }
            }
            None
        });

        let expr = content_expr.ok_or_else(|| CodegenError {
            message: "<component> 标签必须提供 content={expr} 属性".to_string(),
            span: Some(elem.span),
        })?;

        // 表达式可引用 render 方法作用域内的 _window/cx，将其加入作用域变量
        let mut scope_vars: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        for v in ["_window", "cx"] {
            if !scope_vars.contains(&v) {
                scope_vars.push(v);
            }
        }

        let each_clause = elem.directives.iter().find_map(|d| match d {
            Directive::Each { clause: c, .. } => Some(c.clone()),
            _ => None,
        });

        if let Some(clause) = &each_clause {
            if !scope_vars.contains(&clause.item.as_str()) {
                scope_vars.push(clause.item.as_str());
            }
            if let Some(idx) = &clause.index {
                if !scope_vars.contains(&idx.as_str()) {
                    scope_vars.push(idx.as_str());
                }
            }
        }

        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        let raw = gen_expr_code(&expr, &scope_vars, &computed);
        // 通过 IntoContent trait 统一转换：支持 IntoElement/ToString/IVisual
        // 表达式可引用 render 方法作用域内的 _window/cx
        let code = format!("rml_core::content::into_content({}, _window, cx)", raw);

        if let Some(clause) = each_clause {
            let iter_expr = if loop_vars.iter().any(|lv| {
                clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv))
            }) {
                clause.iterable.clone()
            } else if computed.contains(&clause.iterable.as_str()) {
                format!(
                    "{}.{}()",
                    expr::current_self_alias().unwrap_or("self"),
                    clause.iterable
                )
            } else {
                format!(
                    "{}.{}",
                    expr::current_self_alias().unwrap_or("self"),
                    clause.iterable
                )
            };
            return Ok((
                format!("{iter_expr}.iter().map(|{}| {{ {} }})", clause.item, code),
                true,
            ));
        }

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("component", "Component", ComponentCategory::Container)
    }
}

/// 注册 `<component>` translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ComponentTranslator);
}
