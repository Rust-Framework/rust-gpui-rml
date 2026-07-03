//! AST 定义
//!
//! 详见文档 §10.6.1 AST 数据结构。

use std::fmt;

/// AST 节点
#[derive(Debug, Clone)]
pub enum Node {
    /// 元素节点（含标签、属性、子节点）
    Element(Element),
    /// 纯文本节点
    Text(String),
    /// 单个插值 `{expr}`
    Interpolation(String),
    /// 混合文本：字面量 + 插值
    MixedText(Vec<TextSegment>),
}

/// 文本段（混合文本用）
#[derive(Debug, Clone)]
pub enum TextSegment {
    /// 字面量文本
    Literal(String),
    /// `{expr}` 插值
    Interpolation(String),
}

/// 元素节点
#[derive(Debug, Clone)]
pub struct Element {
    /// 标签名（如 "div"、"button" 或自定义组件 "MyComp"）
    pub tag: String,
    /// 标准属性 + 绑定属性 + 事件属性
    pub attributes: Vec<Attribute>,
    /// 指令（if/each/model/show/once/html/ref/slot/else/key）
    pub directives: Vec<Directive>,
    /// 子节点
    pub children: Vec<Node>,
    /// 具名插槽标识（来自 `slot="name"` 属性）
    ///
    /// 用于 `<template slot="header">...</template>` 形式：父视图通过此字段
    /// 声明该子节点应注入到目标组件的哪个具名插槽。
    /// codegen 据此把子节点从普通 children 中分离，路由到对应 slot setter。
    pub slot_name: Option<String>,
}

/// 属性
#[derive(Debug, Clone)]
pub enum Attribute {
    /// 静态属性 `class="card"`
    Static { name: String, value: String },
    /// 绑定属性 `value={field}`
    Bind { name: String, expr: String },
    /// 事件绑定 `onclick={fn}` 或 `onclick="method"`
    Event { name: String, handler: EventHandler },
}

/// 指令
#[derive(Debug, Clone)]
pub enum Directive {
    /// `if={cond}` 条件渲染
    If(String),
    /// `else` 分支
    Else,
    /// `each={item in items}` 列表渲染
    Each(EachClause),
    /// `key={expr}` 列表项唯一标识
    Key(String),
    /// `model={field}` 双向绑定
    Model(String),
    /// `show={cond}` 显示/隐藏
    Show(String),
    /// `once` 仅首次渲染
    Once,
    /// `html={raw}` 渲染 HTML 字符串
    Html(String),
    /// `ref="name"` 元素引用
    Ref(String),
}

/// `each` 子句
#[derive(Debug, Clone)]
pub struct EachClause {
    /// 迭代变量名
    pub item: String,
    /// 索引变量名（可选）
    pub index: Option<String>,
    /// 可迭代表达式
    pub iterable: String,
}

/// 事件处理器
#[derive(Debug, Clone)]
pub enum EventHandler {
    /// `onclick={fn}` 命令引用
    Ident(String),
    /// `onclick="method"` 方法名字符串
    MethodName(String),
    /// `onclick={fn, {expr}, 'literal'}` 带参数
    WithArgs(String, Vec<String>),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Element(e) => write!(f, "<{}>", e.tag),
            Node::Text(t) => write!(f, "text({:?})", t.chars().take(20).collect::<String>()),
            Node::Interpolation(e) => write!(f, "{{{}}}", e),
            Node::MixedText(segs) => write!(f, "mixed({} segs)", segs.len()),
        }
    }
}
