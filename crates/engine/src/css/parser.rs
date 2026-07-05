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
    /// 错误所在行（1-based），由 `err()` 根据 `pos` 计算
    pub line: usize,
    /// 错误所在列（1-based），由 `err()` 根据 `pos` 计算
    pub column: usize,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "CSS parse error at {}:{} (pos {}): {}",
            self.line, self.column, self.pos, self.message
        )
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
        let (line, column) = self.pos_to_line_col(self.pos);
        ParseError {
            message: msg.to_string(),
            pos: self.pos,
            line,
            column,
        }
    }

    /// 根据字符偏移 `pos` 计算 1-based 行列
    ///
    /// 遍历 `chars[0..pos]`，遇 `\n` 行号+1、列号重置为 1，其余列号+1。
    /// 错误是罕见事件，O(n) 遍历可接受，避免在每次 advance 时维护 line/column。
    fn pos_to_line_col(&self, pos: usize) -> (usize, usize) {
        let mut line = 1;
        let mut column = 1;
        for (i, c) in self.chars.iter().enumerate() {
            if i >= pos {
                break;
            }
            if *c == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        (line, column)
    }
}

/// 将关键字解析为颜色或 Keyword 值
///
/// 支持 CSS Color Module Level 3 全部 147 种命名颜色 + transparent + RebeccaPurple。
/// 详见 https://www.w3.org/TR/css-color-3/。
fn parse_keyword_or_color(name: &str) -> Value {
    // transparent 在 rgba 表中难以表达，单独处理
    if name.eq_ignore_ascii_case("transparent") {
        return Value::Color(Color::rgba(0, 0, 0, 0));
    }
    match lookup_named_color(name) {
        Some((r, g, b)) => Value::Color(Color::rgb(r, g, b)),
        None => Value::Keyword(name.to_string()),
    }
}

