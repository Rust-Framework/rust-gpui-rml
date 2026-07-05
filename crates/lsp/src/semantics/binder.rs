//! 绑定路径/命令名解析
//!
//! 遍历 AST，对每个绑定表达式 / 命令名做存在性校验（MVP 不做类型推导）。
//! 绑定路径的根标识符须存在于 `StructMetadata.observable_fields` 或 `computed_methods`；
//! 命令名须存在于 `StructMetadata.commands`。

use std::collections::HashMap;

use rust_rml_engine::build::scanner::StructMetadata;
use rust_rml_engine::parser::ast::{Attribute, Directive, Element, EventHandler, Node, TextSegment};
use rust_rml_engine::parser::Span;

use crate::semantics::diagnostics::SemanticDiagnostic;

/// 对 AST 做语义绑定检查
///
/// `metadata_map` 来自 ProjectIndex，包含 .rml 对应 code-behind 的所有 struct 元信息。
/// MVP 阶段：只检查第一个 struct 的字段/命令（典型 .rml.rs 只有一个 #[window] struct）。
pub fn bind(
    root: &Node,
    metadata_map: Option<&HashMap<String, StructMetadata>>,
) -> Vec<SemanticDiagnostic> {
    let mut diagnostics = Vec::new();
    // 取第一个 struct 的元信息作为 ViewModel（MVP 简化）
    let meta = metadata_map.and_then(|m| m.values().next());
    bind_node(root, meta, &mut diagnostics);
    diagnostics
}

fn bind_node(node: &Node, meta: Option<&StructMetadata>, diags: &mut Vec<SemanticDiagnostic>) {
    match node {
        Node::Element(elem) => bind_element(elem, meta, diags),
        Node::Interpolation { expr, .. } => {
            check_binding_expr(expr, elem_span_or_default(node), meta, diags);
        }
        Node::MixedText(segs) => {
            for seg in segs {
                if let TextSegment::Interpolation { expr, .. } = seg {
                    check_binding_expr(expr, Span::empty(), meta, diags);
                }
            }
        }
        Node::Text(_) => {}
    }
}

fn bind_element(elem: &Element, meta: Option<&StructMetadata>, diags: &mut Vec<SemanticDiagnostic>) {
    // 检查指令中的绑定表达式
    for directive in &elem.directives {
        match directive {
            Directive::If(expr) | Directive::Show(expr) | Directive::Key(expr) => {
                check_binding_expr(expr, elem.span, meta, diags);
            }
            Directive::Model { field, .. } => {
                check_binding_expr(field, elem.span, meta, diags);
            }
            Directive::Html(expr) => {
                check_binding_expr(expr, elem.span, meta, diags);
            }
            Directive::Each(each) => {
                // 检查迭代源（iterable），item/index 变量不检查
                check_binding_expr(&each.iterable, elem.span, meta, diags);
            }
            _ => {}
        }
    }

    // 检查属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Bind { name: _, expr, .. } => {
                check_binding_expr(expr, elem.span, meta, diags);
            }
            Attribute::Event { name: _, handler, .. } => {
                check_event_handler(handler, elem.span, meta, diags);
            }
            Attribute::Static { .. } => {}
        }
    }

    // 递归子节点
    for child in &elem.children {
        bind_node(child, meta, diags);
    }
}

/// 检查绑定表达式：提取根标识符，校验是否存在于 observable_fields / computed_methods
fn check_binding_expr(
    expr: &str,
    span: Span,
    meta: Option<&StructMetadata>,
    diags: &mut Vec<SemanticDiagnostic>,
) {
    let Some(meta) = meta else { return };
    let Some(root_ident) = extract_root_ident(expr) else { return };

    // 跳过常见非字段标识符：_window, cx, 字面量, 循环变量（无法在此处知晓，跳过以字母开头的简单标识）
    if root_ident == "cx" || root_ident == "_window" || root_ident == "true" || root_ident == "false" {
        return;
    }

    let is_valid = meta.observable_fields.contains(&root_ident)
        || meta.computed_methods.contains(&root_ident)
        || root_ident.parse::<i64>().is_ok()  // 数字字面量
        || root_ident.parse::<f64>().is_ok();

    if !is_valid {
        diags.push(SemanticDiagnostic::warning(
            span,
            format!("binding path '{}' not found in ViewModel fields or computed methods", root_ident),
        ));
    }
}

/// 检查事件处理器：命令名须存在于 commands 列表
fn check_event_handler(
    handler: &EventHandler,
    span: Span,
    meta: Option<&StructMetadata>,
    diags: &mut Vec<SemanticDiagnostic>,
) {
    let Some(meta) = meta else { return };
    let cmd_name = match handler {
        EventHandler::Ident(name) => name,
        EventHandler::MethodName(name) => name,
        EventHandler::WithArgs(name, _) => name,
    };
    if cmd_name.is_empty() {
        return;
    }
    if !meta.commands.contains(cmd_name) && !meta.observable_fields.contains(cmd_name) {
        // 可能是闭包或未标注 #[command] 的方法——降级为 hint 而非 error
        diags.push(SemanticDiagnostic::warning(
            span,
            format!("event handler '{}' is not a registered #[command]", cmd_name),
        ));
    }
}

/// 从表达式提取根标识符（第一个 identifier token）
///
/// `count + 1` → `count`
/// `user.name` → `user`
/// `items[0]` → `items`
/// `format!("...")` → `format`
fn extract_root_ident(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    let mut ident = String::new();
    for ch in trimmed.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else {
            break;
        }
    }
    if ident.is_empty() {
        None
    } else {
        Some(ident)
    }
}

fn elem_span_or_default(node: &Node) -> Span {
    match node {
        Node::Element(e) => e.span,
        _ => Span::empty(),
    }
}
