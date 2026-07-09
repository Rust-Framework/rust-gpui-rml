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

    // 颜色属性:保留原始 value(含 Var),由 color_method 决定内联还是运行时主题查询
    // 这样 var(--primary) 不会被构建期内联,而是生成 rml::theme::color("--primary") 调用
    match prop {
        "background" | "background-color" => return color_method("bg", &decl.value, vars),
        "color" => return color_method("text_color", &decl.value, vars),
        _ => {}
    }

    // 非颜色属性:构建期解析 var()(这些变量不参与主题切换)
    let value = resolve_var(&decl.value, vars);

    match prop {
        // ─── 盒模型 ───
        "width" => match &value {
            Value::Length(n, Unit::Percent) if (*n - 100.0).abs() < 1e-6 => Some("w_full()".into()),
            _ => length_or_percentage_method("w", &value),
        },
        "height" => match &value {
            Value::Length(n, Unit::Percent) if (*n - 100.0).abs() < 1e-6 => Some("h_full()".into()),
            _ => length_or_percentage_method("h", &value),
        },
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
        "border" => shorthand_border(&value, vars, ""),
        "border-color" => color_method("border_color", &value, vars),
        "border-top" => shorthand_border(&value, vars, "t"),
        "border-bottom" => shorthand_border(&value, vars, "b"),
        "border-left" => shorthand_border(&value, vars, "l"),
        "border-right" => shorthand_border(&value, vars, "r"),

        // ─── 文本 ───
        "font-size" => length_method("text_size", &value),
        "font-weight" => font_weight_method(&value),
        "font-family" => font_family_method(&value),
        "text-align" => text_align_method(&value),
        "line-height" => line_height_method(&value),
        "white-space" => white_space_method(&value),

        // ─── Flexbox ───
        "display" => match &value {
            Value::Keyword(k) if k == "flex" => Some("flex()".into()),
            Value::Keyword(k) if k == "block" => Some("block()".into()),
            Value::Keyword(k) if k == "grid" => Some("grid()".into()),
            Value::Keyword(k) if k == "none" => Some("hidden()".into()),
            _ => None,
        },
        "flex-direction" => match &value {
            Value::Keyword(k) if k == "row" => Some("flex_row()".into()),
            Value::Keyword(k) if k == "column" => Some("flex_col()".into()),
            Value::Keyword(k) if k == "row-reverse" => Some("flex_row_reverse()".into()),
            Value::Keyword(k) if k == "column-reverse" => Some("flex_col_reverse()".into()),
            _ => None,
        },
        "flex-wrap" => match &value {
            Value::Keyword(k) if k == "wrap" => Some("flex_wrap()".into()),
            Value::Keyword(k) if k == "nowrap" => Some("flex_nowrap()".into()),
            Value::Keyword(k) if k == "wrap-reverse" => Some("flex_wrap_reverse()".into()),
            _ => None,
        },
        "justify-content" => match &value {
            Value::Keyword(k) if k == "center" => Some("justify_center()".into()),
            Value::Keyword(k) if k == "flex-start" || k == "start" => Some("justify_start()".into()),
            Value::Keyword(k) if k == "flex-end" || k == "end" => Some("justify_end()".into()),
            Value::Keyword(k) if k == "space-between" => Some("justify_between()".into()),
            Value::Keyword(k) if k == "space-around" => Some("justify_around()".into()),
            Value::Keyword(k) if k == "space-evenly" => Some("justify_evenly()".into()),
            _ => None,
        },
        "align-items" => match &value {
            Value::Keyword(k) if k == "center" => Some("items_center()".into()),
            Value::Keyword(k) if k == "flex-start" || k == "start" => Some("items_start()".into()),
            Value::Keyword(k) if k == "flex-end" || k == "end" => Some("items_end()".into()),
            Value::Keyword(k) if k == "stretch" => Some("items_stretch()".into()),
            Value::Keyword(k) if k == "baseline" => Some("items_baseline()".into()),
            _ => None,
        },
        "flex" => match &value {
            // `flex: <number>` → grow=N, shrink=0, basis=0（CSS 标准 `flex: <number>` 语义）
            // GPUI 无 flex_basis_0() 简写，使用 flex_basis(gpui::px(0.)) 显式设置 basis=0
            Value::Number(n) => Some(format!(
                "flex_grow({:?}).flex_shrink_0().flex_basis(gpui::px(0.))",
                n
            )),
            _ => None,
        },
        "min-width" => match &value {
            Value::Number(n) if *n == 0.0 => Some("min_w_0()".into()),
            Value::Keyword(k) if k == "0" => Some("min_w_0()".into()),
            _ => length_or_percentage_method("min_w", &value),
        },
        "max-width" => length_or_percentage_method("max_w", &value),
        "min-height" => match &value {
            Value::Number(n) if *n == 0.0 => Some("min_h_0()".into()),
            Value::Keyword(k) if k == "0" => Some("min_h_0()".into()),
            _ => length_or_percentage_method("min_h", &value),
        },
        "max-height" => length_or_percentage_method("max_h", &value),
        "gap" => length_method("gap", &value),

        // ─── 视觉效果 ───
        "opacity" => match &value {
            Value::Number(n) => Some(format!("opacity({:?})", n)),
            _ => None,
        },
        "overflow" => match &value {
            Value::Keyword(k) if k == "hidden" => Some("overflow_hidden()".into()),
            Value::Keyword(k) if k == "scroll" => Some("overflow(gpui::Overflow::Scroll)".into()),
            _ => None,
        },
        "overflow-x" => match &value {
            Value::Keyword(k) if k == "hidden" => Some("overflow_x_hidden()".into()),
            Value::Keyword(k) if k == "scroll" || k == "auto" => {
                Some("overflow_x(gpui::Overflow::Scroll)".into())
            }
            _ => None,
        },
        "overflow-y" => match &value {
            Value::Keyword(k) if k == "hidden" => Some("overflow_y_hidden()".into()),
            Value::Keyword(k) if k == "scroll" || k == "auto" => {
                Some("overflow_y(gpui::Overflow::Scroll)".into())
            }
            _ => None,
        },

        // ─── 定位 ───
        "position" => match &value {
            Value::Keyword(k) if k == "absolute" => Some("absolute()".into()),
            Value::Keyword(k) if k == "relative" => Some("relative()".into()),
            _ => None,
        },
        "top" => length_or_percentage_method("top", &value),
        "right" => length_or_percentage_method("right", &value),
        "bottom" => length_or_percentage_method("bottom", &value),
        "left" => length_or_percentage_method("left", &value),
        "inset" => length_or_percentage_method("inset", &value),

        // ─── 阴影 ───
        "box-shadow" => match &value {
            Value::Keyword(k) => match k.as_str() {
                "none" => Some("shadow_none()".into()),
                "2xs" => Some("shadow_2xs()".into()),
                "xs" => Some("shadow_xs()".into()),
                "sm" => Some("shadow_sm()".into()),
                "md" => Some("shadow_md()".into()),
                "lg" => Some("shadow_lg()".into()),
                "xl" => Some("shadow_xl()".into()),
                "2xl" => Some("shadow_2xl()".into()),
                _ => None,
            },
            _ => None,
        },

        // ─── cursor ───
        "cursor" => match &value {
            Value::Keyword(k) => match k.as_str() {
                "default" => Some("cursor_default()".into()),
                "pointer" => Some("cursor_pointer()".into()),
                "text" => Some("cursor_text()".into()),
                "move" => Some("cursor_move()".into()),
                "not-allowed" => Some("cursor_not_allowed()".into()),
                "context-menu" => Some("cursor_context_menu()".into()),
                "crosshair" => Some("cursor_crosshair()".into()),
                "vertical-text" => Some("cursor_vertical_text()".into()),
                "alias" => Some("cursor_alias()".into()),
                "copy" => Some("cursor_copy()".into()),
                "no-drop" => Some("cursor_no_drop()".into()),
                "grab" => Some("cursor_grab()".into()),
                "grabbing" => Some("cursor_grabbing()".into()),
                "ew-resize" => Some("cursor_ew_resize()".into()),
                "ns-resize" => Some("cursor_ns_resize()".into()),
                "nesw-resize" => Some("cursor_nesw_resize()".into()),
                "nwse-resize" => Some("cursor_nwse_resize()".into()),
                "col-resize" => Some("cursor_col_resize()".into()),
                "row-resize" => Some("cursor_row_resize()".into()),
                "n-resize" => Some("cursor_n_resize()".into()),
                "e-resize" => Some("cursor_e_resize()".into()),
                "s-resize" => Some("cursor_s_resize()".into()),
                "w-resize" => Some("cursor_w_resize()".into()),
                _ => None,
            },
            _ => None,
        },

        // ─── visibility ───
        "visibility" => match &value {
            Value::Keyword(k) if k == "visible" => Some("visible()".into()),
            Value::Keyword(k) if k == "hidden" => Some("invisible()".into()),
            _ => None,
        },

        // ─── 文本截断 ───
        "text-overflow" => match &value {
            Value::Keyword(k) if k == "ellipsis" => Some("text_ellipsis()".into()),
            _ => None,
        },
        "line-clamp" => match &value {
            Value::Number(n) => Some(format!("line_clamp({}usize)", *n as usize)),
            _ => None,
        },
        "truncate" => match &value {
            Value::Keyword(k) if k == "true" => Some("truncate()".into()),
            _ => None,
        },

        // ─── 文本装饰 ───
        "text-decoration" => match &value {
            Value::Keyword(k) => match k.as_str() {
                "underline" => Some("underline()".into()),
                "line-through" => Some("line_through()".into()),
                "none" => Some("text_decoration_none()".into()),
                _ => None,
            },
            _ => None,
        },
        // ─── 字体风格 ───
        "font-style" => match &value {
            Value::Keyword(k) => match k.as_str() {
                "italic" => Some("italic()".into()),
                "normal" => Some("not_italic()".into()),
                _ => None,
            },
            _ => None,
        },
        // ─── align-self ───
        "align-self" => match &value {
            Value::Keyword(k) => match k.as_str() {
                "start" => Some("self_start()".into()),
                "flex-start" => Some("self_flex_start()".into()),
                "end" => Some("self_end()".into()),
                "flex-end" => Some("self_flex_end()".into()),
                "center" => Some("self_center()".into()),
                "stretch" => Some("self_stretch()".into()),
                "baseline" => Some("self_baseline()".into()),
                _ => None,
            },
            _ => None,
        },
        // ─── align-content ───
        "align-content" => match &value {
            Value::Keyword(k) => match k.as_str() {
                "normal" => Some("content_normal()".into()),
                "center" => Some("content_center()".into()),
                "start" | "flex-start" => Some("content_start()".into()),
                "end" | "flex-end" => Some("content_end()".into()),
                "space-between" => Some("content_between()".into()),
                "space-around" => Some("content_around()".into()),
                "space-evenly" => Some("content_evenly()".into()),
                "stretch" => Some("content_stretch()".into()),
                _ => None,
            },
            _ => None,
        },
        // ─── border 细化 ───
        "border-x" => shorthand_border(&value, vars, "x"),
        "border-y" => shorthand_border(&value, vars, "y"),
        "border-style" => match &value {
            Value::Keyword(k) if k == "dashed" => Some("border_dashed()".into()),
            _ => None,
        },
        // ─── 圆角细化（4 角）───
        "border-top-left-radius" => length_method("rounded_tl", &value),
        "border-top-right-radius" => length_method("rounded_tr", &value),
        "border-bottom-right-radius" => length_method("rounded_br", &value),
        "border-bottom-left-radius" => length_method("rounded_bl", &value),
        // ─── flex 分项 ───
        "flex-grow" => match &value {
            Value::Number(n) => Some(format!("flex_grow({:?})", n)),
            _ => None,
        },
        "flex-shrink" => match &value {
            Value::Number(n) => Some(format!("flex_shrink({:?})", n)),
            _ => None,
        },
        "flex-basis" => length_or_percentage_method("flex_basis", &value),
        // ─── aspect-ratio ───
        "aspect-ratio" => match &value {
            Value::Keyword(k) if k == "square" => Some("aspect_square()".into()),
            Value::Number(n) => Some(format!("aspect_ratio({:?})", n)),
            _ => None,
        },

        // ─── CSS Grid ───
        "grid-template-columns" => match &value {
            Value::Number(n) => Some(format!("grid_cols({}u16)", *n as u16)),
            _ => None,
        },
        "grid-template-rows" => match &value {
            Value::Number(n) => Some(format!("grid_rows({}u16)", *n as u16)),
            _ => None,
        },
        "grid-column" => match &value {
            // grid-column: span <N>
            Value::List(items) if items.len() == 2 => {
                if let (Value::Keyword(k), Value::Number(n)) = (&items[0], &items[1]) {
                    if k == "span" {
                        return Some(format!("col_span({}u16)", *n as u16));
                    }
                }
                None
            }
            _ => None,
        },
        "grid-row" => match &value {
            Value::List(items) if items.len() == 2 => {
                if let (Value::Keyword(k), Value::Number(n)) = (&items[0], &items[1]) {
                    if k == "span" {
                        return Some(format!("row_span({}u16)", *n as u16));
                    }
                }
                None
            }
            _ => None,
        },
        "grid-column-start" => match &value {
            Value::Number(n) => Some(format!("col_start({}i16)", *n as i16)),
            _ => None,
        },
        "grid-column-end" => match &value {
            Value::Number(n) => Some(format!("col_end({}i16)", *n as i16)),
            _ => None,
        },
        "grid-row-start" => match &value {
            Value::Number(n) => Some(format!("row_start({}i16)", *n as i16)),
            _ => None,
        },
        "grid-row-end" => match &value {
            Value::Number(n) => Some(format!("row_end({}i16)", *n as i16)),
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

/// line-height 专用映射
///
/// CSS 中 unitless line-height（如 line-height: 1.6）是相对倍数，应乘以字体尺寸。
/// GPUI 的 `line_height` 接受 `gpui::relative(倍数)` 表示相对行高。
/// 带单位长度（px/pt）仍按绝对像素处理。
fn line_height_method(value: &Value) -> Option<String> {
    match value {
        Value::Number(n) => Some(format!("line_height(gpui::relative({:?}))", n)),
        _ => length_method("line_height", value),
    }
}

/// 长度值或百分比 → GPUI 调用
///
/// 百分比映射为 `gpui::relative(分数)`，其中 100% = 1.0。width/height 100%
/// 在外层已特殊处理为 `w_full()` / `h_full()`。
fn length_or_percentage_method(method: &str, value: &Value) -> Option<String> {
    match value {
        Value::Length(n, Unit::Percent) => Some(format!("{}(gpui::relative({:?}))", method, n / 100.0)),
        _ => length_method(method, value),
    }
}

/// 颜色值 → GPUI 调用
///
/// - `Value::Color`:构建期内联为 `gpui::rgb(0xrrggbbaa)`
/// - `Value::Var`:生成运行时主题查询 `rml::theme::color("--name")`
/// - 其他:尝试 `resolve_var` 后再处理
fn color_method(method: &str, value: &Value, vars: &HashMap<String, Value>) -> Option<String> {
    match value {
        Value::Color(c) => {
            let rgba =
                ((c.r as u32) << 24) | ((c.g as u32) << 16) | ((c.b as u32) << 8) | (c.a as u32);
            Some(format!("{}(gpui::rgb(0x{:08x}))", method, rgba))
        }
        Value::Var(name, _) => {
            // 主题变量:生成运行时查询,切换主题时即时生效
            Some(format!("{}(rml::theme::color({:?}))", method, name))
        }
        _ => {
            // 非颜色字面量也非 Var:尝试构建期解析(如嵌套 var 引用非颜色变量)
            let resolved = resolve_var(value, vars);
            match &resolved {
                Value::Color(c) => {
                    let rgba = ((c.r as u32) << 24)
                        | ((c.g as u32) << 16)
                        | ((c.b as u32) << 8)
                        | (c.a as u32);
                    Some(format!("{}(gpui::rgb(0x{:08x}))", method, rgba))
                }
                _ => None,
            }
        }
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

/// font-family 映射
///
/// CSS `font-family: Consolas` / `font-family: "Arial"` 映射为 `.font_family("...")`。
/// 对于逗号分隔的字体列表，仅取第一个可用字体。
fn font_family_method(value: &Value) -> Option<String> {
    let first = match &value {
        Value::Keyword(k) => k.as_str(),
        Value::String(s) => s.as_str(),
        Value::List(items) => {
            let item = items.first()?;
            match item {
                Value::Keyword(k) => k.as_str(),
                Value::String(s) => s.as_str(),
                _ => return None,
            }
        }
        _ => return None,
    };
    Some(format!("font_family({:?})", first))
}

/// white-space 关键字映射
///
/// GPUI 仅支持 `Normal` / `Nowrap` 两种 whitespace 模式，因此 `pre` / `nowrap`
/// 统一映射为 `.whitespace_nowrap()`（保留硬换行并禁止软换行），`normal` 映射为
/// `.whitespace_normal()`。
fn white_space_method(value: &Value) -> Option<String> {
    match &value {
        Value::Keyword(k) => match k.as_str() {
            "nowrap" | "pre" => Some("whitespace_nowrap()".into()),
            "normal" | "pre-wrap" | "pre-line" => Some("whitespace_normal()".into()),
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

/// border 简写：`1px solid <color>` / `1px dashed <color>` / `1px` / `<color>`
///
/// GPUI 限制：`border_color` 应用于所有边，无法 per-side 着色。
/// per-side border（`border_t_1` 等）仅设宽度，color 仍全局生效。
/// border-style（solid/dashed/dotted）GPUI 不支持，忽略。
fn shorthand_border(value: &Value, vars: &HashMap<String, Value>, side: &str) -> Option<String> {
    let items = match value {
        Value::List(items) => items.clone(),
        _ => vec![value.clone()],
    };

    let mut width_n: Option<u32> = None;
    let mut color_value: Option<Value> = None;

    for item in &items {
        match item {
            Value::Length(n, _) if *n == 0.0 => return None,
            Value::Length(n, Unit::Px) => width_n = Some(*n as u32),
            Value::Keyword(k)
                if matches!(k.as_str(), "solid" | "dashed" | "dotted" | "double" | "none" | "hidden") => {}
            Value::Color(_) | Value::Var(_, _) => color_value = Some(item.clone()),
            _ => {}
        }
    }

    let mut code = String::new();

    if let Some(n) = width_n {
        let n_str = match n {
            1 => "1",
            2 => "2",
            3 => "3",
            4 => "4",
            _ => "1",
        };
        if side.is_empty() {
            code.push_str(&format!("border_{}()", n_str));
        } else {
            code.push_str(&format!("border_{}_{}()", side, n_str));
        }
    }

    if let Some(cv) = color_value {
        if let Some(color_code) = color_method("border_color", &cv, vars) {
            if !code.is_empty() {
                code.push('.');
            }
            code.push_str(&color_code);
        }
    }

    if code.is_empty() {
        None
    } else {
        Some(code)
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
        // `flex: 1` → grow=1, shrink=0, basis=0（CSS `flex: <number>` 标准）
        let d = decl("flex", Value::Number(1.0));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_grow(1"));
        assert!(code.contains(".flex_shrink_0()"));
        assert!(code.contains(".flex_basis(gpui::px(0.))"));
    }

    #[test]
    fn map_flex_number() {
        // `flex: 2` / `flex: 3.5` 等任意数字均按 grow=N / shrink=0 / basis=0 映射
        let d = decl("flex", Value::Number(2.0));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_grow(2"));
        assert!(code.contains(".flex_shrink_0()"));
        assert!(code.contains(".flex_basis(gpui::px(0.))"));

        let d = decl("flex", Value::Number(3.5));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_grow(3.5"));
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
    fn map_color_var_generates_runtime_theme_query() {
        // 颜色属性的 var() 生成运行时主题查询,而非构建期内联
        let vars = HashMap::new();
        let d = decl("background", Value::Var("--primary".into(), None));
        let code = map_declarations(&[d], &vars);
        assert!(
            code.contains("rml::theme::color(\"--primary\")"),
            "expected runtime theme query, got: {}",
            code
        );
        assert!(code.contains(".bg("));
    }

    #[test]
    fn map_color_var_in_text_color() {
        let vars = HashMap::new();
        let d = decl("color", Value::Var("--text-color".into(), None));
        let code = map_declarations(&[d], &vars);
        assert!(code.contains("rml::theme::color(\"--text-color\")"));
        assert!(code.contains(".text_color("));
    }

    #[test]
    fn map_non_color_var_still_inlined_at_build_time() {
        // 非颜色属性的 var() 仍构建期内联(如 padding: var(--spacing))
        let mut vars = HashMap::new();
        vars.insert(
            "--spacing".to_string(),
            Value::Length(16.0, Unit::Px),
        );
        let d = decl("padding", Value::Var("--spacing".into(), None));
        let code = map_declarations(&[d], &vars);
        assert!(
            code.contains(".p(gpui::px(16"),
            "expected build-time inlined length, got: {}",
            code
        );
    }

    #[test]
    fn map_unsupported_property_skipped() {
        // transform 仍未映射，应被静默跳过
        let d = decl("transform", Value::Keyword("rotate(45deg)".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.is_empty());
    }

    #[test]
    fn map_position_absolute() {
        let d = decl("position", Value::Keyword("absolute".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".absolute()");
    }

    #[test]
    fn map_position_relative() {
        let d = decl("position", Value::Keyword("relative".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".relative()");
    }

    #[test]
    fn map_top_px() {
        let d = decl("top", Value::Length(10.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".top(gpui::px(10"));
    }

    #[test]
    fn map_left_percent() {
        let d = decl("left", Value::Length(50.0, Unit::Percent));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".left(gpui::relative(0.5))"));
    }

    #[test]
    fn map_inset_px() {
        let d = decl("inset", Value::Length(8.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".inset(gpui::px(8"));
    }

    #[test]
    fn map_box_shadow_md() {
        let d = decl("box-shadow", Value::Keyword("md".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".shadow_md()");
    }

    #[test]
    fn map_box_shadow_none() {
        let d = decl("box-shadow", Value::Keyword("none".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".shadow_none()");
    }

    #[test]
    fn map_cursor_pointer() {
        let d = decl("cursor", Value::Keyword("pointer".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".cursor_pointer()");
    }

    #[test]
    fn map_cursor_not_allowed() {
        let d = decl("cursor", Value::Keyword("not-allowed".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".cursor_not_allowed()");
    }

    #[test]
    fn map_visibility_hidden() {
        let d = decl("visibility", Value::Keyword("hidden".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".invisible()");
    }

    #[test]
    fn map_visibility_visible() {
        let d = decl("visibility", Value::Keyword("visible".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".visible()");
    }

    #[test]
    fn map_text_overflow_ellipsis() {
        let d = decl("text-overflow", Value::Keyword("ellipsis".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".text_ellipsis()");
    }

    #[test]
    fn map_line_clamp_three() {
        let d = decl("line-clamp", Value::Number(3.0));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".line_clamp(3usize)"));
    }

    #[test]
    fn map_truncate_true() {
        let d = decl("truncate", Value::Keyword("true".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".truncate()");
    }

    #[test]
    fn map_white_space_pre() {
        let d = decl("white-space", Value::Keyword("pre".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".whitespace_nowrap()"), "expected whitespace_nowrap, got: {}", code);
    }

    #[test]
    fn map_overflow_x_scroll() {
        let d = decl("overflow-x", Value::Keyword("auto".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(
            code.contains(".overflow_x(gpui::Overflow::Scroll)"),
            "expected overflow_x(Scroll), got: {}", code
        );
    }

    #[test]
    fn map_overflow_x_hidden() {
        // 单轴 hidden 不应污染另一轴：生成 overflow_x_hidden() 而非 overflow_hidden()
        let d = decl("overflow-x", Value::Keyword("hidden".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".overflow_x_hidden()");
    }

    #[test]
    fn map_overflow_y_hidden() {
        let d = decl("overflow-y", Value::Keyword("hidden".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert_eq!(code, ".overflow_y_hidden()");
    }

    #[test]
    fn map_font_family() {
        let d = decl("font-family", Value::Keyword("Consolas".into()));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".font_family(\"Consolas\")"), "expected font_family, got: {}", code);
    }

    #[test]
    fn map_width_100_percent_to_w_full() {
        let d = decl("width", Value::Length(100.0, Unit::Percent));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".w_full()"), "expected w_full, got: {}", code);
    }

    #[test]
    fn map_width_50_percent_to_relative() {
        let d = decl("width", Value::Length(50.0, Unit::Percent));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".w(gpui::relative(0.5))"), "expected relative, got: {}", code);
    }

    #[test]
    fn map_min_width_percent_to_relative() {
        let d = decl("min-width", Value::Length(100.0, Unit::Percent));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".min_w(gpui::relative(1.0))"), "expected min_w relative, got: {}", code);
    }

    #[test]
    fn map_line_height_unitless_to_relative() {
        // CSS unitless line-height 是相对倍数，应映射为 gpui::relative()
        let d = decl("line-height", Value::Number(1.6));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(
            code.contains(".line_height(gpui::relative(1.6))"),
            "expected relative line-height, got: {}",
            code
        );
    }

    #[test]
    fn map_line_height_px_to_absolute() {
        // 带 px 单位的 line-height 仍按绝对像素处理
        let d = decl("line-height", Value::Length(24.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(
            code.contains(".line_height(gpui::px(24"),
            "expected absolute line-height, got: {}",
            code
        );
    }

    #[test]
    fn map_border_shorthand_with_color() {
        let d = decl("border", Value::List(vec![
            Value::Length(1.0, Unit::Px),
            Value::Keyword("solid".into()),
            Value::Color(Color::rgb(229, 231, 235)),
        ]));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_1()"), "expected border_1, got: {}", code);
        assert!(code.contains(".border_color("), "expected border_color, got: {}", code);
    }

    #[test]
    fn map_border_shorthand_with_var() {
        let d = decl("border", Value::List(vec![
            Value::Length(1.0, Unit::Px),
            Value::Keyword("solid".into()),
            Value::Var("--border-color".into(), None),
        ]));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_1()"));
        assert!(code.contains("rml::theme::color(\"--border-color\")"));
    }

    #[test]
    fn map_border_bottom_shorthand() {
        let d = decl("border-bottom", Value::List(vec![
            Value::Length(1.0, Unit::Px),
            Value::Keyword("dashed".into()),
            Value::Var("--border-color".into(), None),
        ]));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_b_1()"), "expected border_b_1, got: {}", code);
        assert!(code.contains(".border_color("));
    }

    #[test]
    fn map_border_color_property() {
        let d = decl("border-color", Value::Var("--border-color".into(), None));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_color("));
        assert!(code.contains("rml::theme::color(\"--border-color\")"));
    }

    #[test]
    fn map_border_width_only() {
        let d = decl("border", Value::Length(2.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_2()"), "expected border_2, got: {}", code);
        assert!(!code.contains(".border_color("));
    }

    #[test]
    fn map_border_zero_skipped() {
        let d = decl("border", Value::List(vec![
            Value::Length(0.0, Unit::Px),
            Value::Keyword("solid".into()),
        ]));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.is_empty(), "expected no border for width=0, got: {}", code);
    }

    // ─── P1 新增映射 ───

    #[test]
    fn map_display_block() {
        let d = decl("display", Value::Keyword("block".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".block()");
    }

    #[test]
    fn map_display_grid() {
        let d = decl("display", Value::Keyword("grid".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".grid()");
    }

    #[test]
    fn map_text_decoration_underline() {
        let d = decl("text-decoration", Value::Keyword("underline".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".underline()");
    }

    #[test]
    fn map_text_decoration_line_through() {
        let d = decl("text-decoration", Value::Keyword("line-through".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".line_through()");
    }

    #[test]
    fn map_text_decoration_none() {
        let d = decl("text-decoration", Value::Keyword("none".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".text_decoration_none()");
    }

    #[test]
    fn map_font_style_italic() {
        let d = decl("font-style", Value::Keyword("italic".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".italic()");
    }

    #[test]
    fn map_font_style_normal() {
        let d = decl("font-style", Value::Keyword("normal".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".not_italic()");
    }

    #[test]
    fn map_align_self_center() {
        let d = decl("align-self", Value::Keyword("center".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".self_center()");
    }

    #[test]
    fn map_align_self_flex_start() {
        let d = decl("align-self", Value::Keyword("flex-start".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".self_flex_start()");
    }

    #[test]
    fn map_align_content_between() {
        let d = decl("align-content", Value::Keyword("space-between".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".content_between()");
    }

    #[test]
    fn map_align_content_stretch() {
        let d = decl("align-content", Value::Keyword("stretch".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".content_stretch()");
    }

    #[test]
    fn map_border_x_shorthand() {
        let d = decl("border-x", Value::Length(1.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_x_1()"), "expected border_x_1, got: {}", code);
    }

    #[test]
    fn map_border_y_shorthand() {
        let d = decl("border-y", Value::Length(2.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".border_y_2()"), "expected border_y_2, got: {}", code);
    }

    #[test]
    fn map_border_style_dashed() {
        let d = decl("border-style", Value::Keyword("dashed".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".border_dashed()");
    }

    #[test]
    fn map_border_style_solid_skipped() {
        let d = decl("border-style", Value::Keyword("solid".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), "");
    }

    #[test]
    fn map_border_top_left_radius() {
        let d = decl("border-top-left-radius", Value::Length(4.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".rounded_tl(gpui::px(4"), "got: {}", code);
    }

    #[test]
    fn map_border_bottom_right_radius() {
        let d = decl("border-bottom-right-radius", Value::Length(8.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".rounded_br(gpui::px(8"), "got: {}", code);
    }

    #[test]
    fn map_flex_grow() {
        let d = decl("flex-grow", Value::Number(2.0));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_grow(2"), "got: {}", code);
    }

    #[test]
    fn map_flex_shrink() {
        let d = decl("flex-shrink", Value::Number(0.0));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_shrink(0"), "got: {}", code);
    }

    #[test]
    fn map_flex_basis_px() {
        let d = decl("flex-basis", Value::Length(100.0, Unit::Px));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".flex_basis(gpui::px(100"), "got: {}", code);
    }

    #[test]
    fn map_aspect_ratio_number() {
        let d = decl("aspect-ratio", Value::Number(1.6));
        let code = map_declarations(&[d], &HashMap::new());
        assert!(code.contains(".aspect_ratio(1.6"), "got: {}", code);
    }

    #[test]
    fn map_aspect_ratio_square() {
        let d = decl("aspect-ratio", Value::Keyword("square".into()));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".aspect_square()");
    }

    // ─── P2 CSS Grid ───

    #[test]
    fn map_grid_template_columns() {
        let d = decl("grid-template-columns", Value::Number(3.0));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".grid_cols(3u16)");
    }

    #[test]
    fn map_grid_template_rows() {
        let d = decl("grid-template-rows", Value::Number(2.0));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".grid_rows(2u16)");
    }

    #[test]
    fn map_grid_column_span() {
        let d = decl("grid-column", Value::List(vec![
            Value::Keyword("span".into()),
            Value::Number(2.0),
        ]));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".col_span(2u16)");
    }

    #[test]
    fn map_grid_row_span() {
        let d = decl("grid-row", Value::List(vec![
            Value::Keyword("span".into()),
            Value::Number(3.0),
        ]));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".row_span(3u16)");
    }

    #[test]
    fn map_grid_column_start() {
        let d = decl("grid-column-start", Value::Number(1.0));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".col_start(1i16)");
    }

    #[test]
    fn map_grid_column_end() {
        let d = decl("grid-column-end", Value::Number(4.0));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".col_end(4i16)");
    }

    #[test]
    fn map_grid_row_start() {
        let d = decl("grid-row-start", Value::Number(2.0));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".row_start(2i16)");
    }

    #[test]
    fn map_grid_row_end() {
        let d = decl("grid-row-end", Value::Number(5.0));
        assert_eq!(map_declarations(&[d], &HashMap::new()), ".row_end(5i16)");
    }
}
