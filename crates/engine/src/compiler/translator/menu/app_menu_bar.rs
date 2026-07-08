//! AppMenuBar translator
//!
//! 薄包装 `compiler::menu::app_menu_bar::gen_app_menu_bar`。
//! AppMenuBar 是无参构造器，不接受 id_counter / loop_vars。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct AppMenuBarTranslator;

impl IRmlTranslator for AppMenuBarTranslator {
    fn tag(&self) -> &'static str {
        "AppMenuBar"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::normalize_component_tag(&elem.tag) == "AppMenuBar"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let mut code = crate::compiler::menu::gen_app_menu_bar(elem, ctx)?;
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, "AppMenuBar", sheet, parents);
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
        TranslatorMetadata::new("AppMenuBar", "AppMenuBar", ComponentCategory::Container)
            .container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(AppMenuBarTranslator);
}
