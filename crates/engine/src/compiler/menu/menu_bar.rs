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
use crate::compiler::menu::item::{gen_command_closure, gen_menu_item_stmt, is_menu_item_tag};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, EachClause, Element, Node};

pub fn gen_menu_bar(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let bar_id = *id_counter;
    *id_counter += 1;

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
        // `children={expr}` 绑定路径——WPF HierarchicalDataTemplate 模式
        // 同一模板在所有层级复用,实现无限层级自动递归
        let has_children_bind = top_items.first().map_or(false, |item| {
            item.attributes
                .iter()
                .any(|a| matches!(a, Attribute::Bind { name, .. } if name == "children"))
        });
        if has_children_bind {
            return gen_menu_bar_with_children_bind(
                top_items.first().unwrap(),
                &clause,
                ctx,
                bar_id,
                loop_vars,
            );
        }

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
                    Attribute::Bind { name, expr, .. } if name == "command" => Some(expr.clone()),
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
            Attribute::Static { name, value, .. } if name == "label" => Some(format!("{value:?}")),
            Attribute::Bind { name, expr, .. } if name == "label" => {
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
            Attribute::Static { name, value, .. } if name == "label" => {
                return Ok(format!("{value:?}"));
            }
            Attribute::Bind { name, expr, .. } if name == "label" => {
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

/// 提取任意 bind 属性的 Rust 表达式代码（如 `children={m.children}` → `m.children`）。
fn bind_expr_code(
    elem: &Element,
    name: &str,
    ctx: &CodegenCtx,
    loop_vars: &[String],
) -> Option<String> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let expr_str = elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name: n, expr, .. } if n == name => Some(expr.clone()),
        _ => None,
    })?;
    Some(crate::compiler::codegen::gen_expr_code(
        &expr_str, &lv, &computed,
    ))
}

