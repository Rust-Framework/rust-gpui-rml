//! RML 语法解析器
//!
//! 将 `.rml` 源码解析为 AST。包含词法分析（tokenizer）与语法分析（parser）。
//! 详见文档 §2 RML 标记语言。

pub mod ast;
pub mod span;
pub mod tokenizer;

use crate::parser::ast::{Attribute, Directive, EachClause, Element, EventHandler, Node, TextSegment};
use crate::parser::tokenizer::{AttrValue, RawAttribute, Token, TokenKind};
use std::fmt;

pub use span::Span;

/// 解析错误
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub line: usize,
    pub column: usize,
    /// 错误所在行的源码片段（可选）
    ///
    /// 由 `parse()` 在返回错误前根据 `line` 从原始源码提取，供 Display 渲染上下文。
    /// 直接构造的错误（如 tokenizer 内部）此字段为 `None`，由 `with_source` 填充。
    pub source_snippet: Option<String>,
}

impl ParseError {
    /// 根据原始源码填充 `source_snippet`
    ///
    /// 从源码按行号（1-based）提取错误所在行内容。若 `line` 为 0（占位）或越界，保持 `None`。
    pub fn with_source(mut self, source: &str) -> Self {
        if self.line > 0 {
            self.source_snippet = source.lines().nth(self.line - 1).map(|s| s.to_string());
        }
        self
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.line, self.column, self.message
        )?;
        if let Some(snippet) = &self.source_snippet {
            // 渲染源码上下文：
            //   |
            //   | <源码行>
            //   |     ^^^
            let caret_pad = " ".repeat(self.column.saturating_sub(1));
            write!(
                f,
                "\n  |\n  | {}\n  | {}^",
                snippet, caret_pad
            )?;
        }
        Ok(())
    }
}

impl std::error::Error for ParseError {}

