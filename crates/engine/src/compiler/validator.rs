//! 语义验证器
//!
//! Phase A：仅校验语法合法性（不校验 ViewModel 字段类型）。
//! Phase C：校验 slot 名合法性 + 未知属性（编译期 error）。

use crate::compiler::UserComponentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};
use crate::tags;
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error: {}", self.message)
    }
}

impl std::error::Error for ValidationError {}

/// 校验 AST 合法性
///
/// `user_components` 传入用户自定义组件注册表，用于校验 `<template slot="x">` 中
/// `x` 是否在目标组件的 `slots` 声明中。
pub fn validate(
    node: &Node,
    user_components: &HashMap<String, UserComponentInfo>,
) -> Result<(), ValidationError> {
    validate_node(node, &mut ValidationCtx::default(), user_components)
}

#[derive(Default)]
struct ValidationCtx {
    ref_names: std::collections::HashSet<String>,
}

fn validate_node(
    node: &Node,
    ctx: &mut ValidationCtx,
    user_components: &HashMap<String, UserComponentInfo>,
) -> Result<(), ValidationError> {
    match node {
        Node::Element(elem) => validate_element(elem, ctx, user_components),
        Node::Text(_) | Node::Interpolation(_) | Node::MixedText(_) => Ok(()),
    }
}

fn validate_element(
    elem: &Element,
    ctx: &mut ValidationCtx,
    user_components: &HashMap<String, UserComponentInfo>,
) -> Result<(), ValidationError> {
    // 校验指令
    let mut has_model = false;

    for d in &elem.directives {
        match d {
            Directive::Model(_) => has_model = true,
            Directive::Ref(name) => {
                if !ctx.ref_names.insert(name.clone()) {
                    return Err(ValidationError {
                        message: format!("duplicate ref name: {}", name),
                    });
                }
            }
            // Phase B-1：允许 if/each/else/once/html/key/show 通过校验
            // Phase B-2 会补全 else 必须紧跟 if 的语义校验、each 子句校验等
            Directive::If(_) | Directive::Each(_) | Directive::Else | Directive::Once
            | Directive::Html(_) | Directive::Key(_) | Directive::Show(_) => {}
        }
    }

    // model 只能用于 input/textarea
    if has_model {
        let tag = elem.tag.as_str();
        if tag != "input" && tag != "textarea" {
            return Err(ValidationError {
                message: format!("`model` directive can only be used on <input>/<textarea>, got <{}>", tag),
            });
        }
    }

    // 校验 slot 名合法性：用户自定义组件的 <template slot="x"> 中 x 必须在组件 slots 声明中
    if let Some(info) = user_components.get(&elem.tag) {
        if !info.slots.is_empty() {
            for child in &elem.children {
                if let Node::Element(child_elem) = child {
                    if child_elem.tag == "template" {
                        if let Some(slot_name) = &child_elem.slot_name {
                            if !info.slots.iter().any(|s| s == slot_name) {
                                return Err(ValidationError {
                                    message: format!(
                                        "unknown slot name `{}` for component <{}>: declared slots are {:?}",
                                        slot_name, elem.tag, info.slots
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 校验未知属性：扩展组件的 bind/event 属性必须在 props_registry 中登记
    validate_unknown_props(elem)?;

    // 递归校验子节点
    for child in &elem.children {
        validate_node(child, ctx, user_components)?;
    }

    Ok(())
}

/// 校验扩展组件和 shell 根标签的未知属性
///
/// - 扩展组件（`tags::is_extension_component`）：bind/event 属性用 `is_prop_registered` 校验
/// - Shell 根标签（tab_window/modern_window/window/dialog）：bind/event 属性用 `is_shell_prop_registered` 校验
/// - static 属性宽松处理（可能有自定义用途，不报错）
fn validate_unknown_props(elem: &Element) -> Result<(), ValidationError> {
    let tag = &elem.tag;

    // Shell 根标签
    if tags::root_tag_lookup(tag).is_some() {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, .. } | Attribute::Event { name, .. } = attr {
                if !crate::compiler::props_registry::is_shell_prop_registered(tag, name) {
                    return Err(ValidationError {
                        message: format!(
                            "unknown property `{}` on <{}>: not in shell property registry",
                            name, tag
                        ),
                    });
                }
            }
        }
        return Ok(());
    }

    // 扩展组件（PascalCase / kebab-case / 特殊小写如 menu/status_bar）
    if tags::is_extension_component(tag) {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, .. } | Attribute::Event { name, .. } = attr {
                if !crate::compiler::props_registry::is_prop_registered(tag, name) {
                    return Err(ValidationError {
                        message: format!(
                            "unknown property `{}` on <{}>: not in component property registry",
                            name, tag
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}
