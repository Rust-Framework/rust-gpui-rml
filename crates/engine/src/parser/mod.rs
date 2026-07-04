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
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Parse error at {}:{}: {}",
            self.line, self.column, self.message
        )
    }
}

impl std::error::Error for ParseError {}

/// 解析 `.rml` 源码，返回根节点
///
/// 要求源码有且仅有一个根元素。
pub fn parse(source: &str) -> Result<Node, ParseError> {
    let tokens = tokenizer::tokenize(source)?;
    let mut parser = Parser { tokens, pos: 0 };
    let nodes = parser.parse_children()?;

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
        }),
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
                        directives.push(Directive::Each(parse_each_expr(&expr)?));
                    }
                }
                "key" => match attr.value {
                    AttrValue::Binding(expr) => directives.push(Directive::Key(expr)),
                    AttrValue::Static(v) => attributes.push(Attribute::Static {
                        name,
                        value: v,
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
                    });
                }
                _ => match attr.value {
                    AttrValue::Static(v) => attributes.push(Attribute::Static {
                        name,
                        value: v,
                    }),
                    AttrValue::Binding(expr) => attributes.push(Attribute::Bind {
                        name,
                        expr,
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
fn parse_each_expr(expr: &str) -> Result<EachClause, ParseError> {
    let parts: Vec<&str> = expr.splitn(2, " in ").collect();
    if parts.len() != 2 {
        return Err(ParseError {
            message: format!(
                "invalid each expression: {} (expected 'item in items')",
                expr
            ),
            line: 0,
            column: 0,
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
