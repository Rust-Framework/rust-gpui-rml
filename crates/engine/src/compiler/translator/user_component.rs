//! 用户自定义组件 translator
//!
//! 将 `#[component]` 标注的 struct 接入 `IRmlTranslator` 注册表。
//! 每个用户组件在 `CodegenCtx.user_components` 中登记，编译前通过
//! `TranslatorRegistry::with_user_components` 动态注册为独立 translator。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata, TranslatorRegistry};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError, UserComponentInfo};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use std::collections::HashMap;

/// 单个用户组件 translator
#[derive(Debug)]
pub struct UserComponentTranslator {
    tag: &'static str,
}

impl UserComponentTranslator {
    /// 创建 translator，标签名通过 `Box::leak` 转为 `'static` 生命周期
    /// 以便作为注册表键和 `IRmlTranslator::tag` 返回值。
    pub fn new(tag: String) -> Self {
        let leaked: &'static str = Box::leak(tag.into_boxed_str());
        Self { tag: leaked }
    }
}

impl IRmlTranslator for UserComponentTranslator {
    fn tag(&self) -> &'static str {
        self.tag
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let info = ctx
            .user_components
            .get(self.tag)
            .ok_or_else(|| CodegenError::new(format!("user component <{}> not found", self.tag)))?;

        let mut code = crate::compiler::user_component::gen_user_component(
            info, elem, ctx, id_counter, loop_vars,
        )?;

        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, self.tag, sheet, parents);
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
        TranslatorMetadata::new(self.tag, self.tag, ComponentCategory::User).container(true)
    }
}

/// 根据 `CodegenCtx` 中的用户组件注册表，向注册表动态追加 translator。
pub fn register_user_components(
    registry: &mut TranslatorRegistry,
    user_components: &HashMap<String, UserComponentInfo>,
) {
    for tag in user_components.keys() {
        registry.register(UserComponentTranslator::new(tag.clone()));
    }
}
