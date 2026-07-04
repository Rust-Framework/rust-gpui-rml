//! 窗口外壳包裹代码生成
//!
//! - `<modern-window>` → `ModernWindowShell` 包裹
//! - `<tab-window>` → `TabWindowShell` 包裹 + 插槽分区
//!
//! ## Slot 语法（Vue 风格）
//!
//! shell 根元素子节点中，形如 `<template slot="name">...</template>` 的块
//! 会被 `partition_slot_children` 拆分到对应 slot setter：
//! - `<template slot="menu">` → `.menu_slot(...)`
//! - `<template slot="title">` → `.title_ext_slot(...)`
//! - `<template slot="footer">` → `.status_slot(...)`
//! - `<template slot="left">` → `.slot_left(...)`（仅 tab-window）
//! - `<template slot="right">` → `.slot_right(...)`（仅 tab-window）
//! - `<template slot="bottom">` → `.slot_bottom(...)`（仅 tab-window）
//! - 其他子节点 → 主内容（`.child(...)`）

use crate::compiler::expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler, Node};

/// 为 `<modern-window>` 根元素生成 ModernWindowShell 包裹代码
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
                            if computed.contains(&field_name.as_str()) =>
                        {
                            format!("self.{}()", field_name)
                        }
                        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, &empty),
                        Err(_) => {
                            let trimmed = expr.trim();
                            if computed.contains(&trimmed) {
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
                            if crate::compiler::props_registry::is_shell_prop_registered("modern-window", name) {
                                eprintln!(
                                    "[rml warning] <modern-window> bind property `{}` is registered in SHELL_PROPS \
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
                            if computed.contains(&field_name.as_str()) =>
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

/// Shell 根元素子节点按 slot 名分区后的结果
///
/// 由 [`partition_slot_children`] 产出。各字段对应 `<template slot="name">`：
/// - `menu` / `title` / `footer`：modern-window 与 tab-window 共有
/// - `left` / `right` / `bottom`：仅 tab-window
/// - `tabs`：仅 tab-window，收集所有 `<Tab>` 子节点而非单一 content
/// - `body`：主内容（无 slot 属性的子节点）
#[derive(Default)]
pub(super) struct ShellSlots {
    pub menu: Option<Node>,
    pub title: Option<Node>,
    pub footer: Option<Node>,
    pub left: Option<Node>,
    pub right: Option<Node>,
    pub bottom: Option<Node>,
    pub tabs: Vec<Node>,
    pub body: Vec<Node>,
}

/// 将 shell 根元素子节点拆分为插槽与主内容
///
/// 识别 Vue 风格 `<template slot="name">...</template>` 形式：
/// - `<template slot="menu">` → `ShellSlots::menu`
/// - `<template slot="title">` → `ShellSlots::title`
/// - `<template slot="footer">` → `ShellSlots::footer`
/// - `<template slot="left">` → `ShellSlots::left`（仅 tab-window）
/// - `<template slot="right">` → `ShellSlots::right`（仅 tab-window）
/// - `<template slot="bottom">` → `ShellSlots::bottom`（仅 tab-window）
/// - `<template slot="tabs">` → `ShellSlots::tabs`（仅 tab-window，收集所有子节点而非单一 content）
/// - 其他子节点（含无 slot 属性的 `<template>`）→ `ShellSlots::body` 主内容
pub(super) fn partition_slot_children(children: &[Node]) -> ShellSlots {
    let mut slots = ShellSlots::default();

    for child in children {
        if let Node::Element(elem) = child {
            if elem.tag == "template" {
                if let Some(name) = &elem.slot_name {
                    match name.as_str() {
                        "menu" => slots.menu = template_block_content(elem),
                        "title" => slots.title = template_block_content(elem),
                        "footer" => slots.footer = template_block_content(elem),
                        "left" => slots.left = template_block_content(elem),
                        "right" => slots.right = template_block_content(elem),
                        "bottom" => slots.bottom = template_block_content(elem),
                        "tabs" => {
                            // tabs slot 收集所有子节点（应为 <Tab> 元素），
                            // 而非取单一 content —— 与其他单 Node slot 不同。
                            let tab_kids: Vec<Node> = elem.children.to_vec();
                            if !tab_kids.is_empty() {
                                slots.tabs = tab_kids;
                            }
                        }
                        _ => {
                            // 未知 slot 名：落入 body（validator 应在编译期拦截）
                            slots.body.push(child.clone());
                        }
                    }
                    continue;
                }
            }
        }
        slots.body.push(child.clone());
    }

    slots
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

/// `<tab-window>` 各 slot 的 codegen 输出（由 render.rs 从 [`ShellSlots`] 生成）
///
/// 字段命名与 `<template slot="name">` 一一对应。`tabs` 为模板定制模式下
/// 各 `<Tab>` 子节点的 codegen 输出列表，与 `tabs={Vec<TabItem>}` 简单模式互斥。
#[derive(Default)]
pub(super) struct TabWindowSlotCodes<'a> {
    pub menu: Option<&'a str>,
    pub title: Option<&'a str>,
    pub footer: Option<&'a str>,
    pub left: Option<&'a str>,
    pub right: Option<&'a str>,
    pub bottom: Option<&'a str>,
    pub tabs: Option<&'a [String]>,
}

/// 从根 `<tab-window>` 的 bind/event 属性生成 `TabWindowShell` 包裹代码
///
/// slot 参数命名与 `<template slot="name">` 的 name 一一对应：
/// - slot_menu / slot_title / slot_footer / slot_left / slot_right / slot_bottom / slot_tabs
///
/// 注意：slot_footer 在 builder 端映射到 `.status_slot(...)`，
/// 因为 TabWindowShell 的 footer slot 装入 gpui-component 的 status-bar 控件。
///
/// `slot_tabs` 为模板定制模式：每个元素是一个 `<Tab>` 子节点的 codegen 输出，
/// 生成 `.tab_children(vec![<Tab1>, <Tab2>, ...])`。
/// 与 `tabs={Vec<TabItem>}` 简单模式互斥（编译期校验）。
pub(super) fn gen_tab_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
    slots: TabWindowSlotCodes,
) -> Result<String, CodegenError> {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let empty: Vec<&str> = Vec::new();

    let mut code = String::from("rml_ui::TabWindowShell::new()");
    code.push_str(".title(self.title().to_string())");

    // 互斥校验：`tabs={...}` bind 属性与 `<template slot="tabs">` 插槽不能并存
    let has_tabs_bind = elem.attributes.iter().any(|a| {
        matches!(a, Attribute::Bind { name, .. } if name == "tabs")
    });
    let has_slot_tabs = slots.tabs.is_some_and(|t| !t.is_empty());
    if has_tabs_bind && has_slot_tabs {
        return Err(CodegenError {
            message: "<tab-window> 不能同时使用 `tabs={...}` 属性和 `<template slot=\"tabs\">` 插槽".into(),
        });
    }

    for attr in &elem.attributes {
        match attr {
            Attribute::Bind { name, expr } => {
                if name == "icon" {
                    let rust_expr = match expr::parse(expr) {
                        Ok(expr::Expr::Field(field_name))
                            if computed.contains(&field_name.as_str()) =>
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
                if name == "tab_item_template" {
                    // `tab_item_template={method}` 是方法名（非字段），
                    // 需包装为 4 参闭包：把业务数据 &Box<dyn Any> 渲染为 TabItem。
                    // setter 签名 `tab_item_template<F: Fn(...) -> TabItem + Send + Sync + 'static>`
                    // 内部已做 Arc::new，这里传裸闭包即可。
                    let method = expr.trim();
                    code.push_str(&format!(
                        ".tab_item_template({{\n                    \
                         let weak = cx.weak_entity();\n                    \
                         move |ix: usize, data: &Box<dyn std::any::Any>, \
                         window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
                         if let Some(entity) = weak.upgrade() {{\n                            \
                         entity.update(app, |this, cx| this.{}(ix, data, window, cx))\n                        \
                         }} else {{\n                            \
                         rml_ui::TabItem::new()\n                        \
                         }}\n                    }})\n                }})",
                        method
                    ));
                    continue;
                }
                let rust_expr = shell_bind_expr(expr, &computed, &empty);
                match name.as_str() {
                    "menu" => code.push_str(&format!(".menu_slot({})", rust_expr)),
                    "footer" => code.push_str(&format!(".status_slot(Some({}))", rust_expr)),
                    "tabs" => code.push_str(&format!(".tabs({})", rust_expr)),
                    "selected_index" => code.push_str(&format!(".selected_index({})", rust_expr)),
                    "show_chrome" => code.push_str(&format!(".show_chrome({})", rust_expr)),
                    "left_size" => code.push_str(&format!(".left_size({})", rust_expr)),
                    "right_size" => code.push_str(&format!(".right_size({})", rust_expr)),
                    "bottom_size" => code.push_str(&format!(".bottom_size({})", rust_expr)),
                    _ => {
                        if crate::compiler::props_registry::is_shell_prop_registered("tab-window", name) {
                            eprintln!(
                                "[rml warning] <tab-window> bind property `{}` is registered in SHELL_PROPS \
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

    if let Some(menu) = slots.menu {
        code.push_str(&format!(".menu_slot({menu})"));
    }
    if let Some(title) = slots.title {
        code.push_str(&format!(".title_ext_slot({title})"));
    }
    if let Some(footer) = slots.footer {
        code.push_str(&format!(".status_slot(Some({footer}))"));
    }
    if let Some(left) = slots.left {
        code.push_str(&format!(".slot_left(Some({left}))"));
    }
    if let Some(right) = slots.right {
        code.push_str(&format!(".slot_right(Some({right}))"));
    }
    if let Some(bottom) = slots.bottom {
        code.push_str(&format!(".slot_bottom(Some({bottom}))"));
    }
    if let Some(tabs) = slots.tabs {
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
    if computed.contains(&trimmed) {
        return format!("self.{}()", trimmed);
    }
    match expr::parse(expr) {
        Ok(expr::Expr::Field(field_name)) if computed.contains(&field_name.as_str()) => {
            format!("self.{}()", field_name)
        }
        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, loop_vars),
        Err(_) => {
            if computed.contains(&trimmed) {
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
        let slots = partition_slot_children(&children);
        assert_eq!(slots.tabs.len(), 2);
        assert!(slots.body.is_empty());
    }

    /// `<template slot="tabs"></template>`（空）不应设置 slot_tabs
    #[test]
    fn partition_slot_children_empty_tabs_slot() {
        let template = make_template_slot("tabs", vec![]);
        let children = vec![Node::Element(template)];
        let slots = partition_slot_children(&children);
        assert!(slots.tabs.is_empty());
        // 空的 tabs slot 不应落入 body
        assert!(slots.body.is_empty());
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
        let slots = partition_slot_children(&children);
        assert!(slots.menu.is_some());
        assert_eq!(slots.tabs.len(), 1);
        assert!(slots.body.is_empty());
    }

    /// `gen_tab_window_wrapper` 生成 `.tab_children(vec![...])`
    #[test]
    fn gen_tab_window_wrapper_with_slot_tabs() {
        let elem = Element {
            tag: "tab-window".into(),
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
            TabWindowSlotCodes {
                tabs: Some(&tabs_codes),
                ..Default::default()
            },
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
            tag: "tab-window".into(),
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
            TabWindowSlotCodes {
                tabs: Some(&tabs_codes),
                ..Default::default()
            },
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
            tag: "tab-window".into(),
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
            TabWindowSlotCodes::default(),
        )
        .unwrap();
        assert!(!code.contains(".tab_children"));
    }

    /// `tab_item_template={render_tab_item}` 生成 4 参裸闭包（无 Arc::new 双重包裹）
    #[test]
    fn tab_item_template_generates_bare_closure_without_arc() {
        let elem = Element {
            tag: "tab-window".into(),
            attributes: vec![Attribute::Bind {
                name: "tab_item_template".into(),
                expr: "render_tab_item".into(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let code = gen_tab_window_wrapper(
            &elem,
            &ctx(),
            "gpui::div()",
            TabWindowSlotCodes::default(),
        )
        .unwrap();
        // 应生成 .tab_item_template({ let weak = cx.weak_entity(); move |ix...| ... })
        assert!(code.contains(".tab_item_template("), "missing tab_item_template call");
        assert!(code.contains("move |ix: usize"), "missing 4-param closure");
        assert!(
            !code.contains("std::sync::Arc::new"),
            "must not double-wrap with Arc::new (setter does it internally)"
        );
        // 闭包体内应调用 render_tab_item 方法
        assert!(code.contains("this.render_tab_item(ix, data, window, cx)"));
        // else 分支应回退到 TabItem::new()
        assert!(code.contains("rml_ui::TabItem::new()"));
    }

    /// `tabs={tab_bar_items}` 当 tab_bar_items 是 #[computed] 方法时生成方法调用
    #[test]
    fn shell_bind_tabs_computed_method_generates_call() {
        let mut c = ctx();
        c.computed_methods = vec!["tab_bar_items".to_string()];
        let elem = Element {
            tag: "tab-window".into(),
            attributes: vec![Attribute::Bind {
                name: "tabs".into(),
                expr: "tab_bar_items".into(),
            }],
            directives: vec![],
            children: vec![],
            slot_name: None,
            ..Default::default()
        };
        let code = gen_tab_window_wrapper(
            &elem,
            &c,
            "gpui::div()",
            TabWindowSlotCodes::default(),
        )
        .unwrap();
        // computed 方法应生成 self.tab_bar_items()（带括号）
        assert!(
            code.contains(".tabs(self.tab_bar_items())"),
            "computed method must generate call with (), got: {code}"
        );
    }
}

