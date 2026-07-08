//! `<component>` 根节点 translator
//!
//! 与 `<component content={...}>` 透明容器区分：
//! - 透明容器带 `content` 属性，生成任意元素表达式；
//! - 根节点 `<component>` 无 `content`，生成 `impl Render`。

use crate::compiler::codegen::render::{gen_render_impl_from_children, ShellWrap};
use crate::compiler::translator::utils::print_element;
use crate::compiler::translator::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

/// `<component>` 根节点 translator
#[derive(Debug)]
pub struct ComponentRootTranslator;

impl IRmlTranslator for ComponentRootTranslator {
    fn tag(&self) -> &'static str {
        "component"
    }

    /// 仅匹配不带 `content` 属性的 `<component>`；
    /// 带 `content` 的实例由 `ComponentTranslator`（透明容器）处理。
    fn matches(&self, elem: &Element) -> bool {
        elem.tag == "component"
            && !elem.attributes.iter().any(|a| {
                matches!(a, Attribute::Bind { name, .. } if name == "content")
            })
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        Ok((
            gen_render_impl_from_children(elem, ctx, ShellWrap::None)?,
            false,
        ))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("component", "ComponentRoot", ComponentCategory::Root).root(true)
    }
}

/// 注册 `<component>` 根节点 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ComponentRootTranslator);
}
