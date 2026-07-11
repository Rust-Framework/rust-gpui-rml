//! AlertDialog 容器组件 translator
//!
//! 与 Dialog 的区别：
//! - AlertDialog 默认 `close_button(false)` + `overlay_closable(false)`（警示场景）
//! - Dialog 默认 `close_button(true)` + `overlay_closable(true)`（通用场景）
//! - AlertDialog 提供 `.description()` / `.confirm()` / `.show_cancel()` 便捷方法
//! - AlertDialog footer 按钮居中对齐，Dialog 右对齐

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct AlertDialogTranslator;

impl IRmlTranslator for AlertDialogTranslator {
    fn tag(&self) -> &'static str {
        "AlertDialog"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "AlertDialog"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let id_val = *id_counter;
        *id_counter += 1;

        let code = crate::compiler::components::alert_dialog::gen_alert_dialog(
            elem,
            ref_name,
            id_val,
            ctx,
            id_counter,
            loop_vars,
            parents,
        )?;

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("AlertDialog", "AlertDialog", ComponentCategory::Feedback)
            .container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(AlertDialogTranslator);
}
