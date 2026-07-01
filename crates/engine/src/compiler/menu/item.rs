//! `MenuItem` / `MenuSeparator` → PopupMenu builder 链

use crate::compiler::codegen::gen_node;
use crate::compiler::menu::hoist::MenuHoist;
use crate::compiler::menu::popup::apply_popup_config;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler, Node};

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
        lines.push(gen_menu_item_stmt(item, ctx, depth, id_counter, loop_vars, hoist)?);
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
        let icon = static_attr(elem, "icon");
        let icon_code = icon
            .map(|i| format!("Some(rml_ui::IconName::{})", i))
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
        let onclick = gen_onclick_closure(elem, ctx)?;
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
        if let Some(icon) = icon {
            return Ok(format!(
                "menu = menu.link_with_icon({label_expr}, rml_ui::IconName::{icon}, {href_expr});"
            ));
        }
        return Ok(format!("menu = menu.link({label_expr}, {href_expr});"));
    }

    let label_expr = bind_or_static_label(elem, loop_vars, ctx, hoist)?
        .unwrap_or_else(|| "\"\"".to_string());
    let disabled = bind_or_static_bool(elem, "disabled", loop_vars, ctx, hoist, false);
    let checked = bind_or_static_bool(elem, "checked", loop_vars, ctx, hoist, false);
    let icon = static_attr(elem, "icon");
    let onclick = gen_onclick_closure(elem, ctx)?;

    let mut stmt = format!(
        "menu = menu.item(\n                rml_ui::PopupMenuItem::new({label_expr})\n                    .disabled({disabled})\n                    .checked({checked})"
    );
    if let Some(icon) = icon {
        stmt.push_str(&format!("\n                    .icon(rml_ui::IconName::{icon})"));
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
        Attribute::Event { name, handler } if name == "onclick" => Some(handler),
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
                    if computed.iter().any(|c| *c == expr_str.as_str()) {
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
}
