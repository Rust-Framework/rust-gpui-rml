//! Input/TextInput 专用 translator —— 注入 placeholder/default_value/masked 到 state_ctor
//!
//! 继承 StatefulComponentTranslator 模式，特化处理 state_ctor 构造：
//! - `value={field}` 路径：委托 `gen_model_input`（InputStateBridge，placeholder 已通过 builder 支持）
//! - ref/无绑定路径：从元素属性提取 `placeholder`（Static/Bind）、`default_value`（Static）、`masked`（Static bool）
//! - 构建自定义 state_ctor：`|w, c| rml_ui::InputState::new(w, c).placeholder("...").masked(true).default_value("...")`
//! - 调用 `gen_stateful_body` 生成构造表达式（含 Input 事件订阅）
//! - 应用剩余 setter（size/disabled/selected 等），跳过已注入 state_ctor 的属性

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use super::stateful::gen_stateful_body;
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::binding::{gen_model_input, gen_model_state_bridge};
use crate::compiler::codegen::extract_field_converter;
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
const SKIP_ATTRS: &[&str] = &["placeholder", "default_value", "masked"];

#[derive(Debug)]
pub struct InputTranslator;

impl IRmlTranslator for InputTranslator {
    fn tag(&self) -> &'static str {
        "Input"
    }

    fn matches(&self, elem: &Element) -> bool {
        let canonical = tags::canonical_tag(&elem.tag);
        matches!(canonical.as_str(), "Input" | "TextInput")
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

        // C2: value={field} 路径 → gen_model_input（InputStateBridge，placeholder 已通过 builder 支持）
        if let Some(expr) = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                (name == "value").then(|| expr.clone())
            } else {
                None
            }
        }) {
            let (field, _) = extract_field_converter(&expr);
            let code = gen_model_input(elem, ctx, _id_counter, field, false, parents)?;
            return Ok((code, false));
        }

        // C4: StateBridge 路径（若组件在 STATE_BRIDGE_REGISTRY 中注册）
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

        // ref/无绑定路径：自定义 state_ctor 注入 placeholder/default_value/masked
        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });

        let masked = extract_static_bool(elem, "masked");
        let default_value = extract_static_string(elem, "default_value");
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        let state_ctor = build_input_state_ctor(elem, masked, default_value, &lv, &computed);

        let mut code = gen_stateful_body(
            elem,
            &component,
            ref_name,
            "input_state",
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
        TranslatorMetadata::new("Input", "Input", ComponentCategory::Layout)
    }
}

/// 构建 InputState 的 state_ctor 闭包表达式
///
/// 根据 placeholder 属性类型（Static/Bind/无）选择不同构造方式：
/// - Static: `|w, c| rml_ui::InputState::new(w, c).placeholder("...").masked(true).default_value("...")`
/// - Bind: `{ let __rml_placeholder = (self.field).clone(); move |w, c| rml_ui::InputState::new(w, c).placeholder(__rml_placeholder).masked(true).default_value("...") }`
/// - 无: `|w, c| rml_ui::InputState::new(w, c).masked(true).default_value("...")`
fn build_input_state_ctor(
    elem: &Element,
    masked: bool,
    default_value: Option<String>,
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    // 检测 placeholder 是 Static 还是 Bind
    let placeholder_static = extract_static_string(elem, "placeholder");
    let placeholder_bind: Option<&str> = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Bind { name, expr, .. } = attr {
            (name == "placeholder").then(|| expr.as_str())
        } else {
            None
        }
    });

    // 构建链式 builder 后缀（masked + default_value）
    let mut suffix = String::new();
    if masked {
        suffix.push_str(".masked(true)");
    }
    if let Some(dv) = &default_value {
        suffix.push_str(&format!(".default_value({:?})", dv));
    }

    match (placeholder_static, placeholder_bind) {
        (Some(text), _) => {
            // Static placeholder
            format!(
                "|w, c| rml_ui::InputState::new(w, c).placeholder({:?}){}",
                text, suffix
            )
        }
        (None, Some(expr)) => {
            // Bind placeholder — clone 前置，move 闭包捕获
            let rust_expr = component_bind_rust_expr(expr, loop_vars, computed);
            format!(
                "{{ let __rml_placeholder = ({}).clone(); move |w, c| rml_ui::InputState::new(w, c).placeholder(__rml_placeholder){} }}",
                rust_expr, suffix
            )
        }
        (None, None) => {
            // 无 placeholder
            format!("|w, c| rml_ui::InputState::new(w, c){}", suffix)
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

fn extract_static_bool(elem: &Element, name: &str) -> bool {
    elem.attributes.iter().any(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            n == name && (value.is_empty() || value.eq_ignore_ascii_case("true"))
        } else {
            false
        }
    })
}

fn extract_static_string(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            if n == name {
                return Some(value.clone());
            }
        }
        None
    })
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(InputTranslator);
}
