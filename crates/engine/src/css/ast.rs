//! CSS AST 定义
//!
//! 详见文档 §7.2 CSS 子集与扩展。

use std::collections::HashMap;

/// CSS 样式表
#[derive(Debug, Clone, Default)]
pub struct StyleSheet {
    /// 规则列表
    pub rules: Vec<Rule>,
    /// `:root` 定义的 CSS 变量（`--name` → 值）
    pub variables: HashMap<String, Value>,
}

/// CSS 规则：选择器 + 声明块
#[derive(Debug, Clone)]
pub struct Rule {
    /// 分组选择器（`h1, h2, h3` 拆分为多个）
    pub selectors: Vec<Selector>,
    /// 声明列表
    pub declarations: Vec<Declaration>,
}

/// CSS 声明：`property: value;`
#[derive(Debug, Clone)]
pub struct Declaration {
    pub property: String,
    pub value: Value,
}

/// CSS 选择器
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// 标签选择器 `div`
    Tag(String),
    /// 类选择器 `.container`
    Class(String),
    /// ID 选择器 `#main`
    Id(String),
    /// 通用选择器 `*`
    Universal,
    /// 后代选择器 `.container .title`
    Descendant(Box<Selector>, Box<Selector>),
    /// 子选择器 `.list > .item`
    Child(Box<Selector>, Box<Selector>),
    /// 交集选择器 `.button.primary`
    Compound(Vec<Selector>),
}

/// CSS 值
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// 长度值 `10px` / `50%`
    Length(f32, Unit),
    /// 颜色 `#ff0000` / `red` / `rgb(255,0,0)`
    Color(Color),
    /// 数字 `1.5` / `100`
    Number(f32),
    /// 关键字/标识符 `flex` / `bold` / `center`
    Keyword(String),
    /// 字符串字面量 `'Arial'`
    String(String),
    /// CSS 变量引用 `var(--name)` 或 `var(--name, fallback)`
    Var(String, Option<Box<Value>>),
    /// 函数调用 `rgba(...)` / `calc(...)` / `linear-gradient(...)`
    Function(String, Vec<Value>),
    /// 简写值列表（如 `10px 20px` 或 `1px solid #ccc`）
    List(Vec<Value>),
}

/// 长度单位
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Px,
    Pt,
    Em,
    Rem,
    Percent,
    Vw,
    Vh,
}

/// 颜色
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b, a: 255 }
    }

    pub const fn rgba(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
}
