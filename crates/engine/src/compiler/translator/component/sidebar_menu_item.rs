//! SidebarMenuItem 组件 translator
//!
//! 侧边栏菜单项，支持 icon/active/on_click/子菜单。无 ElementId（StatelessNoId）。
//! `label` 属性作为构造器参数，`disabled` 映射到 `.disable()` 方法。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct SidebarMenuItemTranslator;

impl IRmlTranslator for SidebarMenuItemTranslator {
    fn tag(&self) -> &'static str {
        "SidebarMenuItem"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "SidebarMenuItem"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::sidebar_menu_item::gen_sidebar_menu_item(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("SidebarMenuItem", "SidebarMenuItem", ComponentCategory::Layout)
            .container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(SidebarMenuItemTranslator);
}