/// 解析 `.rml` 源码，返回根节点
///
/// 要求源码有且仅有一个根元素。
pub fn parse(source: &str) -> Result<Node, ParseError> {
    let tokens = tokenizer::tokenize(source).map_err(|e| e.with_source(source))?;
    let mut parser = Parser { tokens, pos: 0 };
    let nodes = parser.parse_children().map_err(|e| e.with_source(source))?;

    // 找到第一个非空白文本的节点作为根
    let root = nodes.into_iter().find(|n| match n {
        Node::Text(t) => !t.trim().is_empty(),
        _ => true,
    });

    match root {
        Some(node) => Ok(node),
        None => Err(ParseError {
            message: "no root element found".into(),
            line: 1,
            column: 1,
            source_snippet: None,
        })
        .map_err(|e| e.with_source(source)),
    }
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn advance(&mut self) -> Option<&Token> {
        let t = self.tokens.get(self.pos);
        self.pos += 1;
        t
    }

    /// 解析子节点列表，直到遇到 TagEnd 或 Eof
    fn parse_children(&mut self) -> Result<Vec<Node>, ParseError> {
        let mut nodes = Vec::new();
        while let Some(tok) = self.peek() {
            match &tok.kind {
                TokenKind::Text(text) => {
                    let text_owned = text.clone();
                    self.advance();
                    if !text_owned.trim().is_empty() {
                        nodes.push(self.parse_text_node(&text_owned));
                    }
                }
                TokenKind::TagStart { tag, attributes } => {
                    let tag_owned = tag.clone();
                    let attrs_owned = attributes.clone();
                    let line = tok.line;
                    let col = tok.column;
                    let start_byte = tok.span.start;
                    self.advance();
                    // 递归解析子节点，期望遇到匹配的 TagEnd
                    let children = self.parse_children()?;
                    // 期望下一个是 TagEnd
                    let end_byte = match self.peek().map(|t| &t.kind) {
                        Some(TokenKind::TagEnd { tag: end_tag }) if *end_tag == tag_owned => {
                            let end_span = self.peek().unwrap().span;
                            self.advance();
                            end_span.end
                        }
                        _ => {
                            return Err(ParseError {
                                message: format!("expected </{}>", tag_owned),
                                line,
                                column: col,
                                source_snippet: None,
                            });
                        }
                    };
                    let element = self.build_element(
                        tag_owned,
                        attrs_owned,
                        children,
                        Span::new(start_byte, end_byte),
                    )?;
                    nodes.push(Node::Element(element));
                }
                TokenKind::SelfClosingTag { tag, attributes } => {
                    let tag_owned = tag.clone();
                    let attrs_owned = attributes.clone();
                    let span = tok.span;
                    self.advance();
                    let element = self.build_element(tag_owned, attrs_owned, Vec::new(), span)?;
                    nodes.push(Node::Element(element));
                }
                TokenKind::TagEnd { .. } => {
                    // 交给上层处理
                    break;
                }
                TokenKind::Eof => break,
            }
        }
        Ok(nodes)
    }

    fn parse_text_node(&self, raw: &str) -> Node {
        let segments = parse_text_segments(raw);
        match segments.len() {
            0 => Node::Text(raw.to_string()),
            1 => match &segments[0] {
                TextSegment::Literal(s) => Node::Text(s.clone()),
                TextSegment::Interpolation(e) => Node::Interpolation(e.clone()),
            },
            _ => Node::MixedText(segments),
        }
    }

    fn build_element(
        &self,
        tag: String,
        raw_attrs: Vec<RawAttribute>,
        children: Vec<Node>,
        span: Span,
    ) -> Result<Element, ParseError> {
        let mut attributes = Vec::new();
        let mut directives = Vec::new();
        let mut slot_name: Option<String> = None;

        for attr in raw_attrs {
            // RML 强制 kebab-case：解析器在入口处将 `-` 规范化为 `_`
            // 用户写 `label-width` → 内部存储 `label_width`，命中 snake_case setter
            // 单词属性（如 `onclick`/`bordered`）无 `-`，不受影响
            let name = normalize_attr_name(&attr.name);
            match name.as_str() {
                "if" => {
                    if let AttrValue::Binding(expr) = attr.value {
                        directives.push(Directive::If(expr));
                    }
                }
                "else" => directives.push(Directive::Else),
                "each" => {
                    if let AttrValue::Binding(expr) = attr.value {
                        directives.push(Directive::Each(parse_each_expr(
                            &expr, attr.line, attr.column,
                        )?));
                    }
                }
                "key" => match attr.value {
                    AttrValue::Binding(expr) => directives.push(Directive::Key(expr)),
                    AttrValue::Static(v) => attributes.push(Attribute::Static {
                        name,
                        value: v,
                        span: attr.span,
                    }),
                },
                "model" => {
                    if let AttrValue::Binding(expr) = attr.value {
                        let (field, converter) = if let Some((f, c)) = expr.split_once('|') {
                            (f.trim().to_string(), Some(c.trim().to_string()))
                        } else {
                            (expr, None)
                        };
                        directives.push(Directive::Model { field, converter });
                    }
                }
                "show" => {
                    if let AttrValue::Binding(expr) = attr.value {
                        directives.push(Directive::Show(expr));
                    }
                }
                "once" => directives.push(Directive::Once),
                "html" => {
                    if let AttrValue::Binding(expr) = attr.value {
                        directives.push(Directive::Html(expr));
                    }
                }
                "ref" => {
                    if let AttrValue::Static(s) = attr.value {
                        directives.push(Directive::Ref(s));
                    }
                }
                // `slot="name"` 属性：标记此元素为具名插槽内容载体（Vue 风格 `<template slot="x">`）
                // 不再 push 到 directives，而是设置 Element.slot_name 字段，
                // 供 codegen 路由到目标组件的对应 slot setter。
                "slot" => {
                    if let AttrValue::Static(s) = attr.value {
                        slot_name = Some(s);
                    }
                }
                name if name.starts_with("on") => {
                    let handler = match attr.value {
                        AttrValue::Binding(expr) => parse_event_handler(&expr),
                        AttrValue::Static(s) => EventHandler::MethodName(s),
                    };
                    attributes.push(Attribute::Event {
                        name: name.to_string(),
                        handler,
                        span: attr.span,
                    });
                }
                _ => match attr.value {
                    AttrValue::Static(v) => attributes.push(Attribute::Static {
                        name,
                        value: v,
                        span: attr.span,
                    }),
                    AttrValue::Binding(expr) => attributes.push(Attribute::Bind {
                        name,
                        expr,
                        span: attr.span,
                    }),
                },
            }
        }

        Ok(Element {
            tag,
            attributes,
            directives,
            children,
            slot_name,
            span,
        })
    }
}

