//! Table / Column 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter` 在 tag 为
//! "Table" / "table" / "Column" / "column" 时委托调用。未命中返回 None，
//! 由公共 setter 回退到通用属性（Sizable、disabled 等）。

/// 静态属性 → builder 方法
///
/// Table 属性：
/// - `bordered="true"`/`bordered=""` → `.bordered(true)` / `bordered="false"` → `.bordered(false)`
/// - `borderless=""` → `.borderless()`
/// - `stripe="true"`/`stripe=""` → `.stripe(true)` / `stripe="false"` → `.stripe(false)`
///
/// Column 属性：
/// - `width="100"` → `.width(gpui::px(100.))`（数值字面量）
/// - `align="center"`/`"left"`/`"right"` → `.align(gpui::TextAlign::Center/Left/Right)`
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    let canonical = crate::tags::canonical_tag(tag);
    match canonical.as_str() {
        "Table" => match name {
            "bordered" => {
                let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    "true"
                } else {
                    "false"
                };
                Some(format!(".bordered({})", bool_val))
            }
            "borderless" => {
                if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    Some(".borderless()".to_string())
                } else {
                    None
                }
            }
            "stripe" => {
                let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    "true"
                } else {
                    "false"
                };
                Some(format!(".stripe({})", bool_val))
            }
            _ => None,
        },
        "Column" => match name {
            "width" => {
                // 数值字面量 → .width(gpui::px(N.))
                let trimmed = value.trim();
                if let Ok(n) = trimmed.parse::<f32>() {
                    Some(format!(".width(gpui::px({}.))", n))
                } else {
                    // 非数值：可能是表达式，但 static setter 只处理字面量
                    None
                }
            }
            "align" => {
                let align = match value.to_ascii_lowercase().as_str() {
                    "center" => "Center",
                    "right" => "Right",
                    "left" | "" => "Left",
                    _ => return None,
                };
                Some(format!(".align(gpui::TextAlign::{})", align))
            }
            "editable" => {
                // editable="" 或 editable="true" → .editable()
                // editable="false" → 不生成 setter（列默认不可编辑）
                if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    Some(".editable()".to_string())
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
/// Table 属性：
/// - `columns={expr}` → `.columns(self.expr.clone())`
/// - `rows={expr}` → `.rows(self.expr.clone())`
/// - `delegate={expr}` → `.delegate(self.expr.clone())`
/// - `bordered={expr}` → `.bordered(self.expr)`
/// - `stripe={expr}` → `.stripe(self.expr)`
///
/// Column 属性：
/// - `width={expr}` → `.width(self.expr)`
/// - `align={expr}` → `.align(self.expr)`
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    let canonical = crate::tags::canonical_tag(tag);
    let rust_expr = crate::compiler::setters::component_bind_rust_expr(expr_str, loop_vars, computed);

    match canonical.as_str() {
        "Table" => match name {
            "columns" | "rows" | "delegate" => Some(format!(".{}({}.clone())", name, rust_expr)),
            "bordered" | "stripe" => Some(format!(".{}({})", name, rust_expr)),
            _ => None,
        },
        "Column" => match name {
            "width" | "align" => Some(format!(".{}({})", name, rust_expr)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Table static_setter ───

    #[test]
    fn static_setter_table_bordered_true() {
        assert_eq!(static_setter("bordered", "true", "Table").unwrap(), ".bordered(true)");
        assert_eq!(static_setter("bordered", "", "Table").unwrap(), ".bordered(true)");
    }

    #[test]
    fn static_setter_table_bordered_false() {
        assert_eq!(static_setter("bordered", "false", "Table").unwrap(), ".bordered(false)");
    }

    #[test]
    fn static_setter_table_borderless() {
        assert_eq!(static_setter("borderless", "", "Table").unwrap(), ".borderless()");
        assert_eq!(static_setter("borderless", "true", "Table").unwrap(), ".borderless()");
    }

    #[test]
    fn static_setter_table_borderless_false_returns_none() {
        assert!(static_setter("borderless", "false", "Table").is_none());
    }

    #[test]
    fn static_setter_table_stripe() {
        assert_eq!(static_setter("stripe", "", "Table").unwrap(), ".stripe(true)");
        assert_eq!(static_setter("stripe", "false", "Table").unwrap(), ".stripe(false)");
    }

    #[test]
    fn static_setter_table_lowercase_tag() {
        // <table bordered=""> 小写标签也应命中
        assert_eq!(static_setter("bordered", "", "table").unwrap(), ".bordered(true)");
    }

    // ─── Column static_setter ───

    #[test]
    fn static_setter_column_width_numeric() {
        assert_eq!(static_setter("width", "100", "Column").unwrap(), ".width(gpui::px(100.))");
        assert_eq!(static_setter("width", "120.5", "Column").unwrap(), ".width(gpui::px(120.5.))");
    }

    #[test]
    fn static_setter_column_width_non_numeric_returns_none() {
        // 非数值字面量在 static setter 中不处理（应作为 bind 处理）
        assert!(static_setter("width", "auto", "Column").is_none());
    }

    #[test]
    fn static_setter_column_align() {
        assert_eq!(static_setter("align", "center", "Column").unwrap(), ".align(gpui::TextAlign::Center)");
        assert_eq!(static_setter("align", "left", "Column").unwrap(), ".align(gpui::TextAlign::Left)");
        assert_eq!(static_setter("align", "right", "Column").unwrap(), ".align(gpui::TextAlign::Right)");
    }

    #[test]
    fn static_setter_column_lowercase_tag() {
        assert_eq!(static_setter("width", "100", "column").unwrap(), ".width(gpui::px(100.))");
    }

    #[test]
    fn static_setter_column_editable_true() {
        assert_eq!(static_setter("editable", "", "Column").unwrap(), ".editable()");
        assert_eq!(static_setter("editable", "true", "Column").unwrap(), ".editable()");
    }

    #[test]
    fn static_setter_column_editable_false_returns_none() {
        assert!(static_setter("editable", "false", "Column").is_none());
    }

    // ─── Table bind_setter ───

    #[test]
    fn bind_setter_table_columns() {
        let code = bind_setter("columns", "api_columns", &[], &[], "Table").unwrap();
        assert_eq!(code, ".columns(self.api_columns.clone())");
    }

    #[test]
    fn bind_setter_table_rows() {
        let code = bind_setter("rows", "api_rows", &[], &[], "Table").unwrap();
        assert_eq!(code, ".rows(self.api_rows.clone())");
    }

    #[test]
    fn bind_setter_table_delegate() {
        let code = bind_setter("delegate", "table_delegate", &[], &[], "Table").unwrap();
        assert_eq!(code, ".delegate(self.table_delegate.clone())");
    }

    #[test]
    fn bind_setter_table_bordered_expr() {
        let code = bind_setter("bordered", "show_border", &[], &[], "Table").unwrap();
        assert_eq!(code, ".bordered(self.show_border)");
    }

    #[test]
    fn bind_setter_table_stripe_expr() {
        let code = bind_setter("stripe", "has_stripe", &[], &[], "Table").unwrap();
        assert_eq!(code, ".stripe(self.has_stripe)");
    }

    // ─── Column bind_setter ───

    #[test]
    fn bind_setter_column_width() {
        let code = bind_setter("width", "col_width", &[], &[], "Column").unwrap();
        assert_eq!(code, ".width(self.col_width)");
    }

    #[test]
    fn bind_setter_column_align() {
        let code = bind_setter("align", "col_align", &[], &[], "Column").unwrap();
        assert_eq!(code, ".align(self.col_align)");
    }

    // ─── 未知属性返回 None ───

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("label", "x", "Table").is_none());
        assert!(static_setter("key", "x", "Table").is_none());
        assert!(static_setter("label", "x", "Column").is_none());
    }

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(bind_setter("label", "x", &[], &[], "Table").is_none());
        assert!(bind_setter("key", "x", &[], &[], "Column").is_none());
    }
}
