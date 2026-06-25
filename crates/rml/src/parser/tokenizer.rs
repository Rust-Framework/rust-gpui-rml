//! 词法分析器
//!
//! 将 `.rml` 源码切分为 Token 流。

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
}

/// 词法分析主入口
pub fn tokenize(source: &str) -> Result<Vec<Token>, ParseError> {
    let mut tokens = Vec::new();
    let mut chars = CharStream::new(source);
    let mut text_buf = String::new();

    while let Some(c) = chars.peek() {
        if c == '<' {
            // 推入累积文本
            if !text_buf.is_empty() {
                let (line, col) = chars.position();
                tokens.push(Token {
                    kind: TokenKind::Text(std::mem::take(&mut text_buf)),
                    line,
                    column: col,
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
                let tag = read_tag_name(&mut chars)?;
                skip_whitespace(&mut chars);
                if chars.peek() != Some('>') {
                    return Err(ParseError {
                        message: format!("expected '>' after </{}", tag),
                        line,
                        column: col,
                    });
                }
                chars.advance();
                tokens.push(Token {
                    kind: TokenKind::TagEnd { tag },
                    line,
                    column: col,
                });
                continue;
            }
            // 开始标签 <tag attrs> 或 <tag attrs />
            chars.advance(); // 跳过 <
            let (line, col) = chars.position();
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
                    });
                }
                chars.advance();
                tokens.push(Token {
                    kind: TokenKind::SelfClosingTag { tag, attributes },
                    line,
                    column: col,
                });
            } else if chars.peek() == Some('>') {
                chars.advance();
                tokens.push(Token {
                    kind: TokenKind::TagStart { tag, attributes },
                    line,
                    column: col,
                });
            } else {
                return Err(ParseError {
                    message: format!("expected '>' or '/>' after tag <{}", tag),
                    line,
                    column: col,
                });
            }
        } else {
            text_buf.push(c);
            chars.advance();
        }
    }

    if !text_buf.is_empty() {
        let (line, col) = chars.position();
        tokens.push(Token {
            kind: TokenKind::Text(text_buf),
            line,
            column: col,
        });
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        line: 0,
        column: 0,
    });

    Ok(tokens)
}

fn skip_comment(chars: &mut CharStream) -> Result<(), ParseError> {
    while let Some(c) = chars.peek() {
        if chars.starts_with("-->") {
            chars.advance_n(3);
            return Ok(());
        }
        chars.advance();
    }
    Err(ParseError {
        message: "unterminated comment".into(),
        line: 0,
        column: 0,
    })
}

fn read_tag_name(chars: &mut CharStream) -> Result<String, ParseError> {
    let mut name = String::new();
    while let Some(c) = chars.peek() {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
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
        let name = read_attr_name(chars)?;
        skip_whitespace(chars);
        if chars.peek() != Some('=') {
            // 布尔属性（如 disabled）
            attrs.push(RawAttribute {
                name,
                value: AttrValue::Static("true".to_string()),
            });
            continue;
        }
        chars.advance(); // 跳过 =
        skip_whitespace(chars);
        let value = read_attr_value(chars)?;
        attrs.push(RawAttribute { name, value });
    }
    Ok(attrs)
}

fn read_attr_name(chars: &mut CharStream) -> Result<String, ParseError> {
    let mut name = String::new();
    while let Some(c) = chars.peek() {
        if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {
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
            })
        }
        _ => {
            // 无引号值（不支持，要求引号或 {}）
            let (line, col) = chars.position();
            Err(ParseError {
                message: "attribute value must be \"...\" or {...}".into(),
                line,
                column: col,
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
    _phantom: std::marker::PhantomData<&'a str>,
}

impl<'a> CharStream<'a> {
    fn new(source: &'a str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
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
}
