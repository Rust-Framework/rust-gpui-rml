//! AST 定义
//!
//! 详见文档 §10.6.1 AST 数据结构。

use crate::parser::span::Span;
use std::fmt;

/// AST 节点
#[derive(Debug, Clone)]
pub enum Node {
    /// 元素节点（含标签、属性、子节点）
    Element(Element),
    /// 纯文本节点
    Text(String),
    /// 单个插值 `{expr}`
    Interpolation { expr: String, span: Span },
    /// 混合文本：字面量 + 插值
    MixedText(Vec<TextSegment>),
}

/// 文本段（混合文本用）
#[derive(Debug, Clone)]
pub enum TextSegment {
    /// 字面量文本
    Literal(String),
    /// `{expr}` 插值
    Interpolation { expr: String, span: Span },
}

/// 元素节点
#[derive(Debug, Clone, Default)]
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
    /// 源码字节区间 [start, end)，覆盖 `<tag ...>...</tag>` 整个元素
    ///
    /// LSP 定位用；codegen 合成的元素该字段为空区间（`Span::empty()`）。
    pub span: Span,
}

/// 属性
#[derive(Debug, Clone)]
pub enum Attribute {
    /// 静态属性 `class="card"`
    Static {
        name: String,
        value: String,
        /// 属性名+值的字节区间（LSP 跳转定位用）
        span: Span,
    },
    /// 绑定属性 `value={field}`
    Bind {
        name: String,
        expr: String,
        /// 属性名+值的字节区间
        span: Span,
    },
    /// 事件绑定 `onclick={fn}` 或 `onclick="method"`
    Event {
        name: String,
        handler: EventHandler,
        /// 属性名+值的字节区间
        span: Span,
    },
}

/// 指令
///
/// 每个变体携带 `span: Span`，覆盖指令名 + `={值}` 的字节区间（如 `if={cond}`），
/// 供 LSP 语义 token 发射与诊断定位使用。`EachClause` 不携带子 span —— LSP token
/// emitter 从 `Directive::Each.span` 内扫描源码提取 `item`/`in`/`iterable` 子区间。
#[derive(Debug, Clone)]
pub enum Directive {
    /// `if={cond}` 条件渲染
    If { expr: String, span: Span },
    /// `else` 分支
    Else { span: Span },
    /// `each={item in items}` 列表渲染
    Each { clause: EachClause, span: Span },
    /// `key={expr}` 列表项唯一标识
    Key { expr: String, span: Span },
    /// `model={field}` 或 `model={field | Converter}` 双向绑定
    Model {
        field: String,
        /// 可选 converter 名（`| Converter` 语法），codegen 反向绑定时调用 `Converter::convert_back`
        converter: Option<String>,
        span: Span,
    },
    /// `show={cond}` 显示/隐藏
    Show { expr: String, span: Span },
    /// `once` 仅首次渲染
    Once { span: Span },
    /// `html={raw}` 渲染 HTML 字符串
    Html { expr: String, span: Span },
    /// `ref="name"` 元素引用
    Ref { name: String, span: Span },
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
    /// `onclick={self.on_click}` 闭包字段引用（P0-1：用户组件事件绑定）
    ///
    /// 用户组件 .rml 模板内应用注入的事件回调字段。codegen 生成
    /// `.on_click(cx.listener(move |this, ev, _w, cx| {
    ///     if let Some(h) = &this.<field> { h(ev, _w, cx.deref_mut()); }
    /// }))`。
    ClosureField(String),
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Node::Element(e) => write!(f, "<{}>", e.tag),
            Node::Text(t) => write!(f, "text({:?})", t.chars().take(20).collect::<String>()),
            Node::Interpolation { expr, .. } => write!(f, "{{{}}}", expr),
            Node::MixedText(segs) => write!(f, "mixed({} segs)", segs.len()),
        }
    }
}
