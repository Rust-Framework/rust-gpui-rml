//! 表达式解析器（Phase B-2）
//!
//! 将 `.rml` 中的插值表达式 `{expr}` 解析为 AST，供 codegen 生成 Rust 表达式。
//! 详见开发规划 §3.2.2。
//!
//! ## 支持的语法子集
//!
//! | 语法 | 示例 | AST |
//! |------|------|-----|
//! | 字段访问 | `count` | `Field("count")` |
//! | 嵌套字段 | `user.name` | `Member(Field("user"), "name")` |
//! | 索引访问 | `items[0]` | `Index(Field("items"), "0")` |
//! | 方法调用 | `items.len()` | `MethodCall(Field("items"), "len", [])` |
//! | 算术 | `count + 1` | `BinaryOp(Add, Field("count"), Lit("1"))` |
//! | 比较 | `count > 0` | `BinaryOp(Gt, Field("count"), Lit("0"))` |
//! | 逻辑 | `a && b` | `BinaryOp(And, Field("a"), Field("b"))` |
//! | 一元否定 | `!flag` | `Unary(Not, Field("flag"))` |
//! | 字面量 | `42` / `"hi"` / `true` | `Lit(...)` |
//! | 转换器 | `count \| HexConverter` | `Convert(Field("count"), "HexConverter")` |
//! | 括号 | `(a + b) * c` | 嵌套 `BinaryOp(Mul, BinaryOp(Add, ...), Field("c"))` |
//!
//! ## 不支持
//!
//! - 三元运算符 `?:`（Phase B-3 视需求添加）
//! - 闭包、函数指针
//! - 结构体字面量
//! - 完整 Rust 表达式（有意限制复杂度）

use std::fmt;

/// 表达式 AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// 标识符（字段名）：`count`
    Field(String),
    /// 成员访问：`expr.ident`（如 `user.name`）
    Member(Box<Expr>, String),
    /// 索引：`expr[expr]`（如 `items[0]`、`items[i + 1]`）
    /// 内层用 Expr 而非 String，支持表达式索引
    Index(Box<Expr>, Box<Expr>),
    /// 方法调用：`expr.ident(args)`（如 `items.len()`、`name.to_uppercase()`）
    MethodCall(Box<Expr>, String, Vec<Expr>),
    /// 二元运算：`lhs op rhs`
    BinaryOp(Op, Box<Expr>, Box<Expr>),
    /// 一元运算：`op expr`（如 `!flag`、`-count`）
    Unary(UnaryOp, Box<Expr>),
    /// 字面量：`42` / `"hello"` / `true` / `false` / `3.14`
    /// 保留原始字符串，由 to_rust_code 原样输出
    Lit(String),
    /// 转换器：`expr | ConverterName`（管道语法，可串联 `expr | A | B`）
    /// codegen 时生成 `ConverterName.convert(&expr)`（unit struct 实例方法调用）
    Convert(Box<Expr>, String),
}

/// 二元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
    And,
    Or,
}

impl Op {
    /// 转为 Rust 源码表示
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Add => "+",
            Op::Sub => "-",
            Op::Mul => "*",
            Op::Div => "/",
            Op::Mod => "%",
            Op::Eq => "==",
            Op::Ne => "!=",
            Op::Gt => ">",
            Op::Lt => "<",
            Op::Ge => ">=",
            Op::Le => "<=",
            Op::And => "&&",
            Op::Or => "||",
        }
    }
}

/// 一元运算符
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    /// 逻辑非 `!`
    Not,
    /// 算术负 `-`
    Neg,
}

impl UnaryOp {
    pub fn as_str(self) -> &'static str {
        match self {
            UnaryOp::Not => "!",
            UnaryOp::Neg => "-",
        }
    }
}

/// 解析错误
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "expression parse error at position {}: {}", self.pos, self.message)
    }
}

impl std::error::Error for ParseError {}

/// 解析表达式字符串为 AST
pub fn parse(input: &str) -> Result<Expr, ParseError> {
    let mut parser = Parser::new(input);
    let expr = parser.parse_convert()?;
    parser.skip_ws();
    if !parser.is_eof() {
        return Err(parser.err(&format!(
            "unexpected trailing characters: {:?}",
            parser.remaining()
        )));
    }
    Ok(expr)
}

