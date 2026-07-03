//! 窗口外壳包裹代码生成
//!
//! - `<modern_window>` → `ModernWindowShell` 包裹
//! - `<tab_window>` → `TabWindowShell` 包裹 + 插槽分区
//!
//! ## Slot 语法（Vue 风格）
//!
//! shell 根元素子节点中，形如 `<template slot="name">...</template>` 的块
//! 会被 `partition_slot_children` 拆分到对应 slot setter：
//! - `<template slot="menu">` → `.menu_slot(...)`
//! - `<template slot="title">` → `.title_ext_slot(...)`
//! - `<template slot="footer">` → `.status_slot(...)`
//! - `<template slot="left">` → `.slot_left(...)`（仅 tab_window）
//! - `<template slot="right">` → `.slot_right(...)`（仅 tab_window）
//! - `<template slot="bottom">` → `.slot_bottom(...)`（仅 tab_window）
//! - 其他子节点 → 主内容（`.child(...)`）

use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler, Node};

/// 为 `<modern_window>` 根元素生成 ModernWindowShell 包裹代码
///
/// - title 复用 IWindow::title()，不重复定义
/// - menu/footer/icon 从根元素 Attribute::Bind 提取，使用表达式解析器处理 computed 方法
/// - `<template slot="menu/title/footer">` 从子节点插槽提取
pub(super) fn gen_modern_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
    slot_menu: Option<&str>,
    slot_title: Option<&str>,
    slot_footer: Option<&str>,
) -> Result<String, CodegenError> {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let empty: Vec<&str> = Vec::new();

    let mut code = String::from("rml_ui::ModernWindowShell::new()");
    code.push_str(".title(self.title().to_string())");

    for attr in &elem.attributes {
        if let Attribute::Bind { name, expr } = attr {
            match name.as_str() {
                "menu" | "footer" => {
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
                        "menu" => code.push_str(&format!(".menu_slot({})", rust_expr)),
                        "footer" => {
                            code.push_str(&format!(".status_slot({})", rust_expr))
                        }
                        _ => {
                            if crate::compiler::props_registry::is_shell_prop_registered("modern_window", name) {
                                eprintln!(
                                    "[rml warning] <modern_window> bind property `{}` is registered in SHELL_PROPS \
                                     but has no mapping in gen_modern_window_wrapper; property will be silently dropped. \
                                     Add a match arm in crates/engine/src/compiler/codegen/shell.rs.",
                                    name
                                );
                            }
                        }
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

    if let Some(menu) = slot_menu {
        code.push_str(&format!(".menu_slot({menu})"));
    }
    if let Some(title) = slot_title {
        code.push_str(&format!(".title_ext_slot({title})"));
    }
    if let Some(footer) = slot_footer {
        code.push_str(&format!(".status_slot({footer})"));
    }

    code.push_str(&format!(".child({})", children_body));
    Ok(code)
}

/// 将 shell 根元素子节点拆分为插槽与主内容
///
/// 识别 Vue 风格 `<template slot="name">...</template>` 形式：
/// - `<template slot="menu">` → slot_menu
/// - `<template slot="title">` → slot_title
/// - `<template slot="footer">` → slot_footer
/// - `<template slot="left">` → slot_left（仅 tab_window）
/// - `<template slot="right">` → slot_right（仅 tab_window）
/// - `<template slot="bottom">` → slot_bottom（仅 tab_window）
/// - `<template slot="tabs">` → slot_tabs（仅 tab_window，收集所有子节点而非单一 content）
/// - 其他子节点（含无 slot 属性的 `<template>`）→ body 主内容
///
/// 返回 (menu, title, footer, left, right, bottom, tabs, body)
pub(super) fn partition_slot_children(
    children: &[Node],
) -> (
    Option<Node>,
    Option<Node>,
    Option<Node>,
    Option<Node>,
    Option<Node>,
    Option<Node>,
    Vec<Node>,
    Vec<Node>,
) {
    let mut slot_menu = None;
    let mut slot_title = None;
    let mut slot_footer = None;
    let mut slot_left = None;
    let mut slot_right = None;
    let mut slot_bottom = None;
    let mut slot_tabs = Vec::new();
    let mut body = Vec::new();

    for child in children {
        if let Node::Element(elem) = child {
            // 仅识别 `<template slot="name">` 形式
            if elem.tag == "template" {
                if let Some(name) = &elem.slot_name {
                    let content = template_block_content(elem);
                    match name.as_str() {
                        "menu" => {
                            slot_menu = content;
                            continue;
                        }
                        "title" => {
                            slot_title = content;
                            continue;
                        }
                        "footer" => {
                            slot_footer = content;
                            continue;
                        }
                        "left" => {
                            slot_left = content;
                            continue;
                        }
                        "right" => {
                            slot_right = content;
                            continue;
                        }
                        "bottom" => {
                            slot_bottom = content;
                            continue;
                        }
                        "tabs" => {
                            // tabs slot 收集所有子节点（应为 <Tab> 元素），
                            // 而非取单一 content —— 与其他单 Node slot 不同。
                            let tab_kids: Vec<Node> = elem.children.iter().cloned().collect();
                            if !tab_kids.is_empty() {
                                slot_tabs = tab_kids;
                            }
                            continue;
                        }
                        _ => {
                            // 未知 slot 名：忽略并落入 body（validator 应在编译期拦截）
                        }
                    }
                }
            }
        }
        body.push(child.clone());
    }

    (
        slot_menu,
        slot_title,
        slot_footer,
        slot_left,
        slot_right,
        slot_bottom,
        slot_tabs,
        body,
    )
}

/// 取 `<template slot="...">` 块的内部内容
///
/// - 单子节点：直接 unwrap（避免多余 div 包装）
/// - 多子节点：包 `<div>` 作为容器
/// - 无子节点：返回 None
fn template_block_content(elem: &Element) -> Option<Node> {
    match elem.children.len() {
        0 => None,
        1 => Some(elem.children[0].clone()),
        _ => Some(Node::Element(Element {
            tag: "div".into(),
            attributes: vec![],
            directives: vec![],
            children: elem.children.clone(),
            slot_name: None,
            ..Default::default()
        })),
    }
}

/// 从根 `<tab_window>` 的 bind/event 属性生成 `TabWindowShell` 包裹代码
///
/// slot 参数命名与 `<template slot="name">` 的 name 一一对应：
/// - slot_menu / slot_title / slot_footer / slot_left / slot_right / slot_bottom / slot_tabs
///
/// 注意：slot_footer 在 builder 端映射到 `.status_slot(...)`，
/// 因为 TabWindowShell 的 footer slot 装入 gpui-component 的 status_bar 控件。
///
/// `slot_tabs` 为模板定制模式：每个元素是一个 `<Tab>` 子节点的 codegen 输出，
/// 生成 `.tab_children(vec![<Tab1>, <Tab2>, ...])`。
/// 与 `tabs={Vec<TabItem>}` 简单模式互斥（编译期校验）。
pub(super) fn gen_tab_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
    slot_menu: Option<&str>,
    slot_title: Option<&str>,
    slot_footer: Option<&str>,
    slot_left: Option<&str>,
    slot_right: Option<&str>,
    slot_bottom: Option<&str>,
    slot_tabs: Option<&[String]>,
) -> Result<String, CodegenError> {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let empty: Vec<&str> = Vec::new();

    let mut code = String::from("rml_ui::TabWindowShell::new()");
    code.push_str(".title(self.title().to_string())");

    // 互斥校验：`tabs={...}` bind 属性与 `<template slot="tabs">` 插槽不能并存
    let has_tabs_bind = elem.attributes.iter().any(|a| {
        matches!(a, Attribute::Bind { name, .. } if name == "tabs")
    });
    let has_slot_tabs = slot_tabs.map_or(false, |t| !t.is_empty());
    if has_tabs_bind && has_slot_tabs {
        return Err(CodegenError {
            message: "<tab_window> 不能同时使用 `tabs={...}` 属性和 `<template slot=\"tabs\">` 插槽".into(),
        });
    }

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
                    "menu" => code.push_str(&format!(".menu_slot({})", rust_expr)),
                    "footer" => code.push_str(&format!(".status_slot(Some({}))", rust_expr)),
                    "tabs" => code.push_str(&format!(".tabs({}.clone())", rust_expr)),
                    "selected_tab" => code.push_str(&format!(".selected_tab({})", rust_expr)),
                    "show_chrome" => code.push_str(&format!(".show_chrome({})", rust_expr)),
                    "left_size" => code.push_str(&format!(".left_size({})", rust_expr)),
                    "right_size" => code.push_str(&format!(".right_size({})", rust_expr)),
                    "bottom_size" => code.push_str(&format!(".bottom_size({})", rust_expr)),
                    _ => {
                        if crate::compiler::props_registry::is_shell_prop_registered("tab_window", name) {
                            eprintln!(
                                "[rml warning] <tab_window> bind property `{}` is registered in SHELL_PROPS \
                                 but has no mapping in gen_tab_window_wrapper; property will be silently dropped. \
                                 Add a match arm in crates/engine/src/compiler/codegen/shell.rs.",
                                name
                            );
                        }
                    }
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

    if let Some(menu) = slot_menu {
        code.push_str(&format!(".menu_slot({menu})"));
    }
    if let Some(title) = slot_title {
        code.push_str(&format!(".title_ext_slot({title})"));
    }
    if let Some(footer) = slot_footer {
        code.push_str(&format!(".status_slot(Some({footer}))"));
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
    if let Some(tabs) = slot_tabs {
        if !tabs.is_empty() {
            let joined = tabs.join(", ");
            code.push_str(&format!(".tab_children(vec![{}])", joined));
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, Node};

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_template_slot(slot_name: &str, children: Vec<Node>) -> Element {
        Element {
            tag: "template".into(),
            attributes: vec![],
            directives: vec![],
            children,
            slot_name: Some(slot_name.into()),
            ..Default::default()
        }
    }

    fn make_tab(label: &str) -> Element {
        Element {
            tag: "Tab".into(),
            attributes: vec![Attribute::Static {
                name: "label".into(),
                value: label.into(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        }
    }

    /// `<template slot="tabs"><Tab /><Tab /></template>` 正确分到 slot_tabs
    #[test]
    fn partition_slot_children_extracts_tabs_slot() {
        let template = make_template_slot(
            "tabs",
            vec![Node::Element(make_tab("A")), Node::Element(make_tab("B"))],
        );
        let children = vec![Node::Element(template)];
        let (_, _, _, _, _, _, slot_tabs, body) =
            partition_slot_children(&children);
        assert_eq!(slot_tabs.len(), 2);
        assert!(body.is_empty());
    }

    /// `<template slot="tabs"></template>`（空）不应设置 slot_tabs
    #[test]
    fn partition_slot_children_empty_tabs_slot() {
        let template = make_template_slot("tabs", vec![]);
        let children = vec![Node::Element(template)];
        let (_, _, _, _, _, _, slot_tabs, body) =
            partition_slot_children(&children);
        assert!(slot_tabs.is_empty());
        // 空的 tabs slot 不应落入 body
        assert!(body.is_empty());
    }

    /// 其他 slot（如 menu）仍正常工作，且 tabs 与之并存
    #[test]
    fn partition_slot_children_tabs_alongside_other_slots() {
        let menu_tmpl = make_template_slot("menu", vec![Node::Text("Menu".into())]);
        let tabs_tmpl = make_template_slot(
            "tabs",
            vec![Node::Element(make_tab("X"))],
        );
        let children = vec![Node::Element(menu_tmpl), Node::Element(tabs_tmpl)];
        let (slot_menu, _, _, _, _, _, slot_tabs, body) =
            partition_slot_children(&children);
        assert!(slot_menu.is_some());
        assert_eq!(slot_tabs.len(), 1);
        assert!(body.is_empty());
    }

    /// `gen_tab_window_wrapper` 生成 `.tab_children(vec![...])`
    #[test]
    fn gen_tab_window_wrapper_with_slot_tabs() {
        let elem = Element {
            tag: "tab_window".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let tabs_codes = vec![
            "rml_ui::Tab::new().label(\"A\")".to_string(),
            "rml_ui::Tab::new().label(\"B\")".to_string(),
        ];
        let code = gen_tab_window_wrapper(
            &elem,
            &ctx(),
            "gpui::div()",
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&tabs_codes),
        )
        .unwrap();
        assert!(code.contains(".tab_children(vec!["));
        assert!(code.contains("rml_ui::Tab::new().label(\"A\")"));
        assert!(code.contains("rml_ui::Tab::new().label(\"B\")"));
    }

    /// `tabs={...}` 与 `<template slot="tabs">` 并存报错
    #[test]
    fn gen_tab_window_wrapper_tabs_mutual_exclusion_error() {
        let elem = Element {
            tag: "tab_window".into(),
            attributes: vec![Attribute::Bind {
                name: "tabs".into(),
                expr: "tab_items".into(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let tabs_codes = vec!["rml_ui::Tab::new()".to_string()];
        let result = gen_tab_window_wrapper(
            &elem,
            &ctx(),
            "gpui::div()",
            None,
            None,
            None,
            None,
            None,
            None,
            Some(&tabs_codes),
        );
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .message
            .contains("不能同时使用"));
    }

    /// 无 slot_tabs 时正常生成（不输出 .tab_children）
    #[test]
    fn gen_tab_window_wrapper_without_slot_tabs() {
        let elem = Element {
            tag: "tab_window".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let code = gen_tab_window_wrapper(
            &elem,
            &ctx(),
            "gpui::div()",
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .unwrap();
        assert!(!code.contains(".tab_children"));
    }
}

