//! 绑定路径/命令名解析 + 语义 token 发射
//!
//! 遍历 AST，对每个绑定表达式 / 命令名做存在性校验（MVP 不做类型推导），
//! 同时发射 `SpannedSemanticToken` 供 LSP `textDocument/semanticTokens` 使用。
//!
//! 绑定路径的根标识符须存在于 `StructMetadata.observable_fields` 或 `computed_methods`；
//! 命令名须存在于 `StructMetadata.commands`。

use std::collections::HashMap;

use rust_rml_engine::build::scanner::StructMetadata;
use rust_rml_engine::parser::ast::{Attribute, Directive, Element, EventHandler, Node, TextSegment};
use rust_rml_engine::parser::Span;
use rust_rml_engine::tags;

use crate::semantics::diagnostics::SemanticDiagnostic;
use crate::semantics::tokens::{token_modifier, token_type, SpannedSemanticToken};

/// 绑定分析结果（诊断 + 语义 tokens）
#[derive(Default)]
pub struct BindingResult {
    pub diagnostics: Vec<SemanticDiagnostic>,
    pub tokens: Vec<SpannedSemanticToken>,
}

/// 对 AST 做语义绑定检查 + token 发射
///
/// `metadata_map` 来自 ProjectIndex，包含 .rml 对应 code-behind 的所有 struct 元信息。
/// `source` 为原始源码文本，用于从 directive/attribute span 内提取子区间。
/// MVP 阶段：只检查第一个 struct 的字段/命令（典型 .rml.rs 只有一个 #[window] struct）。
pub fn bind(
    root: &Node,
    source: &str,
    metadata_map: Option<&HashMap<String, StructMetadata>>,
) -> BindingResult {
    let mut result = BindingResult::default();
    let meta = metadata_map.and_then(|m| m.values().next());
    bind_node(root, source, meta, &mut result);
    result
}

fn bind_node(node: &Node, source: &str, meta: Option<&StructMetadata>, result: &mut BindingResult) {
    match node {
        Node::Element(elem) => bind_element(elem, source, meta, result),
        Node::Interpolation { expr, span } => {
            check_binding_expr_emit(expr, *span, source, meta, result);
        }
        Node::MixedText(segs) => {
            for seg in segs {
                if let TextSegment::Interpolation { expr, span } = seg {
                    check_binding_expr_emit(expr, *span, source, meta, result);
                }
            }
        }
        Node::Text(_) => {}
    }
}

fn bind_element(elem: &Element, source: &str, meta: Option<&StructMetadata>, result: &mut BindingResult) {
    // 1. 标签名 token（HTML vs 组件）
    emit_tag_token(elem, source, result);

    // 2. 指令中的绑定表达式 + 指令 keyword token
    for directive in &elem.directives {
        emit_directive_token(directive, source, meta, result);
    }

    // 3. 属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Bind { name, expr, span } => {
                emit_attribute_name_token(name, *span, source, result);
                check_binding_expr_emit(expr, *span, source, meta, result);
            }
            Attribute::Event { name, handler, span } => {
                emit_attribute_name_token(name, *span, source, result);
                emit_event_handler_token(handler, *span, source, meta, result);
            }
            Attribute::Static { name, value, span } => {
                emit_attribute_name_token(name, *span, source, result);
                emit_string_value_token(value, *span, source, result);
            }
        }
    }

    // 4. 递归子节点
    for child in &elem.children {
        bind_node(child, source, meta, result);
    }
}

/// 发射标签名 token：HTML 标签 → TAG，组件（PascalCase） → TYPE
fn emit_tag_token(elem: &Element, source: &str, result: &mut BindingResult) {
    let tag_span = tag_name_span(elem, source);
    if tag_span.is_empty() {
        return;
    }
    let token_type = if tags::is_builtin(&elem.tag) {
        token_type::TAG
    } else {
        token_type::TYPE
    };
    result.tokens.push(SpannedSemanticToken::new(tag_span, token_type, 0));
}

/// 推算标签名字节区间
///
/// `<div ...>` 的标签名区间为 `elem.span.start + 1 .. elem.span.start + 1 + tag.len()`。
/// 闭标签 `</div>` 不在此推算范围。
fn tag_name_span(elem: &Element, _source: &str) -> Span {
    let start = elem.span.start + 1;
    let end = start + elem.tag.len();
    Span::new(start, end)
}

