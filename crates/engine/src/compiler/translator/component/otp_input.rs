//! OtpInput 专用 translator —— 注入 length/masked/default_value 到 state_ctor
//!
//! 继承 StatefulComponentTranslator 模式，特化处理 state_ctor 构造：
//! - 从元素属性提取 `length`（默认 6）、`masked`（默认 false）、`default_value`（可选）
//! - 构建自定义 state_ctor：`|w, c| rml_ui::OtpState::new(N, w, c).masked(B)[.default_value("...")]`
//! - 调用 `gen_stateful_body` 生成构造表达式（含 Input 事件订阅）
//! - 应用剩余 setter（groups/size/disabled），跳过已注入 state_ctor 的属性

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use super::stateful::gen_stateful_body;
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element};
use crate::tags;

/// 由 state_ctor 注入的属性，不参与 setter 分发
const SKIP_ATTRS: &[&str] = &["length", "masked", "default_value"];

#[derive(Debug)]
pub struct OtpInputTranslator;

impl IRmlTranslator for OtpInputTranslator {
    fn tag(&self) -> &'static str {
        "OtpInput"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "OtpInput"
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

        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });

        let length = extract_static_usize(elem, "length").unwrap_or(6);
        let masked = extract_static_bool(elem, "masked");
        let default_value = extract_static_string(elem, "default_value");

        let mut state_ctor = format!(
            "|w, c| rml_ui::OtpState::new({}usize, w, c).masked({})",
            length, masked
        );
        if let Some(dv) = default_value {
            state_ctor.push_str(&format!(".default_value({:?})", dv));
        }

        let mut code = gen_stateful_body(
            elem,
            &component,
            ref_name,
            "otp_state",
            &state_ctor,
            loop_vars,
        )?;

        append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
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
                    if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, &resolved) {
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
        TranslatorMetadata::new("OtpInput", "OtpInput", ComponentCategory::Layout)
    }
}

fn attr_name(attr: &Attribute) -> &str {
    match attr {
        Attribute::Static { name, .. }
        | Attribute::Bind { name, .. }
        | Attribute::Event { name, .. } => name,
    }
}

fn extract_static_usize(elem: &Element, name: &str) -> Option<usize> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            if n == name {
                return value.parse::<usize>().ok();
            }
        }
        None
    })
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
    registry.register(OtpInputTranslator);
}
