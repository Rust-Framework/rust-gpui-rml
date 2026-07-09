//! 有状态扩展组件 translator
//!
//! 处理 `ComponentKind::Stateful` 组件：Input、TextInput、Slider 等。
//! Tree 与 CodeEditor 构造特殊，由独立的 `TreeTranslator` / `CodeEditorTranslator` 处理，
//! 本 translator 在 `matches` 中显式排除。
//!
//! 有状态组件围绕 `Option<Entity<T>>` 字段：
//! - 无 ref 时读取 ViewModel 字段
//! - 有 ref 指令时通过 `__rml_state.get_or_init_ref` 惰性创建
//! - Input 事件（on_change/on_enter/on_focus/on_blur）通过 `cx.subscribe` 在构造时注册

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, EventHandler};
use crate::tags;

/// 有状态扩展组件 translator
#[derive(Debug)]
pub struct StatefulComponentTranslator;

impl IRmlTranslator for StatefulComponentTranslator {
    fn tag(&self) -> &'static str {
        "*stateful-component"
    }

    fn matches(&self, elem: &Element) -> bool {
        let canonical = tags::canonical_tag(&elem.tag);
        if matches!(canonical.as_str(), "Tree" | "CodeEditor") {
            return false;
        }
        matches!(
            tags::component_lookup_resolved(&elem.tag).map(|c| c.kind),
            Some(tags::ComponentKind::Stateful { .. })
        )
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
        let component = tags::component_lookup_resolved(tag)
            .ok_or_else(|| CodegenError {
                message: format!("unknown component: <{}>", tag),
                span: Some(elem.span),
            })?;

        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });

        let state_field = match component.kind {
            tags::ComponentKind::Stateful { state_field, .. } => state_field,
            _ => unreachable!(),
        };
        let state_ctor = match component.kind {
            tags::ComponentKind::Stateful { state_ctor, .. } => state_ctor,
            _ => unreachable!(),
        };

        let mut code = gen_stateful_body(elem, &component, ref_name, state_field, state_ctor, loop_vars)?;

        // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        // 应用静态/bind/event setter（Input 事件由 gen_stateful_body 内部处理，setter 返回 None）
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    if let Some(setter) = component_static_setter(name, value, &resolved) {
                        code.push_str(&setter);
                    } else {
                        crate::compiler::setters::check_missing_mapping(ctx, &resolved, name, "static")?;
                    }
                }
                Attribute::Bind { name, expr, .. } => {
                    if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, &resolved) {
                        code.push_str(&setter);
                    } else {
                        crate::compiler::setters::check_missing_mapping(ctx, &resolved, name, "bind")?;
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
        TranslatorMetadata::new("*stateful-component", "Stateful Component", ComponentCategory::Layout)
    }
}

/// 生成通用 Stateful 组件构造表达式
///
/// 返回形如 `({ let __rml_entity = ...; ... Input::new(&__rml_entity) })` 的代码。
fn gen_stateful_body(
    elem: &Element,
    component: &tags::ComponentTag,
    ref_name: Option<&str>,
    state_field: &str,
    state_ctor: &str,
    _loop_vars: &[String],
) -> Result<String, CodegenError> {
    let tag = &elem.tag;
    let resolved = tags::normalize_component_tag(tag);

    // 收集 Input 事件处理器
    let input_event_handlers: Vec<(&str, &EventHandler)> = elem
        .attributes
        .iter()
        .filter_map(|attr| {
            if let Attribute::Event { name, handler, .. } = attr {
                if crate::compiler::components::input::is_input_event(name, &resolved) {
                    Some((name.as_str(), handler))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    let self_prefix = expr::current_self_alias().unwrap_or("self");

    if !input_event_handlers.is_empty() {
        let entity_expr = if let Some(name) = ref_name {
            format!(
                "self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {})",
                name, state_ctor
            )
        } else {
            format!("{}.{}.clone()", self_prefix, state_field)
        };
        let ref_key = ref_name.unwrap_or(state_field);
        let subscribe_code: String = input_event_handlers
            .iter()
            .map(|(event_name, handler)| {
                crate::compiler::components::input::gen_input_event_subscribe(ref_key, event_name, handler)
            })
            .collect::<Vec<_>>()
            .join(" ");
        Ok(format!(
            "({{ let __rml_entity = {entity_expr}; {subscribe_code} {}::new(&__rml_entity) }})",
            component.ctor_path
        ))
    } else if let Some(name) = ref_name {
        Ok(format!(
            "{}::new(&self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {}))",
            component.ctor_path, name, state_ctor
        ))
    } else {
        Ok(format!(
            "{}::new({}.{}.as_ref().expect(\"init {} in on_loaded\"))",
            component.ctor_path, self_prefix, state_field, state_field
        ))
    }
}

/// 注册有状态扩展组件 translator
pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(StatefulComponentTranslator);
}
