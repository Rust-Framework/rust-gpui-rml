//! Notification 反馈组件 translator
//!
//! RML `<Notification>` 编译为 `NotificationTrigger`（声明式通知触发器），
//! 而非直接使用 gpui-component 的 `Notification`（命令式 API）。
//!
//! ## 设计原因
//!
//! `Notification` 实现 `Render`（非 `RenderOnce`），通过 `window.push_notification()` 推送，
//! 无法直接作为 RML 组件。`NotificationTrigger` 是 `RenderOnce` 包装器，
//! 包裹一个 `slot="trigger"` 子元素，点击时自动构造并推送通知。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct NotificationTranslator;

impl IRmlTranslator for NotificationTranslator {
    fn tag(&self) -> &'static str {
        "Notification"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Notification"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::notification::gen_notification(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Notification", "NotificationTrigger", ComponentCategory::Feedback)
            .container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(NotificationTranslator);
}
