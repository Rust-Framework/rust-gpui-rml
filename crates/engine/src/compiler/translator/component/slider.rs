//! Slider 专用 translator —— 注入 min/max/step/default_value 到 state_ctor
//!
//! 继承 StatefulComponentTranslator 模式，特化处理 state_ctor 构造：
//! - `value={field}` 路径：委托 `gen_model_state_bridge`（SliderStateBridge 双向绑定）
//! - ref/无绑定路径：从元素属性提取 `min`/`max`/`step`（Static f32）、`default_value`
//!   （Static f32 或 Bind 表达式），构建链式 builder state_ctor
//! - 调用 `gen_stateful_body` 生成构造表达式（含 on_change 事件订阅，通过 state_event 注册）
//! - 应用剩余 setter（disabled/size 等），跳过已注入 state_ctor 的属性

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use super::stateful::gen_stateful_body;
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::binding::gen_model_state_bridge;
use crate::compiler::codegen::extract_field_converter;
use crate::compiler::expr;
use crate::compiler::setters::{
    component_bind_rust_expr, component_bind_setter, component_event_setter,
    component_static_setter,
};
use crate::compiler::state_bridge::lookup_state_bridge_for_tag;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element};
use crate::tags;

/// 由 state_ctor 注入的属性，不参与 setter 分发
const SKIP_ATTRS: &[&str] = &["min", "max", "step", "default_value"];

#[derive(Debug)]
pub struct SliderTranslator;

impl IRmlTranslator for SliderTranslator {
    fn tag(&self) -> &'static str {
        "Slider"
    }

    fn matches(&self, elem: &Element) -> bool {
        let canonical = tags::canonical_tag(&elem.tag);
        canonical.as_str() == "Slider"
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
        let component = tags::component_lookup_resolved(tag).ok_or_else(|| CodegenError {
            message: format!("unknown component: <{}>", tag),
            span: Some(elem.span),
        })?;

        // StateBridge 路径：value={field} → gen_model_state_bridge（SliderStateBridge 双向绑定）
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

        // ref/无绑定路径：自定义 state_ctor 注入 min/max/step/default_value
        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });

        let min = extract_static_f32(elem, "min");
        let max = extract_static_f32(elem, "max");
        let step = extract_static_f32(elem, "step");
        let default_value_static = extract_static_f32(elem, "default_value");
        let default_value_bind: Option<&str> = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                (name == "default_value").then(|| expr.as_str())
            } else {
                None
            }
        });

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        let state_ctor = build_slider_state_ctor(
            min, max, step, default_value_static, default_value_bind, &lv, &computed,
        );

        let mut code = gen_stateful_body(
            elem,
            &component,
            ref_name,
            "slider_state",
            &state_ctor,
            loop_vars,
        )?;

        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        for attr in &elem.attributes {
            let name = attr_name(attr);
            if SKIP_ATTRS.contains(&name) {
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
        TranslatorMetadata::new("Slider", "Slider", ComponentCategory::Layout)
    }
}

/// 构建 SliderState 的 state_ctor 闭包表达式
///
/// 根据属性组合生成不同构造方式：
/// - Static default_value: `move |_w, _c| rml_ui::SliderState::new().min(0.0).max(100.0).step(1.0).default_value(50.0)`
/// - Bind default_value: `{ let __rml_default_value = (self.field).clone(); move |_w, _c| rml_ui::SliderState::new()...default_value(__rml_default_value) }`
/// - 无 default_value: `move |_w, _c| rml_ui::SliderState::new().min(0.0).max(100.0)`
fn build_slider_state_ctor(
    min: Option<f32>,
    max: Option<f32>,
    step: Option<f32>,
    default_value_static: Option<f32>,
    default_value_bind: Option<&str>,
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    let mut chain = String::new();
    if let Some(v) = min {
        chain.push_str(&format!(".min({}f32)", v));
    }
    if let Some(v) = max {
        chain.push_str(&format!(".max({}f32)", v));
    }
    if let Some(v) = step {
        chain.push_str(&format!(".step({}f32)", v));
    }

    match (default_value_static, default_value_bind) {
        (Some(v), _) => {
            format!(
                "move |_w, _c| rml_ui::SliderState::new(){}.default_value({}f32)",
                chain, v
            )
        }
        (None, Some(expr)) => {
            let rust_expr = expr::with_self_alias("self", || {
                component_bind_rust_expr(expr, loop_vars, computed)
            });
            format!(
                "{{ let __rml_default_value = ({}).clone(); move |_w, _c| rml_ui::SliderState::new(){}.default_value(__rml_default_value) }}",
                rust_expr, chain
            )
        }
        (None, None) => {
            format!("move |_w, _c| rml_ui::SliderState::new(){}", chain)
        }
    }
}

fn attr_name(attr: &Attribute) -> &str {
    match attr {
        Attribute::Static { name, .. }
        | Attribute::Bind { name, .. }
        | Attribute::Event { name, .. } => name,
    }
}

fn extract_static_f32(elem: &Element, name: &str) -> Option<f32> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            if n == name {
                return value.parse::<f32>().ok();
            }
        }
        None
    })
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(SliderTranslator);
}
