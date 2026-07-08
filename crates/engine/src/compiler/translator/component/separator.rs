//! Separator 组件 translator
//!
//! 薄包装 `compiler::separator::gen_separator`，horizontal/vertical/dashed 变体构造。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct SeparatorTranslator;

impl IRmlTranslator for SeparatorTranslator {
    fn tag(&self) -> &'static str {
        "Separator"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Separator"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let mut code = crate::compiler::separator::gen_separator(elem, ctx, id_counter, loop_vars)?;
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, "Separator", sheet, parents);
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
        TranslatorMetadata::new("Separator", "Separator", ComponentCategory::Primitive)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(SeparatorTranslator);
}
