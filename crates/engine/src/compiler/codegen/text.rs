//! 文本与表达式代码生成 —— 混合文本 / 插值表达式 / i18n 调用

use crate::compiler::expr;
use crate::parser::ast::TextSegment;

/// 将混合文本段（字面量 + 插值）编译为 format! 调用
pub(super) fn gen_mixed_text(
    segments: &[TextSegment],
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    let mut fmt_str = String::new();
    let mut args = Vec::new();
    for seg in segments {
        match seg {
            TextSegment::Literal(s) => {
                fmt_str.push_str(&s.replace('{', "{{").replace('}', "}}"));
            }
            TextSegment::Interpolation(expr) => {
                fmt_str.push_str("{}");
                args.push(gen_expr_code(expr, loop_vars, computed));
            }
        }
    }
    if args.is_empty() {
        format!("{:?}", fmt_str)
    } else {
        format!("format!({:?}, {})", fmt_str, args.join(", "))
    }
}

/// 把插值表达式字符串编译为 Rust 表达式字符串
pub(crate) fn gen_expr_code(expr_str: &str, loop_vars: &[&str], computed: &[&str]) -> String {
    if let Some(code) = try_gen_i18n_call(expr_str, loop_vars, computed) {
        return code;
    }
    match expr::parse(expr_str) {
        Ok(expr::Expr::Field(name)) if computed.contains(&name.as_str()) => {
            if loop_vars.iter().any(|v| *v == name) {
                format!("{}()", name)
            } else {
                format!("self.{}()", name)
            }
        }
        Ok(parsed) => expr::to_rust_code_with_ctx(&parsed, loop_vars),
        Err(_) => {
            let trimmed = expr_str.trim();
            if loop_vars.contains(&trimmed) {
                trimmed.to_string()
            } else if computed.contains(&trimmed) {
                format!("self.{}()", trimmed)
            } else {
                format!("self.{}", trimmed)
            }
        }
    }
}

/// `{t("key")}` → `cx.t("key")`
pub(crate) fn try_gen_i18n_call(
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> Option<String> {
    let trimmed = expr_str.trim();
    if !trimmed.starts_with("t(") || !trimmed.ends_with(')') {
        return None;
    }
    let inner = trimmed[2..trimmed.len() - 1].trim();
    if inner.is_empty() {
        return None;
    }
    if (inner.starts_with('"') && inner.ends_with('"') && inner.len() >= 2)
        || (inner.starts_with('\'') && inner.ends_with('\'') && inner.len() >= 2)
    {
        let key = &inner[1..inner.len() - 1];
        return Some(format!("cx.t({key:?})"));
    }
    let key_expr = gen_expr_code(inner, loop_vars, computed);
    Some(format!("cx.t(&{key_expr})"))
}
