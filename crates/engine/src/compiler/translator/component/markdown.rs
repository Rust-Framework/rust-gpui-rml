//! Markdown translator
//!
//! RML `<Markdown>` 编译为 `Markdown`（声明式 Markdown 富文本渲染组件）。
//! 内容通过 `content` 属性传入（static 字符串或 bind 表达式），不支持子节点。
//!
//! ## 属性
//!
//! - `content="..."` (static) → `.content("...")`
//! - `content={field}` (bind) → `.content(self.field)` / `.content(self.method())`

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct MarkdownTranslator;

impl IRmlTranslator for MarkdownTranslator {
    fn tag(&self) -> &'static str {
        "Markdown"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Markdown"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::markdown::gen_markdown(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Markdown", "Markdown", ComponentCategory::Layout)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(MarkdownTranslator);
}
