//! 窗口外壳包裹代码生成
//!
//! - `<modern_window>` → `ModernWindowShell` 包裹
//! - `<tab_window>` → `TabWindowShell` 包裹 + 插槽分区

use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler, Node};

/// 为 `<modern_window>` 根元素生成 ModernWindowShell 包裹代码
///
/// - title 复用 IWindow::title()，不重复定义
/// - menu/status_bar/icon 从根元素 Attribute::Bind 提取，使用表达式解析器处理 computed 方法
pub(super) fn gen_modern_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
) -> Result<String, CodegenError> {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let empty: Vec<&str> = Vec::new();

    let mut code = String::from("rml_ui::ModernWindowShell::new()");
    code.push_str(".title(self.title().to_string())");

    for attr in &elem.attributes {
        if let Attribute::Bind { name, expr } = attr {
            match name.as_str() {
                "menu" | "status_bar" => {
                    let rust_expr = match expr::parse(expr) {
                        Ok(expr::Expr::Field(field_name))
                            if computed.iter().any(|c| *c == field_name.as_str()) =>
                        {
                            format!("self.{}()", field_name)
                        }
                        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, &empty),
                        Err(_) => {
                            let trimmed = expr.trim();
                            if computed.iter().any(|c| *c == trimmed) {
                                format!("self.{}()", trimmed)
                            } else {
                                format!("self.{}", trimmed)
                            }
                        }
                    };
                    match name.as_str() {
                        "menu" => code.push_str(&format!(".menu({}.clone())", rust_expr)),
                        "status_bar" => {
                            code.push_str(&format!(".status_bar({}.clone())", rust_expr))
                        }
                        _ => {}
                    }
                }
                "icon" => {
                    let rust_expr = match expr::parse(expr) {
                        Ok(expr::Expr::Field(field_name))
                            if computed.iter().any(|c| *c == field_name.as_str()) =>
                        {
                            format!("self.{}()", field_name)
                        }
                        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, &empty),
                        Err(_) => expr.trim().to_string(),
                    };
                    code.push_str(&format!(".icon(rml_ui::{})", rust_expr));
                }
                _ => {}
            }
        }
    }

    code.push_str(&format!(".child({})", children_body));
    Ok(code)
}

/// 将 `<tab_window>` 子节点拆分为插槽与主内容
pub(super) fn partition_tab_slot_children(
    children: &[Node],
) -> (Option<Node>, Option<Node>, Option<Node>, Vec<Node>) {
    let mut slot_left = None;
    let mut slot_right = None;
    let mut slot_bottom = None;
    let mut body = Vec::new();

    for child in children {
        if let Node::Element(elem) = child {
            match elem.tag.as_str() {
                "slot_left" => {
                    slot_left = slot_element_content(elem);
                    continue;
                }
                "slot_right" => {
                    slot_right = slot_element_content(elem);
                    continue;
                }
                "slot_bottom" => {
                    slot_bottom = slot_element_content(elem);
                    continue;
                }
                _ => {}
            }
        }
        body.push(child.clone());
    }

    (slot_left, slot_right, slot_bottom, body)
}

/// 取插槽包装元素的内部内容（单子节点直接 unwrap，多子节点包 div）
fn slot_element_content(elem: &Element) -> Option<Node> {
    match elem.children.len() {
        0 => None,
        1 => Some(elem.children[0].clone()),
        _ => Some(Node::Element(Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: elem.children.clone(),
        })),
    }
}

/// 从根 `<tab_window>` 的 bind/event 属性生成 `TabWindowShell` 包裹代码
pub(super) fn gen_tab_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
    slot_left: Option<&str>,
    slot_right: Option<&str>,
    slot_bottom: Option<&str>,
) -> Result<String, CodegenError> {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let empty: Vec<&str> = Vec::new();

    let mut code = String::from("rml_ui::TabWindowShell::new()");
    code.push_str(".title(self.title().to_string())");

    for attr in &elem.attributes {
        match attr {
            Attribute::Bind { name, expr } => {
                if name == "icon" {
                    let rust_expr = match expr::parse(expr) {
                        Ok(expr::Expr::Field(field_name))
                            if computed.iter().any(|c| *c == field_name.as_str()) =>
                        {
                            format!("self.{}()", field_name)
                        }
                        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, &empty),
                        Err(_) => expr.trim().to_string(),
                    };
                    let icon_expr = if rust_expr.contains("IconName::") {
                        format!("rml_ui::{rust_expr}")
                    } else {
                        rust_expr
                    };
                    code.push_str(&format!(".icon({icon_expr})"));
                    continue;
                }
                let rust_expr = shell_bind_expr(expr, &computed, &empty);
                match name.as_str() {
                    "menu" => code.push_str(&format!(".menu({}.clone())", rust_expr)),
                    "status_bar" => code.push_str(&format!(".status_bar({}.clone())", rust_expr)),
                    "tabs" => code.push_str(&format!(".tabs({}.clone())", rust_expr)),
                    "selected_tab" => code.push_str(&format!(".selected_tab({})", rust_expr)),
                    "show_chrome" => code.push_str(&format!(".show_chrome({})", rust_expr)),
                    _ => {}
                }
            }
            Attribute::Event { name, handler } if name == "on_tab_click" => {
                let method = match handler {
                    EventHandler::Ident(m) | EventHandler::MethodName(m) => m.as_str(),
                    EventHandler::WithArgs(m, _) => m.as_str(),
                };
                code.push_str(&format!(
                    ".on_tab_click({{\n                    \
                     let weak = cx.weak_entity();\n                    \
                     move |index: usize, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
                     if let Some(entity) = weak.upgrade() {{\n                            \
                     entity.update(app, |this, cx| {{ this.{}(index, cx); }});\n                        \
                     }}\n                    }}\n                }})",
                    method
                ));
            }
            Attribute::Event { name, handler } if name == "on_chrome_toggle" => {
                let method = match handler {
                    EventHandler::Ident(m) | EventHandler::MethodName(m) => m.as_str(),
                    EventHandler::WithArgs(m, _) => m.as_str(),
                };
                code.push_str(&format!(
                    ".on_chrome_toggle({{\n                    \
                     let weak = cx.weak_entity();\n                    \
                     move |_window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
                     if let Some(entity) = weak.upgrade() {{\n                            \
                     entity.update(app, |this, cx| {{ this.{}(cx); }});\n                        \
                     }}\n                    }}\n                }})",
                    method
                ));
            }
            _ => {}
        }
    }

    if let Some(left) = slot_left {
        code.push_str(&format!(".slot_left(Some({left}))"));
    }
    if let Some(right) = slot_right {
        code.push_str(&format!(".slot_right(Some({right}))"));
    }
    if let Some(bottom) = slot_bottom {
        code.push_str(&format!(".slot_bottom(Some({bottom}))"));
    }

    code.push_str(&format!(".child({})", children_body));
    Ok(code)
}

/// 将 shell 根元素的 bind 表达式编译为 Rust 代码
fn shell_bind_expr(expr: &str, computed: &[&str], loop_vars: &[&str]) -> String {
    let trimmed = expr.trim();
    if computed.iter().any(|c| *c == trimmed) {
        return format!("self.{}()", trimmed);
    }
    match expr::parse(expr) {
        Ok(expr::Expr::Field(field_name)) if computed.iter().any(|c| *c == field_name.as_str()) => {
            format!("self.{}()", field_name)
        }
        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, loop_vars),
        Err(_) => {
            if computed.iter().any(|c| *c == trimmed) {
                format!("self.{}()", trimmed)
            } else {
                format!("self.{}", trimmed)
            }
        }
    }
}
