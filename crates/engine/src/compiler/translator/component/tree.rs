//! Tree 组件 translator
//!
//! Tree 构造器使用 `as_ref()` 而非 `&` 引用（与其他 Stateful 组件不同），
//! 因此从 `StatefulComponentTranslator` 独立出来。
//! 薄包装 `compiler::tree::gen_tree`，并应用静态/bind/event setter + CSS。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};
use crate::tags;

#[derive(Debug)]
pub struct TreeTranslator;

impl IRmlTranslator for TreeTranslator {
    fn tag(&self) -> &'static str {
        "Tree"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Tree"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let tag = &elem.tag;
        let resolved = tags::normalize_component_tag(tag);
        let component = tags::component_lookup_resolved(tag)
            .ok_or_else(|| CodegenError {
                message: format!("unknown component: <{}>", tag),
                span: Some(elem.span),
            })?;

        let mut code =
            crate::compiler::components::tree::gen_tree(elem, component, ctx, 0, id_counter, loop_vars)?;

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    if let Some(setter) = component_static_setter(name, value, &resolved) {
                        code.push_str(&setter);
                    } else {
                        crate::compiler::setters::check_missing_mapping(
                            ctx, &resolved, name, "static",
                        )?;
                    }
                }
                Attribute::Bind { name, expr, .. } => {
                    if let Some(setter) =
                        component_bind_setter(name, expr, &lv, &computed, &resolved)
                    {
                        code.push_str(&setter);
                    } else {
                        crate::compiler::setters::check_missing_mapping(
                            ctx, &resolved, name, "bind",
                        )?;
                    }
                }
                Attribute::Event { name, handler, .. } => {
                    if let Some(setter) = component_event_setter(name, handler, &resolved) {
                        code.push_str(&setter);
                    }
                }
            }
        }

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, tag, sheet, parents);
            if !style_code.is_empty() {
                code.push_str(&style_code);
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Tree", "Tree", ComponentCategory::Data).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(TreeTranslator);
}
