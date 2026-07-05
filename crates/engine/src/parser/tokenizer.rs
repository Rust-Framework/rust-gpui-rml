//! 词法分析器
//!
//! 将 `.rml` 源码切分为 Token 流。每个 Token 携带字节级 `Span` 与起止行列，
//! 供 LSP 定位与未来增量解析使用。

use crate::parser::span::Span;
use crate::parser::ParseError;

/// Token 种类
#[derive(Debug, Clone)]
pub enum TokenKind {
    /// 文本节点（含插值）
    Text(String),
    /// 标签开始 `<div attrs>`（未闭合）
    TagStart {
        tag: String,
        attributes: Vec<RawAttribute>,
    },
    /// 自闭合标签 `<input attrs />`
    SelfClosingTag {
        tag: String,
        attributes: Vec<RawAttribute>,
    },
    /// 标签结束 `</div>`
    TagEnd { tag: String },
    /// 文件结束
    Eof,
}

/// 原始属性（解析前）
#[derive(Debug, Clone)]
pub struct RawAttribute {
    pub name: String,
    pub value: AttrValue,
    /// 属性名+值的字节区间（属性级诊断定位用）
    pub span: Span,
    /// 属性名所在行（1-based），供 build_element 错误诊断使用
    pub line: usize,
    /// 属性名所在列（1-based）
    pub column: usize,
}

/// 属性值
#[derive(Debug, Clone)]
pub enum AttrValue {
    /// 字符串 `"..."` 或 `'...'`
    Static(String),
    /// 绑定表达式 `{expr}`
    Binding(String),
}

/// Token
#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
    /// Token 结束行列（含跨行属性等场景）
    pub end_line: usize,
    pub end_column: usize,
    /// Token 的字节区间 [start, end)
    pub span: Span,
}

/// 词法分析主入口
pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = CharStream::new(source);
    let mut text_buf = String::new();
    let mut text_start = 0usize;

    while let Some(c) = chars.peek() {
        if c == '<' {
            // 推入累积文本
            if !text_buf.is_empty() {
                let (line, col) = chars.position();
                let end_offset = chars.byte_position();
                let text = std::mem::take(&mut text_buf);
                let start = text_start;
                tokens.push(Token {
                    kind: TokenKind::Text(text),
                    line,
                    column: col,
                    end_line: line,
                    end_column: col,
                    span: Span::new(start, end_offset),
                });
            }
            // 注释 <!-- ... -->
            if chars.starts_with("<!--") {
                chars.advance_n(4);
                skip_comment(&mut chars)?;
                continue;
            }
            // 结束标签 </tag>
            if chars.peek_at(1) == Some('/') {
                chars.advance_n(2); // 跳过 </
                let (line, col) = chars.position();
                let start = chars.byte_position() - 2;
                let tag = read_tag_name(&mut chars)?;
                skip_whitespace(&mut chars);
                if chars.peek() != Some('>') {
                    return Err(ParseError {
                        message: format!("expected '>' after </{}", tag),
                        line,
                        column: col,
                        source_snippet: None,
                    });
                }
                chars.advance();
                let (end_line, end_col) = chars.position();
                let end_offset = chars.byte_position();
                tokens.push(Token {
                    kind: TokenKind::TagEnd { tag },
                    line,
                    column: col,
                    end_line,
                    end_column: end_col,
                    span: Span::new(start, end_offset),
                });
                continue;
            }
            // 开始标签 <tag attrs> 或 <tag attrs />
            chars.advance(); // 跳过 <
            let (line, col) = chars.position();
            let start = chars.byte_position() - 1;
            let tag = read_tag_name(&mut chars)?;
            let attributes = read_attributes(&mut chars)?;
            skip_whitespace(&mut chars);
            // 判断是否自闭合
            if chars.peek() == Some('/') {
                chars.advance();
                if chars.peek() != Some('>') {
                    return Err(ParseError {
                        message: "expected '>' after '/'".into(),
                        line,
                        column: col,
                        source_snippet: None,
                    });
                }
                chars.advance();
                let (end_line, end_col) = chars.position();
                let end_offset = chars.byte_position();
                tokens.push(Token {
                    kind: TokenKind::SelfClosingTag { tag, attributes },
                    line,
                    column: col,
                    end_line,
                    end_column: end_col,
                    span: Span::new(start, end_offset),
                });
            } else if chars.peek() == Some('>') {
                chars.advance();
                let (end_line, end_col) = chars.position();
                let end_offset = chars.byte_position();
                tokens.push(Token {
                    kind: TokenKind::TagStart { tag, attributes },
                    line,
                    column: col,
                    end_line,
                    end_column: end_col,
                    span: Span::new(start, end_offset),
                });
            } else {
                return Err(ParseError {
                    message: format!("expected '>' or '/>' after tag <{}", tag),
                    line,
                    column: col,
                    source_snippet: None,
                });
            }
        } else {
            if text_buf.is_empty() {
                // 记录文本块起始字节偏移（当前字符尚未 advance）
                text_start = chars.byte_position();
            }
            text_buf.push(c);
            chars.advance();
        }
    }

    if !text_buf.is_empty() {
        let (line, col) = chars.position();
        let end_offset = chars.byte_position();
        let text = text_buf;
        tokens.push(Token {
            kind: TokenKind::Text(text),
            line,
            column: col,
            end_line: line,
            end_column: col,
            span: Span::new(text_start, end_offset),
        });
    }

    let (eof_line, eof_col) = chars.position();
    let eof_offset = chars.byte_position();
    tokens.push(Token {
        kind: TokenKind::Eof,
        line: eof_line,
        column: eof_col,
        end_line: eof_line,
        end_column: eof_col,
        span: Span::new(eof_offset, eof_offset),
    });

    Ok(tokens)
}

