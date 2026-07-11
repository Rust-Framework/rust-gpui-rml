//! Field 表单字段 translator
//!
//! RML `<Field>` 编译为 `gpui_component::form::Field`，实现 ParentElement。
//! 典型子节点：Input、Switch、Checkbox、Select 等表单控件。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct FieldTranslator;

impl IRmlTranslator for FieldTranslator {
    fn tag(&self) -> &'static str {
        "Field"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Field"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::field::gen_field(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Field", "Field", ComponentCategory::Layout).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(FieldTranslator);
}
