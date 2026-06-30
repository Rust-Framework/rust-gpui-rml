//! CSS → GPUI 样式映射
//!
//! 将 CSS 声明映射为 GPUI `Styled` trait 方法调用代码字符串。
//! 详见文档 §7.2 CSS 子集与扩展。

use super::ast::*;
use std::collections::HashMap;

/// 将 CSS 声明列表映射为 GPUI 方法调用代码
///
/// 返回形如 `.bg(gpui::rgb(0xff0000)).p(gpui::px(10.))` 的字符串。
/// 遇到不支持的属性时静默跳过（向前兼容）。
pub fn map_declarations(decls: &[Declaration], vars: &HashMap<String, Value>) -> String {
    let mut code = String::new();
    for decl in decls {
        if let Some(method) = map_declaration(decl, vars) {
            code.push('.');
            code.push_str(&method);
        }
    }
    code
}

fn map_declaration(decl: &Declaration, vars: &HashMap<String, Value>) -> Option<String> {
    let prop = decl.property.as_str();
    let value = resolve_var(&decl.value, vars);

    match prop {
        // ─── 盒模型 ───
        "width" => length_method("w", &value),
        "height" => length_method("h", &value),
        "padding" => shorthand_padding(&value),
        "padding-top" => length_method("pt", &value),
        "padding-bottom" => length_method("pb", &value),
        "padding-left" => length_method("pl", &value),
        "padding-right" => length_method("pr", &value),
        "margin" => shorthand_margin(&value),
        "margin-top" => length_method("mt", &value),
        "margin-bottom" => length_method("mb", &value),
        "margin-left" => length_method("ml", &value),
        "margin-right" => length_method("mr", &value),
        "border-radius" => length_method("rounded", &value),

        // ─── 背景 ───
        "background" | "background-color" => color_method("bg", &value),

        // ─── 文本 ───
        "color" => color_method("text_color", &value),
        "font-size" => length_method("text_size", &value),
        "font-weight" => font_weight_method(&value),
        "text-align" => text_align_method(&value),
        "line-height" => length_method("line_height", &value),

        // ─── Flexbox ───
        "display" => match &value {
            Value::Keyword(k) if k == "flex" => Some("flex()".into()),
            Value::Keyword(k) if k == "none" => Some("hidden()".into()),
            _ => None,
        },
        "flex-direction" => match &value {
            Value::Keyword(k) if k == "row" => Some("flex_row()".into()),
            Value::Keyword(k) if k == "column" => Some("flex_col()".into()),
            _ => None,
        },
        "justify-content" => match &value {
            Value::Keyword(k) if k == "center" => Some("justify_center()".into()),
            Value::Keyword(k) if k == "flex-start" || k == "start" => Some("justify_start()".into()),
            Value::Keyword(k) if k == "flex-end" || k == "end" => Some("justify_end()".into()),
            Value::Keyword(k) if k == "space-between" => Some("justify_between()".into()),
            _ => None,
        },
        "align-items" => match &value {
            Value::Keyword(k) if k == "center" => Some("items_center()".into()),
            Value::Keyword(k) if k == "flex-start" => Some("items_start()".into()),
            Value::Keyword(k) if k == "flex-end" => Some("items_end()".into()),
            _ => None,
        },
        "flex" => match &value {
            Value::Number(n) if *n == 1.0 => Some("flex_1()".into()),
            _ => None,
        },
        "min-width" => match &value {
            Value::Number(n) if *n == 0.0 => Some("min_w_0()".into()),
            Value::Keyword(k) if k == "0" => Some("min_w_0()".into()),
            _ => length_method("min_w", &value),
        },
        "min-height" => match &value {
            Value::Number(n) if *n == 0.0 => Some("min_h_0()".into()),
            Value::Keyword(k) if k == "0" => Some("min_h_0()".into()),
            _ => length_method("min_h", &value),
        },
        "gap" => length_method("gap", &value),

        // ─── 视觉效果 ───
        "opacity" => match &value {
            Value::Number(n) => Some(format!("opacity({:?})", n)),
            _ => None,
        },
        "overflow" => match &value {
            Value::Keyword(k) if k == "hidden" => Some("overflow_hidden()".into()),
            Value::Keyword(k) if k == "scroll" => Some("overflow_scroll()".into()),
            _ => None,
        },

        _ => None,
    }
}

/// 解析 CSS 变量引用
fn resolve_var(value: &Value, vars: &HashMap<String, Value>) -> Value {
    match value {
        Value::Var(name, fallback) => {
            if let Some(v) = vars.get(name) {
                resolve_var(v, vars)
            } else if let Some(fb) = fallback {
                resolve_var(fb, vars)
            } else {
                value.clone()
            }
        }
        Value::List(items) => Value::List(items.iter().map(|v| resolve_var(v, vars)).collect()),
        Value::Function(name, args) => {
            Value::Function(name.clone(), args.iter().map(|v| resolve_var(v, vars)).collect())
        }
        _ => value.clone(),
    }
}

/// 长度值 → GPUI px() 调用
fn length_method(method: &str, value: &Value) -> Option<String> {
    match value {
        Value::Length(n, Unit::Px) => Some(format!("{}(gpui::px({:?}))", method, n)),
        Value::Length(n, Unit::Pt) => Some(format!("{}(gpui::px({:?}))", method, n * 1.333)),
        Value::Number(n) => Some(format!("{}(gpui::px({:?}))", method, n)),
        _ => None,
    }
}

/// 颜色值 → GPUI rgb() 调用
fn color_method(method: &str, value: &Value) -> Option<String> {
    match value {
        Value::Color(c) => {
            let rgba = ((c.r as u32) << 24) | ((c.g as u32) << 16) | ((c.b as u32) << 8) | (c.a as u32);
            Some(format!("{}(gpui::rgb(0x{:08x}))", method, rgba))
        }
        _ => None,
    }
}

