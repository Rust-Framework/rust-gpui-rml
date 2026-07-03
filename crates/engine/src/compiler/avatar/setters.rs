//! Avatar / AvatarGroup 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter`
//! 在 tag 为 "Avatar" 或 "AvatarGroup" 时委托调用。
//! 未命中返回 None，由公共 setter 回退到通用属性（Sizable、disabled 等）。

/// 静态属性 → builder 方法
///
/// - Avatar: `src="url"` → `.src("url")`，`name="John"` → `.name("John")`，
///   `placeholder="UserCircle"` → `.placeholder(rml_ui::IconName::UserCircle)`
/// - AvatarGroup: `limit="3"` → `.limit(3)`，`ellipsis=""` → `.ellipsis()`
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    match tag {
        "Avatar" => match name {
            "src" => Some(format!(".src({:?})", value)),
            "name" => Some(format!(".name({:?})", value)),
            "placeholder" => Some(format!(".placeholder(rml_ui::IconName::{})", value)),
            _ => None,
        },
        "AvatarGroup" => match name {
            "limit" => Some(format!(".limit({})", value)),
            "ellipsis" => {
                if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    Some(".ellipsis()".to_string())
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// 绑定属性 → builder 方法
///
/// - Avatar: `src={url}` → `.src(self.url.clone())`，`name={user.name}` → `.name(self.user.name.clone())`，
///   `placeholder={icon}` → `.placeholder(self.icon)`
/// - AvatarGroup: `limit={count}` → `.limit(self.count)`
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    match tag {
        "Avatar" => match name {
            "src" | "name" => {
                let rust_expr =
                    super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
                Some(format!(".{}({}.clone())", name, rust_expr))
            }
            "placeholder" => {
                let rust_expr =
                    super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
                Some(format!(".placeholder({})", rust_expr))
            }
            _ => None,
        },
        "AvatarGroup" => match name {
            "limit" => {
                let rust_expr =
                    super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
                Some(format!(".limit({})", rust_expr))
            }
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── static_setter: Avatar ───

    #[test]
    fn static_setter_avatar_src() {
        let code = static_setter("src", "https://example.com/a.jpg", "Avatar").unwrap();
        assert_eq!(code, r#".src("https://example.com/a.jpg")"#);
    }

    #[test]
    fn static_setter_avatar_name() {
        let code = static_setter("name", "John Doe", "Avatar").unwrap();
        assert_eq!(code, r#".name("John Doe")"#);
    }

    #[test]
    fn static_setter_avatar_placeholder() {
        let code = static_setter("placeholder", "UserCircle", "Avatar").unwrap();
        assert_eq!(code, ".placeholder(rml_ui::IconName::UserCircle)");
    }

    #[test]
    fn static_setter_avatar_unknown_returns_none() {
        assert!(static_setter("label", "x", "Avatar").is_none());
        assert!(static_setter("onclick", "", "Avatar").is_none());
    }

    // ─── static_setter: AvatarGroup ───

    #[test]
    fn static_setter_avatar_group_limit() {
        let code = static_setter("limit", "3", "AvatarGroup").unwrap();
        assert_eq!(code, ".limit(3)");
    }

    #[test]
    fn static_setter_avatar_group_ellipsis_empty() {
        let code = static_setter("ellipsis", "", "AvatarGroup").unwrap();
        assert_eq!(code, ".ellipsis()");
    }

    #[test]
    fn static_setter_avatar_group_ellipsis_true() {
        let code = static_setter("ellipsis", "true", "AvatarGroup").unwrap();
        assert_eq!(code, ".ellipsis()");
    }

    #[test]
    fn static_setter_avatar_group_ellipsis_false_returns_none() {
        assert!(static_setter("ellipsis", "false", "AvatarGroup").is_none());
    }

    #[test]
    fn static_setter_avatar_group_unknown_returns_none() {
        assert!(static_setter("src", "x", "AvatarGroup").is_none());
        assert!(static_setter("name", "x", "AvatarGroup").is_none());
    }

    // ─── static_setter: 其他 tag ───

    #[test]
    fn static_setter_other_tag_returns_none() {
        assert!(static_setter("src", "x", "Button").is_none());
        assert!(static_setter("limit", "3", "Badge").is_none());
    }

    // ─── bind_setter: Avatar ───

    #[test]
    fn bind_setter_avatar_src() {
        let code = bind_setter("src", "avatar_url", &[], &[], "Avatar").unwrap();
        assert_eq!(code, ".src(self.avatar_url.clone())");
    }

    #[test]
    fn bind_setter_avatar_name() {
        let code = bind_setter("name", "user.name", &[], &[], "Avatar").unwrap();
        assert_eq!(code, ".name(self.user.name.clone())");
    }

    #[test]
    fn bind_setter_avatar_placeholder() {
        let code = bind_setter("placeholder", "my_icon", &[], &[], "Avatar").unwrap();
        assert_eq!(code, ".placeholder(self.my_icon)");
    }

    #[test]
    fn bind_setter_avatar_unknown_returns_none() {
        assert!(bind_setter("value", "x", &[], &[], "Avatar").is_none());
        assert!(bind_setter("label", "x", &[], &[], "Avatar").is_none());
    }

    // ─── bind_setter: AvatarGroup ───

    #[test]
    fn bind_setter_avatar_group_limit() {
        let code = bind_setter("limit", "max_count", &[], &[], "AvatarGroup").unwrap();
        assert_eq!(code, ".limit(self.max_count)");
    }

    #[test]
    fn bind_setter_avatar_group_unknown_returns_none() {
        assert!(bind_setter("ellipsis", "x", &[], &[], "AvatarGroup").is_none());
        assert!(bind_setter("src", "x", &[], &[], "AvatarGroup").is_none());
    }

    // ─── bind_setter: 其他 tag ───

    #[test]
    fn bind_setter_other_tag_returns_none() {
        assert!(bind_setter("src", "x", &[], &[], "Button").is_none());
        assert!(bind_setter("limit", "3", &[], &[], "Badge").is_none());
    }
}
