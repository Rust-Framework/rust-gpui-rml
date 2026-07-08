//! `<slot>` 插槽占位符 translator
//!
//! 组件模板内声明插槽渲染位置，codegen 从 `self.__rml_state.slots` 查询并调用闭包。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

/// `<slot>` translator
#[derive(Debug)]
pub struct SlotTranslator;

impl IRmlTranslator for SlotTranslator {
    fn tag(&self) -> &'static str {
        "slot"
    }

    fn to_rust(
        &self,
        elem: &Element,
        _ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let slot_name = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Static { name, value, .. } if name == "name" => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "default".to_string());

        Ok((
            format!(
                "self.__rml_state.slot({slot_name:?}).map_or(gpui::Empty.into_any_element(), |f| f(&rml_core::slot::NullSlotScope::new({slot_name:?}), _window, cx))",
                slot_name = slot_name
            ),
            false,
        ))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("slot", "Slot", ComponentCategory::Primitive)
    }
}

/// 注册 `<slot>` translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(SlotTranslator);
}
