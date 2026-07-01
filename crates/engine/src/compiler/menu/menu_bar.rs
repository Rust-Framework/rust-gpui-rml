//! `MenuBar` 声明式 / `items` 绑定 codegen
//!
//! 顶层容器统一生成为 `rml_ui::MenuBar`（ui crate 组件）；`<menu-item>` 子节点编译为
//! `menu_bar_button` + `PopupMenu` 后作为 `MenuBar` 的 children。
//!
//! `<context-menu>` / `<dropdown-menu>` 不在此处理，仍直译 gpui-component 弹层 API。

use crate::compiler::menu::hoist::MenuHoist;
use crate::compiler::menu::item::{gen_menu_item_stmt, is_menu_item_tag};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};

pub fn gen_menu_bar(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let bar_id = *id_counter;
    *id_counter += 1;

    // items 绑定路径 → MenuBar MVVM
    if let Some(items_expr) = elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name, expr } if name == "items" => Some(expr.clone()),
        _ => None,
    }) {
        let rust_expr = format!("self.{}", items_expr);
        return Ok(format!(
            "rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize)).items({rust_expr}.clone())"
        ));
    }

    // 声明式：顶层 MenuItem → MenuBar children（按钮 + dropdown）
    let top_items: Vec<&Element> = elem
        .children
        .iter()
        .filter_map(|c| match c {
            Node::Element(e) if is_menu_item_tag(&e.tag) => Some(e),
            _ => None,
        })
        .collect();

    if top_items.is_empty() {
        return Ok(format!(
            "rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize))"
        ));
    }

    let mut btn_codes = Vec::new();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    for (ix, item) in top_items.iter().enumerate() {
        let label = item
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Static { name, value } if name == "label" => {
                    Some(format!("{value:?}"))
                }
                Attribute::Bind { name, expr } if name == "label" => {
                    if let Some(code) =
                        crate::compiler::codegen::try_gen_i18n_call(expr, &lv, &computed)
                    {
                        Some(code)
                    } else {
                        Some(crate::compiler::codegen::gen_expr_code(expr, &lv, &computed))
                    }
                }
                _ => None,
            })
            .unwrap_or_else(|| "\"\"".to_string());

        let menu_children: Vec<&Element> = item
            .children
            .iter()
            .filter_map(|c| match c {
                Node::Element(e) if is_menu_item_tag(&e.tag) => Some(e),
                _ => None,
            })
            .collect();

        if menu_children.is_empty() {
            btn_codes.push(format!(
                "rml_ui::menu_bar_button((\"rml_menu_bar\", {ix}usize), {label})"
            ));
        } else {
            let mut hoist = MenuHoist::default();
            hoist.collect_menu_items(&menu_children, ctx, loop_vars)?;
            let hoist_lets = hoist.gen_lets(ctx);
            let mut inner_id = *id_counter;
            let mut stmts = Vec::new();
            stmts.push("let mut menu = rml_ui::configure_menu_bar_popup(menu);".to_string());
            for child in &menu_children {
                stmts.push(gen_menu_item_stmt(
                    child,
                    ctx,
                    depth + 1,
                    &mut inner_id,
                    loop_vars,
                    &hoist,
                )?);
            }
            stmts.push("menu".to_string());
            let body = stmts.join("\n                        ");
            *id_counter = inner_id;
            let button = format!(
                "rml_ui::menu_bar_button((\"rml_menu_bar\", {ix}usize), {label})\n                .dropdown_menu(move |menu, window, cx| {{\n                    {body}\n                }})"
            );
            if hoist_lets.is_empty() {
                btn_codes.push(button);
            } else {
                btn_codes.push(format!(
                    "{{\n                {hoist_lets}\n                {button}\n            }}"
                ));
            }
        }
    }

    Ok(format!(
        "{{\n            let __rml_menu_weak = cx.weak_entity();\n            rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize)).children(vec![{}])\n        }}",
        btn_codes.join(", ")
    ))
}
