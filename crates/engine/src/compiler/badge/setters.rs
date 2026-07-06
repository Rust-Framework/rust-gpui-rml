//! Badge 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter`
//! 在 tag 为 "Badge" 时委托调用。
//! 未命中返回 None，由公共 setter 回退到通用属性（Sizable、disabled 等）。
//!
//! ## 属性清单
//!
//! | 属性 | 类型 | 说明 |
//! |------|------|------|
//! | `count` | usize | Number variant 计数（0 时隐藏） |
//! | `max` | usize | Number variant 最大显示（超出显示 `N+`，默认 99） |
//! | `dot` | 标志 | 切换为 Dot variant |
//! | `icon` | 图标名 | 切换为 Icon variant（如 `icon="Bell"`） |

/// 静态属性 → builder 方法
///
/// - `count="5"` → `.count(5)`（解析为 usize）
/// - `max="99"` → `.max(99)`
/// - `dot=""` → `.dot()`（标志属性，空或 "true" 触发）
/// - `icon="Bell"` → `.icon(rml_ui::IconName::Bell)`
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    if tag != "Badge" {
        return None;
    }
    match name {
        "count" => value.parse::<usize>().ok().map(|n| format!(".count({})", n)),
        "max" => value.parse::<usize>().ok().map(|n| format!(".max({})", n)),
        "dot" => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                Some(".dot()".to_string())
            } else {
                None
            }
        }
        "icon" => Some(format!(".icon(rml_ui::IconName::{})", value)),
        _ => None,
    }
}

/// 绑定属性 → builder 方法
///
/// - `count={n}` → `.count(self.n)`
/// - `max={n}` → `.max(self.n)`
///
/// `dot` 与 `icon` 为变体切换，无绑定语义（始终为静态值）。
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    if tag != "Badge" {
        return None;
    }
    match name {
        "count" | "max" => {
            let rust_expr =
                super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".{}({})", name, rust_expr))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── static_setter ───

    #[test]
    fn static_setter_count() {
        let code = static_setter("count", "5", "Badge").unwrap();
        assert_eq!(code, ".count(5)");
    }

    #[test]
    fn static_setter_count_zero() {
        let code = static_setter("count", "0", "Badge").unwrap();
        assert_eq!(code, ".count(0)");
    }

    #[test]
    fn static_setter_count_invalid_returns_none() {
        assert!(static_setter("count", "abc", "Badge").is_none());
    }

    #[test]
    fn static_setter_max() {
        let code = static_setter("max", "99", "Badge").unwrap();
        assert_eq!(code, ".max(99)");
    }

    #[test]
    fn static_setter_dot_empty() {
        let code = static_setter("dot", "", "Badge").unwrap();
        assert_eq!(code, ".dot()");
    }

    #[test]
    fn static_setter_dot_true() {
        let code = static_setter("dot", "true", "Badge").unwrap();
        assert_eq!(code, ".dot()");
    }

    #[test]
    fn static_setter_dot_false_returns_none() {
        assert!(static_setter("dot", "false", "Badge").is_none());
    }

    #[test]
    fn static_setter_icon() {
        let code = static_setter("icon", "Bell", "Badge").unwrap();
        assert_eq!(code, ".icon(rml_ui::IconName::Bell)");
    }

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("label", "x", "Badge").is_none());
        assert!(static_setter("primary", "", "Badge").is_none());
    }

    #[test]
    fn static_setter_other_tag_returns_none() {
        assert!(static_setter("count", "5", "Button").is_none());
        assert!(static_setter("dot", "", "Avatar").is_none());
    }

    // ─── bind_setter ───

    #[test]
    fn bind_setter_count() {
        let code = bind_setter("count", "n", &[], &[], "Badge").unwrap();
        assert_eq!(code, ".count(self.n)");
    }

    #[test]
    fn bind_setter_max() {
        let code = bind_setter("max", "max_count", &[], &[], "Badge").unwrap();
        assert_eq!(code, ".max(self.max_count)");
    }

    #[test]
    fn bind_setter_dot_returns_none() {
        assert!(bind_setter("dot", "is_dot", &[], &[], "Badge").is_none());
    }

    #[test]
    fn bind_setter_icon_returns_none() {
        assert!(bind_setter("icon", "icon_name", &[], &[], "Badge").is_none());
    }

    #[test]
    fn bind_setter_other_tag_returns_none() {
        assert!(bind_setter("count", "5", &[], &[], "Button").is_none());
    }
}
