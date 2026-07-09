//! `<code>` translator

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "code",
    display_name: "Code",
    category: ComponentCategory::Primitive,
    ctor: "gpui::div()",
    is_container: true,
    is_self_closing: true,
    is_styled: true,
};

#[derive(Debug)]
pub struct CodeTranslator;

impl IRmlTranslator for CodeTranslator {
    fn tag(&self) -> &'static str {
        META.tag
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        BuiltinTranslator { meta: META }.to_rust(elem, ctx, id_counter, loop_vars, parents)
    }

    fn to_rml(
        &self,
        elem: &Element,
        ctx: &super::PrinterCtx,
    ) -> Result<String, super::PrintError> {
        BuiltinTranslator { meta: META }.to_rml(elem, ctx)
    }

    fn metadata(&self) -> super::TranslatorMetadata {
        META.to_metadata()
    }
}
