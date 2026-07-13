//! AlertDialog 构造代码生成
//!
//! ## 构造器
//!
//! `AlertDialog::new(cx: &mut App)` —— 直接使用 render 上下文的 `cx` 变量。
//!
//! ## 子节点处理
//!
//! - `slot="trigger"` 的子元素 → `.trigger(element)`（同 Dialog/HoverCard）
//! - `slot="footer"` 的子元素 → `.footer(element)`（自定义页脚，同 Dialog）
//! - 其余子元素 → `.child(element)` / `.children(iterator)`（ParentElement）
//!
//! ## 受控模式 `open={field}`
//!
//! 当 `open` bind 属性存在时，进入受控模式（同 Dialog）：
//! - 不渲染 trigger（受控模式由 ViewModel 状态驱动显示/隐藏）
//! - 生成条件渲染：`if self.field { AlertDialog...into_any_element() } else { Empty }`
//! - 自动注入 `on_close` 回写 `field = false`（与用户 `on_close` 合并）

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, EventHandler};

use super::setters::{event_setter, static_setter};

/// 生成 AlertDialog 构造代码
pub fn gen_alert_dialog(
    elem: &Element,
    _ref_name: Option<&str>,
    _id_val: usize,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    // 受控模式检测：open={field}
    let open_field: Option<String> = elem.attributes.iter().find_map(|attr| {
        if let Attribute::Bind { name, expr, .. } = attr {
            if name == "open" {
                let (field, converter) =
                    crate::compiler::codegen::extract_field_converter(expr);
                // 仅支持简单字段引用（无 converter、无点号、无中括号）
                if converter.is_none()
                    && !field.contains('.')
                    && !field.contains('[')
                    && field != "true"
                    && field != "false"
                    && field.parse::<f64>().is_err()
                {
                    return Some(field);
                }
            }
        }
        None
    });

    // 收集用户 on_close handler（受控模式下需与自动回写合并）
    let user_on_close: Option<&EventHandler> = if open_field.is_some() {
        elem.attributes.iter().find_map(|attr| {
            if let Attribute::Event { name, handler, .. } = attr {
                if name == "on_close" {
                    return Some(handler);
                }
            }
            None
        })
    } else {
        None
    };

    // 1. 构造器：AlertDialog::new(cx)
    let mut code = "rml_ui::AlertDialog::new(cx)".to_string();

    // CSS class 样式
    append_css_class_styles(
        &mut code,
        elem,
        "AlertDialog",
        ctx.stylesheet.as_ref(),
        parents,
    );

    // 2. 属性 → setter（受控模式下跳过 open 和 on_close）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "AlertDialog")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, .. } if name == "open" => {
                // 受控模式：open 属性不生成 setter，由条件渲染处理
            }
            Attribute::Bind { name, expr, .. } => {
                let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, "AlertDialog",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, .. }
                if name == "on_close" && open_field.is_some() =>
            {
                // 受控模式：on_close 由自动回写处理，跳过用户 handler
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = event_setter(name, handler) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "AlertDialog")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 3. 受控模式：注入自动回写 on_close（合并用户 handler）
    if let Some(ref field) = open_field {
        code.push_str(&gen_controlled_on_close(field, user_on_close));
    }

    // 4. 子节点：slot="trigger" → .trigger()，slot="footer" → .footer()，其余 → .child() / .children()
    //    受控模式下跳过 trigger（由 ViewModel 状态驱动）
    let mut trigger_code: Option<String> = None;
    let mut footer_code: Option<String> = None;
    let mut content_codes: Vec<String> = Vec::new();

    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        match child {
            crate::parser::ast::Node::Element(e)
                if e.slot_name.as_deref() == Some("trigger") && open_field.is_none() =>
            {
                if is_iter {
                    return Err(CodegenError {
                        message: "AlertDialog trigger slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if trigger_code.is_some() {
                    return Err(CodegenError {
                        message: "AlertDialog requires exactly one trigger slot (multiple found)"
                            .into(),
                        span: Some(elem.span),
                    });
                }
                trigger_code = Some(child_code);
            }
            crate::parser::ast::Node::Element(e) if e.slot_name.as_deref() == Some("footer") => {
                if is_iter {
                    return Err(CodegenError {
                        message: "AlertDialog footer slot cannot be an each iterator".into(),
                        span: Some(elem.span),
                    });
                }
                if footer_code.is_some() {
                    return Err(CodegenError {
                        message: "AlertDialog requires exactly one footer slot (multiple found)"
                            .into(),
                        span: Some(elem.span),
                    });
                }
                footer_code = Some(child_code);
            }
            _ => {
                if is_iter {
                    content_codes.push(format!(".children({})", child_code));
                } else {
                    content_codes.push(format!(".child({})", child_code));
                }
            }
        }
    }

    if let Some(tc) = trigger_code {
        code.push_str(&format!("\n            .trigger({})", tc));
    }
    if let Some(fc) = footer_code {
        code.push_str(&format!("\n            .footer({})", fc));
    }
    for content_code in content_codes {
        code.push_str(&format!("\n            {}", content_code));
    }

    // 5. 受控模式：条件渲染包装
    if let Some(ref field) = open_field {
        let alias = crate::compiler::expr::current_self_alias().unwrap_or("self");
        code = format!(
            "if {alias}.{field} {{ {code}.into_any_element() }} else {{ gpui::Empty.into_any_element() }}",
            alias = alias,
            field = field,
            code = code
        );
    }

    Ok(code)
}

