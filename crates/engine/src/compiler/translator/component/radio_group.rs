//! RadioGroup 组件 translator
//!
//! 薄包装 `compiler::radio_group::gen_radio_group`，horizontal/vertical 构造器 + Radio 子节点。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct RadioGroupTranslator;

impl IRmlTranslator for RadioGroupTranslator {
    fn tag(&self) -> &'static str {
        "RadioGroup"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "RadioGroup"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::radio_group::gen_radio_group(elem, ctx, id_counter, loop_vars, parents)?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("RadioGroup", "RadioGroup", ComponentCategory::Form).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(RadioGroupTranslator);
}