fn skip_comment(chars: &mut CharStream) -> Result<(), ParseError> {
    while chars.peek().is_some() {
        if chars.starts_with("-->") {
            chars.advance_n(3);
            return Ok(());
        }
        chars.advance();
    }
    let (line, col) = chars.position();
    Err(ParseError {
        message: "unterminated comment".into(),
        line,
        column: col,
        source_snippet: None,
    })
}

fn read_tag_name(chars: &mut CharStream) -> Result<String, ParseError> {
    let mut name = String::new();
    while let Some(c) = chars.peek() {
        // RML 强制 kebab-case 命名：接受字母数字、连字符 `-`、冒号 `:`
        // 严格禁止下划线 `_` —— 遇到下划线时停止读取，触发解析错误
        if c.is_alphanumeric() || c == '-' || c == ':' {
            name.push(c);
            chars.advance();
        } else {
            break;
        }
    }
    if name.is_empty() {
        let (line, col) = chars.position();
        return Err(ParseError {
            message: "expected tag name".into(),
            line,
            column: col,
            source_snippet: None,
        });
    }
    Ok(name)
}

fn read_attributes(chars: &mut CharStream) -> Result<Vec<RawAttribute>, ParseError> {
    let mut attrs = Vec::new();
    loop {
        skip_whitespace(chars);
        match chars.peek() {
            Some('>') | Some('/') | None => break,
            _ => {}
        }
        let name_start = chars.byte_position();
        let (attr_line, attr_col) = chars.position();
        let name = read_attr_name(chars)?;
        skip_whitespace(chars);
        if chars.peek() != Some('=') {
            // 布尔属性（如 disabled）
            let name_end = chars.byte_position();
            attrs.push(RawAttribute {
                name,
                value: AttrValue::Static("true".to_string()),
                span: Span::new(name_start, name_end),
                line: attr_line,
                column: attr_col,
            });
            continue;
        }
        chars.advance(); // 跳过 =
        skip_whitespace(chars);
        let value = read_attr_value(chars)?;
        let name_end = chars.byte_position();
        attrs.push(RawAttribute {
            name,
            value,
            span: Span::new(name_start, name_end),
            line: attr_line,
            column: attr_col,
        });
    }
    Ok(attrs)
}

fn read_attr_name(chars: &mut CharStream) -> Result<String, ParseError> {
    let mut name = String::new();
    while let Some(c) = chars.peek() {
        // RML 强制 kebab-case 命名：接受字母数字、连字符 `-`、冒号 `:`
        // 严格禁止下划线 `_` —— 遇到下划线时停止读取，触发解析错误
        if c.is_alphanumeric() || c == '-' || c == ':' {
            name.push(c);
            chars.advance();
        } else {
            break;
        }
    }
    if name.is_empty() {
        let (line, col) = chars.position();
        return Err(ParseError {
            message: "expected attribute name".into(),
            line,
            column: col,
            source_snippet: None,
        });
    }
    Ok(name)
}

fn read_attr_value(chars: &mut CharStream) -> Result<AttrValue, ParseError> {
    match chars.peek() {
        Some('"') | Some('\'') => {
            let quote = chars.advance().unwrap();
            let mut value = String::new();
            while let Some(c) = chars.peek() {
                if c == quote {
                    chars.advance();
                    return Ok(AttrValue::Static(value));
                }
                value.push(c);
                chars.advance();
            }
            let (line, col) = chars.position();
            Err(ParseError {
                message: "unterminated string attribute".into(),
                line,
                column: col,
                source_snippet: None,
            })
        }
        Some('{') => {
            chars.advance(); // 跳过 {
            let mut expr = String::new();
            let mut depth = 1;
            while let Some(c) = chars.peek() {
                if c == '{' {
                    depth += 1;
                } else if c == '}' {
                    depth -= 1;
                    if depth == 0 {
                        chars.advance();
                        return Ok(AttrValue::Binding(expr.trim().to_string()));
                    }
                }
                expr.push(c);
                chars.advance();
            }
            let (line, col) = chars.position();
            Err(ParseError {
                message: "unterminated binding expression".into(),
                line,
                column: col,
                source_snippet: None,
            })
        }
        _ => {
            // 无引号值（不支持，要求引号或 {}）
            let (line, col) = chars.position();
            Err(ParseError {
                message: "attribute value must be \"...\" or {...}".into(),
                line,
                column: col,
                source_snippet: None,
            })
        }
    }
}

