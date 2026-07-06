//! 悬停功能：标签/属性名/属性值三级细粒度文档
//!
//! 检测优先级（光标从细到粗）：
//! 1. 属性值 span → `format_attribute_value_hover`
//! 2. 属性名 span → `format_attribute_name_hover`
//! 3. 属性整体 span（兜底，如落在 `=` 上）→ `format_attribute_hover`
//! 4. 标签名 span → `format_tag_hover`
//! 5. 其它 → None
//!
//! 所有内容使用 `MarkupContent` Markdown，遵循 LSP 规范。

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use rust_rml_engine::compiler::props_registry;
use rust_rml_engine::parser::ast::{Attribute, Element};
use rust_rml_engine::tags;

use crate::features::ast_util::{
    attr_bind_expr_span, attr_name_span, attr_span, attr_value_span, event_handler_name,
    find_attribute_at_offset, find_element_at_offset, tag_name_span,
};
use crate::server::conv;
use crate::workspace::Workspace;

/// 执行悬停查询
pub fn hover(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
    workspace: &Workspace,
) -> Option<Hover> {
    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = tree.root.as_ref()?;
    let elem = find_element_at_offset(root, byte_offset)?;

    // 三级检测：属性 → 标签名 → 兜底 None
    if let Some(attr) = find_attribute_at_offset(elem, byte_offset) {
        // 属性值优先（最细粒度）
        if let Some(value_span) = attr_value_span(attr, source) {
            if value_span.contains(byte_offset)
                // 零长 span（空字符串值）时，光标落在 start 上视为命中
                || (value_span.start == value_span.end && value_span.start == byte_offset)
            {
                return Some(make_hover(
                    value_span,
                    format_attribute_value_hover(elem, attr, source),
                    source,
                    line_starts,
                ));
            }
        }
        // 属性名次之
        if let Some(name_span) = attr_name_span(attr, source) {
            if name_span.contains(byte_offset) {
                return Some(make_hover(
                    name_span,
                    format_attribute_name_hover(elem, attr),
                    source,
                    line_starts,
                ));
            }
        }
        // 兜底：光标落在属性整体 span 但不在 name/value 上（如 `=`）
        let whole = attr_span(attr);
        return Some(make_hover(
            whole,
            format_attribute_hover(elem, attr),
            source,
            line_starts,
        ));
    }

    // 标签名
    let tag_span = tag_name_span(elem);
    if tag_span.contains(byte_offset) {
        return Some(make_hover(
            tag_span,
            format_tag_hover(elem),
            source,
            line_starts,
        ));
    }

    None
}

/// 构造 Hover：span → LSP Range，content → MarkupContent Markdown
fn make_hover(
    span: rust_rml_engine::parser::Span,
    content: String,
    source: &str,
    line_starts: &[u32],
) -> Hover {
    Hover {
        range: Some(conv::span_to_range(span, source, line_starts)),
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: content,
        }),
    }
}

// ──────────────────────────────────────────────────────────────────────────
// 标签悬停
// ──────────────────────────────────────────────────────────────────────────

/// 生成标签的悬停文档（Markdown）
fn format_tag_hover(elem: &Element) -> String {
    let tag = &elem.tag;
    let mut md = String::new();

    if tags::is_root_tag(tag) {
        md.push_str(&format!("## `<{}>` — Root element\n\n", tag));
        match tag.as_str() {
            "window" => md.push_str("Basic window with transparent title bar.\n"),
            "modern_window" => md.push_str("Modern window with self-drawn TitleBar/Menu/StatusBar.\n"),
            "tab_window" => md.push_str("Advanced window with TabBar title bar and resizable slots.\n"),
            "dialog" => md.push_str("Modal dialog (not a separate OS window).\n"),
            "component" => md.push_str("Reusable component (no window operations).\n"),
            _ => {}
        }
        if let Some(shell_props) = props_registry::shell_props_for(tag) {
            md.push_str("\n**Shell attributes:**\n\n");
            for prop in shell_props {
                md.push_str(&format!("- `{}`\n", prop));
            }
        }
    } else if tags::lookup(tag).is_some() {
        md.push_str(&format!("## `<{}>` — HTML element\n\n", tag));
        md.push_str("Built-in HTML tag mapped to `gpui::div()`.\n");
    } else if tags::component_lookup(tag).is_some() {
        md.push_str(&format!("## `<{}>` — Component\n\n", tag));
        md.push_str("gpui-component extension.\n");

        let (statics, binds, events) = props_registry::props_for(tag);
        if !statics.is_empty() {
            md.push_str("\n**Static attributes**\n\n");
            for prop in &statics {
                md.push_str(&format!("- `{}`\n", prop));
            }
        }
        if !binds.is_empty() {
            md.push_str("\n**Bind attributes** (`{expr}`)\n\n");
            for prop in &binds {
                md.push_str(&format!("- `{{{}}}`\n", prop));
            }
        }
        if !events.is_empty() {
            md.push_str("\n**Event attributes**\n\n");
            for prop in &events {
                md.push_str(&format!("- `{}`\n", prop));
            }
        }
    } else {
        md.push_str(&format!("## `<{}>`\n\n", tag));
        md.push_str("Unknown tag.\n");
    }

    md.trim_end().to_string()
}

// ──────────────────────────────────────────────────────────────────────────
// 属性名悬停
// ──────────────────────────────────────────────────────────────────────────

