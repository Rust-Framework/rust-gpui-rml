//! 悬停功能：标签/属性名/属性值/插值四级细粒度文档
//!
//! 检测优先级（光标从细到粗）：
//! 1. 属性值 span → `format_attribute_value_hover`(含 i18n/CSS 检测)
//! 2. 属性名 span → `format_attribute_name_hover`(含组件文档)
//! 3. 属性整体 span（兜底，如落在 `=` 上）→ `format_attribute_hover`
//! 4. 标签名 span → `format_tag_hover`(反查 ra_ap_ide 源码文档)
//! 5. 插值 `{t("key")}` → `check_interpolation_hover`(i18n 翻译)
//! 6. 其它 → None
//!
//! 所有内容使用 `MarkupContent` Markdown，遵循 LSP 规范。

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use rust_rml_engine::compiler::props_registry;
use rust_rml_engine::parser::ast::{Attribute, Element, Node};
use rust_rml_engine::tags;

use crate::features::ast_util::{
    attr_bind_expr_span, attr_name_span, attr_span, attr_value_span, event_handler_name,
    find_attribute_at_offset, find_element_at_offset, tag_name_span,
};
use crate::rust::RustSemanticQuery;
use crate::server::conv;
use crate::workspace::{CssIndex, CssClassEntry, I18nEntry, I18nIndex, Workspace};

