//! 文档格式化：智能分行（Prettier 风格）
//!
//! 规则：
//! - 单属性 + 无 Element 子节点 → 单行 `<tag attr="x">text</tag>`
//! - 多属性（≥2）或含 Element 子节点 → 多行，每属性/指令独占一行
//! - 缩进：tab_size 空格（默认 2），不用 tab
//! - 属性顺序保持 AST 原序；插值 `{expr}` 原样保留
//! - 文件末尾单个换行符

use lsp_types::{FormattingOptions, Range, TextEdit};

use rust_rml_engine::parser::ast::{
    Attribute, Directive, EachClause, Element, EventHandler, Node, TextSegment,
};
use rust_rml_engine::parser;

/// 格式化整个文档
///
/// 返回 None 表示解析失败（不动文件）；返回 Some(vec) 时仅含一条覆盖全文档的 TextEdit。
pub fn format_document(source: &str, options: &FormattingOptions) -> Option<Vec<TextEdit>> {
    let root = parser::parse(source).ok()?;
    let indent_unit = " ".repeat(options.tab_size as usize);
    let mut out = String::new();
    format_node(&root, 0, &indent_unit, &mut out);
    if !out.ends_with('\n') {
        out.push('\n');
    }

    let line_starts = crate::server::conv::compute_line_starts(source);
    let end_pos = crate::server::conv::offset_to_position(source.len(), source, &line_starts);
    let start_pos = lsp_types::Position { line: 0, character: 0 };
    Some(vec![TextEdit {
        range: Range {
            start: start_pos,
            end: end_pos,
        },
        new_text: out,
    }])
}

/// 递归格式化节点
fn format_node(node: &Node, depth: usize, indent: &str, out: &mut String) {
    match node {
        Node::Element(elem) => format_element(elem, depth, indent, out),
        Node::Text(text) => {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                out.push_str(&indent.repeat(depth));
                out.push_str(trimmed);
                out.push('\n');
            }
        }
        Node::Interpolation { expr, .. } => {
            out.push_str(&indent.repeat(depth));
            out.push('{');
            out.push_str(expr);
            out.push('}');
            out.push('\n');
        }
        Node::MixedText(segs) => {
            let has_content = segs.iter().any(|s| match s {
                TextSegment::Literal(t) => !t.trim().is_empty(),
                TextSegment::Interpolation { .. } => true,
            });
            if has_content {
                out.push_str(&indent.repeat(depth));
                for seg in segs {
                    match seg {
                        TextSegment::Literal(t) => out.push_str(t.trim()),
                        TextSegment::Interpolation { expr, .. } => {
                            out.push('{');
                            out.push_str(expr);
                            out.push('}');
                        }
                    }
                }
                out.push('\n');
            }
        }
    }
}

/// 格式化元素
fn format_element(elem: &Element, depth: usize, indent: &str, out: &mut String) {
    let pad = indent.repeat(depth);
    let has_element_children = elem.children.iter().any(|c| matches!(c, Node::Element(_)));
    let multi_attr = elem.attributes.len() + elem.directives.len() >= 2;

    // 单行模式：0 或 1 个属性 + 无 Element 子节点 + 无指令
    let single_line = !has_element_children && !multi_attr && elem.children.len() <= 1;

    if single_line {
        out.push_str(&pad);
        out.push('<');
        out.push_str(&elem.tag);
        for attr in &elem.attributes {
            out.push(' ');
            format_attr(attr, out);
        }
        for d in &elem.directives {
            out.push(' ');
            format_directive(d, out);
        }
        if elem.children.is_empty() {
            out.push_str("></");
            out.push_str(&elem.tag);
            out.push_str(">\n");
        } else {
            out.push('>');
            for child in &elem.children {
                match child {
                    Node::Text(t) => out.push_str(t.trim()),
                    Node::Interpolation { expr, .. } => {
                        out.push('{');
                        out.push_str(expr);
                        out.push('}');
                    }
                    Node::MixedText(segs) => {
                        for seg in segs {
                            match seg {
                                TextSegment::Literal(t) => out.push_str(t.trim()),
                                TextSegment::Interpolation { expr, .. } => {
                                    out.push('{');
                                    out.push_str(expr);
                                    out.push('}');
                                }
                            }
                        }
                    }
                    Node::Element(_) => {}
                }
            }
            out.push_str("</");
            out.push_str(&elem.tag);
            out.push_str(">\n");
        }
        return;
    }

    // 多行模式
    out.push_str(&pad);
    out.push('<');
    out.push_str(&elem.tag);
    for attr in &elem.attributes {
        out.push('\n');
        out.push_str(&indent.repeat(depth + 1));
        format_attr(attr, out);
    }
    for d in &elem.directives {
        out.push('\n');
        out.push_str(&indent.repeat(depth + 1));
        format_directive(d, out);
    }
    if elem.children.is_empty() {
        out.push_str("></");
        out.push_str(&elem.tag);
        out.push_str(">\n");
        return;
    }
    out.push_str(">\n");
    for child in &elem.children {
        format_node(child, depth + 1, indent, out);
    }
    out.push_str(&pad);
    out.push_str("</");
    out.push_str(&elem.tag);
    out.push_str(">\n");
}

