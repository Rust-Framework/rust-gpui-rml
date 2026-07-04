//! `MenuItem` / `MenuSeparator` → PopupMenu builder 链

use crate::compiler::codegen::gen_node;
use crate::compiler::menu::hoist::MenuHoist;
use crate::compiler::menu::popup::apply_popup_config;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node};

/// 菜单子项标签（PascalCase、kebab-case；菜单内小写 `separator` 为别名）
pub fn is_menu_item_tag(tag: &str) -> bool {
    if tag == "separator" {
        return true;
    }
    matches!(
        crate::tags::normalize_component_tag(tag).as_str(),
        "MenuItem" | "MenuSeparator" | "Separator"
    )
}

/// 从菜单项子节点列表生成 PopupMenu builder 闭包体（不含外层 `|menu, window, cx| {`）
///
/// 参数较多但各调用点异构（`menu_param` / `config_elem` 在递归调用中不同），
/// 强行分组反而增加构造样板。后续可考虑引入 `CodegenState` 统一封装
/// `ctx`/`depth`/`id_counter`/`loop_vars`（跨 21 文件的 P1 重构）。
#[allow(clippy::too_many_arguments)]
pub fn gen_popup_menu_body(
    items: &[&Element],
    config_elem: Option<&Element>,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    menu_param: &str,
    hoist: &MenuHoist,
) -> Result<String, CodegenError> {
    let mut lines = Vec::new();
    lines.push(format!("let mut menu = {menu_param};"));
    for line in hoist.rebind_non_copy_in_closure(ctx) {
        lines.push(line);
    }
    if let Some(config) = config_elem {
        lines.push(apply_popup_config(config)?);
    }
    for item in items {
        // 检测 each 指令——运行时迭代子项
        let each_clause = item.directives.iter().find_map(|d| match d {
            Directive::Each(c) => Some(c.clone()),
            _ => None,
        });
        if let Some(clause) = each_clause {
            let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
            child_loop_vars.push(clause.item.clone());
            let stmt = gen_menu_item_stmt(item, ctx, depth, id_counter, &child_loop_vars, hoist)?;
            // iterable 可能是 self.field 或 loop_var.field
            let iter_expr = if loop_vars
                .iter()
                .any(|lv| clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv)))
            {
                clause.iterable.clone()
            } else {
                format!("self.{}", clause.iterable)
            };
            lines.push(format!(
                "for {} in {}.iter() {{\n                {}\n            }}",
                clause.item, iter_expr, stmt
            ));
        } else {
            lines.push(gen_menu_item_stmt(item, ctx, depth, id_counter, loop_vars, hoist)?);
        }
    }
    lines.push("menu".to_string());
    Ok(lines.join("\n                "))
}

