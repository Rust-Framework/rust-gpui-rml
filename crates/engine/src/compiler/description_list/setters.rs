//! DescriptionList / DescriptionItem 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter` 在 tag 为
//! "DescriptionList" / "descriptions" / "DescriptionItem" / "description" 时委托调用。
//! 未命中返回 None，由公共 setter 回退到通用属性（Sizable 等）。
//!
//! ## 注意
//!
//! - `label` 不在本模块处理：它是 `DescriptionItem::new(label)` 的构造器参数，
//!   由 `item::gen_description_item` 提取。
//! - `vertical` 为静态/绑定属性，映射到 `.layout(gpui::Axis::*)`。默认横向，
//!   `vertical="true"` 或 `vertical={is_vertical}` 控制纵向布局。

/// 静态属性 → builder 方法
///
/// DescriptionList 属性：
/// - `vertical=""` / `vertical="true"` → `.layout(gpui::Axis::Vertical)`
/// - `vertical="false"` → 返回 None（默认横向，不生成 layout 调用）
/// - `bordered="true"`/`""` → `.bordered(true)` / `bordered="false"` → `.bordered(false)`
/// - `columns="3"` → `.columns(3)`
/// - `label_width="200"` → `.label_width(gpui::px(200.))`
///
/// DescriptionItem 属性：
/// - `value="text"` → `.value("text")`
/// - `span="2"` → `.span(2)`
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    let canonical = crate::tags::canonical_tag(tag);
    match canonical.as_str() {
        "DescriptionList" => match name {
            "vertical" => {
                if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    Some(".layout(gpui::Axis::Vertical)".to_string())
                } else {
                    None
                }
            }
            "bordered" => {
                let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    "true"
                } else {
                    "false"
                };
                Some(format!(".bordered({})", bool_val))
            }
            "columns" => {
                let trimmed = value.trim();
                if let Ok(n) = trimmed.parse::<usize>() {
                    Some(format!(".columns({})", n))
                } else {
                    None
                }
            }
            "label_width" => {
                let trimmed = value.trim();
                if let Ok(n) = trimmed.parse::<f32>() {
                    Some(format!(".label_width(gpui::px({}.))", n))
                } else {
                    None
                }
            }
            _ => None,
        },
        "DescriptionItem" => match name {
            "value" => Some(format!(".value({:?})", value)),
            "span" => {
                let trimmed = value.trim();
                if let Ok(n) = trimmed.parse::<usize>() {
                    Some(format!(".span({})", n))
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
/// DescriptionList 属性：
/// - `vertical={is_vertical}` → `.layout(if self.is_vertical { Vertical } else { Horizontal })`
/// - `bordered={expr}` → `.bordered(self.expr)`
/// - `columns={expr}` → `.columns(self.expr)`
/// - `label_width={expr}` → `.label_width(self.expr)`
/// - `items={data}` → `.children(self.data.clone().into_iter().filter_map(|c| ...).collect())`
///   data: Vec<Arc<dyn IValue>>，通过 as_contribution() 获取 name()/id() 构造 DescriptionItem
///
/// DescriptionItem 属性：
/// - `value={expr}` → `.value(self.expr.clone())`（DescriptionText: From<SharedString> 等需要 owned）
/// - `span={expr}` → `.span(self.expr)`
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    let canonical = crate::tags::canonical_tag(tag);
    let rust_expr = super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);

    match canonical.as_str() {
        "DescriptionList" => match name {
            "vertical" => Some(format!(
                ".layout(if {} {{ gpui::Axis::Vertical }} else {{ gpui::Axis::Horizontal }})",
                rust_expr
            )),
            "bordered" | "columns" | "label_width" => {
                Some(format!(".{}({})", name, rust_expr))
            }
            "items" => Some(format!(
                ".children({}.clone().into_iter().filter_map(|c| c.as_contribution().map(|c| rml_ui::DescriptionItem::new(c.name()).value(c.id()))).collect::<Vec<_>>())",
                rust_expr
            )),
            _ => None,
        },
        "DescriptionItem" => match name {
            "value" => Some(format!(".value({}.clone())", rust_expr)),
            "span" => Some(format!(".span({})", rust_expr)),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── DescriptionList static_setter ───

    #[test]
    fn static_setter_vertical() {
        assert_eq!(
            static_setter("vertical", "", "DescriptionList").unwrap(),
            ".layout(gpui::Axis::Vertical)"
        );
        assert_eq!(
            static_setter("vertical", "true", "descriptions").unwrap(),
            ".layout(gpui::Axis::Vertical)"
        );
    }

    #[test]
    fn static_setter_vertical_false_returns_none() {
        assert!(static_setter("vertical", "false", "DescriptionList").is_none());
    }

    #[test]
    fn static_setter_bordered() {
        assert_eq!(
            static_setter("bordered", "true", "DescriptionList").unwrap(),
            ".bordered(true)"
        );
        assert_eq!(
            static_setter("bordered", "", "descriptions").unwrap(),
            ".bordered(true)"
        );
        assert_eq!(
            static_setter("bordered", "false", "DescriptionList").unwrap(),
            ".bordered(false)"
        );
    }

    #[test]
    fn static_setter_columns() {
        assert_eq!(
            static_setter("columns", "3", "DescriptionList").unwrap(),
            ".columns(3)"
        );
        assert_eq!(
            static_setter("columns", "1", "descriptions").unwrap(),
            ".columns(1)"
        );
    }

    #[test]
    fn static_setter_columns_non_numeric_returns_none() {
        assert!(static_setter("columns", "auto", "DescriptionList").is_none());
    }

    #[test]
    fn static_setter_label_width() {
        assert_eq!(
            static_setter("label_width", "200", "DescriptionList").unwrap(),
            ".label_width(gpui::px(200.))"
        );
        assert_eq!(
            static_setter("label_width", "120.5", "descriptions").unwrap(),
            ".label_width(gpui::px(120.5.))"
        );
    }

    #[test]
    fn static_setter_label_width_non_numeric_returns_none() {
        assert!(static_setter("label_width", "auto", "DescriptionList").is_none());
    }

    #[test]
    fn static_setter_lowercase_tag() {
        assert_eq!(
            static_setter("vertical", "", "descriptions").unwrap(),
            ".layout(gpui::Axis::Vertical)"
        );
        assert_eq!(
            static_setter("columns", "2", "descriptions").unwrap(),
            ".columns(2)"
        );
    }

    // ─── DescriptionItem static_setter ───

    #[test]
    fn static_setter_item_value() {
        assert_eq!(
            static_setter("value", "John", "DescriptionItem").unwrap(),
            ".value(\"John\")"
        );
        assert_eq!(
            static_setter("value", "John", "description").unwrap(),
            ".value(\"John\")"
        );
    }

    #[test]
    fn static_setter_item_span() {
        assert_eq!(
            static_setter("span", "2", "DescriptionItem").unwrap(),
            ".span(2)"
        );
        assert_eq!(
            static_setter("span", "1", "description").unwrap(),
            ".span(1)"
        );
    }

    #[test]
    fn static_setter_item_span_non_numeric_returns_none() {
        assert!(static_setter("span", "auto", "DescriptionItem").is_none());
    }

    #[test]
    fn static_setter_item_label_returns_none() {
        // label 是构造器参数，不在 setter 中处理
        assert!(static_setter("label", "Name", "DescriptionItem").is_none());
    }

    #[test]
    fn static_setter_unknown_returns_none() {
        assert!(static_setter("foo", "bar", "DescriptionList").is_none());
        assert!(static_setter("label", "x", "DescriptionItem").is_none());
    }

    #[test]
    fn static_setter_tag_mismatch_returns_none() {
        // DescriptionList 属性在 DescriptionItem 上不生效
        assert!(static_setter("vertical", "", "DescriptionItem").is_none());
        assert!(static_setter("columns", "3", "DescriptionItem").is_none());
        // DescriptionItem 属性在 DescriptionList 上不生效
        assert!(static_setter("value", "x", "DescriptionList").is_none());
        assert!(static_setter("span", "1", "DescriptionList").is_none());
    }

    // ─── DescriptionList bind_setter ───

    #[test]
    fn bind_setter_bordered() {
        let code = bind_setter("bordered", "show_border", &[], &[], "DescriptionList").unwrap();
        assert_eq!(code, ".bordered(self.show_border)");
    }

    #[test]
    fn bind_setter_columns() {
        let code = bind_setter("columns", "col_count", &[], &[], "DescriptionList").unwrap();
        assert_eq!(code, ".columns(self.col_count)");
    }

    #[test]
    fn bind_setter_label_width() {
        let code = bind_setter("label_width", "lw", &[], &[], "DescriptionList").unwrap();
        assert_eq!(code, ".label_width(self.lw)");
    }

    #[test]
    fn bind_setter_vertical() {
        let code = bind_setter("vertical", "is_vertical", &[], &[], "DescriptionList").unwrap();
        assert_eq!(
            code,
            ".layout(if self.is_vertical { gpui::Axis::Vertical } else { gpui::Axis::Horizontal })"
        );
    }

    #[test]
    fn bind_setter_items() {
        let code = bind_setter("items", "desitems", &[], &[], "DescriptionList").unwrap();
        assert_eq!(
            code,
            ".children(self.desitems.clone().into_iter().filter_map(|c| c.as_contribution().map(|c| rml_ui::DescriptionItem::new(c.name()).value(c.id()))).collect::<Vec<_>>())"
        );
    }

    #[test]
    fn bind_setter_lowercase_tag() {
        let code = bind_setter("columns", "col_count", &[], &[], "descriptions").unwrap();
        assert_eq!(code, ".columns(self.col_count)");
    }

    // ─── DescriptionItem bind_setter ───

    #[test]
    fn bind_setter_item_value() {
        let code = bind_setter("value", "user.name", &[], &[], "DescriptionItem").unwrap();
        assert_eq!(code, ".value(self.user.name.clone())");
    }

    #[test]
    fn bind_setter_item_span() {
        let code = bind_setter("span", "item_span", &[], &[], "DescriptionItem").unwrap();
        assert_eq!(code, ".span(self.item_span)");
    }

    #[test]
    fn bind_setter_item_label_returns_none() {
        // label 是构造器参数，不在 bind_setter 中处理
        assert!(bind_setter("label", "x", &[], &[], "DescriptionItem").is_none());
    }

    #[test]
    fn bind_setter_unknown_returns_none() {
        assert!(bind_setter("foo", "x", &[], &[], "DescriptionList").is_none());
        assert!(bind_setter("label", "x", &[], &[], "DescriptionItem").is_none());
    }
}
