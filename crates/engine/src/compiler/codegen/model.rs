//! 双向绑定字段收集 —— 提取 RML 中所有 `value={field}` 绑定的字段名
//!
//! input/textarea 及 PascalCase Input/TextInput/NumberInput 的 `value={field}`
//! bind 属性自动双向绑定（A3 自动推断 + C2 InputStateBridge）。

use crate::compiler::InputHandlers;
use crate::parser::ast::{Attribute, Element, EventHandler, Node};
use std::collections::HashMap;

/// 从 `value={expr}` 绑定表达式中提取 field 名和 converter 名
///
/// - `"username"` → ("username", None)
/// - `"price | Currency"` → ("price", Some("Currency"))
/// - `"a || b"` → ("a || b", None) — 逻辑 OR，非 converter
pub fn extract_field_converter(expr: &str) -> (String, Option<String>) {
    if let Some(pos) = expr.find('|') {
        if expr.as_bytes().get(pos + 1) == Some(&b'|') {
            return (expr.to_string(), None);
        }
        let field = expr[..pos].trim().to_string();
        let converter = expr[pos + 1..].trim().to_string();
        (field, Some(converter))
    } else {
        (expr.to_string(), None)
    }
}

/// 判断标签是否支持 `value={field}` 自动双向绑定
///
/// 小写 `<input>`/`<textarea>` 走 InputTranslator/TextAreaTranslator 路径，
/// PascalCase `<Input>`/`<TextInput>`/`<NumberInput>` 走 StatefulComponentTranslator
/// 的 InputStateBridge 路径（C2）。两者均复用 InputState 双向同步机制。
fn supports_twoway_value(tag: &str) -> bool {
    matches!(tag, "input" | "textarea" | "Input" | "TextInput" | "NumberInput")
}

/// 收集 RML 中所有双向绑定字段名
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
    if supports_twoway_value(&elem.tag) {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "value" {
                    let (field, _) = extract_field_converter(expr);
                    fields.push(field);
                }
            }
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_model_fields_recursive(child_elem, fields);
        }
    }
}

/// 收集双向绑定的 converter 映射
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
    if supports_twoway_value(&elem.tag) {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "value" {
                    let (field, Some(c)) = extract_field_converter(expr) else {
                        continue;
                    };
                    converters.insert(field, c);
                }
            }
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_model_converters_recursive(child_elem, converters);
        }
    }
}

/// 收集 `<input value={field} oninput={fn} onchange={fn} />` 的 handler 映射
///
/// 仅当 input/textarea 同时声明 `value={field}` bind 与 `oninput`/`onchange` 事件时才收集。
/// key 为字段名，value 为对应 handler 方法名。
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
    let bind_field: Option<String> = if supports_twoway_value(&elem.tag) {
        elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "value" {
                    let (field, _) = extract_field_converter(expr);
                    Some(field)
                } else {
                    None
                }
            } else {
                None
            }
        })
    } else {
        None
    };

    if let Some(field) = bind_field {
        let entry = handlers.entry(field).or_default();
        for attr in &elem.attributes {
            if let Attribute::Event { name, handler, .. } = attr {
                let method = match handler {
                    EventHandler::Ident(m) | EventHandler::MethodName(m) => m.clone(),
                    EventHandler::WithArgs(m, _) => m.clone(),
                    EventHandler::ClosureField(_) => String::new(),
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

/// 收集 `<Slider value={field}>` 双向绑定字段名（C3：Slider StateBridge）
pub fn collect_slider_fields(root: &Node) -> Vec<String> {
    let mut fields = Vec::new();
    if let Node::Element(elem) = root {
        collect_slider_fields_recursive(elem, &mut fields);
    }
    fields.sort();
    fields.dedup();
    fields
}

fn collect_slider_fields_recursive(elem: &Element, fields: &mut Vec<String>) {
    let canonical = crate::tags::canonical_tag(&elem.tag);
    if canonical == "Slider" {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "value" {
                    let (field, _) = extract_field_converter(expr);
                    fields.push(field);
                }
            }
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_slider_fields_recursive(child_elem, fields);
        }
    }
}
