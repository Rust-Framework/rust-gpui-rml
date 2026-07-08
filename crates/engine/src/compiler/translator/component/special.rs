//! 特殊扩展组件 translator
//!
//! 处理构造器不遵循标准 Stateless 模式的组件：
//! - Label、Separator、Icon、Kbd、Tag、Alert、RadioGroup
//!
//! 这些组件已各有专用 codegen 模块，translator 仅作路由。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

const SPECIAL_TAGS: &[&str] = &["Label", "Separator", "Icon", "Kbd", "Tag", "Alert", "RadioGroup", "ActivityBar"];

/// 特殊扩展组件 translator
#[derive(Debug)]
pub struct SpecialComponentTranslator;

impl IRmlTranslator for SpecialComponentTranslator {
    fn tag(&self) -> &'static str {
        "*special-component"
    }

    fn matches(&self, elem: &Element) -> bool {
        let canonical = tags::canonical_tag(&elem.tag);
        SPECIAL_TAGS.contains(&canonical.as_str())
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let canonical = tags::canonical_tag(&elem.tag);
        let mut code = match canonical.as_str() {
            "Label" => crate::compiler::label::gen_label(elem, ctx, id_counter, loop_vars),
            "Separator" => crate::compiler::separator::gen_separator(elem, ctx, id_counter, loop_vars),
            "Icon" => crate::compiler::icon::gen_icon(elem, ctx, id_counter, loop_vars),
            "Kbd" => crate::compiler::kbd::gen_kbd(elem, ctx, id_counter, loop_vars),
            "Tag" => crate::compiler::tag::gen_tag(elem, ctx, id_counter, loop_vars),
            "Alert" => crate::compiler::alert::gen_alert(elem, ctx, id_counter, loop_vars),
            "RadioGroup" => crate::compiler::radio_group::gen_radio_group(elem, ctx, id_counter, loop_vars),
            "ActivityBar" => gen_activity_bar(elem),
            _ => Err(CodegenError {
                message: format!("unknown special component: <{}>", elem.tag),
                span: Some(elem.span),
            }),
        }?;

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, &elem.tag, sheet, parents);
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
        TranslatorMetadata::new("*special-component", "Special Component", ComponentCategory::Layout)
    }
}

/// ActivityBar：EntityRef 组件，从 ViewModel 的 `Entity<T>` 字段 clone
fn gen_activity_bar(elem: &Element) -> Result<String, CodegenError> {
    let ref_name = elem.directives.iter().find_map(|d| match d {
        crate::parser::ast::Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    });
    let name = ref_name.ok_or_else(|| CodegenError {
        message: "EntityRef component <ActivityBar> requires `ref=\"field_name\"` directive".to_string(),
        span: Some(elem.span),
    })?;
    Ok(format!(
        "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
        name, name
    ))
}

/// 注册特殊扩展组件 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(SpecialComponentTranslator);
}
