//! Alert 组件 codegen
//!
//! Alert 构造器统一为 `Alert::new(id, message)`，variant 通过 `variant="info"` 属性
//! 映射到 `.with_variant(AlertVariant::Info)` builder 方法。
//!
//! ## variant 属性
//!
//! `variant="info"` / `"success"` / `"warning"` / `"error"` / `"default"` → `.with_variant(AlertVariant::*）`
//! 不写 variant = 默认 Default。
//!
//! ## message 来源
//!
//! 优先级：`message="..."` 静态属性 > `message={expr}` 绑定属性 > 文本子节点 > 空字符串。
//!
//! Alert 不实现 `ParentElement`，子节点仅文本作为 message；元素子节点被忽略。

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
use crate::tags;

/// 生成 Alert 构造代码
pub fn gen_alert(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let resolved = tags::normalize_component_tag(&elem.tag);
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 提取 message（优先级：message 静态 > message 绑定 > 文本子节点 > 空字符串）
    let mut message_code = String::from("\"\"");
    let mut message_set = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == "message" && !message_set => {
                message_code = format!("{:?}", value);
                message_set = true;
            }
            Attribute::Bind { name, expr, .. } if name == "message" && !message_set => {
                let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                message_code = format!("{}.clone()", rust_expr);
                message_set = true;
            }
            _ => {}
        }
    }
    if !message_set {
        for child in &elem.children {
            if let Node::Text(t) = child {
                message_code = format!("{:?}", t);
                break;
            }
        }
    }

    // 2. 决定 ElementId（参考 component.rs 的 ref 处理）
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    });
    let id_code = if let Some(name) = ref_name {
        format!("\"rml_ref:{}\"", name)
    } else {
        let id_val = *id_counter;
        *id_counter += 1;
        format!("(\"rml_el\", {}usize)", id_val)
    };

    // 3. 构造器统一为 Alert::new(id, message)，variant 由 variant 属性 + .with_variant() 设置
    let mut code = format!("rml_ui::Alert::new({}, {})", id_code, message_code);

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, "Alert", ctx.stylesheet.as_ref(), parents);

    // 4. 处理其他属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                // message 属性已用于构造器
                if name == "message" {
                    continue;
                }
                // variant="info" → .with_variant(AlertVariant::Info)
                if name == "variant" {
                    if let Some(v) = parse_variant(value) {
                        code.push_str(&format!(".with_variant(rml_ui::AlertVariant::{})", v));
                        continue;
                    }
                }
                // banner="" → .banner()
                if name == "banner" && (value.is_empty() || value.eq_ignore_ascii_case("true")) {
                    code.push_str(".banner()");
                    continue;
                }
                // visible="true" → .visible(true)
                if name == "visible" {
                    code.push_str(&format!(".visible({})", crate::compiler::setters::parse_bool(value)));
                    continue;
                }
                // title="..." → .title("...")
                if name == "title" {
                    code.push_str(&format!(".title({:?})", value));
                    continue;
                }
                // icon="Settings" → .icon(rml_ui::Icon::new(rml_ui::IconName::Settings))
                if name == "icon" {
                    code.push_str(&format!(
                        ".icon(rml_ui::Icon::new(rml_ui::IconName::{}))",
                        value
                    ));
                    continue;
                }
                // 通用 setter（size / disabled / tooltip 等）
                if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, &resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "message" {
                    continue;
                }
                // variant={expr} → .with_variant(expr)
                if name == "variant" {
                    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".with_variant({})", rust_expr));
                    continue;
                }
                // banner={cond} → .when(cond, |a| a.banner())
                if name == "banner" {
                    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".when({}, |a| a.banner())", rust_expr));
                    continue;
                }
                // visible={cond} → .visible(cond)
                if name == "visible" {
                    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".visible({})", rust_expr));
                    continue;
                }
                // title={expr} → .title(expr.clone())
                if name == "title" {
                    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".title({}.clone())", rust_expr));
                    continue;
                }
                // icon={expr} → .icon(expr)
                if name == "icon" {
                    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".icon({})", rust_expr));
                    continue;
                }
                // 通用 bind setter
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    &resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                // Alert 的 on_close 是 3 参闭包 Fn(&ClickEvent, &mut Window, &mut App)，
                // 与 cx.listener 输出兼容。专用处理生成 .on_close(cx.listener(...))
                if name == "on_close" {
                    if let Some(s) = gen_on_close_setter(handler) {
                        code.push_str(&s);
                        continue;
                    }
                }
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, &resolved)
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 5. 子节点处理
    //
    // Alert 不实现 ParentElement，但 text 子节点已被用作 message。
    // 元素子节点忽略（避免误用 .child()）。
    // 注：each 指令仍可生成迭代器，但 Alert 不支持 .children()，此处保留以兼容 if/show 指令路径。
    let _ = gen_node;

    Ok(code)
}

