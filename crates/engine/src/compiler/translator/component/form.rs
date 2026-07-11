//! Form 表单容器 translator
//!
//! RML `<Form>` 编译为 `gpui_component::form::Form`，支持垂直/水平布局。
//! 子节点必须为 `<Field>` 元素（Form 的 .child() 接受 impl Into<Field>）。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct FormTranslator;

impl IRmlTranslator for FormTranslator {
    fn tag(&self) -> &'static str {
        "Form"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Form"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::form::gen_form(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Form", "Form", ComponentCategory::Layout).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(FormTranslator);
}
