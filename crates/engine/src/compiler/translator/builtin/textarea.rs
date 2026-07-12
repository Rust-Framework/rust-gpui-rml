//! `<textarea>` translator
//!
//! 原生 `<textarea>` 支持 `value={field}` 自动双向绑定（复用 InputState 双向同步机制）。
//! 无双向绑定时退化为普通 div 占位。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::codegen::binding::gen_model_input;
use crate::compiler::codegen::extract_field_converter;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "textarea",
    display_name: "TextArea",
    category: ComponentCategory::Form,
    ctor: "gpui::div()",
    is_container: false,
    is_styled: true,
};

#[derive(Debug)]
pub struct TextAreaTranslator;

impl IRmlTranslator for TextAreaTranslator {
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
        // value={field} bind 属性自动双向绑定
        if let Some(expr) = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                (name == "value").then(|| expr.clone())
            } else {
                None
            }
        }) {
            let (field, _) = extract_field_converter(&expr);
            let code = gen_model_input(elem, ctx, id_counter, field, true, parents)?;
            return Ok((
                crate::compiler::components::key_binding::apply_key_bindings_to_host(
                    elem, code, ctx, id_counter, loop_vars, parents,
                )?,
                false,
            ));
        }
        let (code, is_iter) =
            BuiltinTranslator { meta: META }.to_rust(elem, ctx, id_counter, loop_vars, parents)?;
        Ok((
            crate::compiler::components::key_binding::apply_key_bindings_to_host(
                elem, code, ctx, id_counter, loop_vars, parents,
            )?,
            is_iter,
        ))
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