pub(crate) fn gen_menu_item_stmt(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    hoist: &MenuHoist,
) -> Result<String, CodegenError> {
    let tag = elem.tag.as_str();
    let canonical = crate::tags::normalize_component_tag(tag);
    if matches!(canonical.as_str(), "MenuSeparator" | "Separator") || tag == "separator" {
        return Ok("menu = menu.separator();".to_string());
    }
    if canonical != "MenuItem" {
        return Err(CodegenError {
            message: format!("expected menu-item, got <{}>", elem.tag),
        });
    }

    if has_attr(elem, "header") {
        let label = static_attr(elem, "label").unwrap_or_default();
        let label_expr = bind_or_static_label(elem, loop_vars, ctx, hoist)?
            .unwrap_or_else(|| format!("{label:?}"));
        return Ok(format!("menu = menu.label({label_expr});"));
    }

    let (menu_children, custom_children): (Vec<&Element>, Vec<&Node>) =
        partition_menu_item_children(&elem.children);

    if !menu_children.is_empty() {
        let label_expr = bind_or_static_label(elem, loop_vars, ctx, hoist)?
            .unwrap_or_else(|| "\"\"".to_string());
        let icon_code = bind_or_static_icon(elem, loop_vars, ctx, hoist)?
            .unwrap_or_else(|| "None".to_string());
        let body = gen_popup_menu_body(
            &menu_children,
            None,
            ctx,
            depth + 1,
            id_counter,
            loop_vars,
            "submenu",
            hoist,
        )?;
        return Ok(format!(
            "let __rml_submenu_weak = __rml_menu_weak.clone();\n                menu = menu.submenu_with_icon({icon_code}, {label_expr}, window, cx, move |submenu, window, cx| {{\n                let __rml_menu_weak = __rml_submenu_weak.clone();\n                {body}\n            }});"
        ));
    }

    if !custom_children.is_empty() {
        let onclick = if let Some(cmd_expr) = command_bind_expr(elem) {
            gen_command_closure(&cmd_expr, ctx, loop_vars)?
        } else {
            gen_onclick_closure(elem, ctx)?
        };
        let disabled = bind_or_static_bool(elem, "disabled", loop_vars, ctx, hoist, false);
        let checked = bind_or_static_bool(elem, "checked", loop_vars, ctx, hoist, false);
        let mut custom_parts = Vec::new();
        for child in &custom_children {
            let (code, _) = gen_node(child, ctx, depth + 1, id_counter, loop_vars)?;
            custom_parts.push(hoist.apply_to_code(&code, ctx));
        }
        let custom_body = if custom_parts.len() == 1 {
            custom_parts[0].clone()
        } else {
            format!(
                "gpui::div().children(vec![{}])",
                custom_parts.join(", ")
            )
        };
        return Ok(format!(
            "menu = menu.item(\n                rml_ui::PopupMenuItem::element(move |_window, _cx| {custom_body})\n                    .disabled({disabled})\n                    .checked({checked})\n                    {onclick}\n            );"
        ));
    }

    if static_attr(elem, "href").is_some() || has_bind(elem, "href") {
        let label_expr = bind_or_static_label(elem, loop_vars, ctx, hoist)?
            .unwrap_or_else(|| "\"\"".to_string());
        let href_expr = match bind_attr(elem, "href", loop_vars, ctx, hoist)? {
            Some(expr) => expr,
            None => static_attr(elem, "href")
                .map(|s| format!("{s:?}"))
                .unwrap_or_else(|| "\"\"".to_string()),
        };
        let icon = static_attr(elem, "icon");
        if icon.is_some() {
            let icon_code = bind_or_static_icon(elem, loop_vars, ctx, hoist)?
                .unwrap_or_else(|| "None".to_string());
            return Ok(format!(
                "menu = menu.link_with_icon({label_expr}, {icon_code}.unwrap(), {href_expr});"
            ));
        }
        return Ok(format!("menu = menu.link({label_expr}, {href_expr});"));
    }

    let label_expr = bind_or_static_label(elem, loop_vars, ctx, hoist)?
        .unwrap_or_else(|| "\"\"".to_string());
    let disabled = bind_or_static_bool(elem, "disabled", loop_vars, ctx, hoist, false);
    let checked = bind_or_static_bool(elem, "checked", loop_vars, ctx, hoist, false);
    let icon_code = bind_or_static_icon(elem, loop_vars, ctx, hoist)?;
    let onclick = if let Some(cmd_expr) = command_bind_expr(elem) {
        gen_command_closure(&cmd_expr, ctx, loop_vars)?
    } else {
        gen_onclick_closure(elem, ctx)?
    };

    let mut stmt = format!(
        "menu = menu.item(\n                rml_ui::PopupMenuItem::new({label_expr})\n                    .disabled({disabled})\n                    .checked({checked})"
    );
    if let Some(code) = icon_code {
        // static: Some(rml_ui::IconName::Save) → .icon(rml_ui::IconName::Save)
        // bind: m.icon → .icon(m.icon.clone())
        if let Some(icon_name) = code
            .strip_prefix("Some(rml_ui::IconName::")
            .and_then(|s| s.strip_suffix(')'))
        {
            stmt.push_str(&format!("\n                    .icon(rml_ui::IconName::{icon_name})"));
        } else {
            stmt.push_str(&format!("\n                    .icon({code}.unwrap())"));
        }
    }
    if !onclick.is_empty() {
        stmt.push_str(&format!("\n                    {onclick}"));
    }
    stmt.push_str("\n            );");
    Ok(stmt)
}

pub(crate) fn partition_menu_item_children(children: &[Node]) -> (Vec<&Element>, Vec<&Node>) {
    let mut menu_children = Vec::new();
    let mut custom_children = Vec::new();
    for child in children {
        match child {
            Node::Element(elem) if is_menu_item_tag(&elem.tag) => menu_children.push(elem),
            other => custom_children.push(other),
        }
    }
    (menu_children, custom_children)
}

fn gen_onclick_closure(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError> {
    let handler = elem.attributes.iter().find_map(|a| match a {
        Attribute::Event { name, handler } if name == "on_click" => Some(handler),
        _ => None,
    });
    let Some(handler) = handler else {
        return Ok(String::new());
    };
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m.clone(),
        EventHandler::WithArgs(m, _) => m.clone(),
    };
    let _ = ctx;
    Ok(format!(
        ".on_click({{\n                        let weak = __rml_menu_weak.clone();\n                        move |ev, _window, app| {{\n                            if let Some(entity) = weak.upgrade() {{\n                                entity.update(app, |this, cx| {{\n                                    let rml_ev = rml_convert::from_gpui_click(ev);\n                                    this.{method}(&rml_ev, cx);\n                                }});\n                            }}\n                        }}\n                    }})"
    ))
}

