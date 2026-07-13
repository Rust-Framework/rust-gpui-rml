//! Sheet 构造代码生成
//!
//! ## 构造器
//!
//! `Sheet::new(_: &mut Window, cx: &mut App)` —— 直接使用 render 上下文的
//! `_window` 和 `cx` 变量，不分配 ElementId。
//!
//! ## 子节点处理
//!
//! Sheet 实现 `ParentElement`，所有子节点通过 `.child()` / `.children()` 注入为 content。
//!
//! ## 受控模式 `open={field}`
//!
//! 当 `open` bind 属性存在时，进入受控模式（与 Dialog/AlertDialog 一致）：
//! - 生成条件渲染：`if self.field { Sheet...into_any_element() } else { Empty }`
//! - 自动注入 `on_close` 回写 `field = false`（与用户 `on_close` 合并）
//!
//! Sheet 的 `on_close` 方法签名 `Fn(&ClickEvent, &mut Window, &mut App)` 与 Dialog 一致，
//! 可直接复用 `gen_controlled_on_close` 的实现模式。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, EventHandler};

use super::setters::{event_setter, static_setter};

/// 生成 Sheet 构造代码
pub fn gen_sheet(
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

    // 1. 构造器：Sheet::new(_window, cx) —— 使用 render 上下文变量
    let mut code = "rml_ui::Sheet::new(_window, cx)".to_string();

    // CSS class 样式
    append_css_class_styles(&mut code, elem, "Sheet", ctx.stylesheet.as_ref(), parents);

    // 2. 属性 → setter（受控模式下跳过 open 和 on_close）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "Sheet")
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
                    name, expr, &lv, &computed, "Sheet",
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
                    crate::compiler::setters::component_event_setter(name, handler, "Sheet")
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

    // 4. 子节点：全部通过 .child() / .children() 注入
    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!("\n            .children({})", child_code));
        } else {
            code.push_str(&format!("\n            .child({})", child_code));
        }
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
///
/// Sheet 的 `on_close` 签名 `Fn(&ClickEvent, &mut Window, &mut App)` 与 Dialog 一致，
/// 使用 `cx.listener()` 桥接，内部完成 field 回写与用户 handler 调用。
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

    fn make_div_child() -> Node {
        Node::Element(Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("Content".into())],
            slot_name: None,
            ..Default::default()
        })
    }

    #[test]
    fn gen_sheet_minimal() {
        let elem = make_element("Sheet", vec![], vec![]);
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Sheet::new(_window, cx)"));
    }

    #[test]
    fn gen_sheet_with_title() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "title".into(),
                value: "设置面板".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".title(\"设置面板\")"));
    }

    #[test]
    fn gen_sheet_with_size() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "size".into(),
                value: "400px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".size(gpui::px(400.0))"));
    }

    #[test]
    fn gen_sheet_with_resizable_false() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "resizable".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".resizable(false)"));
    }

    #[test]
    fn gen_sheet_with_children() {
        let elem = make_element("Sheet", vec![], vec![make_div_child()]);
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_sheet_with_multiple_children() {
        let elem = make_element(
            "Sheet",
            vec![],
            vec![make_text_child("First"), make_text_child("Second")],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // Two .child() calls for text children (text nodes don't have nested .child())
        assert_eq!(code.matches(".child(").count(), 2);
        assert!(code.contains("\"First\""));
        assert!(code.contains("\"Second\""));
    }

    #[test]
    fn gen_sheet_with_overlay_false() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "overlay".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".overlay(false)"));
    }

    #[test]
    fn gen_sheet_with_footer() {
        let elem = make_element(
            "Sheet",
            vec![Attribute::Static {
                name: "footer".into(),
                value: "操作栏".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".footer(\"操作栏\")"));
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
    fn gen_sheet_controlled_open_basic() {
        let elem = make_element(
            "Sheet",
            vec![
                make_bind_attr("open", "show_sheet"),
                Attribute::Static {
                    name: "title".into(),
                    value: "受控抽屉".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 条件渲染包装
        assert!(code.contains("if self.show_sheet {"));
        assert!(code.contains("gpui::Empty.into_any_element()"));
        // Sheet 仍然生成
        assert!(code.contains("rml_ui::Sheet::new(_window, cx)"));
        assert!(code.contains(".title(\"受控抽屉\")"));
        // 自动回写 on_close
        assert!(code.contains(".on_close(cx.listener("));
        assert!(code.contains("this.show_sheet = false"));
        assert!(code.contains("this.__rml_bump_version(\"show_sheet\")"));
    }

    #[test]
    fn gen_sheet_controlled_open_with_content() {
        let elem = make_element(
            "Sheet",
            vec![make_bind_attr("open", "visible")],
            vec![make_text_child("内容")],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("if self.visible {"));
        assert!(code.contains(".child(\"内容\")"));
    }

    #[test]
    fn gen_sheet_controlled_open_with_user_on_close() {
        let elem = make_element(
            "Sheet",
            vec![
                make_bind_attr("open", "show"),
                make_event_attr("on_close", "handle_close"),
            ],
            vec![],
        );
        let code = gen_sheet(&elem, None, 0, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        // 自动回写 + 用户 handler 合并
        assert!(code.contains("this.show = false"));
        assert!(code.contains("this.__rml_bump_version(\"show\")"));
        assert!(code.contains("this.handle_close"));
    }
}
