//! Chart 组件 translator
//!
//! 处理 5 种 gpui_component::chart 图表：
//! - LineChart：单系列折线图，`.x(|d| d.x.clone()).y(|d| d.y).stroke(color).tick_margin(n)`
//! - BarChart：柱状图，`.band().value().fill(|d,_,_,_|...).label(|d|...).tick_margin(n)`
//! - AreaChart：多系列面积图，`.x()` + `<Area y-field="..." stroke="..." fill="..." />` 子标签
//! - PieChart：饼图，`.value(|d|...).color(|d|...).outer_radius(f32).inner_radius(f32).pad_angle(f32)`
//! - CandlestickChart：K 线图，`.x().open().high().low().close().body_width_ratio(f32).tick_margin(n)`
//!
//! 构造器统一为 `Type::new(data: Vec<T>)`，`data` 通过绑定属性提供。
//! 字段路径（`x-field`/`y-field` 等）为静态字符串，codegen 生成对应闭包。
//! 颜色（`stroke`/`fill`）支持主题名（`"chart_1"` → `cx.theme().chart_1`）和绑定（`{color}` → `self.color`）。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::setters::component_bind_rust_expr;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

// ── 5 个 ChartTranslator ──────────────────────────────────────────

macro_rules! chart_translator {
    ($struct_name:ident, $tag:literal, $kind:expr) => {
        #[derive(Debug)]
        pub struct $struct_name;

        impl IRmlTranslator for $struct_name {
            fn tag(&self) -> &'static str {
                $tag
            }

            fn matches(&self, elem: &Element) -> bool {
                tags::canonical_tag(&elem.tag) == $tag
            }

            fn to_rust(
                &self,
                elem: &Element,
                ctx: &CodegenCtx,
                _id_counter: &mut usize,
                loop_vars: &[String],
                parents: &[ParentInfo],
            ) -> Result<(String, bool), CodegenError> {
                let code = gen_chart(elem, ctx, loop_vars, parents, $kind)?;
                Ok((code, false))
            }

            fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
                super::super::utils::print_element(elem, ctx)
            }

            fn metadata(&self) -> TranslatorMetadata {
                TranslatorMetadata::new($tag, $tag, ComponentCategory::Primitive)
            }
        }
    };
}

chart_translator!(LineChartTranslator, "LineChart", ChartKind::Line);
chart_translator!(BarChartTranslator, "BarChart", ChartKind::Bar);
chart_translator!(AreaChartTranslator, "AreaChart", ChartKind::Area);
chart_translator!(PieChartTranslator, "PieChart", ChartKind::Pie);
chart_translator!(CandlestickChartTranslator, "CandlestickChart", ChartKind::Candlestick);

// ── ChartKind ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
enum ChartKind {
    Line,
    Bar,
    Area,
    Pie,
    Candlestick,
}

impl ChartKind {
    fn ctor_path(self) -> &'static str {
        match self {
            ChartKind::Line => "rml_ui::LineChart",
            ChartKind::Bar => "rml_ui::BarChart",
            ChartKind::Area => "rml_ui::AreaChart",
            ChartKind::Pie => "rml_ui::PieChart",
            ChartKind::Candlestick => "rml_ui::CandlestickChart",
        }
    }

    fn tag_name(self) -> &'static str {
        match self {
            ChartKind::Line => "LineChart",
            ChartKind::Bar => "BarChart",
            ChartKind::Area => "AreaChart",
            ChartKind::Pie => "PieChart",
            ChartKind::Candlestick => "CandlestickChart",
        }
    }
}

// ── 主代码生成 ────────────────────────────────────────────────────

