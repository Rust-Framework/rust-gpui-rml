//! `<dialog>` 根节点 translator

use crate::compiler::codegen::render::{gen_render_impl_from_children, ShellWrap};
use crate::compiler::codegen::window::gen_dialog_impl;
use crate::compiler::translator::utils::print_element;
use crate::compiler::translator::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

/// `<dialog>` 根节点 translator
#[derive(Debug)]
pub struct DialogTranslator;

impl IRmlTranslator for DialogTranslator {
    fn tag(&self) -> &'static str {
        "dialog"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let mut code = String::new();
        code.push_str(&gen_dialog_impl(elem, ctx)?);
        code.push('\n');
        code.push_str(&gen_render_impl_from_children(elem, ctx, ShellWrap::None)?);
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("dialog", "Dialog", ComponentCategory::Root).root(true)
    }
}

/// 注册 `<dialog>` translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(DialogTranslator);
}
