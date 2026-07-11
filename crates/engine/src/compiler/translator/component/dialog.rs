//! Dialog 容器组件 translator
//!
//! 薄包装 `compiler::components::dialog::gen_dialog`，构造 + 属性 + slot 子节点（trigger/content）。
//!
//! Dialog 构造器需要 `&mut App`，codegen 生成 `Dialog::new(cx)`。
//! 注意：`<Dialog>`（PascalCase）为本组件，`<dialog>`（小写）为 `RootTag::DialogWindow`。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct DialogTranslator;

impl IRmlTranslator for DialogTranslator {
    fn tag(&self) -> &'static str {
        "Dialog"
    }

    fn matches(&self, elem: &Element) -> bool {
        // 仅 PascalCase "Dialog" 匹配，小写 "dialog" 由 RootTag::DialogWindow 处理
        tags::canonical_tag(&elem.tag) == "Dialog"
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

        let code = crate::compiler::components::dialog::gen_dialog(
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
        TranslatorMetadata::new("Dialog", "Dialog", ComponentCategory::Feedback).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(DialogTranslator);
}
