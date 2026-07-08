//! AST → RML 源码 printer
//!
//! 将解析后的 `ast::Node` 还原为 `.rml` 文件文本，供可视化设计器写回源码。
//! 每个元素节点的序列化委托给对应 `IRmlTranslator::to_rml`。

use crate::compiler::translator::{PrintError, PrinterCtx, TranslatorRegistry};
use crate::parser::ast::{Node, TextSegment};

/// 将 AST 根节点打印为 RML 源码
///
/// # 参数
/// - `node`: AST 根节点
/// - `registry`: translator 注册表，用于查询各标签的 `to_rml` 实现
///
/// # 返回
/// 格式化后的 `.rml` 源码字符串。
pub fn print(node: &Node, registry: &TranslatorRegistry) -> Result<String, PrintError> {
    let ctx = PrinterCtx::with_registry(registry.clone());
    print_node(node, registry, &ctx)
}

fn print_node(
    node: &Node,
    registry: &TranslatorRegistry,
    ctx: &PrinterCtx,
) -> Result<String, PrintError> {
    match node {
        Node::Element(elem) => {
            if let Some(translator) = registry.resolve(elem) {
                translator.to_rml(elem, ctx)
            } else {
                // 未找到 translator：打印占位注释，避免丢失节点
                Ok(format!(
                    "{}<!-- unknown tag: {} -->",
                    ctx.indent_str(),
                    elem.tag
                ))
            }
        }
        Node::Text(text) => Ok(text.clone()),
        Node::Interpolation { expr, .. } => Ok(format!("{{{}}}", expr)),
        Node::MixedText(segments) => {
            let mut out = String::new();
            for seg in segments {
                match seg {
                    TextSegment::Literal(s) => out.push_str(s),
                    TextSegment::Interpolation { expr, .. } => out.push_str(&format!("{{{}}}", expr)),
                }
            }
            Ok(out)
        }
    }
}

/// 打印子节点列表
///
/// 供 translator 在 `to_rml` 中递归打印子元素时使用。
pub fn print_children(
    children: &[Node],
    registry: &TranslatorRegistry,
    ctx: &PrinterCtx,
) -> Result<String, PrintError> {
    let mut out = String::new();
    for child in children {
        out.push_str(&print_node(child, registry, ctx)?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::translator::builtin::div::DivTranslator;
    use crate::compiler::translator::TranslatorRegistry;
    use crate::parser;

    fn roundtrip(source: &str) -> String {
        let registry = {
            let mut reg = TranslatorRegistry::empty();
            reg.register(DivTranslator);
            reg
        };
        let node = parser::parse(source).unwrap();
        print(&node, &registry).unwrap()
    }

    #[test]
    fn prints_empty_div() {
        let source = "<div />";
        let output = roundtrip(source);
        assert_eq!(output, "<div />");
    }

    #[test]
    fn prints_div_with_text() {
        let source = "<div>hello</div>";
        let output = roundtrip(source);
        assert!(output.contains("<div>"));
        assert!(output.contains("hello"));
        assert!(output.contains("</div>"));
    }

    #[test]
    fn prints_div_with_static_attr() {
        let source = "<div class=\"card\" />";
        let output = roundtrip(source);
        assert!(output.contains(r#"class="card""#));
    }
}
