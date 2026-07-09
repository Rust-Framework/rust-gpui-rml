//! 语义验证器
//!
//! Phase A：仅校验语法合法性（不校验 ViewModel 字段类型）。
//! Phase C：校验 slot 名合法性 + 未知属性（编译期 error）。

use crate::compiler::translator::TranslatorRegistry;
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
/// - `registry`: translator 注册表，用于查询根节点等内置 translator 的元数据
///   （如允许 slot 名、是否根节点）。
/// - `user_components`: 用户自定义组件注册表，用于校验 `<template slot="x">` 中
///   `x` 是否在组件的 `slots` 声明中。
pub fn validate(
    node: &Node,
    registry: &TranslatorRegistry,
    user_components: &HashMap<String, UserComponentInfo>,
) -> Result<(), ValidationError> {
    validate_node(node, registry, &mut ValidationCtx::default(), user_components, 0)
}

#[derive(Default)]
struct ValidationCtx {
    ref_names: std::collections::HashSet<String>,
}

fn validate_node(
    node: &Node,
    registry: &TranslatorRegistry,
    ctx: &mut ValidationCtx,
    user_components: &HashMap<String, UserComponentInfo>,
    depth: usize,
) -> Result<(), ValidationError> {
    match node {
        Node::Element(elem) => {
            validate_element(elem, registry, ctx, user_components, depth)
        }
        Node::Text(_) | Node::Interpolation { .. } | Node::MixedText(_) => Ok(()),
    }
}

