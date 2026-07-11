//! Notification 构造代码生成
//!
//! ## 构造器
//!
//! `NotificationTrigger::new()` —— 无 ElementId、无 cx 参数（RenderOnce 组件）。
//!
//! ## 子节点处理
//!
//! - `slot="trigger"` 的子元素 → `.trigger(element)`（唯一支持的子节点）
//! - 其余子元素 → 报错（NotificationTrigger 不实现 ParentElement，无 `.child()` 方法）
//!
//! ## variant 布尔属性
//!
//! `success` / `info` / `warning` / `error` → `.with_type(NotificationType::X)`（独立布尔属性）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};

use super::setters::static_setter;

/// 生成 Notification 构造代码
pub fn gen_notification(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 1. 构造器：NotificationTrigger::new()（无 ElementId、无 cx）
    let mut code = "rml_ui::NotificationTrigger::new()".to_string();

    // CSS class 样式
    append_css_class_styles(
        &mut code,
        elem,
        "Notification",
        ctx.stylesheet.as_ref(),
        parents,
    );

    // 2. 属性 → setter
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Notification")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> =
                    ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    "Notification",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { .. } => {
                // NotificationTrigger 无事件回调（点击由内部 on_mouse_down 处理）
            }
        }
    }

    // 3. 子节点：slot="trigger" → .trigger()，其余 → 报错
    let mut trigger_code: Option<String> = None;

    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        match child {
            Node::Element(e) if e.slot_name.as_deref() == Some("trigger") => {
                if is_iter {
                    return Err(CodegenError {
                        message: "Notification trigger slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if trigger_code.is_some() {
                    return Err(CodegenError {
                        message: "Notification requires exactly one trigger slot (multiple found)"
                            .into(),
                        span: Some(elem.span),
                    });
                }
                trigger_code = Some(child_code);
            }
            Node::Text(t) if t.trim().is_empty() => {
                // 忽略空白文本
            }
            _ => {
                return Err(CodegenError {
                    message: format!(
                        "Notification only supports slot=\"trigger\" child; non-trigger children are not allowed (NotificationTrigger does not implement ParentElement)"
                    ),
                    span: Some(elem.span),
                });
            }
        }
    }

    if let Some(tc) = trigger_code {
        code.push_str(&format!("\n            .trigger({})", tc));
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, Node};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_element(tag: &str, attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    fn make_trigger() -> Element {
        Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Save".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_notification_minimal() {
        let elem = make_element("Notification", vec![], vec![]);
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::NotificationTrigger::new()"));
    }

    #[test]
    fn gen_notification_with_title_and_message() {
        let elem = make_element(
            "Notification",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "保存成功".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "message".into(),
                    value: "您的更改已保存".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".title(\"保存成功\")"));
        assert!(code.contains(".message(\"您的更改已保存\")"));
    }

    #[test]
    fn gen_notification_with_success_variant() {
        let elem = make_element(
            "Notification",
            vec![Attribute::Static {
                name: "success".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".with_type(rml_ui::NotificationType::Success)"));
    }

    #[test]
    fn gen_notification_with_error_variant() {
        let elem = make_element(
            "Notification",
            vec![Attribute::Static {
                name: "error".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".with_type(rml_ui::NotificationType::Error)"));
    }

    #[test]
    fn gen_notification_with_autohide_false() {
        let elem = make_element(
            "Notification",
            vec![Attribute::Static {
                name: "autohide".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".autohide(false)"));
    }

    #[test]
    fn gen_notification_with_trigger() {
        let elem = make_element(
            "Notification",
            vec![],
            vec![Node::Element(make_trigger())],
        );
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".trigger("));
    }

    #[test]
    fn gen_notification_multiple_triggers_error() {
        let elem = make_element(
            "Notification",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(make_trigger())],
        );
        let result = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one trigger"));
    }

    #[test]
    fn gen_notification_non_trigger_child_error() {
        let elem = make_element(
            "Notification",
            vec![],
            vec![Node::Text("some content".into())],
        );
        let result = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("only supports slot=\"trigger\""));
    }

    #[test]
    fn gen_notification_whitespace_text_ignored() {
        let elem = make_element(
            "Notification",
            vec![],
            vec![Node::Text("   \n  ".into())],
        );
        let result = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn gen_notification_full_example() {
        let elem = make_element(
            "Notification",
            vec![
                Attribute::Static {
                    name: "success".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "title".into(),
                    value: "操作成功".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "message".into(),
                    value: "数据已保存".into(),
                    span: Span::empty(),
                },
            ],
            vec![Node::Element(make_trigger())],
        );
        let code = gen_notification(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::NotificationTrigger::new()"));
        assert!(code.contains(".with_type(rml_ui::NotificationType::Success)"));
        assert!(code.contains(".title(\"操作成功\")"));
        assert!(code.contains(".message(\"数据已保存\")"));
        assert!(code.contains(".trigger("));
    }
}