/// 发射指令 keyword token + 指令表达式的绑定 token
fn emit_directive_token(
    directive: &Directive,
    source: &str,
    meta: Option<&StructMetadata>,
    result: &mut BindingResult,
) {
    let directive_span = directive_span(directive);
    if directive_span.is_empty() {
        return;
    }
    let directive_text = span_text(directive_span, source);

    match directive {
        Directive::If { expr, .. } => {
            emit_keyword_token("if", directive_span, source, result);
            check_binding_expr_emit(expr, directive_span, source, meta, result);
        }
        Directive::Show { expr, .. } => {
            emit_keyword_token("show", directive_span, source, result);
            check_binding_expr_emit(expr, directive_span, source, meta, result);
        }
        Directive::Key { expr, .. } => {
            emit_keyword_token("key", directive_span, source, result);
            check_binding_expr_emit(expr, directive_span, source, meta, result);
        }
        Directive::Html { expr, .. } => {
            emit_keyword_token("html", directive_span, source, result);
            check_binding_expr_emit(expr, directive_span, source, meta, result);
        }
        Directive::Model { field, .. } => {
            emit_keyword_token("model", directive_span, source, result);
            // model 字段：VARIABLE + DEFINITION + MODIFICATION（若已解析）
            let (is_valid, ident) = check_ident_validity(field, meta);
            if let Some(ident) = ident {
                let ident_span = find_ident_in(directive_span, source, &ident)
                    .unwrap_or(directive_span);
                let (tt, mods) = if is_valid {
                    (token_type::VARIABLE, token_modifier::DEFINITION | token_modifier::MODIFICATION)
                } else {
                    (token_type::PROPERTY, token_modifier::DEPRECATED | token_modifier::MODIFICATION)
                };
                result.tokens.push(SpannedSemanticToken::new(ident_span, tt, mods));
            }
        }
        Directive::Each { clause, .. } => {
            emit_keyword_token("each", directive_span, source, result);
            // each 迭代变量：VARIABLE + DECLARATION
            let item_span = find_ident_in(directive_span, source, &clause.item)
                .unwrap_or(directive_span);
            result.tokens.push(SpannedSemanticToken::new(
                item_span,
                token_type::VARIABLE,
                token_modifier::DECLARATION,
            ));
            // each 迭代源：VARIABLE/PROPERTY + DEFINITION/DEPRECATED
            let (is_valid, ident) = check_ident_validity(&clause.iterable, meta);
            if let Some(ident) = ident {
                let ident_span = find_ident_in(directive_span, source, &ident)
                    .unwrap_or(directive_span);
                let (tt, mods) = if is_valid {
                    (token_type::VARIABLE, token_modifier::DEFINITION)
                } else {
                    (token_type::PROPERTY, token_modifier::DEPRECATED)
                };
                result.tokens.push(SpannedSemanticToken::new(ident_span, tt, mods));
            }
            // 诊断（保持原有逻辑）
            check_binding_expr(&clause.iterable, elem_span_from_directive(directive), meta, &mut result.diagnostics);
            return; // 已经手动处理诊断，跳过下面的通用诊断
        }
        Directive::Else { .. } => {
            emit_keyword_token("else", directive_span, source, result);
        }
        Directive::Once { .. } => {
            emit_keyword_token("once", directive_span, source, result);
        }
        Directive::Ref { name, .. } => {
            emit_keyword_token("ref", directive_span, source, result);
            // ref 目标：VARIABLE + DECLARATION
            let ref_span = find_string_value_span_in(directive_span, source, name)
                .unwrap_or(directive_span);
            result.tokens.push(SpannedSemanticToken::new(
                ref_span,
                token_type::VARIABLE,
                token_modifier::DECLARATION,
            ));
        }
    }

    // 诊断（保持原有逻辑，Except Each 已提前 return）
    match directive {
        Directive::If { expr, .. } | Directive::Show { expr, .. } | Directive::Key { expr, .. } => {
            check_binding_expr(expr, elem_span_from_directive(directive), meta, &mut result.diagnostics);
        }
        Directive::Html { expr, .. } => {
            check_binding_expr(expr, elem_span_from_directive(directive), meta, &mut result.diagnostics);
        }
        Directive::Model { field, .. } => {
            check_binding_expr(field, elem_span_from_directive(directive), meta, &mut result.diagnostics);
        }
        _ => {}
    }
}

