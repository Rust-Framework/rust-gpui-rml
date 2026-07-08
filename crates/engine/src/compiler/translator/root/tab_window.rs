//! `<tab-window>` 根节点 translator

use crate::compiler::codegen::render::{gen_render_impl_from_children, ShellWrap};
use crate::compiler::codegen::window::gen_window_impl;
use crate::compiler::translator::utils::print_element;
use crate::compiler::translator::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

/// `<tab-window>` 根节点 translator
#[derive(Debug)]
pub struct TabWindowTranslator;

impl IRmlTranslator for TabWindowTranslator {
    fn tag(&self) -> &'static str {
        "tab-window"
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
        code.push_str(&gen_render_impl_from_children(elem, ctx, ShellWrap::Tab)?);
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("tab-window", "TabWindow", ComponentCategory::Root)
            .root(true)
            .slots(&["menu", "title", "footer", "left", "right", "bottom", "tabs"])
    }
}

/// 注册 `<tab-window>` translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(TabWindowTranslator);
}
