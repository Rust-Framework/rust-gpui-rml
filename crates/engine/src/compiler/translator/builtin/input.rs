//! `<input>` translator
//!
//! 原生 `<input>` 支持 `model={field}` 双向绑定；无 model 时退化为普通 div 占位。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::codegen::binding::gen_model_input;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "input",
    display_name: "Input",
    category: ComponentCategory::Form,
    ctor: "gpui::div()",
    is_container: false,
    is_styled: true,
};

#[derive(Debug)]
pub struct InputTranslator;

impl IRmlTranslator for InputTranslator {
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
        if let Some(field) = elem.directives.iter().find_map(|d| match d {
            Directive::Model { field: f, .. } => Some(f.clone()),
            _ => None,
        }) {
            let code = gen_model_input(elem, ctx, id_counter, field, parents)?;
            return Ok((code, false));
        }
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
