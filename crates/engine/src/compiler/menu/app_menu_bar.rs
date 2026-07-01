//! `AppMenuBar` codegen

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::Element;

pub fn gen_app_menu_bar(_elem: &Element, _ctx: &CodegenCtx) -> Result<String, CodegenError> {
    Ok("rml_ui::AppMenuBar::new(cx)".to_string())
}