/// CSS 标准命名颜色查找表（CSS Color Module Level 3）
///
/// 共 147 种命名颜色，键均为 lowercase。返回 (r, g, b)。
/// 加上 transparent（在 parse_keyword_or_color 中单独处理），共 148 种。
///
/// 性能：解析器在遇到每个颜色名时线性查找；147 项可接受。
/// 若未来性能瓶颈，可改为 phf 编译期哈希表。
fn lookup_named_color(name: &str) -> Option<(u8, u8, u8)> {
    // 按 lowercase 比较以保持大小写不敏感
    const NAMED_COLORS: &[(&str, u8, u8, u8)] = &[
        // 基础 16 色
        ("black", 0, 0, 0),
        ("silver", 192, 192, 192),
        ("gray", 128, 128, 128),
        ("grey", 128, 128, 128),
        ("white", 255, 255, 255),
        ("maroon", 128, 0, 0),
        ("red", 255, 0, 0),
        ("purple", 128, 0, 128),
        ("fuchsia", 255, 0, 255),
        ("magenta", 255, 0, 255),
        ("green", 0, 128, 0),
        ("lime", 0, 255, 0),
        ("olive", 128, 128, 0),
        ("yellow", 255, 255, 0),
        ("navy", 0, 0, 128),
        ("blue", 0, 0, 255),
        ("teal", 0, 128, 128),
        ("aqua", 0, 255, 255),
        ("cyan", 0, 255, 255),
        // 扩展命名颜色（按字母顺序）
        ("aliceblue", 240, 248, 255),
        ("antiquewhite", 250, 235, 215),
        ("aquamarine", 127, 255, 212),
        ("azure", 240, 255, 255),
        ("beige", 245, 245, 220),
        ("bisque", 255, 228, 196),
        ("blanchedalmond", 255, 235, 205),
        ("blueviolet", 138, 43, 226),
        ("brown", 165, 42, 42),
        ("burlywood", 222, 184, 135),
        ("cadetblue", 95, 158, 160),
        ("chartreuse", 127, 255, 0),
        ("chocolate", 210, 105, 30),
        ("coral", 255, 127, 80),
        ("cornflowerblue", 100, 149, 237),
        ("cornsilk", 255, 248, 220),
        ("crimson", 220, 20, 60),
        ("darkblue", 0, 0, 139),
        ("darkcyan", 0, 139, 139),
        ("darkgoldenrod", 184, 134, 11),
        ("darkgray", 169, 169, 169),
        ("darkgrey", 169, 169, 169),
        ("darkgreen", 0, 100, 0),
        ("darkkhaki", 189, 183, 107),
        ("darkmagenta", 139, 0, 139),
        ("darkolivegreen", 85, 107, 47),
        ("darkorange", 255, 140, 0),
        ("darkorchid", 153, 50, 204),
        ("darkred", 139, 0, 0),
        ("darksalmon", 233, 150, 122),
        ("darkseagreen", 143, 188, 143),
        ("darkslateblue", 72, 61, 139),
        ("darkslategray", 47, 79, 79),
        ("darkslategrey", 47, 79, 79),
        ("darkturquoise", 0, 206, 209),
        ("darkviolet", 148, 0, 211),
        ("deeppink", 255, 20, 147),
        ("deepskyblue", 0, 191, 255),
        ("dimgray", 105, 105, 105),
        ("dimgrey", 105, 105, 105),
        ("dodgerblue", 30, 144, 255),
        ("firebrick", 178, 34, 34),
        ("floralwhite", 255, 250, 240),
        ("forestgreen", 34, 139, 34),
        ("gainsboro", 220, 220, 220),
        ("ghostwhite", 248, 248, 255),
        ("gold", 255, 215, 0),
        ("goldenrod", 218, 165, 32),
        ("greenyellow", 173, 255, 47),
        ("honeydew", 240, 255, 240),
        ("hotpink", 255, 105, 180),
        ("indianred", 205, 92, 92),
        ("indigo", 75, 0, 130),
        ("ivory", 255, 255, 240),
        ("khaki", 240, 230, 140),
        ("lavender", 230, 230, 250),
        ("lavenderblush", 255, 240, 245),
        ("lawngreen", 124, 252, 0),
        ("lemonchiffon", 255, 250, 205),
        ("lightblue", 173, 216, 230),
        ("lightcoral", 240, 128, 128),
        ("lightcyan", 224, 255, 255),
        ("lightgoldenrodyellow", 250, 250, 210),
        ("lightgray", 211, 211, 211),
        ("lightgrey", 211, 211, 211),
        ("lightgreen", 144, 238, 144),
        ("lightpink", 255, 182, 193),
        ("lightsalmon", 255, 160, 122),
        ("lightseagreen", 32, 178, 170),
        ("lightskyblue", 135, 206, 250),
        ("lightslategray", 119, 136, 153),
        ("lightslategrey", 119, 136, 153),
        ("lightsteelblue", 176, 196, 222),
        ("lightyellow", 255, 255, 224),
        ("limegreen", 50, 205, 50),
        ("linen", 250, 240, 230),
        ("mediumaquamarine", 102, 205, 170),
        ("mediumblue", 0, 0, 205),
        ("mediumorchid", 186, 85, 211),
        ("mediumpurple", 147, 112, 219),
        ("mediumseagreen", 60, 179, 113),
        ("mediumslateblue", 123, 104, 238),
        ("mediumspringgreen", 0, 250, 154),
        ("mediumturquoise", 72, 209, 204),
        ("mediumvioletred", 199, 21, 133),
        ("midnightblue", 25, 25, 112),
        ("mintcream", 245, 255, 250),
        ("mistyrose", 255, 228, 225),
        ("moccasin", 255, 228, 181),
        ("navajowhite", 255, 222, 173),
        ("oldlace", 253, 245, 230),
        ("olivedrab", 107, 142, 35),
        ("orange", 255, 165, 0),
        ("orangered", 255, 69, 0),
        ("orchid", 218, 112, 214),
        ("palegoldenrod", 238, 232, 170),
        ("palegreen", 152, 251, 152),
        ("paleturquoise", 175, 238, 238),
        ("palevioletred", 219, 112, 147),
        ("papayawhip", 255, 239, 213),
        ("peachpuff", 255, 218, 185),
        ("peru", 205, 133, 63),
        ("pink", 255, 192, 203),
        ("plum", 221, 160, 221),
        ("powderblue", 176, 224, 230),
        ("rebeccapurple", 102, 51, 153),
        ("rosybrown", 188, 143, 143),
        ("royalblue", 65, 105, 225),
        ("saddlebrown", 139, 69, 19),
        ("salmon", 250, 128, 114),
        ("sandybrown", 244, 164, 96),
        ("seagreen", 46, 139, 87),
        ("seashell", 255, 245, 238),
        ("sienna", 160, 82, 45),
        ("skyblue", 135, 206, 235),
        ("slateblue", 106, 90, 205),
        ("slategray", 112, 128, 144),
        ("slategrey", 112, 128, 144),
        ("snow", 255, 250, 250),
        ("springgreen", 0, 255, 127),
        ("steelblue", 70, 130, 180),
        ("tan", 210, 180, 140),
        ("thistle", 216, 191, 216),
        ("tomato", 255, 99, 71),
        ("turquoise", 64, 224, 208),
        ("violet", 238, 130, 238),
        ("wheat", 245, 222, 179),
        ("whitesmoke", 245, 245, 245),
        ("yellowgreen", 154, 205, 50),
    ];
    NAMED_COLORS
        .iter()
        .find(|(n, _, _, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, r, g, b)| (*r, *g, *b))
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
    fn parse_extended_named_colors() {
        // 扩展命名颜色（CSS Level 3）：rebeccapurple、cornflowerblue、tomato 等
        let cases = [
            ("rebeccapurple", 102, 51, 153),
            ("cornflowerblue", 100, 149, 237),
            ("tomato", 255, 99, 71),
            ("darkslategray", 47, 79, 79),
            ("lightyellow", 255, 255, 224),
            ("whitesmoke", 245, 245, 245),
            ("yellowgreen", 154, 205, 50),
        ];
        for (name, r, g, b) in cases {
            let css = format!(".x {{ color: {}; }}", name);
            let sheet = parse(&css).unwrap();
            match &sheet.rules[0].declarations[0].value {
                Value::Color(c) => assert_eq!(*c, Color::rgb(r, g, b), "failed for color {}", name),
                other => panic!("expected Color for {}, got {:?}", name, other),
            }
        }
    }

    #[test]
    fn parse_named_color_case_insensitive() {
        // CSS 颜色名大小写不敏感：RED / Red / red 都应解析为同一颜色
        for name in ["RED", "Red", "red"] {
            let css = format!(".x {{ color: {}; }}", name);
            let sheet = parse(&css).unwrap();
            match &sheet.rules[0].declarations[0].value {
                Value::Color(c) => assert_eq!(*c, Color::rgb(255, 0, 0)),
                other => panic!("expected Color for {}, got {:?}", name, other),
            }
        }
    }

    #[test]
    fn parse_transparent_color() {
        let css = ".x { color: transparent; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::Color(c) => assert_eq!(*c, Color::rgba(0, 0, 0, 0)),
            other => panic!("expected transparent Color, got {:?}", other),
        }
    }

    #[test]
    fn parse_unknown_keyword_still_keyword() {
        // 未知关键字（非颜色名）保留为 Keyword 值
        let css = ".x { color: notacolor; }";
        let sheet = parse(css).unwrap();
        match &sheet.rules[0].declarations[0].value {
            Value::Keyword(k) => assert_eq!(k, "notacolor"),
            other => panic!("expected Keyword, got {:?}", other),
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

    // ─── line/column 错误诊断 ───

    #[test]
    fn error_reports_line_column_single_line() {
        // 第 1 行第 6 列：".card {" 后缺少 }（在 expect 时报错）
        let css = ".card { padding: 10px;";
        let result = parse(css);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // 单行：line=1, column 应大于 1
        assert_eq!(err.line, 1, "line should be 1, got: {}", err.line);
        assert!(err.column >= 1, "column should be >= 1, got: {}", err.column);
        // Display 应包含 line:column
        let display = format!("{}", err);
        assert!(display.contains("1:"), "display: {}", display);
    }

    #[test]
    fn error_reports_line_column_multi_line() {
        // 第二行的 selector 解析失败
        let css = ".a { color: red; }\n@@invalid";
        let result = parse(css);
        assert!(result.is_err());
        let err = result.unwrap_err();
        // 错误在第 2 行
        assert_eq!(err.line, 2, "line should be 2, got: {}", err.line);
    }

    #[test]
    fn error_pos_to_line_col_handles_newlines() {
        // 直接测试 pos_to_line_col 的换算逻辑
        let css = "ab\ncd\nef";
        let p = Parser {
            chars: css.chars().collect(),
            pos: 0,
        };
        // pos=0 → line 1, col 1
        assert_eq!(p.pos_to_line_col(0), (1, 1));
        // pos=2 → 'b' 仍是 line 1, col 3
        assert_eq!(p.pos_to_line_col(2), (1, 3));
        // pos=3 → '\n' 之后，line 2, col 1
        assert_eq!(p.pos_to_line_col(3), (2, 1));
        // pos=6 → 第二行末尾之后，line 3, col 1
        assert_eq!(p.pos_to_line_col(6), (3, 1));
        // pos=8 → line 3, col 3
        assert_eq!(p.pos_to_line_col(8), (3, 3));
    }
}