fn gen_chart(
    elem: &Element,
    ctx: &CodegenCtx,
    loop_vars: &[String],
    parents: &[ParentInfo],
    kind: ChartKind,
) -> Result<String, CodegenError> {
    let tag = kind.tag_name();
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. data 绑定 → 构造器 Type::new(self.data.clone())
    let data_expr = elem.attributes.iter().find_map(|a| match a {
        Attribute::Bind { name, expr, .. } if name == "data" => Some(expr.as_str()),
        _ => None,
    }).ok_or_else(|| CodegenError {
        message: format!("<{}> requires data={{...}} binding attribute", tag),
        span: Some(elem.span),
    })?;
    let data_rust = component_bind_rust_expr(data_expr, &lv, &computed);
    let mut code = format!("{}::new({}.clone())", kind.ctor_path(), data_rust);

    // CSS class 样式（基础层）
    append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

    // 2. 字段路径闭包（chart 类型专属）
    match kind {
        ChartKind::Line | ChartKind::Area | ChartKind::Candlestick => {
            // x_field → .x(|d| d.field.clone())  （x 通常是 String）
            if let Some(field) = static_attr(elem, "x_field") {
                code.push_str(&format!(".x(|d| d.{}.clone())", field));
            }
        }
        ChartKind::Bar => {
            // BarChart 使用 .band() 而非 .x()（分类轴）
            if let Some(field) = static_attr(elem, "x_field") {
                code.push_str(&format!(".band(|d| d.{}.clone())", field));
            }
        }
        ChartKind::Pie => {} // PieChart 无 x 轴
    }

    match kind {
        ChartKind::Line => {
            // y_field → .y(|d| d.field)  （y 通常是 f64，Copy）
            if let Some(field) = static_attr(elem, "y_field") {
                code.push_str(&format!(".y(|d| d.{})", field));
            }
        }
        ChartKind::Bar => {
            // BarChart 使用 .value() 而非 .y()（数值轴）
            if let Some(field) = static_attr(elem, "y_field") {
                code.push_str(&format!(".value(|d| d.{})", field));
            }
        }
        ChartKind::Area => {
            // AreaChart 的 y 系列由 <Area> 子标签提供，在此处理
            for child in &elem.children {
                if let Node::Element(child_elem) = child {
                    if tags::canonical_tag(&child_elem.tag) == "Area" {
                        if let Some(field) = static_attr(child_elem, "y_field") {
                            code.push_str(&format!(".y(|d| d.{})", field));
                        }
                        apply_color_attr(&mut code, child_elem, "stroke", &lv, &computed);
                        apply_color_attr(&mut code, child_elem, "fill", &lv, &computed);
                    }
                }
            }
        }
        ChartKind::Pie => {
            // value_field → .value(|d| d.field as f32)
            if let Some(field) = static_attr(elem, "value_field") {
                code.push_str(&format!(".value(|d| d.{} as f32)", field));
            }
            // color_field → .color(|d| d.field)
            if let Some(field) = static_attr(elem, "color_field") {
                code.push_str(&format!(".color(|d| d.{})", field));
            }
        }
        ChartKind::Candlestick => {
            // open/high/low/close_field → .open(|d| d.field) 等（f64，Copy）
            for accessor in &["open", "high", "low", "close"] {
                let attr_name = format!("{}_field", accessor);
                if let Some(field) = static_attr(elem, &attr_name) {
                    code.push_str(&format!(".{}(|d| d.{})", accessor, field));
                }
            }
        }
    }

    // 3. BarChart 专属：fill_field（4 参数闭包）/ label_field
    //    BarChart.fill() 签名为 Fn(&T, Bounds<f32>, Bounds<f32>, BarAlignment) -> Bg
    //    不支持静态 stroke/fill 颜色（与 LineChart/AreaChart 不同）
    if kind == ChartKind::Bar {
        if let Some(field) = static_attr(elem, "fill_field") {
            code.push_str(&format!(".fill(|d, _, _, _| d.{})", field));
        }
        if let Some(field) = static_attr(elem, "label_field") {
            code.push_str(&format!(".label(|d| d.{}.to_string())", field));
        }
    }

    // 4. 通用属性：stroke（仅 LineChart）、tick_margin、半径、布尔标志
    //    AreaChart 的 stroke/fill 由 <Area> 子标签处理（见上方）
    //    BarChart 无 stroke/fill 方法（fill_field 走 4 参数闭包）
    //    PieChart/CandlestickChart 无 stroke/fill 方法
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                match name.as_str() {
                    "stroke" if kind == ChartKind::Line => {
                        if let Some(c) = theme_color(value) {
                            code.push_str(&format!(".stroke({})", c));
                        }
                    }
                    "tick_margin" => {
                        code.push_str(&format!(".tick_margin({})", value));
                    }
                    "outer_radius" => {
                        code.push_str(&format!(".outer_radius({})", as_f32_literal(value)));
                    }
                    "inner_radius" => {
                        code.push_str(&format!(".inner_radius({})", as_f32_literal(value)));
                    }
                    "pad_angle" => {
                        code.push_str(&format!(".pad_angle({})", as_f32_literal(value)));
                    }
                    "body_width_ratio" => {
                        code.push_str(&format!(".body_width_ratio({})", as_f32_literal(value)));
                    }
                    "dot" if value.is_empty() || value.eq_ignore_ascii_case("true") => {
                        code.push_str(".dot()");
                    }
                    "linear" if value.is_empty() || value.eq_ignore_ascii_case("true") => {
                        code.push_str(".linear()");
                    }
                    "step_after" if value.is_empty() || value.eq_ignore_ascii_case("true") => {
                        code.push_str(".step_after()");
                    }
                    _ => {}
                }
            }
            Attribute::Bind { name, expr, .. } => match name.as_str() {
                "stroke" if kind == ChartKind::Line => {
                    let rust_expr = component_bind_rust_expr(expr, &lv, &computed);
                    code.push_str(&format!(".stroke({})", rust_expr));
                }
                _ => {}
            },
            Attribute::Event { .. } => {}
        }
    }

    Ok(code)
}

