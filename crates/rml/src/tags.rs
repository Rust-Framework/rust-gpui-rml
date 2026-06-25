//! HTML 标签到 GPUI 元素构造调用的映射表
//!
//! 详见文档 §2.2 标签映射。

use std::collections::HashMap;
use std::sync::OnceLock;

/// 内置 HTML 标签枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinTag {
    Div,
    Span,
    P,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Button,
    Input,
    TextArea,
    Ul,
    Ol,
    Li,
    Img,
    A,
    Label,
    Br,
}

impl BuiltinTag {
    /// 返回该标签在 GPUI 中的构造调用代码（作为字符串）
    pub fn codegen_ctor(self) -> &'static str {
        match self {
            BuiltinTag::Div => "gpui::div()",
            BuiltinTag::Span => "gpui::div()",
            BuiltinTag::P => "gpui::div()",
            BuiltinTag::H1 => "gpui::div()",
            BuiltinTag::H2 => "gpui::div()",
            BuiltinTag::H3 => "gpui::div()",
            BuiltinTag::H4 => "gpui::div()",
            BuiltinTag::H5 => "gpui::div()",
            BuiltinTag::H6 => "gpui::div()",
            BuiltinTag::Button => "gpui::div()",
            BuiltinTag::Input => "gpui::div()",
            BuiltinTag::TextArea => "gpui::div()",
            BuiltinTag::Ul => "gpui::div()",
            BuiltinTag::Ol => "gpui::div()",
            BuiltinTag::Li => "gpui::div()",
            BuiltinTag::Img => "gpui::div()",
            BuiltinTag::A => "gpui::div()",
            BuiltinTag::Label => "gpui::div()",
            BuiltinTag::Br => "gpui::div()",
        }
    }

    /// 是否为自闭合标签
    pub fn is_self_closing(self) -> bool {
        matches!(self, BuiltinTag::Input | BuiltinTag::Img | BuiltinTag::Br)
    }

    /// 标签文本大小（仅 h1~h6 有意义，其他返回 0.0）
    pub fn text_size(self) -> f32 {
        match self {
            BuiltinTag::H1 => 32.0,
            BuiltinTag::H2 => 28.0,
            BuiltinTag::H3 => 24.0,
            BuiltinTag::H4 => 20.0,
            BuiltinTag::H5 => 18.0,
            BuiltinTag::H6 => 16.0,
            _ => 0.0,
        }
    }
}

static TAG_MAP: OnceLock<HashMap<&'static str, BuiltinTag>> = OnceLock::new();

fn build_tag_map() -> HashMap<&'static str, BuiltinTag> {
    let mut m = HashMap::new();
    m.insert("div", BuiltinTag::Div);
    m.insert("span", BuiltinTag::Span);
    m.insert("p", BuiltinTag::P);
    m.insert("h1", BuiltinTag::H1);
    m.insert("h2", BuiltinTag::H2);
    m.insert("h3", BuiltinTag::H3);
    m.insert("h4", BuiltinTag::H4);
    m.insert("h5", BuiltinTag::H5);
    m.insert("h6", BuiltinTag::H6);
    m.insert("button", BuiltinTag::Button);
    m.insert("input", BuiltinTag::Input);
    m.insert("textarea", BuiltinTag::TextArea);
    m.insert("ul", BuiltinTag::Ul);
    m.insert("ol", BuiltinTag::Ol);
    m.insert("li", BuiltinTag::Li);
    m.insert("img", BuiltinTag::Img);
    m.insert("a", BuiltinTag::A);
    m.insert("label", BuiltinTag::Label);
    m.insert("br", BuiltinTag::Br);
    m
}

/// 查找标签名对应的 `BuiltinTag`
pub fn lookup(tag: &str) -> Option<BuiltinTag> {
    TAG_MAP.get_or_init(build_tag_map).get(tag).copied()
}

/// 判断标签是否为内置 HTML 标签（小写）
pub fn is_builtin(tag: &str) -> bool {
    lookup(tag).is_some()
}

/// 判断标签是否为自定义组件（PascalCase）
pub fn is_component(tag: &str) -> bool {
    tag.chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
}