/// 取指令的 span
fn directive_span(d: &Directive) -> Span {
    match d {
        Directive::If { span, .. }
        | Directive::Else { span }
        | Directive::Each { span, .. }
        | Directive::Key { span, .. }
        | Directive::Model { span, .. }
        | Directive::Show { span, .. }
        | Directive::Once { span }
        | Directive::Html { span, .. }
        | Directive::Ref { span, .. } => *span,
    }
}

/// 从指令 span 取元素 span（诊断用，退化为指令 span）
fn elem_span_from_directive(d: &Directive) -> Span {
    directive_span(d)
}

/// 发射 keyword token：在 directive_span 内查找 keyword 名，切分为独立 token
fn emit_keyword_token(keyword: &str, directive_span: Span, source: &str, result: &mut BindingResult) {
    let kw_span = find_keyword_span_in(directive_span, source, keyword).unwrap_or(directive_span);
    result.tokens.push(SpannedSemanticToken::new(
        kw_span,
        token_type::KEYWORD,
        0,
    ));
}

/// 发射属性名 token
fn emit_attribute_name_token(name: &str, attr_span: Span, source: &str, result: &mut BindingResult) {
    let name_span = find_ident_at_start(attr_span, source, name).unwrap_or(attr_span);
    result.tokens.push(SpannedSemanticToken::new(
        name_span,
        token_type::ATTRIBUTE,
        0,
    ));
}

/// 发射静态属性值 string token
fn emit_string_value_token(value: &str, attr_span: Span, source: &str, result: &mut BindingResult) {
    let value_span = find_string_value_span_in(attr_span, source, value).unwrap_or(attr_span);
    result.tokens.push(SpannedSemanticToken::new(
        value_span,
        token_type::STRING,
        0,
    ));
}

/// 发射事件处理器 token：FUNCTION + DEFINITION(已注册) / DEPRECATED(未注册)
fn emit_event_handler_token(
    handler: &EventHandler,
    attr_span: Span,
    source: &str,
    meta: Option<&StructMetadata>,
    result: &mut BindingResult,
) {
    let cmd_name = match handler {
        EventHandler::Ident(name) | EventHandler::MethodName(name) | EventHandler::WithArgs(name, _) => name,
    };
    if cmd_name.is_empty() {
        return;
    }
    let ident_span = find_ident_in(attr_span, source, cmd_name).unwrap_or(attr_span);
    let is_registered = meta
        .map(|m| m.commands.contains(cmd_name) || m.observable_fields.contains(cmd_name))
        .unwrap_or(false);
    let mods = if is_registered {
        token_modifier::DEFINITION
    } else {
        token_modifier::DEPRECATED
    };
    result.tokens.push(SpannedSemanticToken::new(
        ident_span,
        token_type::FUNCTION,
        mods,
    ));

    // 诊断（保持原有逻辑）
    check_event_handler(handler, attr_span, meta, &mut result.diagnostics);
}

/// 检查绑定表达式 + 发射 token
fn check_binding_expr_emit(
    expr: &str,
    span: Span,
    source: &str,
    meta: Option<&StructMetadata>,
    result: &mut BindingResult,
) {
    // 发射 token
    let (is_valid, ident) = check_ident_validity(expr, meta);
    if let Some(ident) = ident {
        let ident_span = find_ident_in(span, source, &ident).unwrap_or(span);
        let (tt, mods) = if is_valid {
            (token_type::VARIABLE, token_modifier::DEFINITION)
        } else {
            (token_type::PROPERTY, token_modifier::DEPRECATED)
        };
        result.tokens.push(SpannedSemanticToken::new(ident_span, tt, mods));
    }

    // 诊断
    check_binding_expr(expr, span, meta, &mut result.diagnostics);
}