/// 执行悬停查询
pub fn hover(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
    workspace: &Workspace,
    rust_query: &dyn RustSemanticQuery,
    i18n_index: &I18nIndex,
    css_index: &CssIndex,
) -> Option<Hover> {
    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = tree.root.as_ref()?;

    // 先检测插值 `{t("key")}`(可能在元素子节点中,但不属于属性/标签名)
    if let Some(h) = check_interpolation_hover(root, byte_offset, i18n_index, source, line_starts)
    {
        return Some(h);
    }

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
                    format_attribute_value_hover(elem, attr, source, i18n_index, css_index),
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
                    format_attribute_name_hover(elem, attr, rust_query),
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
            format_tag_hover(elem, rust_query),
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
// 插值悬停(i18n `t("key")` 检测)
// ──────────────────────────────────────────────────────────────────────────

/// 检测光标是否落在 `Node::Interpolation` 内,若是且表达式为 `t("key")` 则返回 i18n hover
fn check_interpolation_hover(
    root: &Node,
    offset: usize,
    i18n_index: &I18nIndex,
    source: &str,
    line_starts: &[u32],
) -> Option<Hover> {
    let (expr, span) = find_interpolation_at_offset(root, offset)?;
    let key = extract_i18n_key(&expr)?;
    let entries = i18n_index.lookup(&key)?;
    let md = format_i18n_hover(&key, entries);
    Some(make_hover(span, md, source, line_starts))
}

/// 递归查找 offset 落入 span 的 `Node::Interpolation`,返回 (expr, span)
fn find_interpolation_at_offset(node: &Node, offset: usize) -> Option<(String, rust_rml_engine::parser::Span)> {
    match node {
        Node::Element(e) => {
            if !e.span.contains(offset) {
                return None;
            }
            for child in &e.children {
                if let Some(found) = find_interpolation_at_offset(child, offset) {
                    return Some(found);
                }
            }
            None
        }
        Node::Interpolation { expr, span } if span.contains(offset) => {
            Some((expr.clone(), *span))
        }
        _ => None,
    }
}

/// 从表达式文本中提取 `t("key")` 的 key
///
/// 支持: `t("key")`, `t('key')`, `t("key", args)`, `t("key",)`
fn extract_i18n_key(expr: &str) -> Option<String> {
    let trimmed = expr.trim();
    let rest = trimmed.strip_prefix("t(")?;
    let rest = rest.trim_start();
    let (quote, inner) = if let Some(r) = rest.strip_prefix('"') {
        ('"', r)
    } else if let Some(r) = rest.strip_prefix('\'') {
        ('\'', r)
    } else {
        return None;
    };
    let end = inner.find(quote)?;
    Some(inner[..end].to_string())
}

/// 渲染 i18n hover Markdown
fn format_i18n_hover(key: &str, entries: &[I18nEntry]) -> String {
    let mut md = String::new();
    md.push_str(&format!("### i18n: `{}`\n\n", key));
    for e in entries {
        md.push_str(&format!("- **{}**: {}\n", e.locale, e.value));
    }
    if let Some(first) = entries.first() {
        md.push_str(&format!(
            "\n*Defined in {}*\n",
            first.file_uri.path()
        ));
    }
    md.trim_end().to_string()
}

// ──────────────────────────────────────────────────────────────────────────
// 标签悬停
// ──────────────────────────────────────────────────────────────────────────

/// 生成标签的悬停文档（Markdown）
///
/// 对组件标签,额外反查 ra_ap_ide 获取 Rust 源码文档注释(`//!`/`///`)。
fn format_tag_hover(elem: &Element, rust_query: &dyn RustSemanticQuery) -> String {
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

        // 反查 ra_ap_ide 获取源码文档
        if let Some(doc) = lookup_component_doc(tag, rust_query) {
            md.push_str("\n---\n\n");
            md.push_str(&doc);
        }

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

/// 反查 ra_ap_ide 获取组件 struct 的源码文档
///
/// 通过 `tags::component_lookup(tag)` 获取 `ctor_path`(如 `rml_ui::Button`),
/// 提取 struct 名后调用 `find_struct` + `hover` 获取 Markdown 文档。
fn lookup_component_doc(tag: &str, rust_query: &dyn RustSemanticQuery) -> Option<String> {
    let tag_info = tags::component_lookup(tag)?;
    let struct_name = tag_info.ctor_path.rsplit("::").next()?;
    let loc = rust_query.find_struct(struct_name)?;
    let info = rust_query.hover(&loc.uri, loc.range.start)?;
    if info.content.is_empty() {
        None
    } else {
        Some(info.content)
    }
}

// ──────────────────────────────────────────────────────────────────────────
// 属性名悬停
// ──────────────────────────────────────────────────────────────────────────

/// 生成属性名的悬停文档（Markdown）
///
/// 显示属性名、类别（static/bind/event）、所属标签,以及是否在 props_registry 中登记。
/// 对组件标签的属性,附加组件源码文档。
fn format_attribute_name_hover(
    elem: &Element,
    attr: &Attribute,
    rust_query: &dyn RustSemanticQuery,
) -> String {
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

    // 附加组件源码文档
    if tags::component_lookup(tag).is_some() {
        if let Some(doc) = lookup_component_doc(tag, rust_query) {
            md.push_str("\n\n---\n\n");
            md.push_str(&doc);
        }
    }

    md.trim_end().to_string()
}

// ──────────────────────────────────────────────────────────────────────────
// 属性值悬停
// ──────────────────────────────────────────────────────────────────────────

/// 生成属性值的悬停文档（Markdown）
///
/// 显示值内容、类别、所属属性名。
/// 对 `class="xxx"` 额外显示 CSS 声明,对 `{t("key")}` 绑定额外显示 i18n 翻译。
fn format_attribute_value_hover(
    elem: &Element,
    attr: &Attribute,
    source: &str,
    i18n_index: &I18nIndex,
    css_index: &CssIndex,
) -> String {
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

    // i18n 检测:bind 表达式中的 t("key")
    if let Attribute::Bind { .. } = attr {
        let expr_text = attr_bind_expr_span(attr, source)
            .and_then(|s| source.get(s.start..s.end))
            .unwrap_or("");
        if let Some(key) = extract_i18n_key(expr_text) {
            if let Some(entries) = i18n_index.lookup(&key) {
                md.push_str("\n\n---\n\n");
                md.push_str(&format_i18n_hover(&key, entries));
            }
        }
    }

    // CSS class 检测:Static class 属性
    if let Attribute::Static { name, value, .. } = attr {
        if name == "class" {
            let classes: Vec<&str> = value.split_whitespace().collect();
            let mut css_sections = Vec::new();
            for class in classes {
                if let Some(entries) = css_index.lookup(class) {
                    css_sections.push(format_css_class_hover(class, entries));
                }
            }
            if !css_sections.is_empty() {
                md.push_str("\n\n---\n\n");
                md.push_str(&css_sections.join("\n\n---\n\n"));
            }
        }
    }

    md.trim_end().to_string()
}

/// 渲染 CSS class hover Markdown
fn format_css_class_hover(class: &str, entries: &[CssClassEntry]) -> String {
    let mut md = String::new();
    md.push_str(&format!("### CSS: `.{}`\n\n", class));
    for entry in entries {
        md.push_str(&format!("**{}**\n\n", entry.file_uri.path()));
        for (prop, val) in &entry.declarations {
            md.push_str(&format!("- `{}`: `{}`\n", prop, val));
        }
    }
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

// ──────────────────────────────────────────────────────────────────────────
// 测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rust::{
        CompletionEntry, ComponentInfo, HoverInfo, RustDiagnostic, SymbolInfo, SymbolLocation,
        RustSemanticQuery,
    };
    use lsp_types::{DocumentSymbol, FoldingRange, Position, Url};
    use std::collections::HashMap;

    /// 测试用空查询:所有 ra_ap_ide 反查返回 None
    struct TestQuery;

    impl RustSemanticQuery for TestQuery {
        fn open_document(&mut self, _uri: &Url, _text: &str) {}
        fn apply_change(&mut self, _uri: &Url, _text: &str) {}
        fn close_document(&mut self, _uri: &Url) {}
        fn goto_definition(&self, _uri: &Url, _pos: Position) -> Vec<SymbolLocation> {
            Vec::new()
        }
        fn hover(&self, _uri: &Url, _pos: Position) -> Option<HoverInfo> {
            None
        }
        fn completion(&self, _uri: &Url, _pos: Position) -> Vec<CompletionEntry> {
            Vec::new()
        }
        fn diagnostics(&self, _uri: &Url) -> Vec<RustDiagnostic> {
            Vec::new()
        }
        fn resolve_member(
            &self,
            _rml_rs_uri: &Url,
            _struct_name: &str,
            _member: &str,
        ) -> Option<SymbolInfo> {
            None
        }
        fn find_struct(&self, _struct_name: &str) -> Option<SymbolLocation> {
            None
        }
        fn struct_slots(&self, _rml_rs_uri: &Url, _struct_name: &str) -> Vec<String> {
            Vec::new()
        }
        fn command_signature(
            &self,
            _rml_rs_uri: &Url,
            _struct_name: &str,
            _method: &str,
        ) -> Option<SymbolInfo> {
            None
        }
        fn list_components(&self, _prefix: &str) -> Vec<ComponentInfo> {
            Vec::new()
        }
        fn is_ready(&self) -> bool {
            false
        }
        fn find_references(
            &self,
            _uri: &Url,
            _pos: Position,
            _include_declaration: bool,
        ) -> Vec<SymbolLocation> {
            Vec::new()
        }
        fn rename_member(
            &self,
            _rml_rs_uri: &Url,
            _struct_name: &str,
            _member: &str,
            _new_name: &str,
        ) -> Vec<lsp_types::TextEdit> {
            Vec::new()
        }
        fn rename_struct(
            &self,
            _old_name: &str,
            _new_name: &str,
        ) -> HashMap<Url, Vec<lsp_types::TextEdit>> {
            HashMap::new()
        }
        fn document_symbol(&self, _uri: &Url) -> Option<Vec<DocumentSymbol>> {
            None
        }
        fn folding_ranges(&self, _uri: &Url) -> Vec<FoldingRange> {
            Vec::new()
        }
    }

    fn parse_first_elem(src: &str) -> Element {
        match rust_rml_engine::parser::parse(src) {
            Ok(rust_rml_engine::parser::ast::Node::Element(e)) => e,
            other => panic!("expected element, got {:?}", other),
        }
    }

    fn parse_first_node(src: &str) -> Node {
        rust_rml_engine::parser::parse(src).expect("parse failed")
    }

    fn test_query() -> TestQuery {
        TestQuery
    }

    fn empty_i18n() -> I18nIndex {
        I18nIndex::new()
    }

    fn empty_css() -> CssIndex {
        CssIndex::new()
    }

    #[test]
    fn tag_hover_for_html_element() {
        let elem = parse_first_elem(r#"<div class="card"></div>"#);
        let md = format_tag_hover(&elem, &test_query());
        assert!(md.contains("HTML element"));
        assert!(md.contains("<div>"));
    }

    #[test]
    fn tag_hover_for_unknown() {
        let elem = parse_first_elem(r#"<UnknownTag></UnknownTag>"#);
        let md = format_tag_hover(&elem, &test_query());
        assert!(md.contains("Unknown tag"));
    }

    #[test]
    fn tag_hover_for_component_without_rust_doc() {
        // TestQuery 返回 None,应降级为硬编码文档
        let elem = parse_first_elem(r#"<Button></Button>"#);
        let md = format_tag_hover(&elem, &test_query());
        assert!(md.contains("Component"));
        assert!(md.contains("gpui-component extension."));
    }

    #[test]
    fn attr_name_hover_static() {
        let src = r#"<div class="card"></div>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_name_hover(&elem, attr, &test_query());
        assert!(md.contains("`class`"));
        assert!(md.contains("(static)"));
        assert!(md.contains("<div>"));
    }

    #[test]
    fn attr_name_hover_bind() {
        let src = r#"<Input value={field} />"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_name_hover(&elem, attr, &test_query());
        assert!(md.contains("`value`"));
        assert!(md.contains("(bind)"));
        assert!(md.contains("bind expression"));
    }

    #[test]
    fn attr_name_hover_event() {
        let src = r#"<button onclick={handle_click}></button>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_name_hover(&elem, attr, &test_query());
        assert!(md.contains("`onclick`"));
        assert!(md.contains("(event)"));
        assert!(md.contains("event handler"));
    }

    #[test]
    fn attr_value_hover_static() {
        let src = r#"<div class="card"></div>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_value_hover(&elem, attr, src, &empty_i18n(), &empty_css());
        assert!(md.contains("Value of `class`"));
        assert!(md.contains("`\"card\"`"));
    }

    #[test]
    fn attr_value_hover_bind() {
        let src = r#"<Input value={field} />"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_value_hover(&elem, attr, src, &empty_i18n(), &empty_css());
        assert!(md.contains("Value of `value`"));
        assert!(md.contains("`{field}`"));
    }

    #[test]
    fn attr_value_hover_event_handler() {
        let src = r#"<button onclick={handle_click}></button>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();
        let md = format_attribute_value_hover(&elem, attr, src, &empty_i18n(), &empty_css());
        assert!(md.contains("Value of `onclick`"));
        assert!(md.contains("`handle_click`"));
    }

    // ── i18n hover 测试 ──

    #[test]
    fn extract_i18n_key_from_double_quote() {
        let key = extract_i18n_key(r#"t("login.title")"#);
        assert_eq!(key, Some("login.title".to_string()));
    }

    #[test]
    fn extract_i18n_key_from_single_quote() {
        let key = extract_i18n_key(r#"t('login.title')"#);
        assert_eq!(key, Some("login.title".to_string()));
    }

    #[test]
    fn extract_i18n_key_with_args() {
        let key = extract_i18n_key(r#"t("login.title", count)"#);
        assert_eq!(key, Some("login.title".to_string()));
    }

    #[test]
    fn extract_i18n_key_not_t_call() {
        assert_eq!(extract_i18n_key("field"), None);
        assert_eq!(extract_i18n_key("format(...)"), None);
    }

    #[test]
    fn i18n_hover_in_interpolation() {
        // 构建 i18n 索引
        let tmp = std::env::temp_dir().join("rml_hover_test_i18n_interp");
        let _ = std::fs::remove_dir_all(&tmp);
        let i18n_dir = tmp.join("i18n");
        std::fs::create_dir_all(&i18n_dir).unwrap();
        std::fs::write(
            i18n_dir.join("zh-CN.json"),
            r#"{"login.title": "登录"}"#,
        )
        .unwrap();

        let mut idx = I18nIndex::new();
        idx.scan(&tmp);

        // 解析 RML: `<h1>{t("login.title")}</h1>`
        let src = r#"<h1>{t("login.title")}</h1>"#;
        let node = parse_first_node(src);
        // 光标落在 "login.title" 内(offset ≈ 15)
        let offset = src.find("login").unwrap();
        let result = find_interpolation_at_offset(&node, offset);
        assert!(result.is_some());
        let (expr, _span) = result.unwrap();
        assert_eq!(expr, r#"t("login.title")"#);

        let key = extract_i18n_key(&expr).unwrap();
        assert_eq!(key, "login.title");

        let entries = idx.lookup(&key).unwrap();
        let md = format_i18n_hover(&key, entries);
        assert!(md.contains("login.title"));
        assert!(md.contains("zh-CN"));
        assert!(md.contains("登录"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn i18n_hover_in_bind_attribute() {
        let src = r#"<Button label={t("login.submit")} />"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();

        let tmp = std::env::temp_dir().join("rml_hover_test_i18n_bind");
        let _ = std::fs::remove_dir_all(&tmp);
        let i18n_dir = tmp.join("i18n");
        std::fs::create_dir_all(&i18n_dir).unwrap();
        std::fs::write(
            i18n_dir.join("zh-CN.json"),
            r#"{"login.submit": "进入"}"#,
        )
        .unwrap();

        let mut idx = I18nIndex::new();
        idx.scan(&tmp);

        let md = format_attribute_value_hover(&elem, attr, src, &idx, &empty_css());
        assert!(md.contains("i18n"));
        assert!(md.contains("login.submit"));
        assert!(md.contains("zh-CN"));
        assert!(md.contains("进入"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // ── CSS hover 测试 ──

    #[test]
    fn css_hover_for_class_attribute() {
        let src = r#"<div class="case-pane"></div>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();

        let tmp = std::env::temp_dir().join("rml_hover_test_css");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("test.css"),
            ".case-pane {\n  display: flex;\n  padding: 24px;\n}\n",
        )
        .unwrap();

        let mut idx = CssIndex::new();
        idx.scan(&tmp);

        let md = format_attribute_value_hover(&elem, attr, src, &empty_i18n(), &idx);
        assert!(md.contains("CSS"));
        assert!(md.contains(".case-pane"));
        assert!(md.contains("display"));
        assert!(md.contains("flex"));
        assert!(md.contains("padding"));
        assert!(md.contains("24px"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn css_hover_for_multiple_classes() {
        let src = r#"<div class="case-pane doc-pane"></div>"#;
        let elem = parse_first_elem(src);
        let attr = elem.attributes.first().unwrap();

        let tmp = std::env::temp_dir().join("rml_hover_test_css_multi");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(
            tmp.join("test.css"),
            ".case-pane {\n  display: flex;\n}\n.doc-pane {\n  padding: 10px;\n}\n",
        )
        .unwrap();

        let mut idx = CssIndex::new();
        idx.scan(&tmp);

        let md = format_attribute_value_hover(&elem, attr, src, &empty_i18n(), &idx);
        assert!(md.contains(".case-pane"));
        assert!(md.contains(".doc-pane"));
        assert!(md.contains("display"));
        assert!(md.contains("padding"));

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
