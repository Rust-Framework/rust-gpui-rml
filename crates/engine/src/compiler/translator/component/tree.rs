//! Tree 组件 translator
//!
//! Tree 是 StatefulWithDelegate 组件：通过 `ref="name" items={field}` 声明式绑定。
//! - `items={field}` 作为委托数据注入 TreeState 构造器
//! - `ref="name"` 触发 `__rml_state.get_or_init_ref` 惰性创建
//! - Tree::new 接受 `Option<&Entity<TreeState>>`，与其他 StatefulWithDelegate 组件不同
//! - on_activate/on_select 使用 Tree 专用 event_setter（生成 .on_activate_rc/.on_select_rc）

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::extract_field_converter;
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct TreeTranslator;

impl IRmlTranslator for TreeTranslator {
    fn tag(&self) -> &'static str {
        "Tree"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Tree"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let tag = &elem.tag;
        let resolved = tags::normalize_component_tag(tag);
        let component = tags::component_lookup_resolved(tag).ok_or_else(|| CodegenError {
            message: format!("unknown component: <{}>", tag),
            span: Some(elem.span),
        })?;

        let (_state_field, state_ctor, delegate_attr) = match component.kind {
            tags::ComponentKind::StatefulWithDelegate {
                state_field,
                state_ctor,
                delegate_attr,
            } => (state_field, state_ctor, delegate_attr),
            _ => {
                return Err(CodegenError {
                    message: "<Tree> component kind mismatch: expected StatefulWithDelegate".into(),
                    span: Some(elem.span),
                })
            }
        };

        let ref_name: &str = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        }).ok_or_else(|| CodegenError {
            message: "<Tree> requires `ref=\"name\"` directive for delegate injection".into(),
            span: Some(elem.span),
        })?;

        let delegate_expr = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                (name == delegate_attr).then(|| expr.clone())
            } else {
                None
            }
        }).ok_or_else(|| CodegenError {
            message: format!("<{}> requires `{}={{field}}` bind attribute to provide delegate data", tag, delegate_attr),
            span: Some(elem.span),
        })?;

        let (delegate_field, _) = extract_field_converter(&delegate_expr);

        let entity_expr = format!(
            "self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {{ let __rml_delegate = (self.{}).clone(); {} }})",
            ref_name, delegate_field, state_ctor
        );

        let mut code = format!(
            "({{ let __rml_entity = ({}).clone(); rml_ui::Tree::new(Some(&__rml_entity)) }})",
            entity_expr
        );

        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        for attr in &elem.attributes {
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
                    if name == delegate_attr {
                        continue;
                    }
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
                    if let Some(setter) =
                        crate::compiler::components::tree::setters::event_setter(name, handler, &resolved)
                    {
                        code.push_str(&setter);
                    } else if let Some(setter) = component_event_setter(name, handler, &resolved) {
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
        TranslatorMetadata::new("Tree", "Tree", ComponentCategory::Data).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(TreeTranslator);
}
