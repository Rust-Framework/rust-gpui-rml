//! MenuBar translator
//!
//! 薄包装 `compiler::menu::menu_bar::gen_menu_bar`。
//! 匹配 `<MenuBar>` / `<menu-bar>` / `<menu>`（小写别名）。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct MenuBarTranslator;

impl IRmlTranslator for MenuBarTranslator {
    fn tag(&self) -> &'static str {
        "MenuBar"
    }

    fn matches(&self, elem: &Element) -> bool {
        let normalized = tags::normalize_component_tag(&elem.tag);
        normalized == "MenuBar" || normalized == "menu"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let mut code = crate::compiler::components::menu::gen_menu_bar(elem, ctx, 0, id_counter, loop_vars)?;
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, "MenuBar", sheet, parents);
            if !style_code.is_empty() {
                code.push_str(&style_code);
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("MenuBar", "MenuBar", ComponentCategory::Container).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(MenuBarTranslator);
}
