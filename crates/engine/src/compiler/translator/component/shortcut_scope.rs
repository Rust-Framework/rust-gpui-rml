//! ShortcutScope / Shortcut 全局快捷键 translator
//!
//! ## 唯一写法
//!
//! ```rml
//! <ShortcutScope>
//!   <Shortcut key="Ctrl+S" on-press={on_save} />
//!   <div>...</div>
//! </ShortcutScope>
//! ```
//!
//! `<Shortcut>` 仅允许作为 `ShortcutScope` 的声明式元数据子节点。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct ShortcutScopeTranslator;

impl IRmlTranslator for ShortcutScopeTranslator {
    fn tag(&self) -> &'static str {
        "ShortcutScope"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "ShortcutScope"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::shortcut_scope::gen_shortcut_scope(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("ShortcutScope", "ShortcutScope", ComponentCategory::Layout)
            .container(true)
    }
}

#[derive(Debug)]
pub struct ShortcutTranslator;

impl IRmlTranslator for ShortcutTranslator {
    fn tag(&self) -> &'static str {
        "Shortcut"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Shortcut"
    }

    fn to_rust(
        &self,
        elem: &Element,
        _ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let _ = elem;
        Err(CodegenError {
            message: "<Shortcut> 仅允许作为 <ShortcutScope> 的声明式子节点；\
             请使用 <ShortcutScope><Shortcut key=\"Ctrl+S\" on-press={on_save} />…内容…</ShortcutScope>"
                .into(),
            span: Some(elem.span),
        })
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Shortcut", "Shortcut", ComponentCategory::Layout).container(false)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ShortcutScopeTranslator);
    registry.register(ShortcutTranslator);
}
