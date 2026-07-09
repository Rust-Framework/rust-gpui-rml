//! `<style source="..."/>` 指令扫描
//!
//! 在 build.rs 中调用，从 .rml 源码扫描所有页面级 CSS 引用，
//! 返回 CSS 文件路径列表供 build.rs 加载与合并。
//!
//! `<style>` 元素不参与渲染（由 codegen 过滤），仅作为编译期指令：
//! - `source` 属性指定 CSS 文件路径（相对于 .rml 文件所在目录）
//! - 不支持内联 CSS 内容，保持与现有 CSS 文件加载机制一致

use crate::parser;
use crate::parser::ast::{Attribute, Element, Node};

/// 扫描 .rml 源码中所有 `<style source="...">` 指令，返回 CSS 文件路径列表。
///
/// 递归遍历 AST 所有元素节点。`source` 属性必须是静态字符串（不支持 bind 形式）。
/// 路径相对于 .rml 文件所在目录（由调用方在 build.rs 中解析为绝对路径）。
///
/// # 错误
/// - 解析 .rml 失败时返回 `ParseError`
/// - `<style>` 缺少 `source` 属性或 `source` 为空时返回 `ParseError`
///
/// # 顺序
/// 多个 `<style>` 按源码出现顺序返回，build.rs 据此按序合并（后者优先级更高）。
pub fn scan_style_directives(source: &str) -> Result<Vec<String>, parser::ParseError> {
    let root = parser::parse(source)?;
    let mut paths = Vec::new();
    collect_style_sources(&root, &mut paths)?;
    Ok(paths)
}

/// 递归遍历 AST 节点，收集所有 `<style>` 元素的 `source` 属性值
fn collect_style_sources(
    node: &Node,
    paths: &mut Vec<String>,
) -> Result<(), parser::ParseError> {
    if let Node::Element(elem) = node {
        if elem.tag == "style" {
            let source = extract_source_attr(elem)?;
            if source.is_empty() {
                return Err(parser::ParseError {
                    message: "`<style>` 元素的 `source` 属性不能为空".into(),
                    line: 0,
                    column: 0,
                    source_snippet: None,
                });
            }
            paths.push(source);
            return Ok(());
        }
        for child in &elem.children {
            collect_style_sources(child, paths)?;
        }
    }
    Ok(())
}

/// 从 `<style>` 元素提取 `source` 静态属性值
///
/// 仅接受 `Attribute::Static`（`source="index.css"`），拒绝 `Attribute::Bind`（`source={expr}`）。
fn extract_source_attr(elem: &Element) -> Result<String, parser::ParseError> {
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "source" => {
                return Ok(value.clone());
            }
            Attribute::Bind { name, .. } if name == "source" => {
                return Err(parser::ParseError {
                    message: "`<style>` 的 `source` 属性必须是静态字符串（如 `source=\"index.css\"`），不支持绑定形式"
                        .into(),
                    line: 0,
                    column: 0,
                    source_snippet: None,
                });
            }
            _ => {}
        }
    }
    Err(parser::ParseError {
        message: "`<style>` 元素必须包含 `source` 属性（如 `<style source=\"index.css\" />`）".into(),
        line: 0,
        column: 0,
        source_snippet: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_style_directives_returns_empty() {
        let src = r#"<component><div>hello</div></component>"#;
        let paths = scan_style_directives(src).expect("should parse");
        assert!(paths.is_empty(), "expected no style directives, got {:?}", paths);
    }

    #[test]
    fn single_style_directive() {
        let src = r#"<component><style source="index.css" /><div>hello</div></component>"#;
        let paths = scan_style_directives(src).expect("should parse");
        assert_eq!(paths, vec!["index.css".to_string()]);
    }

    #[test]
    fn multiple_style_directives_preserve_order() {
        let src = r#"<component>
            <style source="a.css" />
            <style source="b.css" />
            <style source="c.css" />
        </component>"#;
        let paths = scan_style_directives(src).expect("should parse");
        assert_eq!(
            paths,
            vec!["a.css".to_string(), "b.css".to_string(), "c.css".to_string()]
        );
    }

    #[test]
    fn nested_style_in_deep_tree() {
        let src = r#"<component>
            <div>
                <div>
                    <style source="deep.css" />
                </div>
            </div>
        </component>"#;
        let paths = scan_style_directives(src).expect("should parse");
        assert_eq!(paths, vec!["deep.css".to_string()]);
    }

    #[test]
    fn style_at_window_root() {
        let src = r#"<window title="Test">
            <style source="window.css" />
            <div>content</div>
        </window>"#;
        let paths = scan_style_directives(src).expect("should parse");
        assert_eq!(paths, vec!["window.css".to_string()]);
    }

    #[test]
    fn missing_source_attribute_returns_error() {
        let src = r#"<component><style /></component>"#;
        let result = scan_style_directives(src);
        assert!(result.is_err(), "expected error for missing source attribute");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("source"),
            "error should mention source attribute, got: {}",
            err.message
        );
    }

    #[test]
    fn empty_source_value_returns_error() {
        let src = r#"<component><style source="" /></component>"#;
        let result = scan_style_directives(src);
        assert!(result.is_err(), "expected error for empty source value");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("不能为空"),
            "error should mention empty source, got: {}",
            err.message
        );
    }

    #[test]
    fn bind_form_source_returns_error() {
        let src = r#"<component><style source={css_path} /></component>"#;
        let result = scan_style_directives(src);
        assert!(result.is_err(), "expected error for bind form source");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("静态字符串"),
            "error should mention static string requirement, got: {}",
            err.message
        );
    }
}
