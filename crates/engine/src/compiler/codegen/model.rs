//! Model 字段收集 —— 提取 RML 中所有 `model={field}` 绑定的字段名

use crate::compiler::InputHandlers;
use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
use std::collections::HashMap;

/// 收集 RML 中所有 `model={field}` 绑定的字段名
pub fn collect_model_fields(root: &Node) -> Vec<String> {
    let mut fields = Vec::new();
    if let Node::Element(elem) = root {
        collect_model_fields_recursive(elem, &mut fields);
    }
    fields.sort();
    fields.dedup();
    fields
}

fn collect_model_fields_recursive(elem: &Element, fields: &mut Vec<String>) {
    for directive in &elem.directives {
        if let Directive::Model { field, .. } = directive {
            fields.push(field.clone());
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_model_fields_recursive(child_elem, fields);
        }
    }
}

/// 收集 `model={field | Converter}` 的 converter 映射
///
/// key 为字段名，value 为 converter 类型名（如 "Currency"）。
pub fn collect_model_converters(root: &Node) -> HashMap<String, String> {
    let mut converters = HashMap::new();
    if let Node::Element(elem) = root {
        collect_model_converters_recursive(elem, &mut converters);
    }
    converters
}

fn collect_model_converters_recursive(elem: &Element, converters: &mut HashMap<String, String>) {
    for directive in &elem.directives {
        if let Directive::Model { field, converter: Some(c) } = directive {
            converters.insert(field.clone(), c.clone());
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_model_converters_recursive(child_elem, converters);
        }
    }
}

/// 收集 `<input model={field} oninput={fn} onchange={fn} />` 的 handler 映射
///
/// 仅当元素同时声明 `model` 指令与 `oninput`/`onchange` 事件属性时才收集。
/// key 为 model 字段名，value 为对应 handler 方法名。
pub fn collect_model_input_handlers(root: &Node) -> HashMap<String, InputHandlers> {
    let mut handlers = HashMap::new();
    if let Node::Element(elem) = root {
        collect_model_input_handlers_recursive(elem, &mut handlers);
    }
    handlers
}

fn collect_model_input_handlers_recursive(
    elem: &Element,
    handlers: &mut HashMap<String, InputHandlers>,
) {
    // 查找当前元素的 model 指令（取第一个）
    let model_field = elem.directives.iter().find_map(|d| {
        if let Directive::Model { field, .. } = d {
            Some(field.clone())
        } else {
            None
        }
    });

    if let Some(field) = model_field {
        let entry = handlers.entry(field).or_default();
        for attr in &elem.attributes {
            if let Attribute::Event { name, handler, .. } = attr {
                let method = match handler {
                    EventHandler::Ident(m) | EventHandler::MethodName(m) => m.clone(),
                    EventHandler::WithArgs(m, _) => m.clone(),
                };
                match name.as_str() {
                    "on_input" => entry.on_input = Some(method),
                    "on_change" => entry.on_change = Some(method),
                    _ => {}
                }
            }
        }
    }

    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_model_input_handlers_recursive(child_elem, handlers);
        }
    }
}
