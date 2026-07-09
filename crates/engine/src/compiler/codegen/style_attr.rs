//! RML 归一化样式属性 → GPUI 方法调用
//!
//! 将 RML 直接属性（如 `width="full"` / `gap="8px"` / `display="flex"`）
//! 转换为 GPUI `Styled` trait 方法调用代码字符串。
//!
//! ## 设计
//!
//! 复用 `css::mapper::map_declaration` 作为单一映射源，避免双轨制。
//! RML 直接属性 → 构造 CSS `Declaration` → `map_declaration` → GPUI 方法。
//!
//! ## 语义快捷词
//!
//! `width="full"` / `height="full"` 等价于 `width="100%"` / `height="100%"`，
//! 最终生成 `.w_full()` / `.h_full()`。

use crate::css::{self, Declaration, Unit, Value};

/// 判断属性名是否为归一化样式属性
///
/// 入口参数 `name` 为 normalize 后的 snake_case 形式（如 `flex_direction`），
/// 内部转回 kebab-case 与 `mapper.rs` 的 CSS 属性名匹配。
pub fn is_style_attr(name: &str) -> bool {
    let kebab = name.replace('_', "-");
    matches!(
        kebab.as_str(),
        // 盒模型
        "width" | "height" |
        "padding" | "padding-top" | "padding-right" | "padding-bottom" | "padding-left" |
        "margin" | "margin-top" | "margin-right" | "margin-bottom" | "margin-left" |
        "border-radius" |
        "border" | "border-color" | "border-top" | "border-right" | "border-bottom" | "border-left" |
        // 文本
        "font-size" | "font-weight" | "font-family" |
        "text-align" | "line-height" | "white-space" |
        "color" | "background" | "background-color" |
        // Flexbox
        "display" | "flex-direction" | "flex-wrap" |
        "justify-content" | "align-items" | "flex" | "gap" |
        "min-width" | "max-width" | "min-height" | "max-height" |
        // 视觉效果
        "opacity" | "overflow" | "overflow-x" | "overflow-y" |
        // 定位
        "position" | "top" | "right" | "bottom" | "left" | "inset" |
        // 阴影
        "box-shadow" |
        // cursor
        "cursor" |
        // visibility
        "visibility" |
        // 文本截断
        "text-overflow" | "line-clamp" | "truncate" |
        // P1 文本装饰 / 字体风格 / 对齐
        "text-decoration" | "font-style" | "align-self" | "align-content" |
        // P1 border 细化 / 圆角细化
        "border-x" | "border-y" | "border-style" |
        "border-top-left-radius" | "border-top-right-radius" |
        "border-bottom-right-radius" | "border-bottom-left-radius" |
        // P1 flex 分项 / 尺寸
        "flex-grow" | "flex-shrink" | "flex-basis" | "aspect-ratio" |
        // P2 CSS Grid
        "grid-template-columns" | "grid-template-rows" |
        "grid-column" | "grid-row" |
        "grid-column-start" | "grid-column-end" |
        "grid-row-start" | "grid-row-end"
    )
}

/// 应用样式属性，返回 GPUI 方法调用代码（含前导 `.`）
///
/// 如 `apply_style_attr("width", "full")` → `Some(".w_full()")`
/// 如 `apply_style_attr("gap", "8px")` → `Some(".gap(gpui::px(8.0))")`
/// 如 `apply_style_attr("display", "flex")` → `Some(".flex()")`
///
/// 不支持的值返回 `None`（调用方输出 warning）。
pub fn apply_style_attr(name: &str, value: &str) -> Option<String> {
    let kebab = name.replace('_', "-");
    let css_value = parse_rml_value(value)?;
    let decl = Declaration {
        property: kebab,
        value: css_value,
    };
    let mapped = css::map_declarations(&[decl], &Default::default());
    if mapped.is_empty() {
        None
    } else {
        Some(mapped)
    }
}

