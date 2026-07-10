//! OtpInput 专用属性 → builder 方法映射。
//!
//! - `groups="2"` → `.groups(2usize)`（static）/ `groups={count}` → `.groups(self.count)`（bind）
//! - `length` / `masked` / `default_value` → 不生成 setter（由 OtpInputTranslator 注入 state_ctor）

/// OtpInput 静态属性映射
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    let canonical = crate::tags::canonical_tag(tag);
    if canonical != "OtpInput" {
        return None;
    }
    match name {
        "groups" => Some(format!(".groups({}usize)", value)),
        _ => None,
    }
}

/// OtpInput 绑定属性映射
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    let canonical = crate::tags::canonical_tag(tag);
    if canonical != "OtpInput" {
        return None;
    }
    match name {
        "groups" => {
            let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".groups({})", rust_expr))
        }
        _ => None,
    }
}
