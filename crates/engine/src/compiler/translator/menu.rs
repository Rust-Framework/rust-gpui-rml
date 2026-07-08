//! 菜单容器 translator
//!
//! 将 context-menu / dropdown-menu / menu-bar / app-menu-bar（及小写别名 menu）
//! 接入 `IRmlTranslator` 注册表，内部复用现有 `compiler::menu` 模块的生成逻辑。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::compiler::menu::{gen_menu_element, is_menu_container};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

/// 菜单容器 translator
#[derive(Debug)]
pub struct MenuTranslator;

impl IRmlTranslator for MenuTranslator {
    fn tag(&self) -> &'static str {
        "*menu"
    }

    fn matches(&self, elem: &Element) -> bool {
        is_menu_container(&elem.tag)
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let tag = &elem.tag;
        let mut code = gen_menu_element(elem, ctx, 0, id_counter, loop_vars)?;
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, tag, sheet, parents);
            if !style_code.is_empty() {
                code.push_str(&style_code);
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("*menu", "Menu Container", ComponentCategory::Container)
    }
}

/// 注册菜单 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(MenuTranslator);
}
