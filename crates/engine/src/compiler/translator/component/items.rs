//! Items 容器扩展组件 translator
//!
//! 处理 `ComponentKind::StatelessWithItems` 组件：
//! Tabs、TabBar、Table、DescriptionList、Popover、Accordion
//!
//! 这些组件需要解析结构化子项（Tab、Column、DescriptionItem、AccordionItem 等），
//! 已各有专用 codegen 模块，translator 仅作路由。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

const ITEMS_TAGS: &[&str] = &[
    "Tabs",
    "TabBar",
    "Table",
    "DescriptionList",
    "Popover",
    "Accordion",
];

/// Items 容器扩展组件 translator
#[derive(Debug)]
pub struct ItemsComponentTranslator;

impl IRmlTranslator for ItemsComponentTranslator {
    fn tag(&self) -> &'static str {
        "*items-component"
    }

    fn matches(&self, elem: &Element) -> bool {
        let canonical = tags::canonical_tag(&elem.tag);
        ITEMS_TAGS.contains(&canonical.as_str())
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let tag = &elem.tag;
        let resolved = tags::normalize_component_tag(tag);
        let _component = tags::component_lookup_resolved(tag)
            .ok_or_else(|| CodegenError {
                message: format!("unknown component: <{}>", tag),
                span: Some(elem.span),
            })?;

        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });

        let id_val = *id_counter;
        *id_counter += 1;

        let canonical = tags::canonical_tag(&resolved);
        let mut code = match canonical.as_str() {
            "Tabs" => crate::compiler::tabs::gen_tabs(elem, ref_name, id_val, ctx, id_counter, loop_vars),
            "TabBar" => crate::compiler::tab_bar::gen_tab_bar(elem, ref_name, id_val, ctx, id_counter, loop_vars),
            "Table" => crate::compiler::table::gen_table(elem, ref_name, id_val, ctx, id_counter, loop_vars),
            "DescriptionList" => {
                crate::compiler::description_list::gen_description_list(elem, ref_name, id_val, ctx, id_counter, loop_vars)
            }
            "Popover" => crate::compiler::popover::gen_popover(elem, ref_name, id_val, ctx, id_counter, loop_vars),
            "Accordion" => crate::compiler::accordion::gen_accordion(elem, ref_name, id_val, ctx, id_counter, loop_vars),
            _ => Err(CodegenError {
                message: format!("unknown items component: <{}>", tag),
                span: Some(elem.span),
            }),
        }?;

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, tag, sheet, parents);
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
        TranslatorMetadata::new("*items-component", "Items Component", ComponentCategory::Layout)
    }
}

/// 注册 items 容器扩展组件 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ItemsComponentTranslator);
}