/// 解析 `each` 表达式：`item in items` 或 `index, item in items`
///
/// `line` / `column` 为 `each` 属性所在位置，用于错误诊断。
fn parse_each_expr(expr: &str, line: usize, column: usize) -> Result<EachClause, ParseError> {
    let parts: Vec<&str> = expr.splitn(2, " in ").collect();
    if parts.len() != 2 {
        return Err(ParseError {
            message: format!(
                "invalid each expression: {} (expected 'item in items')",
                expr
            ),
            line,
            column,
            source_snippet: None,
        });
    }
    let left = parts[0].trim();
    let iterable = parts[1].trim().to_string();

    let (index, item) = if let Some(comma_pos) = left.find(',') {
        (
            Some(left[..comma_pos].trim().to_string()),
            left[comma_pos + 1..].trim().to_string(),
        )
    } else {
        (None, left.to_string())
    };

    Ok(EachClause {
        item,
        index,
        iterable,
    })
}

/// 解析事件处理器：`fn` 或 `fn, {expr}, 'literal'`
fn parse_event_handler(expr: &str) -> EventHandler {
    let parts: Vec<&str> = expr.split(',').map(|s| s.trim()).collect();
    if parts.len() == 1 {
        return EventHandler::Ident(parts[0].to_string());
    }
    let method = parts[0].to_string();
    let args: Vec<String> = parts[1..]
        .iter()
        .map(|s| {
            let s = s.trim();
            if s.starts_with('{') && s.ends_with('}') {
                s[1..s.len() - 1].trim().to_string()
            } else {
                s.to_string()
            }
        })
        .collect();
    EventHandler::WithArgs(method, args)
}

