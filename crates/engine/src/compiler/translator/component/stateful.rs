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
use crate::compiler::codegen::binding::{gen_model_input, gen_model_state_bridge};
use crate::compiler::codegen::extract_field_converter;
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::compiler::state_bridge::lookup_state_bridge_for_tag;
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
        if matches!(canonical.as_str(), "Tree" | "CodeEditor" | "OtpInput") {
            return false;
        }
        matches!(
            tags::component_lookup_resolved(&elem.tag).map(|c| c.kind),
            Some(tags::ComponentKind::Stateful { .. })
                | Some(tags::ComponentKind::StatefulWithDelegate { .. })
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
        let canonical = tags::canonical_tag(tag);

        // C2: InputStateBridge — Input/TextInput/NumberInput + value={field} → gen_model_input
        // 复用小写 <input> 的 InputState 双向同步机制（正向版本追踪 + 反向事件订阅）
        if matches!(canonical.as_str(), "Input" | "TextInput" | "NumberInput") {
            if let Some(expr) = elem.attributes.iter().find_map(|attr| {
                if let Attribute::Bind { name, expr, .. } = attr {
                    (name == "value").then(|| expr.clone())
                } else {
                    None
                }
            }) {
                let (field, _) = extract_field_converter(&expr);
                let code = gen_model_input(elem, ctx, _id_counter, field, parents)?;
                return Ok((code, false));
            }
        }

        // C4: 通用 StateBridge — 任意 StateBridge 组件 + bind_property={field} → gen_model_state_bridge
        // 正向同步（VM 字段 → State.set_value）+ 反向同步（StateEvent → VM 字段）
        // 由 STATE_BRIDGE_REGISTRY 驱动，新增组件只需在 state_bridge.rs 注册
        if let Some(spec) = lookup_state_bridge_for_tag(canonical.as_str()) {
            if let Some(expr) = elem.attributes.iter().find_map(|attr| {
                if let Attribute::Bind { name, expr, .. } = attr {
                    (name == spec.bind_property).then(|| expr.clone())
                } else {
                    None
                }
            }) {
                let (field, _) = extract_field_converter(&expr);
                let code = gen_model_state_bridge(spec, elem, ctx, _id_counter, field, parents)?;
                return Ok((code, false));
            }
        }

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
            tags::ComponentKind::StatefulWithDelegate { state_field, .. } => state_field,
            _ => unreachable!(),
        };
        let state_ctor = match component.kind {
            tags::ComponentKind::Stateful { state_ctor, .. } => state_ctor,
            tags::ComponentKind::StatefulWithDelegate { state_ctor, .. } => state_ctor,
            _ => unreachable!(),
        };

        let delegate_attr: Option<&str> = match component.kind {
            tags::ComponentKind::StatefulWithDelegate { delegate_attr, .. } => Some(delegate_attr),
            _ => None,
        };

        let mut code = match component.kind {
            tags::ComponentKind::Stateful { .. } => {
                gen_stateful_body(elem, &component, ref_name, state_field, state_ctor, loop_vars)?
            }
            tags::ComponentKind::StatefulWithDelegate { delegate_attr, .. } => {
                gen_stateful_with_delegate_body(
                    elem, &component, ref_name, state_field, state_ctor, delegate_attr, loop_vars,
                )?
            }
            _ => unreachable!(),
        };

        // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        // 应用静态/bind/event setter（Input 事件由 gen_stateful_body 内部处理，setter 返回 None）
        // StatefulWithDelegate 的 delegate_attr 已在构造器中消费，跳过 setter 循环
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
                    if Some(name.as_str()) == delegate_attr {
                        continue;
                    }
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
pub(crate) fn gen_stateful_body(
    elem: &Element,
    component: &tags::ComponentTag,
    ref_name: Option<&str>,
    state_field: &str,
    state_ctor: &str,
    _loop_vars: &[String],
) -> Result<String, CodegenError> {
    let tag = &elem.tag;
    let resolved = tags::normalize_component_tag(tag);

    // 收集 Input 事件处理器（Input/TextInput/NumberInput/CodeEditor/OtpInput 的 on_change/on_enter/on_focus/on_blur）
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

    // 收集 State 事件处理器（ColorPicker/Calendar/DatePicker 等拥有独立 Event 类型的 Stateful 组件）
    let state_event_handlers: Vec<(&str, &EventHandler)> = elem
        .attributes
        .iter()
        .filter_map(|attr| {
            if let Attribute::Event { name, handler, .. } = attr {
                if crate::compiler::components::state_event::is_state_event(name, &resolved) {
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

    if !input_event_handlers.is_empty() || !state_event_handlers.is_empty() {
        let entity_expr = if let Some(name) = ref_name {
            format!(
                "self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {})",
                name, state_ctor
            )
        } else {
            format!("{}.{}.clone()", self_prefix, state_field)
        };
        let ref_key = ref_name.unwrap_or(state_field);
        let input_subscribe_code: String = input_event_handlers
            .iter()
            .map(|(event_name, handler)| {
                crate::compiler::components::input::gen_input_event_subscribe(ref_key, event_name, handler)
            })
            .collect::<Vec<_>>()
            .join(" ");
        let state_subscribe_code: String = state_event_handlers
            .iter()
            .map(|(event_name, handler)| {
                crate::compiler::components::state_event::gen_state_event_subscribe(
                    ref_key, event_name, handler, &resolved,
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        let subscribe_code = format!("{} {}", input_subscribe_code, state_subscribe_code);
        // .clone() 确保 Fn 闭包内可重复使用 Entity 句柄（slot 场景下 __rml_slot_demo_entity_N 不会被 move）
        Ok(format!(
            "({{ let __rml_entity = ({entity_expr}).clone(); {subscribe_code} {}::new(&__rml_entity) }})",
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

/// 生成带委托注入的 Stateful 组件构造表达式
///
/// 与 `gen_stateful_body` 类似，但在 state_ctor 闭包前提取 ViewModel 的委托字段，
/// 通过 `move` 闭包捕获 `__rml_delegate` 变量传入 state 构造器。
///
/// 生成代码形如：
/// ```ignore
/// ({
///     let __rml_delegate = (self.field).clone();
///     let __rml_entity = (self.__rml_state.get_or_init_ref("ref", _window, &mut *cx, move |w, c| ...)).clone();
///     {subscribe_code}
///     Component::new(&__rml_entity)
/// })
/// ```
pub(crate) fn gen_stateful_with_delegate_body(
    elem: &Element,
    component: &tags::ComponentTag,
    ref_name: Option<&str>,
    state_field: &str,
    state_ctor: &str,
    delegate_attr: &str,
    _loop_vars: &[String],
) -> Result<String, CodegenError> {
    let tag = &elem.tag;
    let resolved = tags::normalize_component_tag(tag);

    // 从 bind 属性提取委托字段名（如 items={my_items} → "my_items"）
    let delegate_expr = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Bind { name, expr, .. } = attr {
            (name == delegate_attr).then(|| expr.clone())
        } else {
            None
        }
    });

    let _ = state_field; // StatefulWithDelegate 要求 ref，state_field 仅用于文档一致性

    let ref_name = ref_name.ok_or_else(|| CodegenError {
        message: format!(
            "<{}> is a StatefulWithDelegate component and requires `ref=\"name\"` directive for delegate injection",
            tag
        ),
        span: Some(elem.span),
    })?;

    let delegate_field = delegate_expr.ok_or_else(|| CodegenError {
        message: format!(
            "<{}> requires `{}={{field}}` bind attribute to provide delegate data",
            tag, delegate_attr
        ),
        span: Some(elem.span),
    })?;

    let (delegate_field, _converter) = extract_field_converter(&delegate_field);

    // 收集 State 事件处理器（同 gen_stateful_body）
    let state_event_handlers: Vec<(&str, &EventHandler)> = elem
        .attributes
        .iter()
        .filter_map(|attr| {
            if let Attribute::Event { name, handler, .. } = attr {
                if crate::compiler::components::state_event::is_state_event(name, &resolved) {
                    Some((name.as_str(), handler))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // 将 delegate 提取内联到 get_or_init_ref 的构造器参数中，形成 block 表达式：
    //   self.__rml_state.get_or_init_ref("ref", _window, &mut *cx, {
    //       let __rml_delegate = (self.field).clone();
    //       move |w, c| SelectState::new(__rml_delegate, None, w, c)
    //   })
    // 使用 `self.__rml_state` 和 `self.field`（而非 self_prefix）使 extract_state_refs
    // 能检测并预提取整个调用到 slot 闭包外的 render 作用域（此处 self 可变），
    // delegate 提取也在 block 内一并带出，避免 slot 闭包内 &Self 无法 &mut __rml_state 的问题。
    let entity_expr = format!(
        "self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {{ let __rml_delegate = (self.{}).clone(); {} }})",
        ref_name, delegate_field, state_ctor
    );

    if !state_event_handlers.is_empty() {
        let ref_key = ref_name;
        let state_subscribe_code: String = state_event_handlers
            .iter()
            .map(|(event_name, handler)| {
                crate::compiler::components::state_event::gen_state_event_subscribe(
                    ref_key, event_name, handler, &resolved,
                )
            })
            .collect::<Vec<_>>()
            .join(" ");
        Ok(format!(
            "({{ let __rml_entity = ({entity_expr}).clone(); {subscribe_code} {}::new(&__rml_entity) }})",
            component.ctor_path,
            entity_expr = entity_expr,
            subscribe_code = state_subscribe_code,
        ))
    } else {
        Ok(format!(
            "({{ let __rml_entity = ({entity_expr}).clone(); {}::new(&__rml_entity) }})",
            component.ctor_path,
            entity_expr = entity_expr,
        ))
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(StatefulComponentTranslator);
}
