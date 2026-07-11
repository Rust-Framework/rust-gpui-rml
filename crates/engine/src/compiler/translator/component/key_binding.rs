//! KeyBinding 键盘快捷键 translator
//!
//! RML `<KeyBinding>` 编译为 `KeyBinding`（声明式键盘快捷键容器）。
//!
//! ## 设计原因
//!
//! GPUI 的 `on_key_down` 在 `InteractiveElement` trait 上，通过事件冒泡接收子元素的键盘事件。
//! `KeyBinding` 封装此模式为声明式容器：解析 `key` 属性为 `Keystroke`，在 keydown 时匹配，
//! 命中后触发 `on_press` 回调。
//!
//! ## 属性
//!
//! - `key="Ctrl+S"` (static) → 快捷键组合（GPUI Keystroke::parse 语法）
//! - `when={cond}` (bind) → 是否启用（默认 true）
//! - `on-press={handler}` (event) → 触发回调（entity 捕获模式，2 参 Fn(&mut Window, &mut App)）

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct KeyBindingTranslator;

impl IRmlTranslator for KeyBindingTranslator {
    fn tag(&self) -> &'static str {
        "KeyBinding"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "KeyBinding"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::key_binding::gen_key_binding(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("KeyBinding", "KeyBinding", ComponentCategory::Layout)
            .container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(KeyBindingTranslator);
}