/// 解析文本段：将 "text {expr} more" 拆分为 [Literal, Interpolation, Literal]
fn parse_text_segments(raw: &str) -> Vec<TextSegment> {
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut chars = raw.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '{' {
            if !current.is_empty() {
                segments.push(TextSegment::Literal(std::mem::take(&mut current)));
            }
            let mut expr = String::new();
            let mut depth = 1;
            while let Some(&next) = chars.peek() {
                if next == '{' {
                    depth += 1;
                } else if next == '}' {
                    depth -= 1;
                    if depth == 0 {
                        chars.next();
                        break;
                    }
                }
                expr.push(next);
                chars.next();
            }
            if !expr.trim().is_empty() {
                segments.push(TextSegment::Interpolation(expr.trim().to_string()));
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        segments.push(TextSegment::Literal(current));
    }
    segments
}

/// 将 RML 属性名从 kebab-case 规范化为内部 snake_case
///
/// RML 强制 kebab-case 命名规范：用户写 `label-width`、`on-activate`、`v-flex`，
/// 解析器在入口处将 `-` 转换为 `_`，使命中现有 snake_case 的 setter 与注册表。
/// 单词属性（如 `onclick`、`bordered`、`columns`）无 `-`，原样返回。
fn normalize_attr_name(name: &str) -> String {
    name.replace('-', "_")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{Attribute, Directive, EachClause, EventHandler, Node, TextSegment};

    // ─── parse() 主流程：基础结构 ───

    #[test]
    fn parse_simple_root_element() {
        let root = parse("<window></window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.tag, "window");
                assert!(e.children.is_empty());
                assert!(e.attributes.is_empty());
                assert!(e.directives.is_empty());
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_self_closing_root() {
        let root = parse("<input />").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.tag, "input");
                assert!(e.children.is_empty());
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_nested_elements() {
        let root = parse("<window><div><span></span></div></window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.tag, "window");
                assert_eq!(e.children.len(), 1);
                match &e.children[0] {
                    Node::Element(div) => {
                        assert_eq!(div.tag, "div");
                        assert_eq!(div.children.len(), 1);
                        match &div.children[0] {
                            Node::Element(span) => assert_eq!(span.tag, "span"),
                            other => panic!("expected span, got {:?}", other),
                        }
                    }
                    other => panic!("expected div, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_skips_whitespace_between_tags() {
        // 标签间的空白应被忽略，不产生 Text 节点
        let root = parse("<window>\n  <div></div>\n</window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.children.len(), 1);
                match &e.children[0] {
                    Node::Element(div) => assert_eq!(div.tag, "div"),
                    other => panic!("expected div, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_no_root_element_returns_error() {
        let result = parse("");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().message, "no root element found");
    }

    #[test]
    fn parse_only_whitespace_returns_error() {
        let result = parse("   \n\t  ");
        assert!(result.is_err());
    }

    #[test]
    fn parse_unclosed_tag_returns_error() {
        let result = parse("<window><div></window>");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.message.contains("expected </div>"),
            "expected error about missing </div>, got: {}",
            err.message
        );
    }

    #[test]
    fn parse_mismatched_close_tag_returns_error() {
        let result = parse("<window><div></span></window>");
        assert!(result.is_err());
    }

    // ─── source_snippet：错误诊断源码上下文 ───

    #[test]
    fn parse_error_fills_source_snippet_from_source() {
        // 第二行的 div 未闭合，错误应定位到第 2 行第 1 列
        let src = "<window>\n<div></window>";
        let result = parse(src);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.line, 2, "line should be 2, got: {}", err.line);
        // source_snippet 应由 with_source 从源码第 2 行提取
        assert!(err.source_snippet.is_some(), "source_snippet should be filled");
        assert_eq!(err.source_snippet.as_deref(), Some("<div></window>"));
    }

    #[test]
    fn parse_error_display_renders_source_snippet_with_caret() {
        let src = "<window>\n  <div></window>";
        let err = parse(src).unwrap_err();
        let display = format!("{}", err);
        // 应包含 "Parse error at 2:..." 行号
        assert!(display.contains("Parse error at 2:"), "display: {}", display);
        // 应包含源码上下文块
        assert!(display.contains("|"), "missing context marker: {}", display);
        // 应包含源码行内容
        assert!(display.contains("  <div></window>"), "missing source line: {}", display);
        // 应包含 ^ 指示符
        assert!(display.contains("^"), "missing caret: {}", display);
    }

    #[test]
    fn parse_error_each_uses_attr_position_not_placeholder() {
        // each 表达式缺少 "in"，错误应使用属性所在行/列，而非 line:0/column:0
        // each 必须用 {绑定} 语法，非 "静态字符串"
        let src = "<window>\n  <div each={item items}></div>\n</window>";
        let result = parse(src);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid each expression"), "msg: {}", err.message);
        // 属性在第 2 行，错误行应透传自 attr.line（而非 line:0 占位）
        assert_eq!(err.line, 2, "line should be 2, got: {}", err.line);
        assert_ne!(err.line, 0, "line must not be placeholder 0");
        assert_ne!(err.column, 0, "column must not be placeholder 0");
    }

    // ─── parse() 主流程：属性解析 ───

    #[test]
    fn parse_static_attribute() {
        let root = parse(r#"<button label="Click"></button>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.attributes.len(), 1);
                match &e.attributes[0] {
                    Attribute::Static { name, value, .. } => {
                        assert_eq!(name, "label");
                        assert_eq!(value, "Click");
                    }
                    other => panic!("expected Static, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_bind_attribute() {
        let root = parse(r#"<button label={title}></button>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.attributes.len(), 1);
                match &e.attributes[0] {
                    Attribute::Bind { name, expr, .. } => {
                        assert_eq!(name, "label");
                        assert_eq!(expr, "title");
                    }
                    other => panic!("expected Bind, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_event_attribute_with_ident_handler() {
        let root = parse(r#"<button onclick={handle_click}></button>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.attributes.len(), 1);
                match &e.attributes[0] {
                    Attribute::Event { name, handler, .. } => {
                        assert_eq!(name, "onclick");
                        assert!(matches!(handler, EventHandler::Ident(s) if s == "handle_click"));
                    }
                    other => panic!("expected Event, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_event_attribute_with_method_name() {
        // onclick="method_name" → MethodName
        let root = parse(r#"<button onclick="method_name"></button>"#).unwrap();
        match root {
            Node::Element(e) => match &e.attributes[0] {
                Attribute::Event { handler, .. } => {
                    assert!(matches!(handler, EventHandler::MethodName(s) if s == "method_name"));
                }
                other => panic!("expected Event, got {:?}", other),
            },
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ─── parse() 主流程：指令解析 ───

    #[test]
    fn parse_if_directive() {
        let root = parse(r#"<div if={visible}></div>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::If(expr) if expr == "visible"));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_else_directive() {
        let root = parse("<div else></div>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::Else));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_each_directive_simple() {
        let root = parse(r#"<li each={item in items}></li>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                match &e.directives[0] {
                    Directive::Each(EachClause { item, index, iterable }) => {
                        assert_eq!(item, "item");
                        assert_eq!(index, &None);
                        assert_eq!(iterable, "items");
                    }
                    other => panic!("expected Each, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_each_directive_with_index() {
        let root = parse(r#"<li each={index, item in items}></li>"#).unwrap();
        match root {
            Node::Element(e) => match &e.directives[0] {
                Directive::Each(EachClause { item, index, iterable }) => {
                    assert_eq!(item, "item");
                    assert_eq!(index.as_deref(), Some("index"));
                    assert_eq!(iterable, "items");
                }
                other => panic!("expected Each, got {:?}", other),
            },
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_model_directive_simple_field() {
        let root = parse(r#"<input model={name}></input>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                match &e.directives[0] {
                    Directive::Model { field, converter } => {
                        assert_eq!(field, "name");
                        assert_eq!(converter, &None);
                    }
                    other => panic!("expected Model, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_model_directive_with_converter() {
        // model={field | Converter} → Model { field, Some("Converter") }
        let root = parse(r#"<input model={price | Currency}></input>"#).unwrap();
        match root {
            Node::Element(e) => match &e.directives[0] {
                Directive::Model { field, converter } => {
                    assert_eq!(field, "price");
                    assert_eq!(converter.as_deref(), Some("Currency"));
                }
                other => panic!("expected Model, got {:?}", other),
            },
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_show_directive() {
        let root = parse(r#"<div show={is_visible}></div>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::Show(s) if s == "is_visible"));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_once_directive() {
        let root = parse("<div once></div>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::Once));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_html_directive() {
        let root = parse(r#"<div html={raw_html}></div>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::Html(s) if s == "raw_html"));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_ref_directive() {
        let root = parse(r#"<input ref="username"></input>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::Ref(s) if s == "username"));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_slot_attribute_sets_slot_name() {
        // slot="header" 不进入 directives 或 attributes，而是设置 element.slot_name
        let root = parse(r#"<template slot="header"></template>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert!(e.directives.is_empty());
                assert!(e.attributes.is_empty());
                assert_eq!(e.slot_name.as_deref(), Some("header"));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_key_directive_with_binding() {
        let root = parse(r#"<li key={item.id}></li>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 1);
                assert!(matches!(&e.directives[0], Directive::Key(s) if s == "item.id"));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_key_attribute_with_static_value() {
        // key="static" 不进入 directives，而是作为 Static 属性
        let root = parse(r#"<li key="static-key"></li>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert!(e.directives.is_empty());
                assert_eq!(e.attributes.len(), 1);
                match &e.attributes[0] {
                    Attribute::Static { name, value, .. } => {
                        assert_eq!(name, "key");
                        assert_eq!(value, "static-key");
                    }
                    other => panic!("expected Static key, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_kebab_case_attribute_normalized() {
        // label-width → label_width（kebab → snake）
        let root = parse(r#"<input label-width="100"></input>"#).unwrap();
        match root {
            Node::Element(e) => match &e.attributes[0] {
                Attribute::Static { name, .. } => assert_eq!(name, "label_width"),
                other => panic!("expected Static, got {:?}", other),
            },
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_multiple_directives_combined() {
        // 一个元素可以同时有多个指令；key={expr}（Binding）也进入 directives
        let root = parse(r#"<li each={item in items} if={item.active} key={item.id}></li>"#).unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.directives.len(), 3); // each + if + key
                // 指令顺序保持源码顺序
                assert!(matches!(&e.directives[0], Directive::Each(_)));
                assert!(matches!(&e.directives[1], Directive::If(_)));
                assert!(matches!(&e.directives[2], Directive::Key(_)));
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ─── parse() 主流程：文本插值 ───

    #[test]
    fn parse_pure_text_child() {
        let root = parse("<window>Hello</window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.children.len(), 1);
                match &e.children[0] {
                    Node::Text(t) => assert_eq!(t, "Hello"),
                    other => panic!("expected Text, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_pure_interpolation_child() {
        // 单个 {expr} 不混字面量 → Interpolation 节点
        let root = parse("<window>{count}</window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.children.len(), 1);
                match &e.children[0] {
                    Node::Interpolation(expr) => assert_eq!(expr, "count"),
                    other => panic!("expected Interpolation, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_mixed_text_child() {
        // "Total: {count} items" → MixedText([Literal, Interpolation, Literal])
        let root = parse("<window>Total: {count} items</window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.children.len(), 1);
                match &e.children[0] {
                    Node::MixedText(segs) => {
                        assert_eq!(segs.len(), 3);
                        assert!(matches!(&segs[0], TextSegment::Literal(s) if s == "Total: "));
                        assert!(matches!(&segs[1], TextSegment::Interpolation(e) if e == "count"));
                        assert!(matches!(&segs[2], TextSegment::Literal(s) if s == " items"));
                    }
                    other => panic!("expected MixedText, got {:?}", other),
                }
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    #[test]
    fn parse_nested_braces_in_interpolation() {
        // {obj.{field}} 中的嵌套 { } 应被正确处理（depth 跟踪）
        // 简单验证：{a + {b}} 应能解析（虽然语义上无意义，但语法上 depth 平衡）
        let root = parse("<window>{a + {b}}</window>").unwrap();
        match root {
            Node::Element(e) => {
                assert_eq!(e.children.len(), 1);
                // 不论是 Interpolation 还是 MixedText，至少要解析成功
                assert!(!e.children.is_empty());
            }
            other => panic!("expected Element, got {:?}", other),
        }
    }

    // ─── 辅助函数：parse_each_expr ───

    #[test]
    fn parse_each_expr_simple_item() {
        let clause = parse_each_expr("item in items", 1, 1).unwrap();
        assert_eq!(clause.item, "item");
        assert_eq!(clause.index, None);
        assert_eq!(clause.iterable, "items");
    }

    #[test]
    fn parse_each_expr_with_index() {
        let clause = parse_each_expr("idx, item in items", 1, 1).unwrap();
        assert_eq!(clause.item, "item");
        assert_eq!(clause.index.as_deref(), Some("idx"));
        assert_eq!(clause.iterable, "items");
    }

    #[test]
    fn parse_each_expr_trims_whitespace() {
        let clause = parse_each_expr("  item  in  items  ", 1, 1).unwrap();
        assert_eq!(clause.item, "item");
        assert_eq!(clause.iterable, "items");
    }

    #[test]
    fn parse_each_expr_missing_in_returns_error() {
        let result = parse_each_expr("item items", 5, 10);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.message.contains("invalid each expression"));
        // 位置应从参数透传（而非 line:0/column:0 占位）
        assert_eq!(err.line, 5);
        assert_eq!(err.column, 10);
    }

    #[test]
    fn parse_each_expr_complex_iterable() {
        // 复杂表达式作为 iterable：`item in self.list.items`
        let clause = parse_each_expr("item in self.list.items", 1, 1).unwrap();
        assert_eq!(clause.item, "item");
        assert_eq!(clause.iterable, "self.list.items");
    }

    // ─── 辅助函数：parse_event_handler ───

    #[test]
    fn parse_event_handler_single_ident() {
        let h = parse_event_handler("handle_click");
        assert!(matches!(h, EventHandler::Ident(s) if s == "handle_click"));
    }

    #[test]
    fn parse_event_handler_with_args() {
        let h = parse_event_handler("on_click, {item.id}, 'literal'");
        match h {
            EventHandler::WithArgs(method, args) => {
                assert_eq!(method, "on_click");
                assert_eq!(args.len(), 2);
                assert_eq!(args[0], "item.id"); // {expr} → expr
                assert_eq!(args[1], "'literal'"); // 'literal' 保留原样
            }
            other => panic!("expected WithArgs, got {:?}", other),
        }
    }

    #[test]
    fn parse_event_handler_trims_args() {
        let h = parse_event_handler("fn,  {expr}  ,  'str'");
        match h {
            EventHandler::WithArgs(_, args) => {
                assert_eq!(args[0], "expr");
                assert_eq!(args[1], "'str'");
            }
            other => panic!("expected WithArgs, got {:?}", other),
        }
    }

    #[test]
    fn parse_event_handler_brace_strip() {
        // {expr} 中 expr 被提取（去除 { } 与首尾空格）
        let h = parse_event_handler("fn, {  complex.expr  }");
        match h {
            EventHandler::WithArgs(_, args) => assert_eq!(args[0], "complex.expr"),
            other => panic!("expected WithArgs, got {:?}", other),
        }
    }

    // ─── 辅助函数：parse_text_segments ───

    #[test]
    fn parse_text_segments_pure_literal() {
        let segs = parse_text_segments("hello world");
        assert_eq!(segs.len(), 1);
        assert!(matches!(&segs[0], TextSegment::Literal(s) if s == "hello world"));
    }

    #[test]
    fn parse_text_segments_pure_interpolation() {
        let segs = parse_text_segments("{count}");
        assert_eq!(segs.len(), 1);
        assert!(matches!(&segs[0], TextSegment::Interpolation(e) if e == "count"));
    }

    #[test]
    fn parse_text_segments_mixed() {
        let segs = parse_text_segments("Total: {count} items");
        assert_eq!(segs.len(), 3);
        assert!(matches!(&segs[0], TextSegment::Literal(s) if s == "Total: "));
        assert!(matches!(&segs[1], TextSegment::Interpolation(e) if e == "count"));
        assert!(matches!(&segs[2], TextSegment::Literal(s) if s == " items"));
    }

    #[test]
    fn parse_text_segments_multiple_interpolations() {
        let segs = parse_text_segments("{a} + {b} = {c}");
        assert_eq!(segs.len(), 5);
        assert!(matches!(&segs[0], TextSegment::Interpolation(e) if e == "a"));
        assert!(matches!(&segs[1], TextSegment::Literal(s) if s == " + "));
        assert!(matches!(&segs[2], TextSegment::Interpolation(e) if e == "b"));
        assert!(matches!(&segs[3], TextSegment::Literal(s) if s == " = "));
        assert!(matches!(&segs[4], TextSegment::Interpolation(e) if e == "c"));
    }

    #[test]
    fn parse_text_segments_trims_interpolation_whitespace() {
        // {  expr  } → "expr"（trim）
        let segs = parse_text_segments("{  trimmed  }");
        assert_eq!(segs.len(), 1);
        assert!(matches!(&segs[0], TextSegment::Interpolation(e) if e == "trimmed"));
    }

    #[test]
    fn parse_text_segments_empty_interpolation_dropped() {
        // {} → 空插值被丢弃
        let segs = parse_text_segments("before {} after");
        // 应为 [Literal("before "), Literal(" after")]（{} 被丢弃，但被分割）
        assert!(segs.iter().all(|s| matches!(s, TextSegment::Literal(_))));
    }

    #[test]
    fn parse_text_segments_nested_braces() {
        // {a + {b}} → depth 平衡后整体作为 Interpolation
        let segs = parse_text_segments("{a + {b}}");
        assert_eq!(segs.len(), 1);
        match &segs[0] {
            TextSegment::Interpolation(e) => {
                // 嵌套 {} 内部内容应被收集（depth 平衡）
                assert!(e.contains("a + ") || e.contains("a + {b}"));
            }
            other => panic!("expected Interpolation, got {:?}", other),
        }
    }

    #[test]
    fn parse_text_segments_empty_string() {
        let segs = parse_text_segments("");
        assert!(segs.is_empty());
    }

    // ─── 辅助函数：normalize_attr_name ───

    #[test]
    fn normalize_attr_name_single_word() {
        assert_eq!(normalize_attr_name("onclick"), "onclick");
        assert_eq!(normalize_attr_name("bordered"), "bordered");
    }

    #[test]
    fn normalize_attr_name_kebab_case() {
        assert_eq!(normalize_attr_name("label-width"), "label_width");
        assert_eq!(normalize_attr_name("on-activate"), "on_activate");
    }

    #[test]
    fn normalize_attr_name_multi_dashes() {
        assert_eq!(normalize_attr_name("data-test-id"), "data_test_id");
        assert_eq!(normalize_attr_name("v-flex-grow"), "v_flex_grow");
    }

    #[test]
    fn normalize_attr_name_no_dashes_unchanged() {
        assert_eq!(normalize_attr_name("bordered"), "bordered");
        assert_eq!(normalize_attr_name("columns"), "columns");
    }

    #[test]
    fn normalize_attr_name_already_snake() {
        // snake_case 输入应原样返回（无 `-`）
        assert_eq!(normalize_attr_name("label_width"), "label_width");
    }
}
