//! 扩展组件 translator
//!
//! 按组件类别拆分为独立 translator：
//! - `stateless`：Stateless / StatelessNoId 组件
//! - `stateful`：Stateful 组件（Input / TextInput / Slider 等）
//! - `items`：StatelessWithItems 容器组件（Tabs / TabBar / Table / DescriptionList / Popover / Accordion）
//! - `special`：构造器特殊的组件（Label / Separator / Icon / Kbd / Tag / Alert / RadioGroup）
//!
//! 本模块保留 `<component content={...}>` 透明容器 translator。

pub mod items;
pub mod special;
pub mod stateful;
pub mod stateless;

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

/// `<component content={...}>` 透明容器 translator
///
/// 将 `content` 属性的表达式作为任意元素直接嵌入，用于需要命令式构造子树的场景。
/// 不带 `content` 属性的 `<component>` 根节点由 `root::ComponentRootTranslator` 处理。
#[derive(Debug)]
pub struct ComponentTranslator;

impl IRmlTranslator for ComponentTranslator {
    fn tag(&self) -> &'static str {
        "*component-transparent"
    }

    fn matches(&self, elem: &Element) -> bool {
        elem.tag == "component"
            && elem.attributes.iter().any(|a| {
                matches!(a, Attribute::Bind { name, .. } if name == "content")
            })
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let content = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Bind { name, expr, .. } if name == "content" => Some(expr.as_str()),
                _ => None,
            })
            .ok_or_else(|| CodegenError {
                message: "<component> must have content={...} attribute".to_string(),
                span: Some(elem.span),
            })?;

        let mut code = content.to_string();
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, &elem.tag, sheet, parents);
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
        TranslatorMetadata::new("component", "Component", ComponentCategory::Layout)
    }
}

/// 注册所有扩展组件 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    stateless::register_all(registry);
    stateful::register_all(registry);
    items::register_all(registry);
    special::register_all(registry);
    registry.register(ComponentTranslator);
}
