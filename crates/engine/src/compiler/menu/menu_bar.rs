//! `MenuBar` 声明式 / `each` 迭代 codegen
//!
//! 顶层容器统一生成为 `rml_ui::MenuBar`（ui crate 组件）；`<menu-item>` 子节点编译为
//! `menu_bar_button` + `PopupMenu` 后作为 `MenuBar` 的 children。
//!
//! 三条路径：
//! 1. **`each` 迭代**（MVVM）：`<menu-item each={m in menus}>` → 运行时 `self.menus.iter().map(...)`
//! 2. **声明式**：编译期遍历 `<MenuItem>` 子节点，生成静态按钮树
//! 3. **空**：无子节点
//!
//! `<context-menu>` / `<dropdown-menu>` 不在此处理，仍直译 gpui-component 弹层 API。

use crate::compiler::menu::hoist::MenuHoist;
use crate::compiler::menu::item::{gen_menu_item_stmt, is_menu_item_tag};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, Node};

pub fn gen_menu_bar(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let bar_id = *id_counter;
    *id_counter += 1;

    // MVVM items 绑定路径：<menu-bar items={menus} />
    // 生成 rml_ui::render_menu_bar_from_items(self.menus.clone())
    if let Some(items_expr) = elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name, expr } if name == "items" => Some(expr.as_str()),
        _ => None,
    }) {
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        let rust_expr = crate::compiler::component::component_bind_rust_expr(
            items_expr,
            &lv,
            &computed,
        );
        return Ok(format!(
            "rml_ui::render_menu_bar_from_items({}.clone())",
            rust_expr
        ));
    }

    // 收集 MenuItem 子节点
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

    // 检测第一个 top_item 是否带 each 指令
    let each_clause = top_items.first().and_then(|item| {
        item.directives.iter().find_map(|d| match d {
            Directive::Each(c) => Some(c.clone()),
            _ => None,
        })
    });

    if let Some(clause) = each_clause {
        // each 迭代路径：运行时 map 生成按钮
        let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
        child_loop_vars.push(clause.item.clone());

        let button_code = gen_menu_bar_button_for_item(
            top_items.first().unwrap(),
            ctx,
            depth,
            id_counter,
            &child_loop_vars,
        )?;

        // iterable 可能是 self.field 或 loop_var.field
        let iter_expr = if loop_vars
            .iter()
            .any(|lv| clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv)))
        {
            clause.iterable.clone()
        } else {
            format!("self.{}", clause.iterable)
        };

        let iter_code = format!(
            "{iter_expr}.iter().map(|{}| {{\n                {}\n            }})",
            clause.item, button_code
        );

        return Ok(format!(
            "{{\n            let __rml_menu_weak = cx.weak_entity();\n            rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize)).children({})\n        }}",
            iter_code
        ));
    }

    // 声明式路径：编译期展开每个 top_item
    let mut btn_codes = Vec::new();
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    for (ix, item) in top_items.iter().enumerate() {
        let button_code = gen_menu_bar_button_static(item, ix, ctx, depth, id_counter, &lv)?;
        btn_codes.push(button_code);
    }

    Ok(format!(
        "{{\n            let __rml_menu_weak = cx.weak_entity();\n            rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize)).children(vec![{}])\n        }}",
        btn_codes.join(", ")
    ))
}