/// 检测 `command={field}` 绑定属性，返回原始表达式（未经 self. 前缀处理）
fn command_bind_expr(elem: &Element) -> Option<String> {
    elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name, expr } if name == "command" => Some(expr.clone()),
        _ => None,
    })
}

/// 生成声明式命令绑定闭包（Phase B-1：`command={field}`）
///
/// 生成 `.on_click` 闭包。两种路径：
/// - **loop_var 上下文**（如 `command={c.command}`）：直接克隆 loop_var 字段，
///   闭包内处理 `Option<Arc<dyn ICommand>>`——`Some(cmd)` 时 `can_execute` + `execute`。
/// - **entity 字段上下文**（如 `command={save_command}`）：通过 `entity.update` 克隆
///   `Arc<dyn ICommand>`，再 `can_execute` + `execute`。
fn gen_command_closure(
    cmd_expr: &str,
    ctx: &CodegenCtx,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    // 检测 cmd_expr 是否以 loop_var 开头（如 "c.command"）
    let loop_prefix = loop_vars.iter().find(|lv| {
        cmd_expr == **lv || cmd_expr.starts_with(&format!("{}.", lv))
    });

    if let Some(_lv) = loop_prefix {
        // loop_var 上下文：直接访问，不经 entity.update
        // cmd_expr 形如 "c.command"，在迭代器中 c 是 &ViewModel，需通过表达式解析
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
        // loop_var 字段为 Option<Arc<dyn ICommand>>，克隆后闭包内处理 Option
        Ok(format!(
            ".on_click({{\n                        let __rml_cmd = {access}.clone();\n                        move |_ev, window, app| {{\n                            if let Some(cmd) = &__rml_cmd {{\n                                let mut __rml_ctx = rml_core::command::CallContext::new(window, app);\n                                if cmd.can_execute(&mut __rml_ctx) {{\n                                    cmd.execute(&mut __rml_ctx);\n                                }}\n                            }}\n                        }}\n                    }})"
        ))
    } else {
        // entity 字段上下文：经 entity.update 克隆
        let is_computed = ctx.computed_methods.iter().any(|c| c == cmd_expr);
        let field_access = if is_computed {
            format!("this.{}()", cmd_expr)
        } else {
            format!("this.{}", cmd_expr)
        };
        Ok(format!(
            ".on_click({{\n                        let weak = __rml_menu_weak.clone();\n                        move |_ev, window, app| {{\n                            if let Some(entity) = weak.upgrade() {{\n                                let __rml_cmd = entity.update(app, |this, _cx| {{\n                                    {field_access}.clone()\n                                }});\n                                let mut __rml_ctx = rml_core::command::CallContext::new(window, app);\n                                if __rml_cmd.can_execute(&mut __rml_ctx) {{\n                                    __rml_cmd.execute(&mut __rml_ctx);\n                                }}\n                            }}\n                        }}\n                    }})"
        ))
    }
}

fn has_attr(elem: &Element, name: &str) -> bool {
    elem.attributes.iter().any(|a| match a {
        Attribute::Static { name: n, value } if n == name => {
            value.is_empty() || value.eq_ignore_ascii_case("true")
        }
        _ => false,
    })
}

fn has_bind(elem: &Element, name: &str) -> bool {
    elem.attributes
        .iter()
        .any(|a| matches!(a, Attribute::Bind { name: n, .. } if n == name))
}

fn static_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|a| match a {
        Attribute::Static { name: n, value } if n == name => Some(value.clone()),
        _ => None,
    })
}

fn bind_or_static_label(
    elem: &Element,
    loop_vars: &[String],
    ctx: &CodegenCtx,
    hoist: &MenuHoist,
) -> Result<Option<String>, CodegenError> {
    if let Some(expr) = bind_attr(elem, "label", loop_vars, ctx, hoist)? {
        return Ok(Some(expr));
    }
    Ok(static_attr(elem, "label").map(|s| format!("{s:?}")))
}

/// icon 属性：优先 bind 表达式（如 `m.icon`，返回 `Option<IconName>` 或 `IconName`），
/// 否则 static 属性（`icon="Save"` → `Some(rml_ui::IconName::Save)`）。
///
/// 返回 `Some(code)` 或 `None`，code 形如 `Some(rml_ui::IconName::Save)` 或 `m.icon.clone()`。
fn bind_or_static_icon(
    elem: &Element,
    loop_vars: &[String],
    ctx: &CodegenCtx,
    hoist: &MenuHoist,
) -> Result<Option<String>, CodegenError> {
    if let Some(expr) = bind_attr(elem, "icon", loop_vars, ctx, hoist)? {
        return Ok(Some(expr));
    }
    Ok(static_attr(elem, "icon").map(|i| format!("Some(rml_ui::IconName::{})", i)))
}

