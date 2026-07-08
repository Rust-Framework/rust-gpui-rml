//! 扩展组件 translator
//!
//! Phase 2 过渡实现：将所有 gpui-component 扩展组件统一接入 `IRmlTranslator` 注册表，
//! 内部复用现有 `compiler::component::gen_component` 逻辑，避免一次性重写所有组件。
//! 后续按 Stateless / Stateful / StatelessNoId / StatelessWithItems 等类别拆分为独立 translator。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

/// 扩展组件通用 translator
#[derive(Debug)]
pub struct ExtensionComponentTranslator;

impl IRmlTranslator for ExtensionComponentTranslator {
    fn tag(&self) -> &'static str {
        "*component"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::is_extension_component(&elem.tag)
            && !crate::compiler::menu::is_menu_container(&elem.tag)
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
        let mut code = crate::compiler::component::gen_component(
            elem, ctx, 0, id_counter, loop_vars,
        )?;
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, tag, sheet, parents);
            if !style_code.is_empty() {
                code.push_str(&style_code);
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        // 通配 translator 自身不提供单组件元数据；具体组件元数据通过 tags::component_lookup 查询
        TranslatorMetadata::new("*component", "Extension Component", ComponentCategory::Layout)
    }
}

/// 注册扩展组件 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ExtensionComponentTranslator);
}