/// 生成受控模式的 on_close 回调（自动回写 field=false + 合并用户 handler）
fn gen_controlled_on_close(field: &str, user_handler: Option<&EventHandler>) -> String {
    let user_call = match user_handler {
        Some(EventHandler::Ident(m)) | Some(EventHandler::MethodName(m)) => {
            format!(
                "\n    let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n    this.{}(&rml_ev, cx);",
                m
            )
        }
        Some(EventHandler::WithArgs(m, args)) if args.is_empty() => {
            format!(
                "\n    let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n    this.{}(&rml_ev, cx);",
                m
            )
        }
        Some(EventHandler::WithArgs(m, args)) => {
            let arg = &args[0];
            format!(
                "\n    let p0 = {}.clone();\n    let rml_ev = rml::runtime::event_flow::convert::from_gpui_click(_ev);\n    this.{}(p0, &rml_ev, cx);",
                arg, m
            )
        }
        _ => String::new(),
    };

    format!(
        ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n    \
         this.{field} = false;\n    \
         this.__rml_bump_version({field:?});{user_call}\n    \
         cx.notify();\n}}))",
        field = field,
        user_call = user_call
    )
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

    fn make_text_child(text: &str) -> Node {
        Node::Text(text.into())
    }

    fn make_trigger() -> Element {
        Element {
            tag: "Button".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: "Delete".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: Some("trigger".into()),
            ..Default::default()
        }
    }

    #[test]
    fn gen_alert_dialog_minimal() {
        let elem = make_element("AlertDialog", vec![], vec![]);
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::AlertDialog::new(cx)"));
    }

    #[test]
    fn gen_alert_dialog_with_title_and_description() {
        let elem = make_element(
            "AlertDialog",
            vec![
                Attribute::Static {
                    name: "title".into(),
                    value: "确认删除".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "description".into(),
                    value: "此操作不可撤销".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".title(\"确认删除\")"));
        assert!(code.contains(".description(\"此操作不可撤销\")"));
    }

    #[test]
    fn gen_alert_dialog_with_confirm() {
        let elem = make_element(
            "AlertDialog",
            vec![Attribute::Static {
                name: "confirm".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".confirm()"));
    }

    #[test]
    fn gen_alert_dialog_with_width() {
        let elem = make_element(
            "AlertDialog",
            vec![Attribute::Static {
                name: "width".into(),
                value: "420px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".width(gpui::px(420.0))"));
    }

    #[test]
    fn gen_alert_dialog_with_trigger() {
        let elem = make_element("AlertDialog", vec![], vec![Node::Element(make_trigger())]);
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".trigger("));
    }

    #[test]
    fn gen_alert_dialog_with_content_children() {
        let elem = make_element(
            "AlertDialog",
            vec![],
            vec![make_text_child("First"), make_text_child("Second")],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert_eq!(code.matches(".child(").count(), 2);
        assert!(code.contains("\"First\""));
        assert!(code.contains("\"Second\""));
    }

    #[test]
    fn gen_alert_dialog_multiple_triggers_error() {
        let elem = make_element(
            "AlertDialog",
            vec![],
            vec![Node::Element(make_trigger()), Node::Element(make_trigger())],
        );
        let result = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one trigger"));
    }

    #[test]
    fn gen_alert_dialog_with_close_button_true() {
        let elem = make_element(
            "AlertDialog",
            vec![Attribute::Static {
                name: "close_button".into(),
                value: "true".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".close_button(true)"));
    }

    #[test]
    fn gen_alert_dialog_with_footer_slot() {
        let mut footer_btn = make_trigger();
        footer_btn.slot_name = Some("footer".into());
        footer_btn.attributes = vec![Attribute::Static {
            name: "label".into(),
            value: "自定义确认".into(),
            span: Span::empty(),
        }];
        let elem = make_element("AlertDialog", vec![], vec![Node::Element(footer_btn)]);
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".footer("));
    }

    #[test]
    fn gen_alert_dialog_multiple_footers_error() {
        let mut footer1 = make_trigger();
        footer1.slot_name = Some("footer".into());
        let mut footer2 = make_trigger();
        footer2.slot_name = Some("footer".into());
        let elem = make_element(
            "AlertDialog",
            vec![],
            vec![Node::Element(footer1), Node::Element(footer2)],
        );
        let result = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().message.contains("exactly one footer"));
    }

    // ─── 受控模式 open={field} 测试 ───

    fn make_bind_attr(name: &str, expr: &str) -> Attribute {
        Attribute::Bind {
            name: name.into(),
            expr: expr.into(),
            span: Span::empty(),
        }
    }

    fn make_event_attr(name: &str, handler: &str) -> Attribute {
        Attribute::Event {
            name: name.into(),
            handler: EventHandler::Ident(handler.into()),
            span: Span::empty(),
        }
    }

    #[test]
    fn gen_alert_dialog_controlled_open_basic() {
        let elem = make_element(
            "AlertDialog",
            vec![
                make_bind_attr("open", "show_alert"),
                Attribute::Static {
                    name: "title".into(),
                    value: "受控警示框".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 条件渲染包装
        assert!(code.contains("if self.show_alert {"));
        assert!(code.contains("gpui::Empty.into_any_element()"));
        // AlertDialog 仍然生成
        assert!(code.contains("rml_ui::AlertDialog::new(cx)"));
        assert!(code.contains(".title(\"受控警示框\")"));
        // 自动回写 on_close
        assert!(code.contains(".on_close(cx.listener("));
        assert!(code.contains("this.show_alert = false"));
        assert!(code.contains("this.__rml_bump_version(\"show_alert\")"));
        // 无 trigger
        assert!(!code.contains(".trigger("));
    }

    #[test]
    fn gen_alert_dialog_controlled_open_with_content() {
        let elem = make_element(
            "AlertDialog",
            vec![make_bind_attr("open", "visible")],
            vec![make_text_child("内容")],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("if self.visible {"));
        assert!(code.contains(".child(\"内容\")"));
    }

    #[test]
    fn gen_alert_dialog_controlled_open_with_user_on_close() {
        let elem = make_element(
            "AlertDialog",
            vec![
                make_bind_attr("open", "show"),
                make_event_attr("on_close", "handle_close"),
            ],
            vec![],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 自动回写 + 用户 handler 合并
        assert!(code.contains("this.show = false"));
        assert!(code.contains("this.__rml_bump_version(\"show\")"));
        assert!(code.contains("this.handle_close"));
    }

    #[test]
    fn gen_alert_dialog_controlled_open_ignores_trigger() {
        let elem = make_element(
            "AlertDialog",
            vec![make_bind_attr("open", "show")],
            vec![Node::Element(make_trigger())],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 受控模式下 trigger 被忽略（作为普通子节点处理）
        assert!(!code.contains(".trigger("));
    }

    #[test]
    fn gen_alert_dialog_controlled_open_with_footer_slot() {
        let mut footer_btn = make_trigger();
        footer_btn.slot_name = Some("footer".into());
        let elem = make_element(
            "AlertDialog",
            vec![make_bind_attr("open", "show")],
            vec![Node::Element(footer_btn)],
        );
        let code = gen_alert_dialog(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("if self.show {"));
        assert!(code.contains(".footer("));
    }
}
