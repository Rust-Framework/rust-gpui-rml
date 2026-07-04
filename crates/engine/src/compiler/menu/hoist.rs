//! 菜单闭包 `move` 捕获前，将 `self.*` 表达式提升为局部变量（满足 `'static`）

use std::collections::HashMap;

use crate::compiler::codegen::{gen_expr_code, try_gen_i18n_call};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node, TextSegment};

/// 收集菜单树中需在闭包外求值的 `self.*` 表达式
#[derive(Default)]
pub struct MenuHoist {
    entries: Vec<(String, String)>,
    index: HashMap<String, String>,
}

impl MenuHoist {
    pub fn collect_menu_items(
        &mut self,
        items: &[&Element],
        ctx: &CodegenCtx,
        loop_vars: &[String],
    ) -> Result<(), CodegenError> {
        for item in items {
            self.collect_element(item, ctx, loop_vars)?;
        }
        Ok(())
    }

    pub fn resolve(&self, expr: &str) -> Option<&str> {
        self.index.get(expr).map(|s| s.as_str())
    }

    pub fn apply_to_code(&self, code: &str, ctx: &CodegenCtx) -> String {
        let mut out = code.to_string();
        let mut sorted: Vec<_> = self.entries.iter().collect();
        sorted.sort_by_key(|a| std::cmp::Reverse(a.0.len()));
        for (expr, var) in sorted {
            let replacement = if is_copy_hoist(expr, ctx) {
                var.clone()
            } else {
                format!("{var}.clone()")
            };
            out = out.replace(expr, &replacement);
        }
        out
    }

    pub fn rebind_non_copy_in_closure(&self, ctx: &CodegenCtx) -> Vec<String> {
        self.entries
            .iter()
            .filter(|(expr, _)| !is_copy_hoist(expr, ctx))
            .map(|(_, var)| format!("let {var} = {var}.clone();"))
            .collect()
    }

    pub fn gen_lets(&self, ctx: &CodegenCtx) -> String {
        self.entries
            .iter()
            .map(|(expr, var)| gen_let_stmt(expr, var, ctx))
            .collect::<Vec<_>>()
            .join("\n            ")
    }

    fn collect_element(
        &mut self,
        elem: &Element,
        ctx: &CodegenCtx,
        loop_vars: &[String],
    ) -> Result<(), CodegenError> {
        for name in ["label", "disabled", "checked", "href"] {
            if let Some(expr) = bind_expr(elem, name, loop_vars, ctx)? {
                self.register(&expr);
            }
        }

        let (menu_children, custom_children) = super::item::partition_menu_item_children(&elem.children);
        for child in &menu_children {
            self.collect_element(child, ctx, loop_vars)?;
        }
        for child in custom_children {
            self.collect_node(child, ctx, loop_vars)?;
        }
        Ok(())
    }

    fn collect_node(
        &mut self,
        node: &Node,
        ctx: &CodegenCtx,
        loop_vars: &[String],
    ) -> Result<(), CodegenError> {
        match node {
            Node::Interpolation(expr_str) => {
                let code = to_rust_expr(expr_str, loop_vars, ctx);
                self.register(&code);
            }
            Node::MixedText(segments) => {
                for seg in segments {
                    if let TextSegment::Interpolation(expr_str) = seg {
                        let code = to_rust_expr(expr_str, loop_vars, ctx);
                        self.register(&code);
                    }
                }
            }
            Node::Element(elem) => {
                for attr in &elem.attributes {
                    if let Attribute::Bind { expr, .. } = attr {
                        let code = to_rust_expr(expr, loop_vars, ctx);
                        self.register(&code);
                    }
                }
                for child in &elem.children {
                    self.collect_node(child, ctx, loop_vars)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn register(&mut self, expr: &str) {
        if !expr.contains("self.") {
            return;
        }
        if self.index.contains_key(expr) {
            return;
        }
        let var = format!("__rml_m{}", self.entries.len());
        self.index.insert(expr.to_string(), var.clone());
        self.entries.push((expr.to_string(), var));
    }
}

fn bind_expr(
    elem: &Element,
    name: &str,
    loop_vars: &[String],
    ctx: &CodegenCtx,
) -> Result<Option<String>, CodegenError> {
    let bind = elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name: n, expr } if n == name => Some(expr.clone()),
        _ => None,
    });
    let Some(expr_str) = bind else {
        return Ok(None);
    };
    Ok(Some(to_rust_expr(&expr_str, loop_vars, ctx)))
}

fn to_rust_expr(expr_str: &str, loop_vars: &[String], ctx: &CodegenCtx) -> String {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    if let Some(code) = try_gen_i18n_call(expr_str, &lv, &computed) {
        return code;
    }
    gen_expr_code(expr_str, &lv, &computed)
}

fn gen_let_stmt(expr: &str, var: &str, ctx: &CodegenCtx) -> String {
    if expr.ends_with("()") || expr.contains("cx.t(") {
        return format!("let {var} = {expr};");
    }
    if let Some(field) = expr.strip_prefix("self.") {
        let ty = ctx.field_types.get(field).map(|s| s.as_str()).unwrap_or("");
        if matches!(
            ty,
            "bool" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64" | "usize" | "isize"
        ) {
            return format!("let {var} = {expr};");
        }
        return format!("let {var} = {expr}.clone();");
    }
    format!("let {var} = {expr};")
}

pub(crate) fn is_copy_hoist(expr: &str, ctx: &CodegenCtx) -> bool {
    if expr.ends_with("()") || expr.contains("cx.t(") {
        return false;
    }
    if let Some(field) = expr.strip_prefix("self.") {
        let ty = ctx.field_types.get(field).map(|s| s.as_str()).unwrap_or("");
        return matches!(
            ty,
            "bool" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64" | "usize" | "isize"
        );
    }
    false
}