/// 检查标识符有效性（返回 (is_valid, ident_name)）
///
/// `count + 1` → `(true/false, "count")`
/// 返回 None 表示无法提取根标识符（空表达式等）
fn check_ident_validity(expr: &str, meta: Option<&StructMetadata>) -> (bool, Option<String>) {
    let Some(root_ident) = extract_root_ident(expr) else {
        return (false, None);
    };
    if root_ident == "cx" || root_ident == "_window" || root_ident == "true" || root_ident == "false" {
        return (true, Some(root_ident)); // 内置标识符，不发射 token（返回 valid 避免诊断）
    }
    let is_valid = meta
        .map(|m| {
            m.observable_fields.contains(&root_ident)
                || m.computed_methods.contains(&root_ident)
                || root_ident.parse::<i64>().is_ok()
                || root_ident.parse::<f64>().is_ok()
        })
        .unwrap_or(false);
    (is_valid, Some(root_ident))
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

    if root_ident == "cx" || root_ident == "_window" || root_ident == "true" || root_ident == "false" {
        return;
    }

    let is_valid = meta.observable_fields.contains(&root_ident)
        || meta.computed_methods.contains(&root_ident)
        || root_ident.parse::<i64>().is_ok()
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
        diags.push(SemanticDiagnostic::warning(
            span,
            format!("event handler '{}' is not a registered #[command]", cmd_name),
        ));
    }
}

/// 从表达式提取根标识符（第一个 identifier token）
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

// === 子 span 提取工具 ===

/// 取 span 内的源码文本
fn span_text(span: Span, source: &str) -> &str {
    let start = span.start.min(source.len());
    let end = span.end.min(source.len());
    if start <= end {
        &source[start..end]
    } else {
        ""
    }
}

/// 在 span 内查找 keyword（指令名），返回精确子 span
fn find_keyword_span_in(span: Span, source: &str, keyword: &str) -> Option<Span> {
    let text = span_text(span, source);
    let pos = text.find(keyword)?;
    let start = span.start + pos;
    let end = start + keyword.len();
    Some(Span::new(start, end))
}

/// 在 span 内查找标识符（跳过 `{`、`=` 等前缀字符），返回精确子 span
fn find_ident_in(span: Span, source: &str, ident: &str) -> Option<Span> {
    let text = span_text(span, source);
    let pos = text.find(ident)?;
    let start = span.start + pos;
    let end = start + ident.len();
    Some(Span::new(start, end))
}

/// 在 span 起始处匹配标识符（属性名场景：`class="..."` 中 `class` 在 span 开头）
fn find_ident_at_start(span: Span, source: &str, ident: &str) -> Option<Span> {
    let text = span_text(span, source);
    if text.starts_with(ident) {
        Some(Span::new(span.start, span.start + ident.len()))
    } else {
        None
    }
}

