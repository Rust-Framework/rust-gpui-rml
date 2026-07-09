//! CodeEditor 组件 translator
//!
//! CodeEditor 基于 Input，自动应用代码编辑器语义默认值。
//! 构造器特殊（需应用默认 mono 字体/padding/height 等），从 `StatefulComponentTranslator` 独立。
//! 薄包装 `compiler::code_editor::gen_code_editor`，并应用 setter + CSS。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};
use crate::tags;

#[derive(Debug)]
pub struct CodeEditorTranslator;

impl IRmlTranslator for CodeEditorTranslator {
    fn tag(&self) -> &'static str {
        "CodeEditor"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "CodeEditor"
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
        let component = tags::component_lookup_resolved(tag)
            .ok_or_else(|| CodegenError {
                message: format!("unknown component: <{}>", tag),
                span: Some(elem.span),
            })?;

        let mut code = crate::compiler::components::code_editor::gen_code_editor(
            elem,
            component,
            ctx,
            0,
            id_counter,
            loop_vars,
        )?;

        // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        for attr in &elem.attributes {
            // value/language/bordered/focus_bordered/context_menu 由 gen_code_editor 内联处理，
            // 不走 setter 链路（避免生成 Input 不支持的 .value() 等方法）
            let is_handled_inline = match attr {
                Attribute::Static { name, .. } => {
                    name == "value"
                        || name == "language"
                        || name == "context_menu"
                        || name == "bordered"
                        || name == "focus_bordered"
                }
                Attribute::Bind { name, .. } => name == "value",
                _ => false,
            };
            if is_handled_inline {
                continue;
            }
            match attr {
                Attribute::Static { name, value, .. } => {
                    if let Some(setter) = component_static_setter(name, value, &resolved) {
                        code.push_str(&setter);
                    } else {
                        crate::compiler::setters::check_missing_mapping(
                            ctx, &resolved, name, "static",
                        )?;
                    }
                }
                Attribute::Bind { name, expr, .. } => {
                    if let Some(setter) =
                        component_bind_setter(name, expr, &lv, &computed, &resolved)
                    {
                        code.push_str(&setter);
                    } else {
                        crate::compiler::setters::check_missing_mapping(
                            ctx, &resolved, name, "bind",
                        )?;
                    }
                }
                Attribute::Event { name, handler, .. } => {
                    if let Some(setter) = component_event_setter(name, handler, &resolved) {
                        code.push_str(&setter);
                    }
                }
            }
        }

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("CodeEditor", "CodeEditor", ComponentCategory::Form)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(CodeEditorTranslator);
}
