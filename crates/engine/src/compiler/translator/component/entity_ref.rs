//! 通用 EntityRef 组件 translator
//!
//! 处理 `ComponentKind::EntityRef` 组件（Terminal、Chat 等）：
//! 从 ViewModel 的 `Option<Entity<T>>` 字段 clone Entity。
//! 通过 `ref="field_name"` 指令指定字段名，生成
//! `self.<field>.as_ref().expect("init <field> in on_loaded").clone()`。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::expr::current_self_alias;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct EntityRefComponentTranslator;

impl IRmlTranslator for EntityRefComponentTranslator {
    fn tag(&self) -> &'static str {
        "*entity-ref-component"
    }

    fn matches(&self, elem: &Element) -> bool {
        matches!(
            tags::component_lookup_resolved(&elem.tag).map(|c| c.kind),
            Some(tags::ComponentKind::EntityRef)
        )
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let canonical = tags::canonical_tag(&elem.tag);

        let ref_name = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let name = ref_name.ok_or_else(|| CodegenError {
            message: format!(
                "EntityRef component <{}> requires `ref=\"field_name\"` directive",
                canonical
            ),
            span: Some(elem.span),
        })?;

        let alias = current_self_alias().unwrap_or("self");
        let mut code = format!(
            "{}.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
            alias, name, name
        );

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, &canonical, sheet, parents);
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
        TranslatorMetadata::new(
            "*entity-ref-component",
            "EntityRef Component",
            ComponentCategory::Layout,
        )
        .requires_ref(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(EntityRefComponentTranslator);
}