// ── 辅助函数 ──────────────────────────────────────────────────────

/// 查找静态属性值
fn static_attr<'a>(elem: &'a Element, name: &str) -> Option<&'a str> {
    elem.attributes.iter().find_map(|a| match a {
        Attribute::Static { name: n, value, .. } if n == name => Some(value.as_str()),
        _ => None,
    })
}

/// 主题颜色名 → cx.theme().xxx 表达式
///
/// 支持 chart_1..chart_5、success、warning、destructive、info、primary、secondary
fn theme_color(value: &str) -> Option<String> {
    let valid = matches!(
        value,
        "chart_1" | "chart_2" | "chart_3" | "chart_4" | "chart_5"
            | "success" | "warning" | "destructive" | "info"
            | "primary" | "secondary" | "danger"
    );
    if valid {
        Some(format!("cx.theme().{}", value))
    } else {
        None
    }
}

/// 颜色属性（stroke/fill）：静态主题名或绑定表达式
fn apply_color_attr(
    code: &mut String,
    elem: &Element,
    attr_name: &str,
    loop_vars: &[&str],
    computed: &[&str],
) {
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } if name == attr_name => {
                if let Some(c) = theme_color(value) {
                    code.push_str(&format!(".{}({})", attr_name, c));
                }
            }
            Attribute::Bind { name, expr, .. } if name == attr_name => {
                let rust_expr = component_bind_rust_expr(expr, loop_vars, computed);
                code.push_str(&format!(".{}({})", attr_name, rust_expr));
            }
            _ => {}
        }
    }
}

/// 数值字符串 → f32 字面量（自动补小数点）
fn as_f32_literal(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.contains('.') || trimmed.contains('e') || trimmed.contains('E') {
        trimmed.to_string()
    } else {
        format!("{}.", trimmed)
    }
}

