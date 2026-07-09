//! `<anchored>` translator —— 映射到 GPUI 原生 `gpui::anchored()`
//!
//! Anchored 实现 `ParentElement` 但未实现 `Styled`，因此 `is_styled: false`。
//! CSS / 归一化样式属性由 `builtin_engine::translate` 的 `is_styled` 守卫自动跳过，
//! 样式应作用于其内部子元素（如 `<div>` 包装层）。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "anchored",
    display_name: "Anchored",
    category: ComponentCategory::Layout,
    ctor: "gpui::anchored()",
    is_container: true,
    is_styled: false,
};

#[derive(Debug)]
pub struct AnchoredTranslator;

impl IRmlTranslator for AnchoredTranslator {
    fn tag(&self) -> &'static str {
        META.tag
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        BuiltinTranslator { meta: META }.to_rust(elem, ctx, id_counter, loop_vars, parents)
    }

    fn to_rml(
        &self,
        elem: &Element,
        ctx: &super::PrinterCtx,
    ) -> Result<String, super::PrintError> {
        BuiltinTranslator { meta: META }.to_rml(elem, ctx)
    }

    fn metadata(&self) -> super::TranslatorMetadata {
        META.to_metadata()
    }
}