/// `children={expr}` 绑定路径——WPF `HierarchicalDataTemplate` 模式。
///
/// 模板 `<menu-item each={m in menus} label={m.name} command={m.command} children={m.children} />`
/// 生成递归 `macro_rules!`,同一模板在所有层级复用,实现无限层级自动递归解析。
///
/// - **顶层(MenuBar)**:`menu_bar_button` + `.dropdown_menu(...)`(分支)或 `.on_click(...)`(叶子)
/// - **嵌套层(PopupMenu)**:`menu.item(PopupMenuItem::new(label))`(叶子)或
///   `menu.submenu(label, ..., |submenu, ...| { ...递归... })`(分支)
///
/// `children` 表达式应为 loop_var 字段访问(如 `m.children`),在宏体内经
/// `let m = $item;` 绑定后正确解析(宏卫生保证同一作用域)。
fn gen_menu_bar_with_children_bind(
    template_item: &Element,
    clause: &EachClause,
    ctx: &CodegenCtx,
    bar_id: usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let item_var = &clause.item;
    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    child_loop_vars.push(item_var.clone());

    // 提取 label / children / command 绑定代码
    let label_code = bind_label_or_static(template_item, ctx, &child_loop_vars)?;
    let children_code = bind_expr_code(template_item, "children", ctx, &child_loop_vars)
        .ok_or_else(|| CodegenError {
            message: "children={expr} 绑定缺失——此路径要求 children 属性".to_string(),
        })?;
    let cmd_expr = template_item.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name, expr, .. } if name == "command" => Some(expr.clone()),
        _ => None,
    });
    let onclick_code = if let Some(cmd) = &cmd_expr {
        gen_command_closure(cmd, ctx, &child_loop_vars)?
    } else {
        String::new()
    };

    // iterable:self.field 或 loop_var.field
    let iter_expr = if loop_vars
        .iter()
        .any(|lv| clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv)))
    {
        clause.iterable.clone()
    } else {
        format!("self.{}", clause.iterable)
    };

    let macro_name = format!("__rml_popup_item_{bar_id}");

    // 递归 macro_rules! —— 嵌套层级(PopupMenu)渲染,同一模板复用于所有深度
    let macro_def = format!(
        "macro_rules! {macro_name} {{\n                ($menu:ident, $item:expr, $window:expr, $cx:expr) => {{\n                    let {item_var} = $item;\n                    let __rml_label = {label_code}.clone();\n                    let __rml_children = {children_code}.clone();\n                    if __rml_children.is_empty() {{\n                        $menu = $menu.item(\n                            rml_ui::PopupMenuItem::new(__rml_label)\n                            {onclick_code}\n                        );\n                    }} else {{\n                        $menu = $menu.submenu(__rml_label, $window, $cx, move |submenu, window, cx| {{\n                            let mut submenu = rml_ui::configure_menu_bar_popup(submenu);\n                            for __rml_c in &__rml_children {{\n                                {macro_name}!(submenu, __rml_c, window, cx);\n                            }}\n                            submenu\n                        }});\n                    }}\n                }};\n            }}"
    );

    // 顶层 MenuBar + children map —— 顶层叶子用 menu_bar_button,分支用 dropdown_menu
    let top_code = format!(
        "rml_ui::MenuBar::new((\"rml_menu_bar\", {bar_id}usize)).children({iter_expr}.iter().map(|{item_var}| {{\n                let __rml_label = {label_code}.clone();\n                let __rml_children = {children_code}.clone();\n                if __rml_children.is_empty() {{\n                    rml_ui::menu_bar_button((\"rml_menu_bar\", 0usize), __rml_label)\n                    {onclick_code}\n                }} else {{\n                    rml_ui::menu_bar_button((\"rml_menu_bar\", 0usize), __rml_label)\n                        .dropdown_menu(move |menu, window, cx| {{\n                            let mut menu = rml_ui::configure_menu_bar_popup(menu);\n                            for __rml_c in &__rml_children {{\n                                {macro_name}!(menu, __rml_c, window, cx);\n                            }}\n                            menu\n                        }})\n                }}\n            }}))"
    );

    Ok(format!(
        "{{\n            let __rml_menu_weak = cx.weak_entity();\n            {macro_def}\n            {top_code}\n        }}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    fn make_ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".to_string(),
            ..CodegenCtx::default()
        }
    }

    #[test]
    fn children_bind_generates_recursive_macro() {
        let src = r#"<menu-bar>
            <menu-item each={m in menus} label={m.name} command={m.command} children={m.children} />
        </menu-bar>"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = make_ctx();
        let mut id = 0usize;
        let code = gen_menu_bar(&elem, &ctx, 0, &mut id, &[]).unwrap();

        // 生成递归 macro_rules!
        assert!(
            code.contains("macro_rules! __rml_popup_item_"),
            "应生成递归宏定义,实际:\n{}",
            code
        );
        // 宏递归调用自身
        assert!(
            code.contains("__rml_popup_item_0!(submenu,"),
            "宏应递归调用自身(submenu),实际:\n{}",
            code
        );
        // 顶层 dropdown_menu 调用宏
        assert!(
            code.contains("__rml_popup_item_0!(menu,"),
            "顶层 dropdown_menu 应调用宏,实际:\n{}",
            code
        );
        // 顶层 MenuBar + children map
        assert!(
            code.contains("MenuBar::new"),
            "应生成 MenuBar::new"
        );
        assert!(
            code.contains("self.menus.iter().map"),
            "应生成 self.menus.iter().map"
        );
        // 叶子/分支判断
        assert!(
            code.contains("__rml_children.is_empty()"),
            "应根据 children 是否为空判断叶子/分支"
        );
        // submenu 递归调用
        assert!(
            code.contains("$menu.submenu("),
            "分支应生成 menu.submenu() 调用"
        );
    }

    #[test]
    fn children_bind_without_command_omits_onclick() {
        let src = r#"<menu-bar>
            <menu-item each={m in menus} label={m.name} children={m.children} />
        </menu-bar>"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = make_ctx();
        let mut id = 0usize;
        let code = gen_menu_bar(&elem, &ctx, 0, &mut id, &[]).unwrap();

        assert!(
            !code.contains("can_execute"),
            "无 command 绑定时不应生成 on_click 命令闭包,实际:\n{}",
            code
        );
    }

    #[test]
    fn children_bind_label_field_access() {
        let src = r#"<menu-bar>
            <menu-item each={m in menus} label={m.name} children={m.children} />
        </menu-bar>"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = make_ctx();
        let mut id = 0usize;
        let code = gen_menu_bar(&elem, &ctx, 0, &mut id, &[]).unwrap();

        // label={m.name} → m.name (在宏体内由 `let m = $item;` 解析)
        assert!(
            code.contains("let __rml_label = m.name.clone();"),
            "label 绑定应生成 m.name.clone(),实际:\n{}",
            code
        );
        // children={m.children} → m.children
        assert!(
            code.contains("let __rml_children = m.children.clone();"),
            "children 绑定应生成 m.children.clone(),实际:\n{}",
            code
        );
    }
}