// ── 注册 ──────────────────────────────────────────────────────────

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(LineChartTranslator);
    registry.register(BarChartTranslator);
    registry.register(AreaChartTranslator);
    registry.register(PieChartTranslator);
    registry.register(CandlestickChartTranslator);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, Node};
    use crate::parser::Span;

    fn ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".into(),
            view_module_path: "test::view".into(),
            ..Default::default()
        }
    }

    fn make_element(tag: &str, attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: tag.into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    fn r#static(name: &str, value: &str) -> Attribute {
        Attribute::Static {
            name: name.into(),
            value: value.into(),
            span: Span::empty(),
        }
    }

    fn bind(name: &str, expr: &str) -> Attribute {
        Attribute::Bind {
            name: name.into(),
            expr: expr.into(),
            span: Span::empty(),
        }
    }

    #[test]
    fn gen_linechart_basic() {
        let elem = make_element(
            "LineChart",
            vec![
                bind("data", "history"),
                r#static("x_field", "label"),
                r#static("y_field", "value"),
                r#static("stroke", "chart_1"),
                r#static("tick_margin", "2"),
            ],
            vec![],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Line).unwrap();
        assert!(code.contains("rml_ui::LineChart::new(self.history.clone())"), "{}", code);
        assert!(code.contains(".x(|d| d.label.clone())"), "{}", code);
        assert!(code.contains(".y(|d| d.value)"), "{}", code);
        assert!(code.contains(".stroke(cx.theme().chart_1)"), "{}", code);
        assert!(code.contains(".tick_margin(2)"), "{}", code);
    }

    #[test]
    fn gen_linechart_dot_linear() {
        let elem = make_element(
            "LineChart",
            vec![
                bind("data", "history"),
                r#static("x_field", "label"),
                r#static("y_field", "value"),
                r#static("dot", ""),
                r#static("linear", ""),
            ],
            vec![],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Line).unwrap();
        assert!(code.contains(".dot()"), "{}", code);
        assert!(code.contains(".linear()"), "{}", code);
    }

    #[test]
    fn gen_linechart_stroke_bind() {
        let elem = make_element(
            "LineChart",
            vec![
                bind("data", "history"),
                r#static("x_field", "label"),
                r#static("y_field", "value"),
                bind("stroke", "color"),
            ],
            vec![],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Line).unwrap();
        assert!(code.contains(".stroke(self.color)"), "{}", code);
    }

    #[test]
    fn gen_areachart_multi_series() {
        let area1 = make_element(
            "Area",
            vec![
                r#static("y_field", "desktop"),
                r#static("stroke", "chart_1"),
            ],
            vec![],
        );
        let area2 = make_element(
            "Area",
            vec![
                r#static("y_field", "mobile"),
                r#static("stroke", "chart_2"),
            ],
            vec![],
        );
        let elem = make_element(
            "AreaChart",
            vec![bind("data", "usage"), r#static("x_field", "date")],
            vec![Node::Element(area1), Node::Element(area2)],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Area).unwrap();
        assert!(code.contains("rml_ui::AreaChart::new(self.usage.clone())"), "{}", code);
        assert!(code.contains(".x(|d| d.date.clone())"), "{}", code);
        assert!(code.contains(".y(|d| d.desktop)"), "{}", code);
        assert!(code.contains(".stroke(cx.theme().chart_1)"), "{}", code);
        assert!(code.contains(".y(|d| d.mobile)"), "{}", code);
        assert!(code.contains(".stroke(cx.theme().chart_2)"), "{}", code);
    }

    #[test]
    fn gen_barchart_fill_label() {
        let elem = make_element(
            "BarChart",
            vec![
                bind("data", "sales"),
                r#static("x_field", "category"),
                r#static("y_field", "value"),
                r#static("fill_field", "color"),
                r#static("label_field", "value"),
                r#static("tick_margin", "2"),
            ],
            vec![],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Bar).unwrap();
        assert!(code.contains(".band(|d| d.category.clone())"), "{}", code);
        assert!(code.contains(".value(|d| d.value)"), "{}", code);
        assert!(code.contains(".fill(|d, _, _, _| d.color)"), "{}", code);
        assert!(code.contains(".label(|d| d.value.to_string())"), "{}", code);
    }

    #[test]
    fn gen_piechart_basic() {
        let elem = make_element(
            "PieChart",
            vec![
                bind("data", "slices"),
                r#static("value_field", "amount"),
                r#static("color_field", "color"),
                r#static("outer_radius", "100"),
                r#static("inner_radius", "60"),
                r#static("pad_angle", "0.04"),
            ],
            vec![],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Pie).unwrap();
        assert!(code.contains("rml_ui::PieChart::new(self.slices.clone())"), "{}", code);
        assert!(code.contains(".value(|d| d.amount as f32)"), "{}", code);
        assert!(code.contains(".color(|d| d.color)"), "{}", code);
        assert!(code.contains(".outer_radius(100.)"), "{}", code);
        assert!(code.contains(".inner_radius(60.)"), "{}", code);
        assert!(code.contains(".pad_angle(0.04)"), "{}", code);
    }

    #[test]
    fn gen_candlestick_basic() {
        let elem = make_element(
            "CandlestickChart",
            vec![
                bind("data", "prices"),
                r#static("x_field", "date"),
                r#static("open_field", "open"),
                r#static("high_field", "high"),
                r#static("low_field", "low"),
                r#static("close_field", "close"),
                r#static("body_width_ratio", "0.4"),
                r#static("tick_margin", "2"),
            ],
            vec![],
        );
        let code = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Candlestick).unwrap();
        assert!(code.contains("rml_ui::CandlestickChart::new(self.prices.clone())"), "{}", code);
        assert!(code.contains(".x(|d| d.date.clone())"), "{}", code);
        assert!(code.contains(".open(|d| d.open)"), "{}", code);
        assert!(code.contains(".high(|d| d.high)"), "{}", code);
        assert!(code.contains(".low(|d| d.low)"), "{}", code);
        assert!(code.contains(".close(|d| d.close)"), "{}", code);
        assert!(code.contains(".body_width_ratio(0.4)"), "{}", code);
    }

    #[test]
    fn gen_linechart_missing_data_errors() {
        let elem = make_element(
            "LineChart",
            vec![r#static("x_field", "label")],
            vec![],
        );
        let result = gen_chart(&elem, &ctx(), &[], &[], ChartKind::Line);
        assert!(result.is_err());
    }
}