fn skip_whitespace(chars: &mut CharStream) {
    while let Some(c) = chars.peek() {
        if c.is_whitespace() {
            chars.advance();
        } else {
            break;
        }
    }
}

struct CharStream<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    byte_offset: usize,
    _phantom: std::marker::PhantomData<&'a str>,
}

impl<'a> CharStream<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            byte_offset: 0,
            _phantom: std::marker::PhantomData,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        // 字节偏移按 UTF-8 编码长度累加（多字节字符正确推进）
        self.byte_offset += c.len_utf8();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(c)
    }

    fn advance_n(&mut self, n: usize) {
        for _ in 0..n {
            self.advance();
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        let target: Vec<char> = s.chars().collect();
        if self.pos + target.len() > self.chars.len() {
            return false;
        }
        for (i, c) in target.iter().enumerate() {
            if self.chars[self.pos + i] != *c {
                return false;
            }
        }
        true
    }

    fn position(&self) -> (usize, usize) {
        (self.line, self.col)
    }

    fn byte_position(&self) -> usize {
        self.byte_offset
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// read_tag_name 拒绝下划线，强制 kebab-case
    #[test]
    fn read_tag_name_rejects_underscore() {
        // <tab_bar> 应解析失败：遇到 `_` 时停止读取，tag 名为 "tab"
        // 随后遇到 `_` 触发 "expected >" 或属性解析错误
        let result = tokenize("<tab_bar></tab_bar>");
        assert!(result.is_err(), "expected error for snake_case tag name");

        // <tab-bar> 应正常解析
        let result = tokenize("<tab-bar></tab-bar>");
        assert!(result.is_ok(), "kebab-case tag name should parse");
    }

    /// Token 的 span 应覆盖整个标签区间（字节偏移）
    #[test]
    fn span_covers_full_tag() {
        let src = "<div></div>";
        let tokens = tokenize(src).unwrap();
        // 0: TagStart(div), 1: TagEnd(div), 2: Eof
        assert_eq!(tokens.len(), 3);
        let tag_start = &tokens[0];
        assert!(matches!(tag_start.kind, TokenKind::TagStart { .. }));
        assert_eq!(tag_start.span, Span::new(0, 5)); // "<div>"
        let tag_end = &tokens[1];
        assert!(matches!(tag_end.kind, TokenKind::TagEnd { .. }));
        assert_eq!(tag_end.span, Span::new(5, 11)); // "</div>"
    }

    /// 自闭合标签的 span 覆盖 `<input ... />`
    #[test]
    fn span_covers_self_closing_tag() {
        let src = "<input value=\"x\" />";
        let tokens = tokenize(src).unwrap();
        let t = &tokens[0];
        assert!(matches!(t.kind, TokenKind::SelfClosingTag { .. }));
        assert_eq!(t.span, Span::new(0, src.len()));
    }

    /// RawAttribute 的 span 覆盖属性名+值
    #[test]
    fn attr_span_covers_name_and_value() {
        let src = r#"<a href="/x" class="c">"#;
        let tokens = tokenize(src).unwrap();
        let TokenKind::TagStart { attributes, .. } = &tokens[0].kind else {
            panic!("expected TagStart");
        };
        // `<a ` 占 3 字节；href="/x" 占 9 字节 → 3..12
        assert_eq!(attributes[0].name, "href");
        assert_eq!(attributes[0].span, Span::new(3, 12));
        // 空格 1 字节后 class="c" 占 9 字节 → 13..22
        assert_eq!(attributes[1].name, "class");
        assert_eq!(attributes[1].span, Span::new(13, 22));
    }

    /// 多字节字符（中文）下字节偏移按 UTF-8 累加
    #[test]
    fn span_handles_multibyte_chars() {
        // "你好" = 6 字节，后接 <br/>
        let src = "你好<br/>";
        let tokens = tokenize(src).unwrap();
        // 0: Text("你好"), 1: SelfClosingTag(br), 2: Eof
        let text = &tokens[0];
        assert!(matches!(text.kind, TokenKind::Text(_)));
        assert_eq!(text.span, Span::new(0, 6));
        let br = &tokens[1];
        assert!(matches!(br.kind, TokenKind::SelfClosingTag { .. }));
        assert_eq!(br.span, Span::new(6, 11)); // "<br/>"
    }

    /// Element 的 span 覆盖整个元素（起止标签）
    #[test]
    fn element_span_covers_full_element() {
        use crate::parser::parse;
        let src = "<div><span></span></div>";
        let root = parse(src).unwrap();
        let crate::parser::ast::Node::Element(elem) = &root else {
            panic!("expected root element");
        };
        assert_eq!(elem.span, Span::new(0, src.len()));
        // 内层 <span>(5..11) + </span>(11..18) → 5..18
        let crate::parser::ast::Node::Element(span_el) = &elem.children[0] else {
            panic!("expected span child");
        };
        assert_eq!(span_el.span, Span::new(5, 18));
    }
}
