//! `<modern-window>` 根节点 translator

use crate::compiler::codegen::render::{gen_render_impl_from_children, ShellWrap};
use crate::compiler::codegen::window::gen_window_impl;
use crate::compiler::translator::utils::print_element;
use crate::compiler::translator::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

/// `<modern-window>` 根节点 translator
#[derive(Debug)]
pub struct ModernWindowTranslator;

impl IRmlTranslator for ModernWindowTranslator {
    fn tag(&self) -> &'static str {
        "modern-window"
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
        code.push_str(&gen_window_impl(elem, ctx, true)?);
        code.push('\n');
        code.push_str(&gen_render_impl_from_children(elem, ctx, ShellWrap::Modern)?);
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("modern-window", "ModernWindow", ComponentCategory::Root)
            .root(true)
            .slots(&["menu", "title", "footer"])
    }
}

/// 注册 `<modern-window>` translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ModernWindowTranslator);
}