/// 将 RML 属性值字符串解析为 CSS `Value`
///
/// 语义快捷词：
/// - `full` → `Value::Length(100.0, Unit::Percent)`（特殊映射为 `w_full()` / `h_full()`）
/// - `0` → `Value::Number(0.0)`（min-width/min-height=0 特殊映射为 `min_w_0()` / `min_h_0()`）
///
/// 其他值委托 `css::parse` 解析单个声明值。
fn parse_rml_value(s: &str) -> Option<Value> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == "full" {
        return Some(Value::Length(100.0, Unit::Percent));
    }
    // 用 css::parse 解析 `prop: value;` 形式，取首条声明的 value
    let fake = format!("* {{ tmp: {}; }}", trimmed);
    let sheet = css::parse(&fake).ok()?;
    sheet
        .rules
        .into_iter()
        .next()?
        .declarations
        .into_iter()
        .next()
        .map(|d| d.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── is_style_attr ───

    #[test]
    fn is_style_attr_recognizes_box_model() {
        assert!(is_style_attr("width"));
        assert!(is_style_attr("height"));
        assert!(is_style_attr("padding"));
        assert!(is_style_attr("padding_top"));
        assert!(is_style_attr("padding_right"));
        assert!(is_style_attr("padding_bottom"));
        assert!(is_style_attr("padding_left"));
        assert!(is_style_attr("margin"));
        assert!(is_style_attr("margin_top"));
        assert!(is_style_attr("border_radius"));
        assert!(is_style_attr("border"));
        assert!(is_style_attr("border_color"));
        assert!(is_style_attr("border_top"));
    }

    #[test]
    fn is_style_attr_recognizes_text() {
        assert!(is_style_attr("font_size"));
        assert!(is_style_attr("font_weight"));
        assert!(is_style_attr("font_family"));
        assert!(is_style_attr("text_align"));
        assert!(is_style_attr("line_height"));
        assert!(is_style_attr("white_space"));
        assert!(is_style_attr("color"));
        assert!(is_style_attr("background"));
        assert!(is_style_attr("background_color"));
    }

    #[test]
    fn is_style_attr_recognizes_flexbox() {
        assert!(is_style_attr("display"));
        assert!(is_style_attr("flex_direction"));
        assert!(is_style_attr("flex_wrap"));
        assert!(is_style_attr("justify_content"));
        assert!(is_style_attr("align_items"));
        assert!(is_style_attr("flex"));
        assert!(is_style_attr("gap"));
        assert!(is_style_attr("min_width"));
        assert!(is_style_attr("max_width"));
        assert!(is_style_attr("min_height"));
        assert!(is_style_attr("max_height"));
    }

    #[test]
    fn is_style_attr_recognizes_visual() {
        assert!(is_style_attr("opacity"));
        assert!(is_style_attr("overflow"));
        assert!(is_style_attr("overflow_x"));
        assert!(is_style_attr("overflow_y"));
    }

    #[test]
    fn is_style_attr_recognizes_p0_additions() {
        // 定位
        assert!(is_style_attr("position"));
        assert!(is_style_attr("top"));
        assert!(is_style_attr("right"));
        assert!(is_style_attr("bottom"));
        assert!(is_style_attr("left"));
        assert!(is_style_attr("inset"));
        // 阴影 / cursor / visibility
        assert!(is_style_attr("box_shadow"));
        assert!(is_style_attr("cursor"));
        assert!(is_style_attr("visibility"));
        // 文本截断
        assert!(is_style_attr("text_overflow"));
        assert!(is_style_attr("line_clamp"));
        assert!(is_style_attr("truncate"));
    }

    #[test]
    fn is_style_attr_rejects_non_style() {
        assert!(!is_style_attr("label"));
        assert!(!is_style_attr("primary"));
        assert!(!is_style_attr("on_click"));
        assert!(!is_style_attr("variant"));
        assert!(!is_style_attr("selected_index"));
        // 已废弃的 Tailwind 式散落属性不属于归一化样式属性
        assert!(!is_style_attr("h_flex"));
        assert!(!is_style_attr("v_flex"));
        assert!(!is_style_attr("h_full"));
        assert!(!is_style_attr("w_full"));
    }

    // ─── apply_style_attr: width/height + full 快捷词 ───

    #[test]
    fn apply_width_full_returns_w_full() {
        let code = apply_style_attr("width", "full").unwrap();
        assert_eq!(code, ".w_full()");
    }

    #[test]
    fn apply_height_full_returns_h_full() {
        let code = apply_style_attr("height", "full").unwrap();
        assert_eq!(code, ".h_full()");
    }

    #[test]
    fn apply_width_100px() {
        let code = apply_style_attr("width", "100px").unwrap();
        assert!(code.contains(".w(gpui::px(100"));
    }

    #[test]
    fn apply_width_50_percent() {
        let code = apply_style_attr("width", "50%").unwrap();
        assert!(code.contains(".w(gpui::relative(0.5))"), "got: {}", code);
    }

    #[test]
    fn apply_height_200px() {
        let code = apply_style_attr("height", "200px").unwrap();
        assert!(code.contains(".h(gpui::px(200"));
    }

    // ─── apply_style_attr: 盒模型 ───

    #[test]
    fn apply_gap_8px() {
        let code = apply_style_attr("gap", "8px").unwrap();
        assert!(code.contains(".gap(gpui::px(8"));
    }

    #[test]
    fn apply_padding_10px() {
        let code = apply_style_attr("padding", "10px").unwrap();
        assert!(code.contains(".p(gpui::px(10"));
    }

    #[test]
    fn apply_padding_shorthand_two_values() {
        let code = apply_style_attr("padding", "10px 20px").unwrap();
        assert!(code.contains(".py(gpui::px(10"));
        assert!(code.contains(".px(gpui::px(20"));
    }

    #[test]
    fn apply_margin_16px() {
        let code = apply_style_attr("margin", "16px").unwrap();
        assert!(code.contains(".m(gpui::px(16"));
    }

    #[test]
    fn apply_border_radius() {
        let code = apply_style_attr("border_radius", "4px").unwrap();
        assert!(code.contains(".rounded(gpui::px(4"));
    }

    #[test]
    fn apply_border_shorthand() {
        let code = apply_style_attr("border", "1px solid #ccc").unwrap();
        assert!(code.contains(".border_1()"));
        assert!(code.contains(".border_color("));
    }

    // ─── apply_style_attr: 文本 ───

    #[test]
    fn apply_font_size() {
        let code = apply_style_attr("font_size", "14px").unwrap();
        assert!(code.contains(".text_size(gpui::px(14"));
    }

    #[test]
    fn apply_font_weight_bold() {
        let code = apply_style_attr("font_weight", "bold").unwrap();
        assert!(code.contains("FontWeight::BOLD"));
    }

    #[test]
    fn apply_font_family() {
        let code = apply_style_attr("font_family", "Consolas").unwrap();
        assert!(code.contains(".font_family(\"Consolas\")"));
    }

    #[test]
    fn apply_text_align_center() {
        let code = apply_style_attr("text_align", "center").unwrap();
        assert_eq!(code, ".text_center()");
    }

    #[test]
    fn apply_color_red() {
        let code = apply_style_attr("color", "red").unwrap();
        assert!(code.contains(".text_color(gpui::rgb("));
    }

    #[test]
    fn apply_background_color_var() {
        let code = apply_style_attr("background", "var(--primary)").unwrap();
        assert!(code.contains(".bg(rml::theme::color(\"--primary\"))"));
    }

    #[test]
    fn apply_color_var_runtime_query() {
        let code = apply_style_attr("color", "var(--text-color)").unwrap();
        assert!(code.contains(".text_color(rml::theme::color(\"--text-color\"))"));
    }

    // ─── apply_style_attr: Flexbox ───

    #[test]
    fn apply_display_flex() {
        let code = apply_style_attr("display", "flex").unwrap();
        assert_eq!(code, ".flex()");
    }

    #[test]
    fn apply_display_none() {
        let code = apply_style_attr("display", "none").unwrap();
        assert_eq!(code, ".hidden()");
    }

    #[test]
    fn apply_flex_direction_column() {
        let code = apply_style_attr("flex_direction", "column").unwrap();
        assert_eq!(code, ".flex_col()");
    }

    #[test]
    fn apply_flex_direction_row() {
        let code = apply_style_attr("flex_direction", "row").unwrap();
        assert_eq!(code, ".flex_row()");
    }

    #[test]
    fn apply_flex_wrap_wrap() {
        let code = apply_style_attr("flex_wrap", "wrap").unwrap();
        assert_eq!(code, ".flex_wrap()");
    }

    #[test]
    fn apply_justify_content_center() {
        let code = apply_style_attr("justify_content", "center").unwrap();
        assert_eq!(code, ".justify_center()");
    }

    #[test]
    fn apply_justify_content_flex_start() {
        let code = apply_style_attr("justify_content", "flex-start").unwrap();
        assert_eq!(code, ".justify_start()");
    }

    #[test]
    fn apply_align_items_center() {
        let code = apply_style_attr("align_items", "center").unwrap();
        assert_eq!(code, ".items_center()");
    }

    #[test]
    fn apply_align_items_stretch() {
        let code = apply_style_attr("align_items", "stretch").unwrap();
        assert_eq!(code, ".items_stretch()");
    }

    #[test]
    fn apply_flex_number() {
        let code = apply_style_attr("flex", "1").unwrap();
        assert!(code.contains(".flex_grow(1"));
        assert!(code.contains(".flex_shrink_0()"));
        assert!(code.contains(".flex_basis(gpui::px(0.))"));
    }

    #[test]
    fn apply_min_width_zero() {
        let code = apply_style_attr("min_width", "0").unwrap();
        assert_eq!(code, ".min_w_0()");
    }

    #[test]
    fn apply_min_height_zero() {
        let code = apply_style_attr("min_height", "0").unwrap();
        assert_eq!(code, ".min_h_0()");
    }

    #[test]
    fn apply_max_width_50_percent() {
        let code = apply_style_attr("max_width", "50%").unwrap();
        assert!(code.contains(".max_w(gpui::relative(0.5))"));
    }

    // ─── apply_style_attr: 视觉效果 ───

    #[test]
    fn apply_opacity() {
        let code = apply_style_attr("opacity", "0.5").unwrap();
        assert!(code.contains(".opacity(0.5"));
    }

    #[test]
    fn apply_overflow_hidden() {
        let code = apply_style_attr("overflow", "hidden").unwrap();
        assert_eq!(code, ".overflow_hidden()");
    }

    #[test]
    fn apply_overflow_y_scroll() {
        let code = apply_style_attr("overflow_y", "scroll").unwrap();
        assert_eq!(code, ".overflow_y_scroll()");
    }

    #[test]
    fn apply_overflow_x_auto() {
        let code = apply_style_attr("overflow_x", "auto").unwrap();
        assert_eq!(code, ".overflow_x_scroll()");
    }

    #[test]
    fn apply_overflow_x_hidden() {
        let code = apply_style_attr("overflow_x", "hidden").unwrap();
        assert_eq!(code, ".overflow_x_hidden()");
    }

    #[test]
    fn apply_overflow_y_hidden() {
        let code = apply_style_attr("overflow_y", "hidden").unwrap();
        assert_eq!(code, ".overflow_y_hidden()");
    }

    // ─── apply_style_attr: P0 新增（定位/阴影/cursor/visibility/文本截断） ───

    #[test]
    fn apply_position_absolute() {
        let code = apply_style_attr("position", "absolute").unwrap();
        assert_eq!(code, ".absolute()");
    }

    #[test]
    fn apply_position_relative() {
        let code = apply_style_attr("position", "relative").unwrap();
        assert_eq!(code, ".relative()");
    }

    #[test]
    fn apply_top_px() {
        let code = apply_style_attr("top", "10px").unwrap();
        assert!(code.contains(".top(gpui::px(10"));
    }

    #[test]
    fn apply_left_percent() {
        let code = apply_style_attr("left", "50%").unwrap();
        assert!(code.contains(".left(gpui::relative(0.5))"));
    }

    #[test]
    fn apply_inset_px() {
        let code = apply_style_attr("inset", "8px").unwrap();
        assert!(code.contains(".inset(gpui::px(8"));
    }

    #[test]
    fn apply_box_shadow_lg() {
        let code = apply_style_attr("box_shadow", "lg").unwrap();
        assert_eq!(code, ".shadow_lg()");
    }

    #[test]
    fn apply_box_shadow_none() {
        let code = apply_style_attr("box_shadow", "none").unwrap();
        assert_eq!(code, ".shadow_none()");
    }

    #[test]
    fn apply_cursor_pointer() {
        let code = apply_style_attr("cursor", "pointer").unwrap();
        assert_eq!(code, ".cursor_pointer()");
    }

    #[test]
    fn apply_cursor_not_allowed() {
        let code = apply_style_attr("cursor", "not-allowed").unwrap();
        assert_eq!(code, ".cursor_not_allowed()");
    }

    #[test]
    fn apply_visibility_hidden() {
        let code = apply_style_attr("visibility", "hidden").unwrap();
        assert_eq!(code, ".invisible()");
    }

    #[test]
    fn apply_visibility_visible() {
        let code = apply_style_attr("visibility", "visible").unwrap();
        assert_eq!(code, ".visible()");
    }

    #[test]
    fn apply_text_overflow_ellipsis() {
        let code = apply_style_attr("text_overflow", "ellipsis").unwrap();
        assert_eq!(code, ".text_ellipsis()");
    }

    #[test]
    fn apply_line_clamp_two() {
        let code = apply_style_attr("line_clamp", "2").unwrap();
        assert!(code.contains(".line_clamp(2usize)"));
    }

    #[test]
    fn apply_truncate_true() {
        let code = apply_style_attr("truncate", "true").unwrap();
        assert_eq!(code, ".truncate()");
    }

    // ─── apply_style_attr: P1 新增（文本装饰/字体/对齐/border/flex/尺寸） ───

    #[test]
    fn apply_display_block() {
        assert_eq!(apply_style_attr("display", "block").unwrap(), ".block()");
    }

    #[test]
    fn apply_display_grid() {
        assert_eq!(apply_style_attr("display", "grid").unwrap(), ".grid()");
    }

    #[test]
    fn apply_text_decoration_underline() {
        assert_eq!(apply_style_attr("text_decoration", "underline").unwrap(), ".underline()");
    }

    #[test]
    fn apply_font_style_italic() {
        assert_eq!(apply_style_attr("font_style", "italic").unwrap(), ".italic()");
    }

    #[test]
    fn apply_align_self_center() {
        assert_eq!(apply_style_attr("align_self", "center").unwrap(), ".self_center()");
    }

    #[test]
    fn apply_align_content_between() {
        assert_eq!(apply_style_attr("align_content", "space-between").unwrap(), ".content_between()");
    }

    #[test]
    fn apply_border_x_shorthand() {
        let code = apply_style_attr("border_x", "1px").unwrap();
        assert!(code.contains(".border_x_1()"), "got: {}", code);
    }

    #[test]
    fn apply_border_style_dashed() {
        assert_eq!(apply_style_attr("border_style", "dashed").unwrap(), ".border_dashed()");
    }

    #[test]
    fn apply_border_top_left_radius() {
        let code = apply_style_attr("border_top_left_radius", "4px").unwrap();
        assert!(code.contains(".rounded_tl("), "got: {}", code);
    }

    #[test]
    fn apply_flex_grow() {
        let code = apply_style_attr("flex_grow", "2").unwrap();
        assert!(code.contains(".flex_grow(2"), "got: {}", code);
    }

    #[test]
    fn apply_aspect_ratio_square() {
        assert_eq!(apply_style_attr("aspect_ratio", "square").unwrap(), ".aspect_square()");
    }

    #[test]
    fn is_style_attr_recognizes_p1_additions() {
        assert!(is_style_attr("text_decoration"));
        assert!(is_style_attr("font_style"));
        assert!(is_style_attr("align_self"));
        assert!(is_style_attr("align_content"));
        assert!(is_style_attr("border_x"));
        assert!(is_style_attr("border_y"));
        assert!(is_style_attr("border_style"));
        assert!(is_style_attr("border_top_left_radius"));
        assert!(is_style_attr("border_bottom_right_radius"));
        assert!(is_style_attr("flex_grow"));
        assert!(is_style_attr("flex_shrink"));
        assert!(is_style_attr("flex_basis"));
        assert!(is_style_attr("aspect_ratio"));
    }

    // ─── apply_style_attr: 错误路径 ───

    #[test]
    fn apply_invalid_value_returns_none() {
        assert!(apply_style_attr("width", "invalid!").is_none());
    }

    #[test]
    fn apply_empty_value_returns_none() {
        assert!(apply_style_attr("width", "").is_none());
        assert!(apply_style_attr("width", "  ").is_none());
    }

    #[test]
    fn apply_unsupported_property_returns_none() {
        // mapper.rs 不支持 transform 属性，应返回 None
        assert!(apply_style_attr("transform", "rotate(45deg)").is_none());
    }
}