fn validate_element(
    elem: &Element,
    registry: &TranslatorRegistry,
    ctx: &mut ValidationCtx,
    user_components: &HashMap<String, UserComponentInfo>,
    depth: usize,
) -> Result<(), ValidationError> {
    // 校验 <style> 元素的合法使用
    if elem.tag == "style" {
        validate_style_element(elem, depth)?;
    }
    // 校验指令
    let mut has_model = false;

    for d in &elem.directives {
        match d {
            Directive::Model { .. } => has_model = true,
            Directive::Ref { name, .. } => {
                if !ctx.ref_names.insert(name.clone()) {
                    return Err(ValidationError {
                        message: format!("duplicate ref name: {}", name),
                    });
                }
            }
            // Phase B-1：允许 if/each/else/once/html/key/show 通过校验
            // Phase B-2 会补全 else 必须紧跟 if 的语义校验、each 子句校验等
            Directive::If { .. } | Directive::Each { .. } | Directive::Else { .. } | Directive::Once { .. }
            | Directive::Html { .. } | Directive::Key { .. } | Directive::Show { .. } => {}
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

    // Shell 根标签的 slot 名白名单校验
    // 防止未知 slot 名（如 `<template slot="tabs">` 误用在 modern-window 上）静默落入 body
    if let Some(meta) = registry.metadata(&elem.tag) {
        if meta.is_root && !meta.allowed_slots.is_empty() {
            let allowed_slots = meta.allowed_slots;
            for child in &elem.children {
                if let Node::Element(child_elem) = child {
                    if child_elem.tag == "template" {
                        if let Some(slot_name) = &child_elem.slot_name {
                            if !allowed_slots.contains(&slot_name.as_str()) {
                                return Err(ValidationError {
                                    message: format!(
                                        "unknown slot name `{}` for <{}>: allowed slots are {:?}",
                                        slot_name, elem.tag, allowed_slots
                                    ),
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    // 校验 scope 属性：仅可在 <template slot="..."> 上使用，且必须为简单标识符。
    // scope 用于作用域插槽，向 slot 内容注入 &dyn ISlotScope 变量（如 scope={panel}）。
    // 复杂表达式（如 scope={foo.bar}）不支持 — codegen 只识别简单标识符。
    // 在无 resizable 的 shell slot（menu/title/footer/tabs）上写 scope 仅警告（不阻塞）。
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            if child_elem.tag == "template" {
                let has_slot = child_elem.slot_name.is_some();
                let scope_attr = child_elem.attributes.iter().find_map(|a| match a {
                    Attribute::Bind { name, expr, .. } if name == "scope" => Some(expr.clone()),
                    _ => None,
                });
                if let Some(expr) = scope_attr {
                    if !has_slot {
                        return Err(ValidationError {
                            message: format!(
                                "scope 属性仅可出现在 `<template slot=\"...\">` 上，得到无 slot 属性的 <template>"
                            ),
                        });
                    }
                    // scope 表达式必须是简单标识符
                    let trimmed = expr.trim();
                    let is_simple_ident = !trimmed.is_empty()
                        && trimmed
                            .chars()
                            .all(|c| c.is_alphanumeric() || c == '_')
                        && trimmed
                            .chars()
                            .next()
                            .map_or(false, |c| c.is_alphabetic() || c == '_');
                    if !is_simple_ident {
                        return Err(ValidationError {
                            message: format!(
                                "scope 属性必须是简单标识符，得到 `{}`（示例：scope={{panel}}）",
                                expr
                            ),
                        });
                    }
                    // 在无 resizable 的 shell slot 上使用 scope → 警告（不阻塞编译）
                    if let Some(slot_name) = &child_elem.slot_name {
                        if matches!(
                            slot_name.as_str(),
                            "menu" | "title" | "footer" | "tabs"
                        ) {
                            eprintln!(
                                "[rml warning] <template slot=\"{}\"> 不支持 resizable 操控，scope 变量将仅暴露插槽名",
                                slot_name
                            );
                        }
                    }
                }
            }
        }
    }

    // 校验未知属性：扩展组件的 bind/event 属性必须在 props_registry 中登记
    validate_unknown_props(elem, registry)?;

    // 递归校验子节点
    for child in &elem.children {
        validate_node(child, registry, ctx, user_components, depth + 1)?;
    }

    Ok(())
}

/// 校验 `<style>` 元素的合法使用
///
/// 规则：
/// 1. 必须包含 `source` 静态属性（不支持 bind 形式）
/// 2. 不能有子节点（自闭合或空）
/// 3. 必须是根元素的直接子节点（depth == 1），不能嵌套在其他元素或 `<template slot>` 内
fn validate_style_element(elem: &Element, depth: usize) -> Result<(), ValidationError> {
    // 规则 3：必须是根元素的直接子节点
    if depth != 1 {
        return Err(ValidationError {
            message: "`<style>` 必须是根元素的直接子节点，不能嵌套在其他元素或 `<template slot>` 内部".into(),
        });
    }

    // 规则 1：必须有 source 静态属性
    let has_static_source = elem.attributes.iter().any(|attr| {
        matches!(attr, Attribute::Static { name, .. } if name == "source")
    });
    let has_bind_source = elem.attributes.iter().any(|attr| {
        matches!(attr, Attribute::Bind { name, .. } if name == "source")
    });
    if has_bind_source {
        return Err(ValidationError {
            message: "`<style>` 的 `source` 属性必须是静态字符串（如 `source=\"index.css\"`），不支持绑定形式"
                .into(),
        });
    }
    if !has_static_source {
        return Err(ValidationError {
            message: "`<style>` 元素必须包含 `source` 属性（如 `<style source=\"index.css\" />`）".into(),
        });
    }

    // 规则 2：不能有子节点
    if !elem.children.is_empty() {
        return Err(ValidationError {
            message: "`<style>` 元素不能包含子节点，请使用自闭合形式 `<style source=\"...\" />`".into(),
        });
    }

    Ok(())
}

/// 校验扩展组件和 shell 根标签的未知属性
///
/// - 扩展组件（`tags::is_extension_component`）：bind/event 属性用 `is_prop_registered` 校验
/// - Shell 根标签（tab-window/modern-window/window/dialog）：bind/event 属性用 `is_shell_prop_registered` 校验
/// - static 属性宽松处理（可能有自定义用途，不报错）
fn validate_unknown_props(
    elem: &Element,
    registry: &TranslatorRegistry,
) -> Result<(), ValidationError> {
    let tag = &elem.tag;

    // Shell 根标签：通过注册表元数据识别
    if registry.metadata(tag).map(|m| m.is_root).unwrap_or(false) {
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

    // 扩展组件（PascalCase / kebab-case / 特殊小写如 menu/status-bar）
    // 或 item builder 子标签（如 AccordionItem，不在 component_lookup 中，
    // 通过 is_item_builder_tag 识别）
    if tags::is_extension_component(tag) || tags::is_item_builder_tag(tag) {
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