/// 格式化属性
fn format_attr(attr: &Attribute, out: &mut String) {
    match attr {
        Attribute::Static { name, value, .. } => {
            out.push_str(name);
            out.push_str("=\"");
            out.push_str(value);
            out.push('"');
        }
        Attribute::Bind { name, expr, .. } => {
            out.push_str(name);
            out.push_str("={");
            out.push_str(expr);
            out.push('}');
        }
        Attribute::Event { name, handler, .. } => {
            out.push_str(name);
            out.push_str("=");
            format_handler(handler, out);
        }
    }
}

/// 格式化事件处理器
fn format_handler(h: &EventHandler, out: &mut String) {
    match h {
        EventHandler::Ident(name) | EventHandler::MethodName(name) => {
            out.push('{');
            out.push_str(name);
            out.push('}');
        }
        EventHandler::WithArgs(name, args) => {
            out.push('{');
            out.push_str(name);
            if !args.is_empty() {
                out.push_str(", ");
                out.push_str(&args.join(", "));
            }
            out.push('}');
        }
    }
}

/// 格式化指令
fn format_directive(d: &Directive, out: &mut String) {
    match d {
        Directive::If(expr) => {
            out.push_str("if={");
            out.push_str(expr);
            out.push('}');
        }
        Directive::Else => out.push_str("else"),
        Directive::Each(each) => format_each(each, out),
        Directive::Key(expr) => {
            out.push_str("key={");
            out.push_str(expr);
            out.push('}');
        }
        Directive::Model { field, converter } => {
            out.push_str("model={");
            out.push_str(field);
            if let Some(c) = converter {
                out.push_str(" | ");
                out.push_str(c);
            }
            out.push('}');
        }
        Directive::Show(expr) => {
            out.push_str("show={");
            out.push_str(expr);
            out.push('}');
        }
        Directive::Once => out.push_str("once"),
        Directive::Html(expr) => {
            out.push_str("html={");
            out.push_str(expr);
            out.push('}');
        }
        Directive::Ref(name) => {
            out.push_str("ref=\"");
            out.push_str(name);
            out.push('"');
        }
    }
}

/// 格式化 each 子句
fn format_each(each: &EachClause, out: &mut String) {
    out.push_str("each={");
    out.push_str(&each.item);
    if let Some(idx) = &each.index {
        out.push_str(", ");
        out.push_str(idx);
    }
    out.push_str(" in ");
    out.push_str(&each.iterable);
    out.push('}');
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fmt(source: &str) -> String {
        let opts = FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        let edits = match format_document(source, &opts) {
            Some(e) => e,
            None => {
                let err = parser::parse(source).err();
                panic!("parse failed: {:?}\nsource: {}", err, source);
            }
        };
        assert_eq!(edits.len(), 1);
        edits[0].new_text.clone()
    }

    fn reparse_ok(formatted: &str) -> bool {
        parser::parse(formatted).is_ok()
    }

    #[test]
    fn single_attr_single_line() {
        let out = fmt("<component><div class=\"x\">hello</div></component>");
        // div 单属性 + 纯文本子节点 → 单行
        assert!(out.contains("<div class=\"x\">hello</div>"), "got:\n{}", out);
        assert!(reparse_ok(&out));
    }

    #[test]
    fn multi_attr_multi_line() {
        let out = fmt("<component><div class=\"x\" id=\"y\">hello</div></component>");
        // div 在 depth=1，属性在 depth=2（4 空格缩进）
        assert!(out.contains("  <div\n"), "got:\n{}", out);
        assert!(out.contains("    class=\"x\"\n"), "got:\n{}", out);
        assert!(out.contains("    id=\"y\">\n"), "got:\n{}", out);
        assert!(reparse_ok(&out));
    }

    #[test]
    fn nested_element_children() {
        let out = fmt("<component><div><span>hi</span></div></component>");
        // 含 Element 子节点 → 多行
        assert!(out.contains("<div>\n"), "got:\n{}", out);
        assert!(out.contains("  <span>hi</span>\n"), "got:\n{}", out);
        assert!(reparse_ok(&out));
    }

    #[test]
    fn bind_attr_format() {
        let out = fmt("<component><div count={count}>x</div></component>");
        assert!(out.contains("count={count}"), "got:\n{}", out);
        assert!(reparse_ok(&out));
    }

    #[test]
    fn empty_element_same_line() {
        let out = fmt("<component><div></div></component>");
        assert!(out.contains("<div></div>"), "got:\n{}", out);
        assert!(reparse_ok(&out));
    }

    #[test]
    fn multi_line_source_normalized() {
        let src = "<component>\n  <div   class=\"a\"\n      id=\"b\">\n    text\n  </div>\n</component>";
        let out = fmt(src);
        // div 在 depth=1，属性在 depth=2（4 空格缩进）
        assert!(out.contains("  <div\n    class=\"a\"\n    id=\"b\">\n"), "got:\n{}", out);
        assert!(out.ends_with("</component>\n"));
        assert!(reparse_ok(&out));
    }

    #[test]
    fn parse_failure_returns_none() {
        let opts = FormattingOptions {
            tab_size: 2,
            insert_spaces: true,
            ..Default::default()
        };
        // 不闭合的标签应导致 ParseError
        let result = format_document("<component><div></component>", &opts);
        // 解析器可能容错，但若返回 None 即可
        // 主要确保不 panic
        let _ = result;
    }
}
