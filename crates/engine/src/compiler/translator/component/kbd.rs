//! Kbd 组件 translator
//!
//! 薄包装 `compiler::kbd::gen_kbd`，构造器接受 Keystroke。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct KbdTranslator;

impl IRmlTranslator for KbdTranslator {
    fn tag(&self) -> &'static str {
        "Kbd"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Kbd"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let mut code = crate::compiler::components::kbd::gen_kbd(elem, ctx, id_counter, loop_vars)?;
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, "Kbd", sheet, parents);
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
        TranslatorMetadata::new("Kbd", "Kbd", ComponentCategory::Primitive)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(KbdTranslator);
}
