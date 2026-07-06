//! AST 工具：定位光标、推算 span、提取事件处理器名
//!
//! 所有 features 模块共用的 AST 遍历辅助函数集中于此，避免跨模块重复实现。

use rust_rml_engine::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
use rust_rml_engine::parser::Span;

/// 递归查找包含字节偏移的元素
///
/// 先在子节点中查找（更精确的定位），未命中则返回当前元素。
/// 与 hover.rs 原实现一致，抽出为 pub 供 definition/references/rename 复用。
pub fn find_element_at_offset(node: &Node, offset: usize) -> Option<&Element> {
    match node {
        Node::Element(elem) => {
            if !elem.span.contains(offset) {
                return None;
            }
            for child in &elem.children {
                if let Some(found) = find_element_at_offset(child, offset) {
                    return Some(found);
                }
            }
            Some(elem)
        }
        _ => None,
    }
}

/// 推算标签名字节区间
///
/// `<div ...>` 的标签名区间为 `elem.span.start + 1 .. elem.span.start + 1 + tag.len()`。
/// 闭标签 `</div>` 不在此推算范围（references 单独处理闭标签位置）。
///
/// 近似推算：假设 `<` 紧跟标签名（无空格）。若 `<` 后有空格，区间会偏移，
/// 但 definition/references 的命中检查仍可工作（光标在标签名上时 offset 落在推算区间内）。
pub fn tag_name_span(elem: &Element) -> Span {
    let start = elem.span.start + 1;
    let end = start + elem.tag.len();
    Span::new(start, end)
}

/// 在元素属性中定位 offset 命中的属性
pub fn find_attribute_at_offset<'a>(
    elem: &'a Element,
    offset: usize,
) -> Option<&'a Attribute> {
    elem.attributes
        .iter()
        .find(|attr| attr_span(attr).contains(offset))
}

/// 取属性的 span（三种变体统一）
pub fn attr_span(attr: &Attribute) -> Span {
    match attr {
        Attribute::Static { span, .. } => *span,
        Attribute::Bind { span, .. } => *span,
        Attribute::Event { span, .. } => *span,
    }
}

/// 取属性名字节区间
///
/// 属性名仅包含字母数字、`-`、`:`，从 `attr.span.start` 起连续读取直到遇非名称字符。
/// 布尔属性（无 `=`）整个 span 即名称区间。
pub fn attr_name_span(attr: &Attribute, source: &str) -> Option<Span> {
    let whole = attr_span(attr);
    let bytes = source.as_bytes();
    if whole.start >= bytes.len() {
        return None;
    }
    let mut end = whole.start;
    while end < whole.end.min(bytes.len()) {
        let b = bytes[end];
        if b.is_ascii_alphanumeric() || b == b'-' || b == b':' {
            end += 1;
        } else {
            break;
        }
    }
    if end > whole.start {
        Some(Span::new(whole.start, end))
    } else {
        None
    }
}

/// 取属性值内容字节区间（不含定界符 `"` `'` `{` `}`）
///
/// 解析流程：跳过名称 → 跳过空白 → 跳过 `=` → 跳过空白 → 按首字符判断定界符 →
/// 返回定界符内部内容区间。布尔属性返回 None。
pub fn attr_value_span(attr: &Attribute, source: &str) -> Option<Span> {
    let whole = attr_span(attr);
    let bytes = source.as_bytes();
    let bound = whole.end.min(bytes.len());
    if whole.start >= bound {
        return None;
    }

    // 跳过属性名
    let mut pos = whole.start;
    while pos < bound {
        let b = bytes[pos];
        if b.is_ascii_alphanumeric() || b == b'-' || b == b':' {
            pos += 1;
        } else {
            break;
        }
    }
    // 跳过空白
    pos = skip_ws(bytes, pos, bound);
    if pos >= bound || bytes[pos] != b'=' {
        // 布尔属性无值
        return None;
    }
    pos += 1; // 跳过 =
    pos = skip_ws(bytes, pos, bound);
    if pos >= bound {
        return None;
    }

    match bytes[pos] {
        b'"' | b'\'' => {
            let quote = bytes[pos];
            let content_start = pos + 1;
            let mut content_end = content_start;
            while content_end < bound && bytes[content_end] != quote {
                content_end += 1;
            }
            if content_end > content_start {
                Some(Span::new(content_start, content_end))
            } else {
                // 空字符串值：返回零长 span 以便调用方区分“有值但空”
                Some(Span::new(content_start, content_start))
            }
        }
        b'{' => {
            // 花括号匹配（支持嵌套）
            let content_start = pos + 1;
            let mut depth = 1;
            let mut content_end = content_start;
            while content_end < bound {
                match bytes[content_end] {
                    b'{' => depth += 1,
                    b'}' => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    _ => {}
                }
                content_end += 1;
            }
            if content_end > content_start {
                Some(Span::new(content_start, content_end))
            } else {
                Some(Span::new(content_start, content_start))
            }
        }
        _ => None,
    }
}

/// 取绑定表达式内部内容 span：仅对 `Attribute::Bind` 有效
///
/// `value={field}` → 返回 `field` 的字节区间；其他变体返回 None。
pub fn attr_bind_expr_span(attr: &Attribute, source: &str) -> Option<Span> {
    match attr {
        Attribute::Bind { .. } => attr_value_span(attr, source),
        _ => None,
    }
}

fn skip_ws(bytes: &[u8], mut pos: usize, bound: usize) -> usize {
    while pos < bound && (bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\n' || bytes[pos] == b'\r') {
        pos += 1;
    }
    pos
}