/// 为 `each` 模板项生成按钮代码（运行时迭代上下文，loop_vars 已含迭代变量）。
///
/// 支持叶子节点（`command={c.command}`）和子菜单（`<menu-item each={c in m.children}>`）。
fn gen_menu_bar_button_for_item(
    item: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let label = bind_label_or_static(item, ctx, loop_vars)?;
    let menu_children: Vec<&Element> = item
        .children
        .iter()
        .filter_map(|c| match c {
            Node::Element(e) if is_menu_item_tag(&e.tag) => Some(e),
            _ => None,
        })
        .collect();

    if menu_children.is_empty() {
        // 叶子节点：检查 command 绑定
        let has_command = item.attributes.iter().any(|a| matches!(a, Attribute::Bind { name, .. } if name == "command"));
        if has_command {
            // command 绑定由 gen_menu_item_stmt 处理，但 menu_bar_button 不走 PopupMenu
            // 这里生成带 on_click 的按钮
            let cmd_expr = item
                .attributes
                .iter()
                .find_map(|a| match a {
                    Attribute::Bind { name, expr } if name == "command" => Some(expr.clone()),
                    _ => None,
                })
                .unwrap();
            let onclick = gen_top_button_onclick(&cmd_expr, ctx, loop_vars)?;
            Ok(format!(
                "rml_ui::menu_bar_button((\"rml_menu_bar\", 0usize), {label}){onclick}"
            ))
        } else {
            Ok(format!(
                "rml_ui::menu_bar_button((\"rml_menu_bar\", 0usize), {label})"
            ))
        }
    } else {
        // 子菜单：生成 dropdown_menu 闭包
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
            "rml_ui::menu_bar_button((\"rml_menu_bar\", 0usize), {label})\n                .dropdown_menu(move |menu, window, cx| {{\n                    {body}\n                }})"
        );
        if hoist_lets.is_empty() {
            Ok(button)
        } else {
            Ok(format!(
                "{{\n                {hoist_lets}\n                {button}\n            }}"
            ))
        }
    }
}

/// 生成顶层按钮的 `on_click` 闭包（`each` 叶子节点 `command={c.command}` 绑定）。
///
/// 与 `item.rs::gen_command_closure` loop_var 路径一致，但不经 PopupMenu。
fn gen_top_button_onclick(
    cmd_expr: &str,
    ctx: &CodegenCtx,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let access = if let Some(code) =
        crate::compiler::codegen::try_gen_i18n_call(cmd_expr, &lv, &computed)
    {
        code
    } else {
        crate::compiler::expr::parse(cmd_expr)
            .map(|p| crate::compiler::expr::to_rust_code_with_ctx(&p, &lv))
            .unwrap_or_else(|_| cmd_expr.to_string())
    };
    Ok(format!(
        ".on_click({{\n                        let __rml_cmd = {access}.clone();\n                        move |_ev, window, app| {{\n                            if let Some(cmd) = &__rml_cmd {{\n                                let mut __rml_ctx = rml_core::command::CallContext::new(window, app);\n                                if cmd.can_execute(&mut __rml_ctx) {{\n                                    cmd.execute(&mut __rml_ctx);\n                                }}\n                            }}\n                        }}\n                    }})"
    ))
}

/// 声明式路径：为单个 top_item 生成按钮代码（编译期展开，无 each）。
fn gen_menu_bar_button_static(
    item: &Element,
    ix: usize,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[&str],
) -> Result<String, CodegenError> {
    let label = item
        .attributes
        .iter()
        .find_map(|a| match a {
            Attribute::Static { name, value } if name == "label" => Some(format!("{value:?}")),
            Attribute::Bind { name, expr } if name == "label" => {
                if let Some(code) =
                    crate::compiler::codegen::try_gen_i18n_call(expr, loop_vars, &ctx.computed_methods.iter().map(|s| s.as_str()).collect::<Vec<_>>())
                {
                    Some(code)
                } else {
                    Some(crate::compiler::codegen::gen_expr_code(expr, loop_vars, &ctx.computed_methods.iter().map(|s| s.as_str()).collect::<Vec<_>>()))
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
        Ok(format!(
            "rml_ui::menu_bar_button((\"rml_menu_bar\", {ix}usize), {label})"
        ))
    } else {
        let mut hoist = MenuHoist::default();
        let lv_owned: Vec<String> = loop_vars.iter().map(|s| s.to_string()).collect();
        hoist.collect_menu_items(&menu_children, ctx, &lv_owned)?;
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
                &lv_owned,
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
            Ok(button)
        } else {
            Ok(format!(
                "{{\n                {hoist_lets}\n                {button}\n            }}"
            ))
        }
    }
}

/// 提取 label 绑定或静态属性（loop_var 上下文）。
fn bind_label_or_static(
    elem: &Element,
    ctx: &CodegenCtx,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } if name == "label" => {
                return Ok(format!("{value:?}"));
            }
            Attribute::Bind { name, expr } if name == "label" => {
                if let Some(code) = crate::compiler::codegen::try_gen_i18n_call(expr, &lv, &computed)
                {
                    return Ok(code);
                }
                return Ok(crate::compiler::codegen::gen_expr_code(expr, &lv, &computed));
            }
            _ => {}
        }
    }
    Ok("\"\"".to_string())
}