/// 将 AST 转为 Rust 源码表达式（默认上下文，所有字段加 `self.` 前缀）
///
/// 字段访问会自动加 `self.` 前缀：`Field("count")` → `self.count`
/// 嵌套字段会保持前缀：`Member(Field("user"), "name")` → `self.user.name`
pub fn to_rust_code(expr: &Expr) -> String {
    to_rust_code_with_ctx(expr, &[])
}

/// 将 AST 转为 Rust 源码表达式（带循环变量上下文）
///
/// `loop_vars` 中的字段名不加 `self.` 前缀（它们是 each 闭包的迭代变量）。
/// 例如 `each={todo in todos}` 内的 `{todo.text}` 应生成 `todo.text` 而非 `self.todo.text`。
pub fn to_rust_code_with_ctx(expr: &Expr, loop_vars: &[&str]) -> String {
    match expr {
        Expr::Field(name) => {
            // `self` 是 Rust 关键字，直接输出（不加 self. 前缀）
            if name == "self" {
                "self".to_string()
            } else if loop_vars.iter().any(|v| *v == name) {
                name.clone()
            } else {
                format!("self.{}", name)
            }
        }
        Expr::Member(target, name) => {
            format!("{}.{}", to_rust_code_with_ctx(target, loop_vars), name)
        }
        Expr::Index(target, index) => {
            format!(
                "{}[{}]",
                to_rust_code_with_ctx(target, loop_vars),
                to_rust_code_with_ctx(index, loop_vars)
            )
        }
        Expr::MethodCall(target, name, args) => {
            let args_str: Vec<String> = args
                .iter()
                .map(|e| to_rust_code_with_ctx(e, loop_vars))
                .collect();
            format!(
                "{}.{}({})",
                to_rust_code_with_ctx(target, loop_vars),
                name,
                args_str.join(", ")
            )
        }
        Expr::BinaryOp(op, lhs, rhs) => format!(
            "({} {} {})",
            to_rust_code_with_ctx(lhs, loop_vars),
            op.as_str(),
            to_rust_code_with_ctx(rhs, loop_vars)
        ),
        Expr::Unary(op, expr) => format!("({}{})", op.as_str(), to_rust_code_with_ctx(expr, loop_vars)),
        Expr::Lit(s) => s.clone(),
        Expr::Convert(target, converter) => format!(
            "{}.convert(&{})",
            converter,
            to_rust_code_with_ctx(target, loop_vars)
        ),
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  递归下降解析器实现
//
//  文法（从低到高优先级）：
//    convert    := logic (',' ident)?
//    logic      := comparison (('&&' | '||') comparison)*
//    comparison := add       (('==' | '!=' | '>=' | '<=' | '>' | '<') add)*
//    add        := mul       (('+' | '-') mul)*
//    mul        := unary     (('*' | '/' | '%') unary)*
//    unary      := ('!' | '-') unary | postfix
//    postfix    := primary (('.' ident) | ('[' expr ']') | ('(' args ')'))*
//    primary    := ident | literal | '(' expr ')'
// ──────────────────────────────────────────────────────────────────────────

struct Parser<'a> {
    chars: Vec<char>,
    pos: usize,
    _input: &'a str,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            _input: input,
        }
    }

    fn err(&self, msg: &str) -> ParseError {
        ParseError {
            message: msg.to_string(),
            pos: self.pos,
        }
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let c = self.peek();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// 匹配并消费指定字符串，匹配成功返回 true
    fn eat_str(&mut self, s: &str) -> bool {
        let target: Vec<char> = s.chars().collect();
        if self.pos + target.len() > self.chars.len() {
            return false;
        }
        for (i, &c) in target.iter().enumerate() {
            if self.chars[self.pos + i] != c {
                return false;
            }
        }
        self.pos += target.len();
        true
    }

    /// 查看下一个是否是指定字符串（不消费）
    fn peek_str(&self, s: &str) -> bool {
        let target: Vec<char> = s.chars().collect();
        if self.pos + target.len() > self.chars.len() {
            return false;
        }
        for (i, &c) in target.iter().enumerate() {
            if self.chars[self.pos + i] != c {
                return false;
            }
        }
        true
    }

    fn remaining(&self) -> String {
        self.chars[self.pos..].iter().collect()
    }

    /// 顶层：解析 convert 表达式
    fn parse_convert(&mut self) -> Result<Expr, ParseError> {
        let lhs = self.parse_logic()?;
        self.skip_ws();
        // 转换器管道语法：expr | Ident | Ident | ...
        // 借鉴 shell 管道，从左到右串联，前一个的输出作为后一个的输入。
        // 单个 | 与逻辑或 || 不会混淆：|| 在 parse_logic 中用 eat_str("||") 匹配。
        let mut result = lhs;
        while self.peek() == Some('|') {
            self.advance();
            self.skip_ws();
            let converter = self.parse_ident()?;
            result = Expr::Convert(Box::new(result), converter);
            self.skip_ws();
        }
        Ok(result)
    }

    fn parse_logic(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_comparison()?;
        loop {
            self.skip_ws();
            let op = if self.eat_str("&&") {
                Op::And
            } else if self.eat_str("||") {
                Op::Or
            } else {
                break;
            };
            self.skip_ws();
            let rhs = self.parse_comparison()?;
            lhs = Expr::BinaryOp(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_add()?;
        loop {
            self.skip_ws();
            let op = if self.eat_str("==") {
                Op::Eq
            } else if self.eat_str("!=") {
                Op::Ne
            } else if self.eat_str(">=") {
                Op::Ge
            } else if self.eat_str("<=") {
                Op::Le
            } else if self.peek_str(">") && !self.peek_str(">=") {
                self.advance();
                Op::Gt
            } else if self.peek_str("<") && !self.peek_str("<=") {
                self.advance();
                Op::Lt
            } else {
                break;
            };
            self.skip_ws();
            let rhs = self.parse_add()?;
            lhs = Expr::BinaryOp(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_add(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_mul()?;
        loop {
            self.skip_ws();
            let op = if self.peek() == Some('+') {
                self.advance();
                Op::Add
            } else if self.peek() == Some('-') {
                self.advance();
                Op::Sub
            } else {
                break;
            };
            self.skip_ws();
            let rhs = self.parse_mul()?;
            lhs = Expr::BinaryOp(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_mul(&mut self) -> Result<Expr, ParseError> {
        let mut lhs = self.parse_unary()?;
        loop {
            self.skip_ws();
            let op = if self.peek() == Some('*') {
                self.advance();
                Op::Mul
            } else if self.peek() == Some('/') {
                self.advance();
                Op::Div
            } else if self.peek() == Some('%') {
                self.advance();
                Op::Mod
            } else {
                break;
            };
            self.skip_ws();
            let rhs = self.parse_unary()?;
            lhs = Expr::BinaryOp(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        if self.peek() == Some('!') {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::Not, Box::new(expr)));
        }
        if self.peek() == Some('-') {
            self.advance();
            let expr = self.parse_unary()?;
            return Ok(Expr::Unary(UnaryOp::Neg, Box::new(expr)));
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;
        loop {
            self.skip_ws();
            match self.peek() {
                Some('.') => {
                    self.advance();
                    let name = self.parse_ident()?;
                    // 检查是否是方法调用
                    self.skip_ws();
                    if self.peek() == Some('(') {
                        self.advance();
                        let args = self.parse_args()?;
                        expr = Expr::MethodCall(Box::new(expr), name, args);
                    } else {
                        expr = Expr::Member(Box::new(expr), name);
                    }
                }
                Some('[') => {
                    self.advance();
                    self.skip_ws();
                    // 索引表达式不能用 convert 语法（`| Converter`），
                    // 否则 `arr[a, b]` 会被误解为转换器
                    let index = self.parse_logic()?;
                    self.skip_ws();
                    if self.peek() != Some(']') {
                        return Err(self.err("expected ']' after index expression"));
                    }
                    self.advance();
                    expr = Expr::Index(Box::new(expr), Box::new(index));
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        self.skip_ws();
        if self.peek() == Some(')') {
            self.advance();
            return Ok(args);
        }
        loop {
            // 参数不能用 convert 语法（`| Converter`），
            // 否则 `f(a, b)` 中的逗号会被误解为转换器分隔符
            let arg = self.parse_logic()?;
            args.push(arg);
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.advance();
                    self.skip_ws();
                }
                Some(')') => {
                    self.advance();
                    break;
                }
                _ => return Err(self.err("expected ',' or ')' in argument list")),
            }
        }
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        self.skip_ws();
        match self.peek() {
            Some('(') => {
                self.advance();
                // 括号内不能用 convert 语法，否则 `(a, b)` 不会被误解（现在用 | 不是逗号）
                let inner = self.parse_logic()?;
                self.skip_ws();
                if self.peek() != Some(')') {
                    return Err(self.err("expected ')' after expression"));
                }
                self.advance();
                Ok(inner)
            }
            Some('"') => self.parse_string_literal(),
            Some(c) if c.is_ascii_digit() => self.parse_number_literal(),
            Some(c) if is_ident_start(c) => {
                let name = self.parse_ident()?;
                // 字面量 true/false
                if name == "true" || name == "false" {
                    Ok(Expr::Lit(name))
                } else {
                    Ok(Expr::Field(name))
                }
            }
            _ => Err(self.err(&format!(
                "unexpected character {:?}, expected identifier, literal, or '('",
                self.peek()
            ))),
        }
    }

    fn parse_ident(&mut self) -> Result<String, ParseError> {
        self.skip_ws();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if is_ident_char(c) {
                self.advance();
            } else {
                break;
            }
        }
        if self.pos == start {
            return Err(self.err("expected identifier"));
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn parse_string_literal(&mut self) -> Result<Expr, ParseError> {
        // 假设 peek 是 '"'
        let start = self.pos;
        self.advance(); // 消费开引号
        let mut s = String::from("\"");
        while let Some(c) = self.peek() {
            if c == '\\' {
                s.push('\\');
                self.advance();
                if let Some(esc) = self.advance() {
                    s.push(esc);
                }
                continue;
            }
            if c == '"' {
                s.push('"');
                self.advance();
                return Ok(Expr::Lit(self.chars[start..self.pos].iter().collect()));
            }
            s.push(c);
            self.advance();
        }
        Err(self.err("unterminated string literal"))
    }

    fn parse_number_literal(&mut self) -> Result<Expr, ParseError> {
        let start = self.pos;
        // 整数部分
        while let Some(c) = self.peek() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        // 小数部分
        if self.peek() == Some('.') && self.peek_at(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
            self.advance();
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    self.advance();
                } else {
                    break;
                }
            }
        }
        // 后缀：f32 / f64 / u32 / i32 等
        while let Some(c) = self.peek() {
            if matches!(c, 'f' | 'u' | 'i' | 'F' | 'U' | 'I')
                || self.peek().map(|c| c.is_ascii_digit()).unwrap_or(false)
            {
                self.advance();
            } else {
                break;
            }
        }
        Ok(Expr::Lit(self.chars[start..self.pos].iter().collect()))
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_ok(s: &str) -> Expr {
        parse(s).unwrap_or_else(|e| panic!("parse {:?} failed: {}", s, e))
    }

    #[test]
    fn parses_simple_field() {
        assert_eq!(parse_ok("count"), Expr::Field("count".into()));
    }

    #[test]
    fn parses_member_access() {
        assert_eq!(
            parse_ok("user.name"),
            Expr::Member(Box::new(Expr::Field("user".into())), "name".into())
        );
    }

    #[test]
    fn parses_index() {
        assert_eq!(
            parse_ok("items[0]"),
            Expr::Index(
                Box::new(Expr::Field("items".into())),
                Box::new(Expr::Lit("0".into()))
            )
        );
    }

    #[test]
    fn parses_method_call_no_args() {
        assert_eq!(
            parse_ok("items.len()"),
            Expr::MethodCall(Box::new(Expr::Field("items".into())), "len".into(), vec![])
        );
    }

    #[test]
    fn parses_method_call_with_args() {
        assert_eq!(
            parse_ok("name.to_uppercase()"),
            Expr::MethodCall(Box::new(Expr::Field("name".into())), "to_uppercase".into(), vec![])
        );
    }

    #[test]
    fn parses_arithmetic() {
        assert_eq!(
            parse_ok("count + 1"),
            Expr::BinaryOp(
                Op::Add,
                Box::new(Expr::Field("count".into())),
                Box::new(Expr::Lit("1".into()))
            )
        );
    }

    #[test]
    fn parses_precedence() {
        // a + b * c → a + (b * c)
        let parsed = parse_ok("a + b * c");
        match parsed {
            Expr::BinaryOp(Op::Add, _, rhs) => {
                assert_eq!(*rhs, Expr::BinaryOp(Op::Mul, Box::new(Expr::Field("b".into())), Box::new(Expr::Field("c".into()))));
            }
            other => panic!("expected BinaryOp(Add), got {:?}", other),
        }
    }

    #[test]
    fn parses_parens() {
        // (a + b) * c → (a + b) * c
        let parsed = parse_ok("(a + b) * c");
        match parsed {
            Expr::BinaryOp(Op::Mul, lhs, rhs) => {
                assert_eq!(*lhs, Expr::BinaryOp(Op::Add, Box::new(Expr::Field("a".into())), Box::new(Expr::Field("b".into()))));
                assert_eq!(*rhs, Expr::Field("c".into()));
            }
            other => panic!("expected BinaryOp(Mul), got {:?}", other),
        }
    }

    #[test]
    fn parses_comparison() {
        assert_eq!(
            parse_ok("count > 0"),
            Expr::BinaryOp(
                Op::Gt,
                Box::new(Expr::Field("count".into())),
                Box::new(Expr::Lit("0".into()))
            )
        );
    }

    #[test]
    fn parses_unary_not() {
        assert_eq!(
            parse_ok("!flag"),
            Expr::Unary(UnaryOp::Not, Box::new(Expr::Field("flag".into())))
        );
    }

    #[test]
    fn parses_converter() {
        assert_eq!(
            parse_ok("count | HexConverter"),
            Expr::Convert(Box::new(Expr::Field("count".into())), "HexConverter".into())
        );
    }

    #[test]
    fn parses_converter_chain() {
        // value | A | B → Convert(Convert(value, "A"), "B")
        assert_eq!(
            parse_ok("value | Trim | Upper"),
            Expr::Convert(
                Box::new(Expr::Convert(
                    Box::new(Expr::Field("value".into())),
                    "Trim".into()
                )),
                "Upper".into()
            )
        );
    }

    #[test]
    fn parses_string_literal() {
        assert_eq!(parse_ok("\"hello\""), Expr::Lit("\"hello\"".into()));
    }

    #[test]
    fn parses_number_literal() {
        assert_eq!(parse_ok("42"), Expr::Lit("42".into()));
        assert_eq!(parse_ok("3.14"), Expr::Lit("3.14".into()));
    }

    #[test]
    fn parses_bool_literal() {
        assert_eq!(parse_ok("true"), Expr::Lit("true".into()));
        assert_eq!(parse_ok("false"), Expr::Lit("false".into()));
    }

    #[test]
    fn to_rust_simple_field() {
        assert_eq!(to_rust_code(&Expr::Field("count".into())), "self.count");
    }

    #[test]
    fn to_rust_member() {
        let expr = Expr::Member(Box::new(Expr::Field("user".into())), "name".into());
        assert_eq!(to_rust_code(&expr), "self.user.name");
    }

    #[test]
    fn to_rust_binary() {
        let expr = Expr::BinaryOp(
            Op::Add,
            Box::new(Expr::Field("count".into())),
            Box::new(Expr::Lit("1".into())),
        );
        assert_eq!(to_rust_code(&expr), "(self.count + 1)");
    }

    #[test]
    fn to_rust_method_call() {
        let expr = Expr::MethodCall(Box::new(Expr::Field("items".into())), "len".into(), vec![]);
        assert_eq!(to_rust_code(&expr), "self.items.len()");
    }

    #[test]
    fn to_rust_converter() {
        let expr = Expr::Convert(Box::new(Expr::Field("count".into())), "HexConverter".into());
        assert_eq!(to_rust_code(&expr), "HexConverter.convert(&self.count)");
    }

    #[test]
    fn to_rust_index() {
        let expr = Expr::Index(
            Box::new(Expr::Field("items".into())),
            Box::new(Expr::Lit("0".into())),
        );
        assert_eq!(to_rust_code(&expr), "self.items[0]");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(parse("").is_err());
    }

    #[test]
    fn rejects_trailing_garbage() {
        assert!(parse("count @").is_err());
    }

    #[test]
    fn parses_complex_expr() {
        // user.profile.age + 1
        let parsed = parse_ok("user.profile.age + 1");
        let expected = Expr::BinaryOp(
            Op::Add,
            Box::new(Expr::Member(
                Box::new(Expr::Member(Box::new(Expr::Field("user".into())), "profile".into())),
                "age".into(),
            )),
            Box::new(Expr::Lit("1".into())),
        );
        assert_eq!(parsed, expected);
    }

    // ─── 运算符覆盖 ───

    #[test]
    fn parses_subtraction() {
        assert_eq!(
            parse_ok("a - b"),
            Expr::BinaryOp(
                Op::Sub,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_multiplication() {
        assert_eq!(
            parse_ok("a * b"),
            Expr::BinaryOp(
                Op::Mul,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_division() {
        assert_eq!(
            parse_ok("a / b"),
            Expr::BinaryOp(
                Op::Div,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_modulo() {
        assert_eq!(
            parse_ok("a % b"),
            Expr::BinaryOp(
                Op::Mod,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_equality() {
        assert_eq!(
            parse_ok("a == b"),
            Expr::BinaryOp(
                Op::Eq,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_not_equal() {
        assert_eq!(
            parse_ok("a != b"),
            Expr::BinaryOp(
                Op::Ne,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_greater_equal() {
        assert_eq!(
            parse_ok("a >= b"),
            Expr::BinaryOp(
                Op::Ge,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_less_equal() {
        assert_eq!(
            parse_ok("a <= b"),
            Expr::BinaryOp(
                Op::Le,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_less_than() {
        assert_eq!(
            parse_ok("a < b"),
            Expr::BinaryOp(
                Op::Lt,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_logical_and() {
        assert_eq!(
            parse_ok("a && b"),
            Expr::BinaryOp(
                Op::And,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    #[test]
    fn parses_logical_or() {
        assert_eq!(
            parse_ok("a || b"),
            Expr::BinaryOp(
                Op::Or,
                Box::new(Expr::Field("a".into())),
                Box::new(Expr::Field("b".into()))
            )
        );
    }

    // ─── 一元运算符 ───

    #[test]
    fn parses_unary_neg() {
        assert_eq!(
            parse_ok("-count"),
            Expr::Unary(UnaryOp::Neg, Box::new(Expr::Field("count".into())))
        );
    }

    #[test]
    fn parses_unary_double_neg() {
        // --count → -(-count)
        assert_eq!(
            parse_ok("--count"),
            Expr::Unary(
                UnaryOp::Neg,
                Box::new(Expr::Unary(UnaryOp::Neg, Box::new(Expr::Field("count".into()))))
            )
        );
    }

    #[test]
    fn parses_unary_not_with_parens() {
        // !(a == b)
        assert_eq!(
            parse_ok("!(a == b)"),
            Expr::Unary(
                UnaryOp::Not,
                Box::new(Expr::BinaryOp(
                    Op::Eq,
                    Box::new(Expr::Field("a".into())),
                    Box::new(Expr::Field("b".into()))
                ))
            )
        );
    }

    // ─── 嵌套索引与方法调用 ───

    #[test]
    fn parses_nested_index() {
        // matrix[0][1] → Index(Index(Field("matrix"), Lit("0")), Lit("1"))
        let parsed = parse_ok("matrix[0][1]");
        let inner = Expr::Index(
            Box::new(Expr::Field("matrix".into())),
            Box::new(Expr::Lit("0".into())),
        );
        assert_eq!(
            parsed,
            Expr::Index(Box::new(inner), Box::new(Expr::Lit("1".into())))
        );
    }

    #[test]
    fn parses_method_call_with_single_arg() {
        // items.get(0) → MethodCall(Field("items"), "get", [Lit("0")])
        assert_eq!(
            parse_ok("items.get(0)"),
            Expr::MethodCall(
                Box::new(Expr::Field("items".into())),
                "get".into(),
                vec![Expr::Lit("0".into())]
            )
        );
    }

    #[test]
    fn parses_method_call_with_multiple_args() {
        // slice(0, len) → MethodCall(Field("items"), "slice", [Lit("0"), Field("len")])
        assert_eq!(
            parse_ok("items.slice(0, len)"),
            Expr::MethodCall(
                Box::new(Expr::Field("items".into())),
                "slice".into(),
                vec![Expr::Lit("0".into()), Expr::Field("len".into())]
            )
        );
    }

    #[test]
    fn parses_method_call_with_expr_arg() {
        // items.get(i + 1)
        let parsed = parse_ok("items.get(i + 1)");
        match parsed {
            Expr::MethodCall(target, name, args) => {
                assert_eq!(*target, Expr::Field("items".into()));
                assert_eq!(name, "get");
                assert_eq!(args.len(), 1);
                assert_eq!(
                    args[0],
                    Expr::BinaryOp(
                        Op::Add,
                        Box::new(Expr::Field("i".into())),
                        Box::new(Expr::Lit("1".into()))
                    )
                );
            }
            other => panic!("expected MethodCall, got {:?}", other),
        }
    }

    #[test]
    fn parses_chained_method_calls() {
        // name.to_uppercase().len()
        let parsed = parse_ok("name.to_uppercase().len()");
        match parsed {
            Expr::MethodCall(target, name, args) => {
                assert_eq!(name, "len");
                assert!(args.is_empty());
                assert_eq!(
                    *target,
                    Expr::MethodCall(
                        Box::new(Expr::Field("name".into())),
                        "to_uppercase".into(),
                        vec![]
                    )
                );
            }
            other => panic!("expected MethodCall, got {:?}", other),
        }
    }

    // ─── 转换器组合 ───

    #[test]
    fn parses_converter_with_binary_expr() {
        // (count + 1) | HexConverter → Convert(BinaryOp(Add, ...), "HexConverter")
        let parsed = parse_ok("(count + 1) | HexConverter");
        match parsed {
            Expr::Convert(target, converter) => {
                assert_eq!(converter, "HexConverter");
                assert_eq!(
                    *target,
                    Expr::BinaryOp(
                        Op::Add,
                        Box::new(Expr::Field("count".into())),
                        Box::new(Expr::Lit("1".into()))
                    )
                );
            }
            other => panic!("expected Convert, got {:?}", other),
        }
    }

    #[test]
    fn parses_converter_with_member_access() {
        // user.name | UpperConverter
        assert_eq!(
            parse_ok("user.name | UpperConverter"),
            Expr::Convert(
                Box::new(Expr::Member(
                    Box::new(Expr::Field("user".into())),
                    "name".into()
                )),
                "UpperConverter".into()
            )
        );
    }

    // ─── 错误场景 ───

    #[test]
    fn rejects_unclosed_paren() {
        assert!(parse("(a + b").is_err());
    }

    #[test]
    fn rejects_unclosed_string() {
        assert!(parse("\"hello").is_err());
    }

    #[test]
    fn rejects_unclosed_bracket() {
        assert!(parse("items[0").is_err());
    }

    #[test]
    fn rejects_unclosed_method_call() {
        assert!(parse("items.get(0").is_err());
    }

    #[test]
    fn rejects_invalid_char() {
        // @ 不是合法起始字符
        assert!(parse("@invalid").is_err());
    }

    #[test]
    fn rejects_invalid_operator_combination() {
        // a + * b 不是合法表达式
        assert!(parse("a + * b").is_err());
    }

    #[test]
    fn rejects_empty_converter_name() {
        // 管道符后必须跟转换器名
        assert!(parse("count |").is_err());
    }

    #[test]
    fn rejects_trailing_comma_in_args() {
        // 方法参数末尾不允许尾随逗号
        assert!(parse("items.slice(0,)").is_err());
    }

    #[test]
    fn rejects_parenthesized_pair_as_convert() {
        // (a, b) 在括号内不应被误解为 Convert(a, "b")
        assert!(parse("(a, b)").is_err());
    }

    // ─── to_rust_code 覆盖 ───

    #[test]
    fn to_rust_unary_neg() {
        let expr = Expr::Unary(UnaryOp::Neg, Box::new(Expr::Field("count".into())));
        assert_eq!(to_rust_code(&expr), "(-self.count)");
    }

    #[test]
    fn to_rust_unary_not() {
        let expr = Expr::Unary(UnaryOp::Not, Box::new(Expr::Field("flag".into())));
        assert_eq!(to_rust_code(&expr), "(!self.flag)");
    }

    #[test]
    fn to_rust_logical_op() {
        let expr = Expr::BinaryOp(
            Op::And,
            Box::new(Expr::Field("a".into())),
            Box::new(Expr::Field("b".into())),
        );
        assert_eq!(to_rust_code(&expr), "(self.a && self.b)");
    }

    #[test]
    fn to_rust_comparison_op() {
        let expr = Expr::BinaryOp(
            Op::Gt,
            Box::new(Expr::Field("count".into())),
            Box::new(Expr::Lit("0".into())),
        );
        assert_eq!(to_rust_code(&expr), "(self.count > 0)");
    }

    #[test]
    fn to_rust_nested_method_call() {
        // name.to_uppercase().len() → self.name.to_uppercase().len()
        let inner = Expr::MethodCall(
            Box::new(Expr::Field("name".into())),
            "to_uppercase".into(),
            vec![],
        );
        let expr = Expr::MethodCall(Box::new(inner), "len".into(), vec![]);
        assert_eq!(to_rust_code(&expr), "self.name.to_uppercase().len()");
    }

    #[test]
    fn to_rust_method_call_with_args() {
        let expr = Expr::MethodCall(
            Box::new(Expr::Field("items".into())),
            "get".into(),
            vec![Expr::Lit("0".into())],
        );
        assert_eq!(to_rust_code(&expr), "self.items.get(0)");
    }

    #[test]
    fn to_rust_nested_index() {
        // matrix[0][1]
        let inner = Expr::Index(
            Box::new(Expr::Field("matrix".into())),
            Box::new(Expr::Lit("0".into())),
        );
        let expr = Expr::Index(Box::new(inner), Box::new(Expr::Lit("1".into())));
        assert_eq!(to_rust_code(&expr), "self.matrix[0][1]");
    }

    #[test]
    fn to_rust_convert_with_binary() {
        let expr = Expr::Convert(
            Box::new(Expr::BinaryOp(
                Op::Add,
                Box::new(Expr::Field("count".into())),
                Box::new(Expr::Lit("1".into())),
            )),
            "HexConverter".into(),
        );
        assert_eq!(
            to_rust_code(&expr),
            "HexConverter.convert(&(self.count + 1))"
        );
    }

    #[test]
    fn to_rust_converter_chain() {
        // value | Trim | Upper → Upper.convert(&Trim.convert(&self.value))
        let expr = Expr::Convert(
            Box::new(Expr::Convert(
                Box::new(Expr::Field("value".into())),
                "Trim".into(),
            )),
            "Upper".into(),
        );
        assert_eq!(
            to_rust_code(&expr),
            "Upper.convert(&Trim.convert(&self.value))"
        );
    }

    #[test]
    fn to_rust_lit_preserves_string() {
        // 字符串字面量应原样输出（保留引号）
        assert_eq!(to_rust_code(&Expr::Lit("\"hello\"".into())), "\"hello\"");
        assert_eq!(to_rust_code(&Expr::Lit("42".into())), "42");
        assert_eq!(to_rust_code(&Expr::Lit("3.14f64".into())), "3.14f64");
        assert_eq!(to_rust_code(&Expr::Lit("true".into())), "true");
    }

    #[test]
    fn to_rust_unary_neg_double() {
        // --count → -(-self.count)
        let inner = Expr::Unary(UnaryOp::Neg, Box::new(Expr::Field("count".into())));
        let expr = Expr::Unary(UnaryOp::Neg, Box::new(inner));
        assert_eq!(to_rust_code(&expr), "(-(-self.count))");
    }
}
