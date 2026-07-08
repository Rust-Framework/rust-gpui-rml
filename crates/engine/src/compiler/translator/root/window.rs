//! `<window>` 根节点 translator

use crate::compiler::codegen::render::{gen_render_impl_from_children, ShellWrap};
use crate::compiler::codegen::window::gen_window_impl;
use crate::compiler::translator::utils::print_element;
use crate::compiler::translator::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

/// `<window>` 根节点 translator
#[derive(Debug)]
pub struct WindowTranslator;

impl IRmlTranslator for WindowTranslator {
    fn tag(&self) -> &'static str {
        "window"
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
        code.push_str(&gen_window_impl(elem, ctx, false)?);
        code.push('\n');
        code.push_str(&gen_render_impl_from_children(elem, ctx, ShellWrap::Window)?);
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("window", "Window", ComponentCategory::Root).root(true)
    }
}

/// 注册 `<window>` translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(WindowTranslator);
}