/// 生成属性名的悬停文档（Markdown）
///
/// 显示属性名、类别（static/bind/event）、所属标签，以及是否在 props_registry 中登记。
fn format_attribute_name_hover(elem: &Element, attr: &Attribute) -> String {
    let tag = &elem.tag;
    let (name, kind_label) = match attr {
        Attribute::Static { name, .. } => (name.as_str(), "static"),
        Attribute::Bind { name, .. } => (name.as_str(), "bind"),
        Attribute::Event { name, .. } => (name.as_str(), "event"),
    };

    let mut md = String::new();
    md.push_str(&format!("### `{}` ({})\n\n", name, kind_label));
    md.push_str(&format!("Applicable tag: `<{}>`\n\n", tag));

    // 类型说明
    match attr {
        Attribute::Static { .. } => {
            md.push_str("Type: `string` literal (`\"...\"` or `'...'`).\n\n");
        }
        Attribute::Bind { .. } => {
            md.push_str("Type: bind expression (`{expr}`).\n\n");
            md.push_str("The expression is evaluated against the component model and updated reactively.\n");
        }
        Attribute::Event { .. } => {
            md.push_str("Type: event handler (`{fn}` or `\"method\"`).\n\n");
            md.push_str("The handler is invoked when the event fires.\n");
        }
    }

    // 是否登记
    if props_registry::is_prop_registered(tag, name) {
        md.push_str("\nRegistered in `props_registry`.\n");
    } else {
        md.push_str("\nNot registered in `props_registry` (may be a custom or unknown attribute).\n");
    }

    md.trim_end().to_string()
}

// ──────────────────────────────────────────────────────────────────────────
// 属性值悬停
// ──────────────────────────────────────────────────────────────────────────

/// 生成属性值的悬停文档（Markdown）
///
/// 显示值内容、类别、所属属性名。
fn format_attribute_value_hover(elem: &Element, attr: &Attribute, source: &str) -> String {
    let tag = &elem.tag;
    let (name, kind_label, value_desc) = match attr {
        Attribute::Static { name, value, .. } => {
            (name.as_str(), "static string", format!("`\"{}\"`", value))
        }
        Attribute::Bind { name, .. } => {
            let expr_text = attr_bind_expr_span(attr, source)
                .and_then(|s| source.get(s.start..s.end))
                .unwrap_or("");
            (name.as_str(), "bind expression", format!("`{{{}}}`", expr_text))
        }
        Attribute::Event { name, handler, .. } => {
            let handler_name = event_handler_name(handler);
            (name.as_str(), "event handler", format!("`{}`", handler_name))
        }
    };

    let mut md = String::new();
    md.push_str(&format!("### Value of `{}`\n\n", name));
    md.push_str(&format!("- Tag: `<{}>`\n", tag));
    md.push_str(&format!("- Kind: {}\n", kind_label));
    md.push_str(&format!("- Value: {}\n", value_desc));
    md.trim_end().to_string()
}

// ──────────────────────────────────────────────────────────────────────────
// 属性整体悬停（兜底，如光标在 `=` 上）
// ──────────────────────────────────────────────────────────────────────────

/// 生成属性整体的悬停文档（Markdown）
fn format_attribute_hover(elem: &Element, attr: &Attribute) -> String {
    let tag = &elem.tag;
    let (name, kind_label) = match attr {
        Attribute::Static { name, .. } => (name.as_str(), "static"),
        Attribute::Bind { name, .. } => (name.as_str(), "bind"),
        Attribute::Event { name, .. } => (name.as_str(), "event"),
    };
    let mut md = String::new();
    md.push_str(&format!("### `{}` ({})\n\n", name, kind_label));
    md.push_str(&format!("Attribute of `<{}>`.\n", tag));
    md.trim_end().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_first_elem(src: &str) -> Element {
        match rust_rml_engine::parser::parse(src) {
            Ok(rust_rml_engine::parser::ast::Node::Element(e)) => e,
            other => panic!("expected element, got {:?}", other),
        }
    }

    #[test]
    fn tag_hover_for_html_element() {
        let elem = parse_first_elem(r#"<div class="card"></div>"#);
        let md = format_tag_hover(&elem);
        assert!(md.contains("HTML element"));
        assert!(md.contains("<div>"));
    }

    #[test]
    fn tag_hover_for_unknown() {
        let elem = parse_first_elem(r#"<UnknownTag></UnknownTag>"#);
        let md = format_tag_hover(&elem);
        assert!(md.contains("Unknown tag"));
    }

    #[test]
    fn attr_name_hover_static() {
        let src = r#"<div class="card"></div>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_name_hover(&elem, attr);
        assert!(md.contains("`class`"));
        assert!(md.contains("(static)"));
        assert!(md.contains("<div>"));
    }

    #[test]
    fn attr_name_hover_bind() {
        let src = r#"<Input value={field} />"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_name_hover(&elem, attr);
        assert!(md.contains("`value`"));
        assert!(md.contains("(bind)"));
        assert!(md.contains("bind expression"));
    }

    #[test]
    fn attr_name_hover_event() {
        let src = r#"<button onclick={handle_click}></button>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_name_hover(&elem, attr);
        assert!(md.contains("`onclick`"));
        assert!(md.contains("(event)"));
        assert!(md.contains("event handler"));
    }

    #[test]
    fn attr_value_hover_static() {
        let src = r#"<div class="card"></div>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_value_hover(&elem, attr, src);
        assert!(md.contains("Value of `class`"));
        assert!(md.contains("`\"card\"`"));
    }

    #[test]
    fn attr_value_hover_bind() {
        let src = r#"<Input value={field} />"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_value_hover(&elem, attr, src);
        assert!(md.contains("Value of `value`"));
        assert!(md.contains("`{field}`"));
    }

    #[test]
    fn attr_value_hover_event_handler() {
        let src = r#"<button onclick={handle_click}></button>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_value_hover(&elem, attr, src);
        assert!(md.contains("Value of `onclick`"));
        assert!(md.contains("`handle_click`"));
    }
}