fn bind_or_static_bool(
    elem: &Element,
    name: &str,
    loop_vars: &[String],
    ctx: &CodegenCtx,
    hoist: &MenuHoist,
    default: bool,
) -> String {
    bind_attr(elem, name, loop_vars, ctx, hoist)
        .ok()
        .flatten()
        .or_else(|| static_attr(elem, name).map(|v| v.eq_ignore_ascii_case("true").to_string()))
        .unwrap_or_else(|| default.to_string())
}

fn resolve_hoisted(expr: String, hoist: &MenuHoist) -> String {
    hoist.resolve(&expr).unwrap_or(&expr).to_string()
}

fn bind_attr(
    elem: &Element,
    name: &str,
    loop_vars: &[String],
    ctx: &CodegenCtx,
    hoist: &MenuHoist,
) -> Result<Option<String>, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let bind = elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name: n, expr } if n == name => Some(expr.clone()),
        _ => None,
    });
    let Some(expr_str) = bind else {
        return Ok(None);
    };
    let rust_expr =
        if let Some(code) = crate::compiler::codegen::try_gen_i18n_call(&expr_str, &lv, &computed) {
            code
        } else {
            crate::compiler::expr::parse(&expr_str)
                .map(|p| crate::compiler::expr::to_rust_code_with_ctx(&p, &lv))
                .unwrap_or_else(|_| {
                    if computed.contains(&expr_str.as_str()) {
                        format!("self.{}()", expr_str)
                    } else {
                        format!("self.{}", expr_str)
                    }
                })
        };
    Ok(Some(resolve_hoisted(rust_expr, hoist)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn separator_generates_separator_call() {
        let src = r#"<MenuSeparator />"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = CodegenCtx::default();
        let mut id = 0usize;
        let hoist = MenuHoist::default();
        let code = gen_menu_item_stmt(&elem, &ctx, 0, &mut id, &[], &hoist).unwrap();
        assert!(code.contains("menu.separator()"));
    }

    // ─── Phase B-1: command={field} 声明式命令绑定 ───

    fn make_menu_ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".to_string(),
            ..CodegenCtx::default()
        }
    }

    #[test]
    fn command_attr_generates_execute_call() {
        let src = r#"<MenuItem command={save_command} label="Save" />"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = make_menu_ctx();
        let mut id = 0usize;
        let hoist = MenuHoist::default();
        let code = gen_menu_item_stmt(&elem, &ctx, 0, &mut id, &[], &hoist).unwrap();
        assert!(
            code.contains("this.save_command"),
            "command={{save_command}} 应生成 this.save_command 访问，实际：\n{}",
            code
        );
        assert!(
            code.contains("CallContext::new(window, app)"),
            "应构造 CallContext"
        );
        assert!(
            code.contains("can_execute(&mut __rml_ctx)"),
            "应调用 can_execute 判断"
        );
        assert!(
            code.contains("__rml_cmd.execute(&mut __rml_ctx)"),
            "应调用 execute 执行命令"
        );
    }

    #[test]
    fn command_takes_precedence_over_on_click() {
        // command 与 on-click 同时存在时，command 优先，不生成 on-click 的 method 调用
        let src = r#"<MenuItem command={save} on-click={legacy} label="Save" />"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = make_menu_ctx();
        let mut id = 0usize;
        let hoist = MenuHoist::default();
        let code = gen_menu_item_stmt(&elem, &ctx, 0, &mut id, &[], &hoist).unwrap();
        assert!(
            code.contains("this.save"),
            "command 优先，应生成 this.save 访问"
        );
        assert!(
            code.contains("execute(&mut __rml_ctx)"),
            "应走命令执行路径"
        );
        // 不应出现 legacy 方法的直接调用（this.legacy 在 rml_convert 路径中）
        assert!(
            !code.contains("rml_convert::from_gpui_click"),
            "command 优先时不应生成 on-click 的 rml_convert 路径"
        );
    }

    #[test]
    fn menu_item_without_command_uses_on_click() {
        // 无 command 属性时仍走 gen_onclick_closure
        let src = r#"<MenuItem on-click={do_click} label="Click" />"#;
        let root = parser::parse(src).unwrap();
        let Node::Element(elem) = root else {
            panic!("expected element");
        };
        let ctx = make_menu_ctx();
        let mut id = 0usize;
        let hoist = MenuHoist::default();
        let code = gen_menu_item_stmt(&elem, &ctx, 0, &mut id, &[], &hoist).unwrap();
        assert!(
            code.contains("rml_convert::from_gpui_click"),
            "无 command 时应走 on-click 路径，实际：\n{}",
            code
        );
        assert!(
            code.contains("this.do_click"),
            "应生成 this.do_click 方法调用"
        );
        // 不应出现命令执行路径
        assert!(
            !code.contains("CallContext::new"),
            "无 command 时不应生成 CallContext"
        );
    }
}