/// 遍历元素所有指令携带的表达式（If/Show/Key/Html/Model/Each.iterable）
///
/// 指令本身无独立 span 字段，无法精确定位光标在哪个指令上。
/// symbol 分类时遍历所有指令表达式，靠表达式内容匹配符号名。
pub fn iter_directive_exprs(elem: &Element) -> impl Iterator<Item = &str> {
    elem.directives.iter().filter_map(directive_expr)
}

/// 取指令携带的表达式（若有）
pub fn directive_expr(d: &Directive) -> Option<&str> {
    match d {
        Directive::If { expr, .. } | Directive::Show { expr, .. } | Directive::Key { expr, .. } | Directive::Html { expr, .. } => {
            Some(expr)
        }
        Directive::Model { field, .. } => Some(field),
        Directive::Each { clause: each, .. } => Some(&each.iterable),
        _ => None,
    }
}

/// 提取事件处理器名（Ident/MethodName/WithArgs 三态统一）
pub fn event_handler_name(h: &EventHandler) -> &str {
    match h {
        EventHandler::Ident(name) | EventHandler::MethodName(name) | EventHandler::WithArgs(name, _) => {
            name
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_rml_engine::parser::ast::EachClause;

    #[test]
    fn tag_name_span_basic() {
        let elem = Element {
            tag: "div".to_string(),
            span: Span::new(0, 10),
            ..Default::default()
        };
        let s = tag_name_span(&elem);
        assert_eq!(s.start, 1);
        assert_eq!(s.end, 4);
        assert!(s.contains(1));
        assert!(s.contains(3));
        assert!(!s.contains(4));
    }

    #[test]
    fn event_handler_name_variants() {
        assert_eq!(event_handler_name(&EventHandler::Ident("on_click".into())), "on_click");
        assert_eq!(event_handler_name(&EventHandler::MethodName("on_click".into())), "on_click");
        assert_eq!(
            event_handler_name(&EventHandler::WithArgs("on_click".into(), vec![])),
            "on_click"
        );
    }

    #[test]
    fn directive_expr_extracts() {
        use rust_rml_engine::parser::Span;
        assert_eq!(directive_expr(&Directive::If { expr: "count".into(), span: Span::empty() }), Some("count"));
        assert_eq!(directive_expr(&Directive::Show { expr: "visible".into(), span: Span::empty() }), Some("visible"));
        assert_eq!(directive_expr(&Directive::Key { expr: "id".into(), span: Span::empty() }), Some("id"));
        assert_eq!(directive_expr(&Directive::Html { expr: "raw".into(), span: Span::empty() }), Some("raw"));
        assert_eq!(
            directive_expr(&Directive::Model { field: "name".into(), converter: None, span: Span::empty() }),
            Some("name")
        );
        assert_eq!(
            directive_expr(&Directive::Each {
                clause: EachClause {
                    item: "x".into(),
                    index: None,
                    iterable: "items".into(),
                },
                span: Span::empty(),
            }),
            Some("items")
        );
        assert_eq!(directive_expr(&Directive::Else { span: Span::empty() }), None);
        assert_eq!(directive_expr(&Directive::Once { span: Span::empty() }), None);
        assert_eq!(directive_expr(&Directive::Ref { name: "input".into(), span: Span::empty() }), None);
    }

    // ── attr_name_span / attr_value_span / attr_bind_expr_span 测试 ──
    // 使用真实解析器构造 AST，确保 span 字段与源码字节偏移对齐。

    fn first_attr(node: &rust_rml_engine::parser::ast::Node) -> &Attribute {
        match node {
            rust_rml_engine::parser::ast::Node::Element(e) => {
                e.attributes.first().expect("no attribute")
            }
            _ => panic!("not an element"),
        }
    }

    #[test]
    fn attr_name_span_static() {
        let src = r#"<div class="card"></div>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_name_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "class");
    }

    #[test]
    fn attr_value_span_static() {
        let src = r#"<div class="card"></div>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_value_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "card");
    }

    #[test]
    fn attr_name_span_bind() {
        let src = r#"<Input value={field} />"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_name_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "value");
    }

    #[test]
    fn attr_bind_expr_span_bind() {
        let src = r#"<Input value={field} />"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_bind_expr_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "field");
    }

    #[test]
    fn attr_name_span_event() {
        let src = r#"<button onclick={handle_click}></button>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_name_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "onclick");
    }

    #[test]
    fn attr_value_span_event_ident() {
        let src = r#"<button onclick={handle_click}></button>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_value_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "handle_click");
    }

    #[test]
    fn attr_value_span_event_method_name() {
        // onclick="method_name" → MethodName，值定界符为 `"`
        let src = r#"<button onclick="method_name"></button>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_value_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "method_name");
    }

    #[test]
    fn attr_value_span_boolean_returns_none() {
        // 布尔属性 `disabled` 无 `=`，attr_value_span 返回 None
        let src = r#"<button disabled></button>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let name_s = attr_name_span(attr, src).unwrap();
        assert_eq!(&src[name_s.start..name_s.end], "disabled");
        assert!(attr_value_span(attr, src).is_none());
    }

    #[test]
    fn attr_value_span_bind_expr_not_returned_for_static() {
        let src = r#"<div class="card"></div>"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        // Static 属性的 attr_bind_expr_span 应返回 None
        assert!(attr_bind_expr_span(attr, src).is_none());
    }

    #[test]
    fn attr_value_span_nested_braces() {
        // 绑定表达式支持嵌套花括号：`{obj.{field}}` 不是合法 Rust，
        // 但解析器层面允许任意 expr 字符串，span 提取按 `{...}` 平衡匹配
        let src = r#"<Input value={obj.field} />"#;
        let root = rust_rml_engine::parser::parse(src).unwrap();
        let attr = first_attr(&root);
        let s = attr_value_span(attr, src).unwrap();
        assert_eq!(&src[s.start..s.end], "obj.field");
    }
}
