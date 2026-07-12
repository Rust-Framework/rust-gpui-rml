//! KeyBinding 键盘快捷键 translator
//!
//! RML `<KeyBinding>` 编译为 `KeyBinding`（声明式键盘快捷键容器）。
//!
//! ## 唯一写法：焦点宿主子节点
//!
//! ```rml
//! <Input ref="demo_input" placeholder="...">
//!   <KeyBinding key="Ctrl+S" on-press={on_save} />
//!   <KeyBinding key="Escape" on-press={on_clear} />
//! </Input>
//! ```
//!
//! `KeyBinding` 作为 Input 等的声明式子节点，由宿主 translator 统一包裹生成。
//! 不支持 `<KeyBinding>…子元素…</KeyBinding>` 外层包裹写法。
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
        if !elem.children.is_empty() {
            return Err(CodegenError {
                message: "<KeyBinding> 不支持包裹子元素；请将快捷键声明为焦点宿主（Input、CodeEditor、NumberInput、textarea 等）的子节点，例如 <Input><KeyBinding key=\"Ctrl+S\" on-press={on_save} /></Input>"
                    .into(),
                span: Some(elem.span),
            });
        }
        let code = crate::compiler::components::key_binding::gen_key_binding_shell(
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