/// 解析 `variant="info"` 静态值为 AlertVariant 枚举变体名
fn parse_variant(value: &str) -> Option<&'static str> {
    match value {
        "default" | "Default" => Some("Default"),
        "info" | "Info" => Some("Info"),
        "success" | "Success" => Some("Success"),
        "warning" | "Warning" => Some("Warning"),
        "error" | "Error" => Some("Error"),
        _ => None,
    }
}

/// 生成 `.on_close(cx.listener(...))` 调用
///
/// Alert 的 `.on_close()` 接受 `impl Fn(&ClickEvent, &mut Window, &mut App)`，
/// `cx.listener` 输出 `impl Fn(&ClickEvent, &mut Window, &mut App)`（含 this），签名兼容。
///
/// 命令方法签名 `fn(&ClickEvent, &mut Context<Self>)`，故闭包内需：
/// 1. 将 gpui::ClickEvent 转为 rml::ClickEvent
/// 2. 调用 `this.method(&rml_ev, cx)` 传递事件参数
fn gen_on_close_setter(handler: &EventHandler) -> Option<String> {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
    };
    match handler {
        EventHandler::Ident(_) | EventHandler::MethodName(_) => Some(format!(
            ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
             let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
             this.{}(&rml_ev, cx);\n                }}))",
            method
        )),
        EventHandler::WithArgs(_, args) if args.is_empty() => Some(format!(
            ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
             let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
             this.{}(&rml_ev, cx);\n                }}))",
            method
        )),
        EventHandler::WithArgs(_, args) => {
            let arg = &args[0];
            Some(format!(
                ".on_close(cx.listener(move |this, _ev: &gpui::ClickEvent, _window, cx| {{\n                    \
                 let p0 = {}.clone();\n                    \
                 let rml_ev = rml_convert::from_gpui_click(_ev);\n                    \
                 this.{}(p0, &rml_ev, cx);\n                }}))",
                arg, method
            ))
        }
    }
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
            ..Default::default()
        }
    }

    #[test]
    fn gen_alert_default_minimal() {
        // <Alert>消息</Alert> → Alert::new(id, "消息")
        let elem = make_element("Alert", vec![], vec![Node::Text("消息".into())]);
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Alert::new("));
        assert!(code.contains("\"rml_el\""));
        assert!(code.contains("\"消息\""));
    }

    #[test]
    fn gen_alert_info_variant() {
        // <Alert variant="info">提示</Alert> → Alert::new(id, "提示").with_variant(AlertVariant::Info)
        let elem = make_element(
            "Alert",
            vec![Attribute::Static {
                name: "variant".into(),
                value: "info".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("提示".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Alert::new("));
        assert!(code.contains(".with_variant(rml_ui::AlertVariant::Info)"));
        assert!(code.contains("\"提示\""));
    }

    #[test]
    fn gen_alert_success_variant() {
        let elem = make_element(
            "Alert",
            vec![Attribute::Static {
                name: "variant".into(),
                value: "success".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("成功".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Alert::new("));
        assert!(code.contains(".with_variant(rml_ui::AlertVariant::Success)"));
    }

    #[test]
    fn gen_alert_variant_attr() {
        // <Alert variant="warning" message="警告" />
        let elem = make_element(
            "Alert",
            vec![
                Attribute::Static {
                    name: "variant".into(),
                    value: "warning".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "message".into(),
                    value: "警告".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        // 默认构造器（无 variant 关联属性）
        assert!(code.contains("rml_ui::Alert::new("));
        // .with_variant(AlertVariant::Warning)
        assert!(code.contains(".with_variant(rml_ui::AlertVariant::Warning)"));
        // message 来自属性而非子节点
        assert!(code.contains("\"警告\""));
    }

    #[test]
    fn gen_alert_with_title_and_banner() {
        // <Alert variant="info" title="提示" banner="">消息</Alert>
        let elem = make_element(
            "Alert",
            vec![
                Attribute::Static {
                    name: "variant".into(),
                    value: "info".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "title".into(),
                    value: "标题".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "banner".into(),
                    value: "".into(),
                    span: Span::empty(),
                },
            ],
            vec![Node::Text("消息".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::Alert::new("));
        assert!(code.contains(".with_variant(rml_ui::AlertVariant::Info)"));
        assert!(code.contains(".title(\"标题\")"));
        assert!(code.contains(".banner()"));
    }

    #[test]
    fn gen_alert_message_attr_priority_over_child() {
        // <Alert message="attr_msg">child_msg</Alert> → 使用属性
        let elem = make_element(
            "Alert",
            vec![Attribute::Static {
                name: "message".into(),
                value: "attr_msg".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("child_msg".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("\"attr_msg\""));
        assert!(!code.contains("child_msg"));
    }

    #[test]
    fn gen_alert_message_bind() {
        // <Alert message={error_msg} />
        let elem = make_element(
            "Alert",
            vec![Attribute::Bind {
                name: "message".into(),
                expr: "error_msg".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("self.error_msg.clone()"));
    }

    #[test]
    fn gen_alert_with_size() {
        // <Alert size="small" />
        let elem = make_element(
            "Alert",
            vec![Attribute::Static {
                name: "size".into(),
                value: "small".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("消息".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".with_size(rml_ui::Size::Small)"));
    }

    #[test]
    fn gen_alert_with_on_close() {
        // <Alert on-close={handle_close}>消息</Alert>
        let elem = make_element(
            "Alert",
            vec![Attribute::Event {
                name: "on_close".into(),
                handler: EventHandler::Ident("handle_close".into()),
                span: Span::empty(),
            }],
            vec![Node::Text("消息".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".on_close(cx.listener("));
        assert!(code.contains("this.handle_close"));
        assert!(code.contains("rml_convert::from_gpui_click"));
        assert!(code.contains("&rml_ev, cx"));
    }

    #[test]
    fn gen_alert_with_ref() {
        // <Alert ref="my_alert">消息</Alert>
        let elem = Element {
            tag: "Alert".into(),
            attributes: vec![],
            directives: vec![Directive::Ref {
                name: "my_alert".into(),
                span: Span::empty(),
            }],
            children: vec![Node::Text("消息".into())],
            ..Default::default()
        };
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("\"rml_ref:my_alert\""));
        assert!(!code.contains("rml_el"));
    }

    #[test]
    fn gen_alert_increments_id_counter() {
        let elem = make_element("Alert", vec![], vec![Node::Text("消息".into())]);
        let mut id = 5;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains("5usize"));
        assert_eq!(id, 6);
    }

    #[test]
    fn gen_alert_icon_attr() {
        // <Alert icon="Bell" variant="info">消息</Alert>
        let elem = make_element(
            "Alert",
            vec![
                Attribute::Static {
                    name: "icon".into(),
                    value: "Bell".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "variant".into(),
                    value: "info".into(),
                    span: Span::empty(),
                },
            ],
            vec![Node::Text("消息".into())],
        );
        let mut id = 0;
        let code = gen_alert(&elem, &ctx(), &mut id, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".icon(rml_ui::Icon::new(rml_ui::IconName::Bell))"));
    }
}