/// 在 span 内查找字符串值（`"value"` 中的 `value`），返回内容子 span
fn find_string_value_span_in(span: Span, source: &str, value: &str) -> Option<Span> {
    let text = span_text(span, source);
    // 查找带引号的 value
    let quoted = format!("\"{}\"", value);
    let pos = text.find(&quoted)?;
    let start = span.start + pos + 1; // 跳过开引号
    let end = start + value.len();
    Some(Span::new(start, end))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_meta() -> HashMap<String, StructMetadata> {
        HashMap::new()
    }

    fn meta_with_fields(fields: &[&str], commands: &[&str]) -> HashMap<String, StructMetadata> {
        let mut m = HashMap::new();
        m.insert(
            "Test".to_string(),
            StructMetadata {
                observable_fields: fields.iter().map(|s| s.to_string()).collect(),
                version_fields: Vec::new(),
                computed_methods: Vec::new(),
                computed_deps: HashMap::new(),
                computed_returns: HashMap::new(),
                commands: commands.iter().map(|s| s.to_string()).collect(),
                ..Default::default()
            },
        );
        m
    }

    #[test]
    fn bind_emits_keyword_for_if_directive() {
        let source = "<div if={count}>x</div>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let result = bind(&root, source, Some(&empty_meta()));
        // 应包含 keyword token (if)
        let has_keyword = result.tokens.iter().any(|t| t.token_type == token_type::KEYWORD);
        assert!(has_keyword, "expected keyword token for 'if' directive");
    }

    #[test]
    fn bind_emits_variable_for_resolved_binding() {
        let source = "<div if={count}>x</div>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let meta = meta_with_fields(&["count"], &[]);
        let result = bind(&root, source, Some(&meta));
        // count 是已解析字段 → VARIABLE + DEFINITION
        let has_resolved = result.tokens.iter().any(|t| {
            t.token_type == token_type::VARIABLE && (t.token_modifiers & token_modifier::DEFINITION) != 0
        });
        assert!(has_resolved, "expected VARIABLE+DEFINITION for resolved 'count'");
    }

    #[test]
    fn bind_emits_property_for_unresolved_binding() {
        let source = "<div if={unknown}>x</div>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let result = bind(&root, source, Some(&empty_meta()));
        // unknown 是未解析字段 → PROPERTY + DEPRECATED
        let has_unresolved = result.tokens.iter().any(|t| {
            t.token_type == token_type::PROPERTY && (t.token_modifiers & token_modifier::DEPRECATED) != 0
        });
        assert!(has_unresolved, "expected PROPERTY+DEPRECATED for unresolved 'unknown'");
    }

    #[test]
    fn bind_emits_tag_for_html_element() {
        let source = "<div>x</div>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let result = bind(&root, source, Some(&empty_meta()));
        let has_tag = result.tokens.iter().any(|t| t.token_type == token_type::TAG);
        assert!(has_tag, "expected TAG token for <div>");
    }

    #[test]
    fn bind_emits_type_for_component() {
        let source = "<MyComponent>x</MyComponent>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let result = bind(&root, source, Some(&empty_meta()));
        let has_type = result.tokens.iter().any(|t| t.token_type == token_type::TYPE);
        assert!(has_type, "expected TYPE token for <MyComponent>");
    }

    #[test]
    fn bind_emits_function_for_event_handler() {
        let source = "<button onclick={on_click}>x</button>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let meta = meta_with_fields(&[], &["on_click"]);
        let result = bind(&root, source, Some(&meta));
        let has_function = result
            .tokens
            .iter()
            .any(|t| t.token_type == token_type::FUNCTION && (t.token_modifiers & token_modifier::DEFINITION) != 0);
        assert!(has_function, "expected FUNCTION+DEFINITION for 'on_click'");
    }

    #[test]
    fn bind_emits_attribute_for_property_name() {
        let source = r#"<div class="x">x</div>"#;
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let result = bind(&root, source, Some(&empty_meta()));
        let has_attribute = result.tokens.iter().any(|t| t.token_type == token_type::ATTRIBUTE);
        assert!(has_attribute, "expected ATTRIBUTE token for 'class'");
    }

    #[test]
    fn bind_emits_string_for_static_value() {
        let source = r#"<div class="container">x</div>"#;
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let result = bind(&root, source, Some(&empty_meta()));
        let has_string = result.tokens.iter().any(|t| t.token_type == token_type::STRING);
        assert!(has_string, "expected STRING token for 'container'");
    }

    #[test]
    fn bind_emits_each_iteration_variable() {
        let source = "<li each={item in items}>{item}</li>";
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let meta = meta_with_fields(&["items"], &[]);
        let result = bind(&root, source, Some(&meta));
        // item 迭代变量 → VARIABLE + DECLARATION
        let has_declaration = result.tokens.iter().any(|t| {
            t.token_type == token_type::VARIABLE && (t.token_modifiers & token_modifier::DECLARATION) != 0
        });
        assert!(has_declaration, "expected VARIABLE+DECLARATION for 'item'");
    }

    #[test]
    fn bind_emits_model_with_modification() {
        let source = r#"<input model={name} />"#;
        let root = rust_rml_engine::parser::parse(source).unwrap();
        let meta = meta_with_fields(&["name"], &[]);
        let result = bind(&root, source, Some(&meta));
        // model 字段 → VARIABLE + DEFINITION + MODIFICATION
        let has_modification = result.tokens.iter().any(|t| {
            t.token_type == token_type::VARIABLE
                && (t.token_modifiers & token_modifier::MODIFICATION) != 0
                && (t.token_modifiers & token_modifier::DEFINITION) != 0
        });
        assert!(has_modification, "expected VARIABLE+DEFINITION+MODIFICATION for model 'name'");
    }
}
