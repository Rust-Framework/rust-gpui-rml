//! Translator 公共辅助函数
//!
//! 供各 translator 在 to_rust / to_rml 过程中复用。

use crate::parser::ast::{Attribute, Directive, Element};

/// 将任意元素序列化为 RML 源码。
///
/// 输出当前缩进、标签名、属性、指令、子节点，并在无子节点且 `ctx.self_closing`
/// 为 true 时使用自闭合语法。供用户组件、扩展组件、菜单容器等 translator 复用。
pub fn print_element(
    elem: &Element,
    ctx: &super::PrinterCtx,
) -> Result<String, super::PrintError> {
    use crate::parser::ast::{Node, TextSegment};

    let mut out = String::new();
    out.push_str(&ctx.indent_str());
    out.push('<');
    out.push_str(&elem.tag);

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                out.push_str(&format!(" {}=\"{}\"", name, escape_attr_value(value)));
            }
            Attribute::Bind { name, expr, .. } => {
                out.push_str(&format!(" {}={{{}}}", name, expr));
            }
            Attribute::Event { name, .. } => {
                out.push_str(&format!(" {}=\"...\"", name));
            }
        }
    }

    for d in &elem.directives {
        match d {
            Directive::If { expr, .. } => out.push_str(&format!(" if={{{}}}", expr)),
            Directive::ElseIf { expr, .. } => out.push_str(&format!(" else-if={{{}}}", expr)),
            Directive::Each { clause, .. } => {
                if let Some(idx) = &clause.index {
                    out.push_str(&format!(
                        " each={{{}, {} in {}}}",
                        clause.item, idx, clause.iterable
                    ));
                } else {
                    out.push_str(&format!(" each={{{} in {}}}", clause.item, clause.iterable));
                }
            }
            Directive::Show { expr, .. } => out.push_str(&format!(" show={{{}}}", expr)),
            Directive::Once { .. } => out.push_str(" once"),
            Directive::Html { expr, .. } => out.push_str(&format!(" html={{{}}}", expr)),
            Directive::Ref { name, .. } => out.push_str(&format!(" ref=\"{}\"", name)),
            Directive::Key { expr, .. } => out.push_str(&format!(" key={{{}}}", expr)),
            Directive::Else { .. } => {}
        }
    }

    if elem.children.is_empty() && ctx.self_closing {
        out.push_str(" />");
        return Ok(out);
    }

    out.push('>');
    let child_ctx = ctx.indent();
    for child in &elem.children {
        match child {
            Node::Text(text) => {
                out.push_str(&child_ctx.newline_indent());
                out.push_str(text);
            }
            Node::Element(child_elem) => {
                out.push_str(&child_ctx.newline_indent());
                if let Some(translator) = ctx.registry.resolve(child_elem) {
                    out.push_str(&translator.to_rml(child_elem, &child_ctx)?);
                } else {
                    out.push_str(&format!("<!-- unknown tag: {} -->", child_elem.tag));
                }
            }
            Node::Interpolation { expr, .. } => {
                out.push_str(&child_ctx.newline_indent());
                out.push_str(&format!("{{{}}}", expr));
            }
            Node::MixedText(segs) => {
                out.push_str(&child_ctx.newline_indent());
                for seg in segs {
                    match seg {
                        TextSegment::Literal(s) => out.push_str(s),
                        TextSegment::Interpolation { expr, .. } => {
                            out.push_str(&format!("{{{}}}", expr))
                        }
                    }
                }
            }
        }
    }
    out.push_str(&ctx.newline_indent());
    out.push_str("</");
    out.push_str(&elem.tag);
    out.push('>');
    Ok(out)
}

/// 判断元素是否声明了指定静态属性
pub fn has_static_attr(elem: &Element, name: &str) -> bool {
    elem.attributes.iter().any(|attr| match attr {
        Attribute::Static { name: n, .. } => n == name,
        _ => false,
    })
}

/// 读取静态属性值
pub fn static_attr_value<'a>(elem: &'a Element, name: &'a str) -> Option<&'a str> {
    elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name: n, value, .. } if n == name => Some(value.as_str()),
        _ => None,
    })
}

/// 读取 ref 指令指定的名称
pub fn ref_name(elem: &Element) -> Option<&str> {
    elem.directives.iter().find_map(|d| match d {
        Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    })
}

/// 读取 key 指令表达式
pub fn key_expr(elem: &Element) -> Option<&str> {
    elem.directives.iter().find_map(|d| match d {
        Directive::Key { expr, .. } => Some(expr.as_str()),
        _ => None,
    })
}

/// 判断元素是否有事件属性
pub fn has_event_attr(elem: &Element) -> bool {
    elem.attributes.iter().any(|attr| matches!(attr, Attribute::Event { .. }))
}

/// 解析 RML 布尔属性值
pub fn parse_bool(value: &str) -> &'static str {
    if value.eq_ignore_ascii_case("true") || value == "1" {
        "true"
    } else {
        "false"
    }
}

/// 生成 ElementId：ref 优先，其次 key，最后自增计数器
pub fn element_id(elem: &Element, id_counter: &mut usize) -> String {
    if let Some(name) = ref_name(elem) {
        return format!("{:?}", format!("rml_ref:{}", name));
    }
    if let Some(key) = key_expr(elem) {
        return format!(
            "(\"rml_key\", rml_core::element_id::from_key(&{}))",
            key
        );
    }
    let id = *id_counter;
    *id_counter += 1;
    format!("(\"rml_el\", {}usize)", id)
}

/// 判断元素是否为空（无子节点且无文本内容）
pub fn is_empty_element(elem: &Element) -> bool {
    elem.children.is_empty()
}

/// 转义 RML 属性值中的引号
pub fn escape_attr_value(value: &str) -> String {
    value.replace('"', "&quot;")
}
