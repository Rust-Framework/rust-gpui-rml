//! ShortcutScope 子节点拆分与校验
//!
//! ## 唯一写法
//!
//! ```rml
//! <ShortcutScope>
//!   <Shortcut key="Ctrl+S" on-press={on_save} />
//!   <div>...</div>
//! </ShortcutScope>
//! ```
//!
//! `<Shortcut>` 为声明式元数据，不渲染；其余子节点为作用域内容。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

use super::setters::gen_shortcut_call;

/// 将子节点分为 Shortcut 声明（元数据）与其余内容节点
pub fn partition_shortcut_scope_children(children: &[Node]) -> (Vec<&Element>, Vec<&Node>) {
    let mut shortcuts = Vec::new();
    let mut content = Vec::new();
    for child in children {
        if let Node::Element(elem) = child {
            if tags::canonical_tag(&elem.tag) == "Shortcut" {
                shortcuts.push(elem);
                continue;
            }
        }
        content.push(child);
    }
    (shortcuts, content)
}

/// 校验 ShortcutScope 子节点
pub fn validate_shortcut_scope_children(
    children: &[Node],
    span: crate::parser::Span,
) -> Result<(), CodegenError> {
    for child in children {
        if let Node::Element(elem) = child {
            if tags::canonical_tag(&elem.tag) == "Shortcut" && !elem.children.is_empty() {
                return Err(CodegenError {
                    message: "<Shortcut> 作为 ShortcutScope 子节点时必须自闭合，勿嵌套子元素".into(),
                    span: Some(elem.span),
                });
            }
        }
    }
    if children.is_empty() {
        return Err(CodegenError {
            message: "<ShortcutScope> 至少需要一个内容子节点；快捷键请声明为 <Shortcut/> 元数据子节点"
                .into(),
            span: Some(span),
        });
    }
    let (shortcuts, content) = partition_shortcut_scope_children(children);
    if shortcuts.is_empty() {
        return Err(CodegenError {
            message: "<ShortcutScope> 至少声明一个 <Shortcut key=\"...\" on-press={...} />".into(),
            span: Some(span),
        });
    }
    if content.is_empty() {
        return Err(CodegenError {
            message: "<ShortcutScope> 除 <Shortcut> 外还需至少一个内容子节点".into(),
            span: Some(span),
        });
    }
    Ok(())
}

/// 生成 ShortcutScope 构造代码
pub fn gen_shortcut_scope(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    validate_shortcut_scope_children(&elem.children, elem.span)?;
    let (shortcuts, content) = partition_shortcut_scope_children(&elem.children);

    let mut code = "rml_ui::ShortcutScope::new()".to_string();

    append_css_class_styles(
        &mut code,
        elem,
        "ShortcutScope",
        ctx.stylesheet.as_ref(),
        parents,
    );

    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "ShortcutScope")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    "ShortcutScope",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = crate::compiler::setters::component_event_setter(
                    name,
                    handler,
                    "ShortcutScope",
                ) {
                    code.push_str(&s);
                }
            }
        }
    }

    for sc in shortcuts {
        code.push_str(&gen_shortcut_call(sc)?);
    }

    for child in content {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!("\n            .children({})", child_code));
        } else {
            code.push_str(&format!("\n            .child({})", child_code));
        }
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, EventHandler, Node};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn shortcut(key: &str, handler: &str) -> Element {
        Element {
            tag: "Shortcut".into(),
            attributes: vec![
                Attribute::Static {
                    name: "key".into(),
                    value: key.into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_press".into(),
                    handler: EventHandler::Ident(handler.into()),
                    span: Span::empty(),
                },
            ],
            directives: vec![],
            children: vec![],
            slot_name: None,
            span: Span::empty(),
        }
    }

    fn scope_with_shortcuts() -> Element {
        Element {
            tag: "ShortcutScope".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![
                Node::Element(shortcut("Ctrl+S", "on_save")),
                Node::Element(shortcut("Ctrl+O", "on_open")),
                Node::Text("content".into()),
            ],
            slot_name: None,
            span: Span::empty(),
        }
    }

    #[test]
    fn partition_splits_shortcuts_and_content() {
        let elem = scope_with_shortcuts();
        let (shortcuts, content) = partition_shortcut_scope_children(&elem.children);
        assert_eq!(shortcuts.len(), 2);
        assert_eq!(content.len(), 1);
    }

    #[test]
    fn gen_shortcut_scope_emits_shortcut_calls_and_child() {
        let elem = scope_with_shortcuts();
        let code = gen_shortcut_scope(&elem, &ctx(), &mut 1, &[], &[]).unwrap();
        assert!(code.contains("rml_ui::ShortcutScope::new()"));
        assert!(code.contains(".shortcut(\"Ctrl+S\", true,"));
        assert!(code.contains(".shortcut(\"Ctrl+O\", true,"));
        assert!(code.contains(".child("));
    }

    #[test]
    fn reject_scope_without_shortcuts() {
        let elem = Element {
            tag: "ShortcutScope".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("only content".into())],
            slot_name: None,
            span: Span::empty(),
        };
        let err = validate_shortcut_scope_children(&elem.children, elem.span).unwrap_err();
        assert!(err.message.contains("Shortcut"));
    }

    #[test]
    fn reject_shortcut_with_nested_children() {
        let elem = Element {
            tag: "ShortcutScope".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![
                Node::Element(Element {
                    tag: "Shortcut".into(),
                    attributes: vec![Attribute::Static {
                        name: "key".into(),
                        value: "Ctrl+S".into(),
                        span: Span::empty(),
                    }],
                    directives: vec![],
                    children: vec![Node::Text("bad".into())],
                    slot_name: None,
                    span: Span::empty(),
                }),
                Node::Text("content".into()),
            ],
            slot_name: None,
            span: Span::empty(),
        };
        let err = validate_shortcut_scope_children(&elem.children, elem.span).unwrap_err();
        assert!(err.message.contains("自闭合"));
    }

    #[test]
    fn reject_standalone_shortcut() {
        use crate::compiler::translator::component::shortcut_scope::ShortcutTranslator;
        use crate::compiler::translator::IRmlTranslator;
        let elem = Element {
            tag: "Shortcut".into(),
            attributes: vec![Attribute::Static {
                name: "key".into(),
                value: "Ctrl+S".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            span: Span::empty(),
        };
        let translator = ShortcutTranslator;
        let err = translator
            .to_rust(&elem, &ctx(), &mut 1, &[], &[])
            .unwrap_err();
        assert!(err.message.contains("ShortcutScope"));
    }
}
