//! Model 字段收集 —— 提取 RML 中所有 `model={field}` 绑定的字段名

use crate::parser::ast::{Directive, Element, Node};

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
        if let Directive::Model(field) = directive {
            fields.push(field.clone());
        }
    }
    for child in &elem.children {
        if let Node::Element(child_elem) = child {
            collect_model_fields_recursive(child_elem, fields);
        }
    }
}
