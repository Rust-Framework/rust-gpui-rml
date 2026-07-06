//! 单个 `<Tab>` 子节点 codegen —— 直接构造 `rml_ui::Tab::new()...` 表达式。
//!
//! 与 `accordion::item` 的闭包式 builder 不同，TabBar 的子节点通过
//! `.child(Tab::new()...)` 直接注入，因此本模块生成的是普通构造表达式（非闭包）。
//!
//! ## label 与子节点互斥
//!
//! - `label` 属性 / 文本子节点 → `.label("...")`（互斥，属性优先）
//! - element 子节点 → `.child(<element>)`（每个子节点一次，用于模板定制）
//!
//! gpui-component Tab 运行时：若 `icon` 已设置则渲染 icon，否则渲染 label 或 children。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element, Node};

/// 为 `<Tab>` 子节点生成 `rml_ui::Tab::new().<setters>.child(...)` 表达式
///
/// 返回 `(代码, 是否迭代器)`：
/// - 无 `each` 指令：`(构造表达式, false)` → 父用 `.child(...)`
/// - 有 `each` 指令：`(iter().map(...), true)` → 父用 `.children(...)`
///
/// 生成形如：
/// ```text
/// rml_ui::Tab::new().label("Account").icon(rml_ui::IconName::User).child(<element>)
/// ```
pub fn gen_tab_child(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<(String, bool), CodegenError> {
    let each_clause = elem.directives.iter().find_map(|d| match d {
        Directive::Each { clause: c, .. } => Some(c.clone()),
        _ => None,
    });

    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    if let Some(clause) = &each_clause {
        child_loop_vars.push(clause.item.clone());
        if let Some(idx) = &clause.index {
            child_loop_vars.push(idx.clone());
        }
    }

    let lv: Vec<&str> = child_loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("rml_ui::Tab::new()");

    // 静态/绑定/事件属性 → 先调 tab_bar 专用 setter，未命中回退到公共 setter
    let mut label_set_by_attr = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = super::setters::static_setter(name, value, "Tab") {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_static_setter(
                    name, value, "Tab",
                ) {
                    code.push_str(&s);
                    if name == "label" {
                        label_set_by_attr = true;
                    }
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) =
                    super::setters::bind_setter(name, expr, &lv, &computed, "Tab")
                {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, "Tab",
                ) {
                    code.push_str(&s);
                    if name == "label" {
                        label_set_by_attr = true;
                    }
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::setters::event_setter(name, handler, "Tab") {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_event_setter(
                    name, handler, "Tab",
                ) {
                    code.push_str(&s);
                }
            }
        }
    }

    // 子节点处理：
    // - element 子节点 → .child(...)（模板定制路径）
    // - 文本子节点 → .label("...")（仅当无 label 属性时，与 Button 一致行为）
    if !label_set_by_attr {
        for child in &elem.children {
            if let Node::Text(text) = child {
                code.push_str(&format!(".label({:?})", text));
                break;
            }
        }
    }
    for child in &elem.children {
        if matches!(child, Node::Text(_)) {
            continue;
        }
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, &child_loop_vars)?;
        if is_iter {
            code.push_str(&format!(".children({})", child_code));
        } else {
            code.push_str(&format!(".child({})", child_code));
        }
    }

    if let Some(clause) = each_clause {
        let iter_code = format!(
            "self.{}.iter().map(|{}| {{\n                \
             let {} = {}.clone();\n                \
             {}\n            }})",
            clause.iterable, clause.item, clause.item, clause.item, code
        );
        return Ok((iter_code, true));
    }

    Ok((code, false))
}
