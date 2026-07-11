//! Notification 专用属性 setter
//!
//! ## 设计原因
//!
//! RML `<Notification>` 编译为 `NotificationTrigger`（非 gpui-component `Notification`）。
//! `Notification` 是 `Render`（非 `RenderOnce`），通过 `window.push_notification()` 命令式推送；
//! `NotificationTrigger` 是 `RenderOnce` 声明式包装器，点击 trigger 时自动推送通知。
//!
//! ## 属性映射
//!
//! - `title="标题"` → `.title("标题")`
//! - `message="消息"` → `.message("消息")`
//! - `success` / `info` / `warning` / `error` → `.with_type(NotificationType::X)`（独立布尔属性）
//! - `autohide="false"` → `.autohide(false)`（默认 true，显式 false 关闭）

/// Notification variant 布尔属性名 → NotificationType 枚举变体名
///
/// `info` → `Info`，`success` → `Success`，`warning` → `Warning`，`error` → `Error`
pub fn notification_variant_from_attr(name: &str) -> Option<&'static str> {
    match name {
        "info" => Some("Info"),
        "success" => Some("Success"),
        "warning" => Some("Warning"),
        "error" => Some("Error"),
        _ => None,
    }
}

/// Notification 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "title" => Some(format!(".title({:?})", value)),
        "message" => Some(format!(".message({:?})", value)),
        // variant 布尔属性：success/info/warning/error → .with_type(NotificationType::X)
        n if notification_variant_from_attr(n).is_some() => {
            if value.is_empty() || value.eq_ignore_ascii_case("true") {
                let variant = notification_variant_from_attr(n).unwrap();
                Some(format!(
                    ".with_type(rml_ui::NotificationType::{})",
                    variant
                ))
            } else {
                Some(String::new())
            }
        }
        // autohide：默认 true，显式 false 关闭 → .autohide(false)
        "autohide" => parse_bool_disabled(name, value),
        _ => None,
    }
}

/// 布尔属性：默认 true，显式 "false" 关闭 → .method(false)
fn parse_bool_disabled(name: &str, value: &str) -> Option<String> {
    if value.eq_ignore_ascii_case("false") {
        Some(format!(".{}(false)", name))
    } else {
        Some(String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_title() {
        assert_eq!(
            static_setter("title", "保存成功"),
            Some(".title(\"保存成功\")".to_string())
        );
    }

    #[test]
    fn static_setter_message() {
        assert_eq!(
            static_setter("message", "您的更改已保存"),
            Some(".message(\"您的更改已保存\")".to_string())
        );
    }

    #[test]
    fn static_setter_success_variant() {
        assert_eq!(
            static_setter("success", ""),
            Some(".with_type(rml_ui::NotificationType::Success)".to_string())
        );
        assert_eq!(
            static_setter("success", "true"),
            Some(".with_type(rml_ui::NotificationType::Success)".to_string())
        );
    }

    #[test]
    fn static_setter_info_variant() {
        assert_eq!(
            static_setter("info", ""),
            Some(".with_type(rml_ui::NotificationType::Info)".to_string())
        );
    }

    #[test]
    fn static_setter_warning_variant() {
        assert_eq!(
            static_setter("warning", ""),
            Some(".with_type(rml_ui::NotificationType::Warning)".to_string())
        );
    }

    #[test]
    fn static_setter_error_variant() {
        assert_eq!(
            static_setter("error", ""),
            Some(".with_type(rml_ui::NotificationType::Error)".to_string())
        );
    }

    #[test]
    fn static_setter_variant_false_no_op() {
        let s = static_setter("success", "false").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_autohide_false() {
        assert_eq!(
            static_setter("autohide", "false"),
            Some(".autohide(false)".to_string())
        );
    }

    #[test]
    fn static_setter_autohide_true_no_op() {
        let s = static_setter("autohide", "true").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_autohide_empty_no_op() {
        let s = static_setter("autohide", "").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }

    #[test]
    fn variant_from_attr_info() {
        assert_eq!(notification_variant_from_attr("info"), Some("Info"));
    }

    #[test]
    fn variant_from_attr_unknown() {
        assert_eq!(notification_variant_from_attr("foo"), None);
    }
}
