//! `<svg>` translator —— 映射到 GPUI 原生 `gpui::svg()`
//!
//! GPUI `svg()` 构造器无参数，`Svg` 实现了 `Styled` trait。
//! `path` 属性通过 `.path(impl Into<SharedString>)` 链式调用设置，
//! `color` 走归一化样式属性（`.text_color()`），`width`/`height` 走 `.w()`/`.h()`。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "svg",
    display_name: "Svg",
    category: ComponentCategory::Primitive,
    ctor: "gpui::svg()",
    is_container: false,
    is_styled: true,
};

#[derive(Debug)]
pub struct SvgTranslator;

impl IRmlTranslator for SvgTranslator {
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
