//! CSS 解析器（递归下降）
//!
//! 将 CSS 文本解析为 `StyleSheet`。
//! 详见文档 §7.2 CSS 子集与扩展。

use super::ast::*;

/// 解析错误
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "CSS parse error at {}: {}", self.pos, self.message)
    }
}

/// 解析 CSS 文本为 `StyleSheet`
pub fn parse(input: &str) -> Result<StyleSheet, ParseError> {
    let mut p = Parser {
        chars: input.chars().collect(),
        pos: 0,
    };
    p.parse_stylesheet()
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn parse_stylesheet(&mut self) -> Result<StyleSheet, ParseError> {
        let mut sheet = StyleSheet::default();

        while self.pos < self.chars.len() {
            self.skip_ws_and_comments();
            if self.pos >= self.chars.len() {
                break;
            }

            // 检测 :root 块
            if self.peek_str(":root") {
                self.advance_str(":root".len());
                self.skip_ws_and_comments();
                if self.peek() == Some('{') {
                    self.advance();
                    self.parse_root_block(&mut sheet)?;
                    continue;
                }
            }

            // 普通规则
            let selectors = self.parse_selectors()?;
            self.skip_ws_and_comments();
            self.expect('{')?;
            self.skip_ws_and_comments();

            let declarations = self.parse_declarations()?;
            self.expect('}')?;

            sheet.rules.push(Rule {
                selectors,
                declarations,
            });
        }

        Ok(sheet)
    }

    fn parse_root_block(&mut self, sheet: &mut StyleSheet) -> Result<(), ParseError> {
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') {
                self.advance();
                break;
            }
            // --var: value;
            if self.peek() == Some('-') && self.peek_at(1) == Some('-') {
                let var_name = self.parse_var_name()?;
                self.skip_ws_and_comments();
                self.expect(':')?;
                self.skip_ws_and_comments();
                let value = self.parse_value()?;
                self.skip_ws_and_comments();
                if self.peek() == Some(';') {
                    self.advance();
                }
                sheet.variables.insert(var_name, value);
            } else {
                // 跳过未知内容
                self.skip_until_char(&[';', '}']);
                if self.peek() == Some(';') {
                    self.advance();
                }
            }
        }
        Ok(())
    }

    fn parse_var_name(&mut self) -> Result<String, ParseError> {
        let mut name = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if name.starts_with("--") {
            Ok(name)
        } else {
            Err(self.err("expected CSS variable name (--name)"))
        }
    }

    fn parse_selectors(&mut self) -> Result<Vec<Selector>, ParseError> {
        let mut selectors = Vec::new();
        loop {
            self.skip_ws_and_comments();
            let sel = self.parse_selector()?;
            selectors.push(sel);
            self.skip_ws_and_comments();
            if self.peek() == Some(',') {
                self.advance();
            } else {
                break;
            }
        }
        Ok(selectors)
    }

    fn parse_selector(&mut self) -> Result<Selector, ParseError> {
        let first = self.parse_simple_selector()?;
        self.skip_ws_and_comments();

        // 后代或子选择器
        if self.peek() == Some('>') {
            self.advance();
            self.skip_ws_and_comments();
            let child = self.parse_simple_selector()?;
            let combined = Selector::Child(Box::new(first), Box::new(child));
            return self.maybe_descendant(combined);
        }

        // 检查是否有后代选择器（空格后跟另一个选择器，但不是 { 或 ,）
        if self.peek().map(|c| c.is_alphabetic() || c == '.' || c == '#' || c == '*').unwrap_or(false) {
            let descendant = self.parse_simple_selector()?;
            let combined = Selector::Descendant(Box::new(first), Box::new(descendant));
            return self.maybe_descendant(combined);
        }

        Ok(first)
    }

    fn maybe_descendant(&mut self, current: Selector) -> Result<Selector, ParseError> {
        let mut result = current;
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('>') {
                self.advance();
                self.skip_ws_and_comments();
                let child = self.parse_simple_selector()?;
                result = Selector::Child(Box::new(result), Box::new(child));
            } else if self.peek().map(|c| c.is_alphabetic() || c == '.' || c == '#' || c == '*').unwrap_or(false) {
                let descendant = self.parse_simple_selector()?;
                result = Selector::Descendant(Box::new(result), Box::new(descendant));
            } else {
                break;
            }
        }
        Ok(result)
    }

    fn parse_simple_selector(&mut self) -> Result<Selector, ParseError> {
        let mut parts = Vec::new();
        loop {
            match self.peek() {
                Some('.') => {
                    self.advance();
                    let name = self.parse_ident();
                    parts.push(Selector::Class(name));
                }
                Some('#') => {
                    self.advance();
                    let name = self.parse_ident();
                    parts.push(Selector::Id(name));
                }
                Some('*') => {
                    self.advance();
                    parts.push(Selector::Universal);
                }
                Some(c) if c.is_alphabetic() => {
                    let name = self.parse_ident();
                    parts.push(Selector::Tag(name));
                }
                _ => break,
            }
        }
        match parts.len() {
            0 => Err(self.err("expected selector")),
            1 => Ok(parts.into_iter().next().unwrap()),
            _ => Ok(Selector::Compound(parts)),
        }
    }

    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ParseError> {
        let mut decls = Vec::new();
        loop {
            self.skip_ws_and_comments();
            if self.peek() == Some('}') {
                break;
            }
            let property = self.parse_ident();
            self.skip_ws_and_comments();
            self.expect(':')?;
            self.skip_ws_and_comments();
            let value = self.parse_value()?;
            self.skip_ws_and_comments();
            if self.peek() == Some(';') {
                self.advance();
            }
            decls.push(Declaration { property, value });
        }
        Ok(decls)
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        let mut values = Vec::new();
        loop {
            self.skip_ws();
            if values.is_empty() || self.peek().map(|c| c.is_alphanumeric() || c == '#' || c == '-' || c == '\'' || c == '"' || c == '(').unwrap_or(false) {
                if self.peek().is_none() || self.peek() == Some(';') || self.peek() == Some('}') {
                    break;
                }
                let v = self.parse_single_value()?;
                values.push(v);
                self.skip_ws();
                // 检查是否是简写列表的一部分（空格分隔的多个值）
                if self.peek().map(|c| c.is_alphanumeric() || c == '#' || c == '-' || c == '\'' || c == '"').unwrap_or(false) {
                    continue;
                } else {
                    break;
                }
            } else {
                break;
            }
        }
        match values.len() {
            0 => Err(self.err("expected value")),
            1 => Ok(values.into_iter().next().unwrap()),
            _ => Ok(Value::List(values)),
        }
    }

    fn parse_single_value(&mut self) -> Result<Value, ParseError> {
        match self.peek() {
            Some('#') => self.parse_hex_color(),
            Some('\'') | Some('"') => self.parse_string_literal(),
            Some(c) if c.is_ascii_digit() || c == '-' && self.peek_at(1) != Some('-') || c == '.' => {
                self.parse_number_or_length()
            }
            Some(c) if c == '-' && self.peek_at(1) == Some('-') => {
                // CSS 变量名 --name（独立出现时，如 var() 参数）
                let name = self.parse_var_name()?;
                Ok(Value::String(name))
            }
            Some(c) if c.is_alphabetic() => {
                let name = self.parse_ident();
                self.skip_ws();
                if self.peek() == Some('(') {
                    self.advance();
                    if name == "var" {
                        // var(--name) 或 var(--name, fallback)
                        return self.parse_var_function();
                    }
                    let args = self.parse_function_args()?;
                    Ok(Value::Function(name, args))
                } else {
                    // 关键字或颜色名
                    Ok(parse_keyword_or_color(&name))
                }
            }
            _ => Err(self.err("expected value")),
        }
    }

    /// 解析 var() 函数体：--name 或 --name, fallback
    fn parse_var_function(&mut self) -> Result<Value, ParseError> {
        self.skip_ws();
        let name = self.parse_var_name()?;
        self.skip_ws();
        let fallback = if self.peek() == Some(',') {
            self.advance();
            self.skip_ws();
            let fb = self.parse_single_value()?;
            Some(Box::new(fb))
        } else {
            None
        };
        self.skip_ws();
        self.expect(')')?;
        Ok(Value::Var(name, fallback))
    }

    fn parse_function_args(&mut self) -> Result<Vec<Value>, ParseError> {
        let mut args = Vec::new();
        loop {
            self.skip_ws();
            if self.peek() == Some(')') {
                self.advance();
                break;
            }
            let v = self.parse_single_value()?;
            args.push(v);
            self.skip_ws();
            if self.peek() == Some(',') {
                self.advance();
            }
        }
        Ok(args)
    }

    fn parse_hex_color(&mut self) -> Result<Value, ParseError> {
        self.advance(); // consume #
        let mut hex = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_hexdigit() {
                hex.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let color = match hex.len() {
            3 => Color::rgb(
                u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0),
                u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0),
                u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0),
            ),
            6 => Color::rgb(
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
            ),
            8 => Color::rgba(
                u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
                u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
                u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
                u8::from_str_radix(&hex[6..8], 16).unwrap_or(0),
            ),
            _ => return Err(self.err("invalid hex color")),
        };
        Ok(Value::Color(color))
    }

    fn parse_string_literal(&mut self) -> Result<Value, ParseError> {
        let quote = self.peek().unwrap();
        self.advance();
        let mut s = String::new();
        while self.pos < self.chars.len() && self.chars[self.pos] != quote {
            s.push(self.chars[self.pos]);
            self.advance();
        }
        if self.peek() == Some(quote) {
            self.advance();
        }
        Ok(Value::String(s))
    }

    fn parse_number_or_length(&mut self) -> Result<Value, ParseError> {
        let mut num = String::new();
        if self.peek() == Some('-') {
            num.push('-');
            self.advance();
        }
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_digit() || c == '.' {
                num.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let n: f32 = num.parse().map_err(|_| self.err("invalid number"))?;

        // 检查单位
        let mut unit_str = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_alphabetic() || c == '%' {
                unit_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        let unit = match unit_str.as_str() {
            "" => return Ok(Value::Number(n)),
            "px" => Unit::Px,
            "pt" => Unit::Pt,
            "em" => Unit::Em,
            "rem" => Unit::Rem,
            "%" => Unit::Percent,
            "vw" => Unit::Vw,
            "vh" => Unit::Vh,
            _ => return Ok(Value::Keyword(format!("{}{}", n, unit_str))),
        };
        Ok(Value::Length(n, unit))
    }

    fn parse_ident(&mut self) -> String {
        let mut name = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '-' || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }

    // ─── 辅助方法 ───

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn peek_str(&self, s: &str) -> bool {
        let s_chars: Vec<char> = s.chars().collect();
        for (i, &c) in s_chars.iter().enumerate() {
            if self.chars.get(self.pos + i) != Some(&c) {
                return false;
            }
        }
        true
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn advance_str(&mut self, len: usize) {
        self.pos += len;
    }

    fn expect(&mut self, c: char) -> Result<(), ParseError> {
        if self.peek() == Some(c) {
            self.advance();
            Ok(())
        } else {
            Err(self.err(&format!("expected '{}'", c)))
        }
    }

    fn skip_ws(&mut self) {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
    }

    fn skip_ws_and_comments(&mut self) {
        loop {
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }
            // 跳过 /* comments */
            if self.pos + 1 < self.chars.len()
                && self.chars[self.pos] == '/'
                && self.chars[self.pos + 1] == '*'
            {
                self.pos += 2;
                while self.pos + 1 < self.chars.len() {
                    if self.chars[self.pos] == '*' && self.chars[self.pos + 1] == '/' {
                        self.pos += 2;
                        break;
                    }
                    self.pos += 1;
                }
            } else {
                break;
            }
        }
    }

    fn skip_until_char(&mut self, stops: &[char]) {
        while self.pos < self.chars.len() && !stops.contains(&self.chars[self.pos]) {
            self.pos += 1;
        }
    }

    fn err(&self, msg: &str) -> ParseError {
        ParseError {
            message: msg.to_string(),
            pos: self.pos,
        }
    }
}

/// 将关键字解析为颜色或 Keyword 值
fn parse_keyword_or_color(name: &str) -> Value {
    // 常见 CSS 颜色名
    let color = match name.to_lowercase().as_str() {
        "red" => Some(Color::rgb(255, 0, 0)),
        "green" => Some(Color::rgb(0, 128, 0)),
        "blue" => Some(Color::rgb(0, 0, 255)),
        "white" => Some(Color::rgb(255, 255, 255)),
        "black" => Some(Color::rgb(0, 0, 0)),
        "yellow" => Some(Color::rgb(255, 255, 0)),
        "orange" => Some(Color::rgb(255, 165, 0)),
        "purple" => Some(Color::rgb(128, 0, 128)),
        "pink" => Some(Color::rgb(255, 192, 203)),
        "gray" | "grey" => Some(Color::rgb(128, 128, 128)),
        "transparent" => Some(Color::rgba(0, 0, 0, 0)),
        _ => None,
    };
    match color {
        Some(c) => Value::Color(c),
        None => Value::Keyword(name.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_class_rule() {
        let css = ".card { padding: 10px; }";
        let sheet = parse(css).unwrap();
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].selectors, vec![Selector::Class("card".into())]);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
        assert_eq!(sheet.rules[0].declarations[0].property, "padding");
    }

    #[test]
    fn parse_hex_color() {
        let css = ".x { color: #ff0000; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::Color(c) => assert_eq!(*c, Color::rgb(255, 0, 0)),
            other => panic!("expected Color, got {:?}", other),
        }
    }

    #[test]
    fn parse_short_hex_color() {
        let css = ".x { color: #f00; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::Color(c) => assert_eq!(*c, Color::rgb(255, 0, 0)),
            _ => panic!("expected Color"),
        }
    }

    #[test]
    fn parse_named_color() {
        let css = ".x { color: red; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::Color(c) => assert_eq!(*c, Color::rgb(255, 0, 0)),
            _ => panic!("expected Color"),
        }
    }

    #[test]
    fn parse_root_variables() {
        let css = ":root { --primary: #007bff; --spacing: 8px; }";
        let sheet = parse(css).unwrap();
        assert_eq!(sheet.variables.len(), 2);
        assert!(sheet.variables.contains_key("--primary"));
        assert!(sheet.variables.contains_key("--spacing"));
    }

    #[test]
    fn parse_group_selectors() {
        let css = "h1, h2, h3 { font-weight: bold; }";
        let sheet = parse(css).unwrap();
        assert_eq!(sheet.rules[0].selectors.len(), 3);
    }

    #[test]
    fn parse_compound_selector() {
        let css = ".button.primary { background: blue; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].selectors[0] {
            Selector::Compound(parts) => assert_eq!(parts.len(), 2),
            other => panic!("expected Compound, got {:?}", other),
        }
    }

    #[test]
    fn parse_descendant_selector() {
        let css = ".container .title { font-size: 24px; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].selectors[0] {
            Selector::Descendant(_, _) => {}
            other => panic!("expected Descendant, got {:?}", other),
        }
    }

    #[test]
    fn parse_child_selector() {
        let css = ".list > .item { border: 1px solid #ccc; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].selectors[0] {
            Selector::Child(_, _) => {}
            other => panic!("expected Child, got {:?}", other),
        }
    }

    #[test]
    fn parse_shorthand_list() {
        let css = ".x { margin: 10px 20px; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::List(items) => assert_eq!(items.len(), 2),
            other => panic!("expected List, got {:?}", other),
        }
    }

    #[test]
    fn parse_function_value() {
        let css = ".x { background: rgba(0, 0, 0, 0.5); }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::Function(name, args) => {
                assert_eq!(name, "rgba");
                assert_eq!(args.len(), 4);
            }
            other => panic!("expected Function, got {:?}", other),
        }
    }

    #[test]
    fn parse_comments() {
        let css = "/* comment */ .x { /* inner */ padding: 10px; }";
        let sheet = parse(css).unwrap();
        assert_eq!(sheet.rules.len(), 1);
        assert_eq!(sheet.rules[0].declarations.len(), 1);
    }

    #[test]
    fn parse_multiple_rules() {
        let css = ".a { color: red; } .b { color: blue; }";
        let sheet = parse(css).unwrap();
        assert_eq!(sheet.rules.len(), 2);
    }
}
