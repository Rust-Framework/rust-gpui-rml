//! Scroll 容器组件 translator
//!
//! 封装 `div().overflow_y_scrollbar()` 模式为声明式 `<Scroll>` 容器组件。
//! 通过 `vertical` / `horizontal` / `both` 独立布尔属性选择滚动方向，默认 vertical。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct ScrollTranslator;

impl IRmlTranslator for ScrollTranslator {
    fn tag(&self) -> &'static str {
        "Scroll"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Scroll"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::scroll::gen_scroll(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Scroll", "Scroll", ComponentCategory::Layout).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ScrollTranslator);
}