/// font-weight 关键字映射
fn font_weight_method(value: &Value) -> Option<String> {
    match value {
        Value::Keyword(k) => {
            let weight = match k.as_str() {
                "normal" => "FontWeight::NORMAL",
                "bold" => "FontWeight::BOLD",
                "thin" => "FontWeight::THIN",
                "light" => "FontWeight::LIGHT",
                "medium" => "FontWeight::MEDIUM",
                "semibold" => "FontWeight::SEMIBOLD",
                "extrabold" => "FontWeight::EXTRABOLD",
                "black" => "FontWeight::BLACK",
                _ => return None,
            };
            Some(format!("font_weight(gpui::{})", weight))
        }
        _ => None,
    }
}

/// text-align 关键字映射
fn text_align_method(value: &Value) -> Option<String> {
    match value {
        Value::Keyword(k) => match k.as_str() {
            "left" => Some("text_left()".into()),
            "center" => Some("text_center()".into()),
            "right" => Some("text_right()".into()),
            _ => None,
        },
        _ => None,
    }
}

/// padding 简写：1-4 值
fn shorthand_padding(value: &Value) -> Option<String> {
    let values = match value {
        Value::List(items) => items.clone(),
        _ => vec![value.clone()],
    };
    match values.len() {
        1 => length_method("p", &values[0]),
        2 => {
            // 上下 左右
            let py = length_method("py", &values[0])?;
            let px = length_method("px", &values[1])?;
            Some(format!("{}.{}", py, px))
        }
        3 => {
            // 上 左右 下
            let pt = length_method("pt", &values[0])?;
            let px = length_method("px", &values[1])?;
            let pb = length_method("pb", &values[2])?;
            Some(format!("{}.{}.{}", pt, px, pb))
        }
        4 => {
            // 上 右 下 左
            let pt = length_method("pt", &values[0])?;
            let pr = length_method("pr", &values[1])?;
            let pb = length_method("pb", &values[2])?;
            let pl = length_method("pl", &values[3])?;
            Some(format!("{}.{}.{}.{}", pt, pr, pb, pl))
        }
        _ => None,
    }
}

/// margin 简写：1-4 值（逻辑同 padding）
fn shorthand_margin(value: &Value) -> Option<String> {
    let values = match value {
        Value::List(items) => items.clone(),
        _ => vec![value.clone()],
    };
    match values.len() {
        1 => length_method("m", &values[0]),
        2 => {
            let my = length_method("my", &values[0])?;
            let mx = length_method("mx", &values[1])?;
            Some(format!("{}.{}", my, mx))
        }
        3 => {
            let mt = length_method("mt", &values[0])?;
            let mx = length_method("mx", &values[1])?;
            let mb = length_method("mb", &values[2])?;
            Some(format!("{}.{}.{}", mt, mx, mb))
        }
        4 => {
            let mt = length_method("mt", &values[0])?;
            let mr = length_method("mr", &values[1])?;
            let mb = length_method("mb", &values[2])?;
            let ml = length_method("ml", &values[3])?;
            Some(format!("{}.{}.{}.{}", mt, mr, mb, ml))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decl(prop: &str, value: Value) -> Declaration {
        Declaration {
            property: prop.to_string(),
            value,
        }
    }

    #[test]
    fn map_padding() {
        let d = decl("padding", Value::Length(10.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".p(gpui::px(10"));
    }

    #[test]
    fn map_background_color() {
        let d = decl("background-color", Value::Color(Color::rgb(255, 0, 0)));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".bg(gpui::rgb("));
    }

    #[test]
    fn map_color() {
        let d = decl("color", Value::Color(Color::rgb(51, 51, 51)));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".text_color(gpui::rgb("));
    }

    #[test]
    fn map_font_size() {
        let d = decl("font-size", Value::Length(14.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".text_size(gpui::px(14"));
    }

    #[test]
    fn map_display_flex() {
        let d = decl("display", Value::Keyword("flex".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex()"));
    }

    #[test]
    fn map_flex_direction_column() {
        let d = decl("flex-direction", Value::Keyword("column".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_col()"));
    }

    #[test]
    fn map_flex_one() {
        let d = decl("flex", Value::Number(1.0));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_1()"));
    }

    #[test]
    fn map_justify_content_start() {
        let d = decl("justify-content", Value::Keyword("flex-start".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".justify_start()"));
        assert!(!code.contains(".items_start()"));
    }

    #[test]
    fn map_align_items_center() {
        let d = decl("align-items", Value::Keyword("center".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".items_center()"));
    }

    #[test]
    fn map_font_weight_bold() {
        let d = decl("font-weight", Value::Keyword("bold".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains("FontWeight::BOLD"));
    }

    #[test]
    fn map_padding_shorthand_two_values() {
        let d = decl("padding", Value::List(vec![
            Value::Length(10.0, Unit::Px),
            Value::Length(20.0, Unit::Px),
        ]));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".py(gpui::px(10"));
        assert!(code.contains(".px(gpui::px(20"));
    }

    #[test]
    fn map_var_resolution() {
        let mut vars = HashMap::new();
        vars.insert("--primary".to_string(), Value::Color(Color::rgb(0, 123, 255)));
        let d = decl("background", Value::Var("--primary".into(), None));
        let code = map_declarations(&[d], &vars);
        assert!(code.contains(".bg(gpui::rgb("));
    }

    #[test]
    fn map_unsupported_property_skipped() {
        let d = decl("cursor", Value::Keyword("pointer".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.is_empty());
    }
}
