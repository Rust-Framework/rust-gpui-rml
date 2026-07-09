//! Tabs 容器组件 translator
//!
//! 薄包装 `compiler::tabs::gen_tabs`，构造 + 属性 + Tab 子节点注入。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct TabsTranslator;

impl IRmlTranslator for TabsTranslator {
    fn tag(&self) -> &'static str {
        "Tabs"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Tabs"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let id_val = *id_counter;
        *id_counter += 1;

        let mut code = crate::compiler::components::tabs::gen_tabs(
            elem,
            ref_name,
            id_val,
            ctx,
            id_counter,
            loop_vars,
        )?;

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, "Tabs", sheet, parents);
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
        TranslatorMetadata::new("Tabs", "Tabs", ComponentCategory::Navigation).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(TabsTranslator);
}
