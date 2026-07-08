//! ActivityBar 组件 translator
//!
//! EntityRef 组件：从 ViewModel 的 `Entity<T>` 字段 clone。
//! 通过 `ref="field_name"` 指令指定字段名，生成
//! `self.<field>.as_ref().expect("init <field> in on_loaded").clone()`。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct ActivityBarTranslator;

impl IRmlTranslator for ActivityBarTranslator {
    fn tag(&self) -> &'static str {
        "ActivityBar"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "ActivityBar"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let ref_name = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let name = ref_name.ok_or_else(|| CodegenError {
            message: "EntityRef component <ActivityBar> requires `ref=\"field_name\"` directive"
                .to_string(),
            span: Some(elem.span),
        })?;

        let mut code = format!(
            "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
            name, name
        );

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, "ActivityBar", sheet, parents);
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
        TranslatorMetadata::new("ActivityBar", "ActivityBar", ComponentCategory::Navigation)
            .container(true)
            .requires_ref(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ActivityBarTranslator);
}
