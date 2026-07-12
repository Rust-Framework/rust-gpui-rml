//! 属性代码生成 —— 静态属性 / CSS 样式 / 内联样式 / 绑定属性
//!
//! 将元素属性转换为 GPUI 构建器方法调用代码。
//!
//! ## fallback 策略
//!
//! 未知属性不生成错误代码（避免把属性错当成文本子节点），改为输出 `eprintln!` warning
//! 并返回空字符串。这样既能提醒开发者补全 setter 或注册 props_registry，又不会产生
//! 编译错误的 Rust 代码。

use crate::css;
use crate::parser::ast::{Attribute, Element};
use crate::tags;

use super::text::gen_expr_code;

/// 判断 content 绑定表达式是否为简单字段访问，需要 `&` 前缀以借用而非移动
///
/// `render(&self)` 中无法 move 非 Copy 字段（如 `String`/`SharedString`），
/// codegen 对简单字段访问自动添加 `&` 前缀，由 `IntoContent for &T` blanket impl 接管。
///
/// 简单字段访问：仅含字母数字、下划线、点号，不含括号（排除方法调用）和运算符。
/// 循环变量（来自 `.iter()`，已是 `&T`）和作用域变量（`_window`/`cx`）不加前缀。
pub(crate) fn needs_borrow_for_content(code: &str, scope_vars: &[&str]) -> bool {
    if scope_vars.contains(&code) {
        return false;
    }
    !code.is_empty()
        && !code.contains('(')
        && !code.contains(')')
        && code.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '.')
}

/// 应用静态属性（class/id/style/src/href/type 等字面量属性）
///
/// 未知属性输出 warning 并返回空字符串（不生成错误代码）。
pub(crate) fn apply_static_attr(name: &str, value: &str) -> String {
    match name {
        "class" | "id" => String::new(),
        "ref" => String::new(),
        "style" => apply_inline_style(value),
        "src" | "href" => String::new(),
        // svg 专用：path 设置 SVG path 数据，由 gpui::svg().path(impl Into<SharedString>) 处理
        "path" => format!(".path({:?})", value),
        "type" => String::new(),
        // focusable：使 StatefulInteractiveElement 可接收焦点，配合 on-focus/on-blur 使用
        // GPUI focusable() 不接受参数
        "focusable" => {
            if value == "true" || value.is_empty() {
                ".focusable()".to_string()
            } else {
                String::new()
            }
        }
        // anchored 专用：anchor 定位角（8 变体）
        "anchor" => match value {
            "top-left" => ".anchor(gpui::Anchor::TopLeft)".to_string(),
            "top-right" => ".anchor(gpui::Anchor::TopRight)".to_string(),
            "bottom-left" => ".anchor(gpui::Anchor::BottomLeft)".to_string(),
            "bottom-right" => ".anchor(gpui::Anchor::BottomRight)".to_string(),
            "top-center" => ".anchor(gpui::Anchor::TopCenter)".to_string(),
            "bottom-center" => ".anchor(gpui::Anchor::BottomCenter)".to_string(),
            "left-center" => ".anchor(gpui::Anchor::LeftCenter)".to_string(),
            "right-center" => ".anchor(gpui::Anchor::RightCenter)".to_string(),
            _ => {
                eprintln!(
                    "[rml warning] unknown anchor value `{}`, expected one of: \
                     top-left, top-right, bottom-left, bottom-right, top-center, \
                     bottom-center, left-center, right-center",
                    value
                );
                String::new()
            }
        },
        // anchored 专用：offset 偏移量 "x,y"（如 "10px,5px"）
        "offset" => parse_point_method("offset", value),
        // anchored 专用：snap_to_window 布尔
        "snap-to-window" => {
            if value == "true" {
                ".snap_to_window()".to_string()
            } else {
                String::new()
            }
        }
        // overflow 布尔标志（Tailwind 风格）
        _ if let Some(s) = super::style_attr::apply_overflow_flag_attr(name, value) => s,
        // 已废弃的 Tailwind 式散落属性：输出 deprecation warning 并丢弃
        "h_flex" | "v_flex" | "h_full" | "w_full" | "min_w_0" | "min_h_0" => {
            eprintln!(
                "[rml deprecation] `{}` is deprecated; use normalized CSS attribute instead \
                 (e.g. display=\"flex\" flex-direction=\"row\" for h-flex, width=\"full\" for w-full, \
                 min-width=\"0\" for min-w-0)",
                name
            );
            String::new()
        }
        _ => {
            // 归一化样式属性：复用 css::mapper 单一映射源
            if let Some(s) = super::style_attr::apply_style_attr(name, value) {
                return s;
            }
            eprintln!(
                "[rml warning] unknown static attribute `{}` (value={:?}) on native element; \
                 property will be dropped. Register it in props_registry or add a match arm.",
                name, value
            );
            String::new()
        }
    }
}

/// 从元素的 class/id 属性提取值，匹配 CSS 样式表，返回 GPUI 方法调用代码
///
/// `parents` 为父元素链（从根到直接父元素），用于后代/子选择器匹配。
pub(crate) fn apply_css_styles(
    elem: &Element,
    tag: &str,
    sheet: &css::StyleSheet,
    parents: &[css::ParentInfo],
) -> String {
    let mut class_value: String = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "class" => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();

    // 组件标签隐式携带与其小写标签名相同的 class，使 CSS 类选择器可直接命中组件
    if let Some(implicit) = tags::implicit_class_for(tag) {
        if !class_value.is_empty() {
            class_value.push(' ');
        }
        class_value.push_str(&implicit);
    }

    let id_value: Option<&str> = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "id" => Some(value.as_str()),
            _ => None,
        });

    css::styles_for_class_with_parents(sheet, tag, &class_value, id_value, parents)
}

/// 追加 CSS class 样式到 code（构造器之后、属性 setter 之前调用）
///
/// 封装 `stylesheet` 存在性检查 + `apply_css_styles` 调用 + 空字符串过滤，
/// 供所有扩展组件 translator/gen 函数在构造器与 setter 循环之间统一调用。
/// GPUI "last write wins"：class 先应用，setter 后应用 → setter 优先级更高。
pub(crate) fn append_css_class_styles(
    code: &mut String,
    elem: &Element,
    tag: &str,
    sheet: Option<&css::StyleSheet>,
    parents: &[css::ParentInfo],
) {
    if let Some(sheet) = sheet {
        let style_code = apply_css_styles(elem, tag, sheet, parents);
        if !style_code.is_empty() {
            code.push_str(&style_code);
        }
    }
}

/// 解析 inline style 属性（如 `style="padding: 10px; color: red;"`）
pub(crate) fn apply_inline_style(style_str: &str) -> String {
    let wrapped = format!("* {{ {} }}", style_str);
    match css::parse(&wrapped) {
        Ok(sheet) => {
            if sheet.rules.is_empty() || sheet.rules[0].declarations.is_empty() {
                return String::new();
            }
            css::mapper::map_declarations(&sheet.rules[0].declarations, &sheet.variables)
        }
        Err(_) => String::new(),
    }
}

/// 应用绑定属性（{field} 形式）
///
/// - `content={expr}`：通过 `IntoContent` trait 转换为 child（支持 IntoElement/ToString/IVisual）
/// - `value={expr}`：格式化为文本 child
/// - `class`/`id`：由 `apply_css_styles` 处理 static 形式，bind 形式输出 warning
/// - `style={expr}`：输出 warning（bind 形式不支持，应使用 static `style="..."`）
/// - `disabled`/`checked`/`readonly`：条件 `.when(...)` 包装
/// - 未知属性：输出 warning + 返回空字符串
pub(crate) fn apply_bind_attr(
    name: &str,
    expr: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    match name {
        // content={expr}：通过 IntoContent trait 统一转换
        // 支持 IntoElement（String/SharedString/AnyElement）、ToString（i32/bool 等）、IVisual（&dyn IVisual 等）
        // 表达式经 gen_expr_code 处理：slot 上下文中 self. 替换为 __rml_self_ref.，
        // _window/cx 作为 scope_vars 识别为 render 方法作用域变量（不加 self. 前缀）
        "content" => {
            let mut scope_vars: Vec<&str> = loop_vars.iter().copied().collect();
            for v in ["_window", "cx"] {
                if !scope_vars.contains(&v) {
                    scope_vars.push(v);
                }
            }
            let code = gen_expr_code(expr, &scope_vars, computed);
            let final_code = if needs_borrow_for_content(&code, &scope_vars) {
                format!("&{}", code)
            } else {
                code
            };
            format!(".child(rml_core::content::into_content({}, _window, cx))", final_code)
        }
        "value" => format!(".child(format!(\"{{}}\", {}))", gen_expr_code(expr, loop_vars, computed)),
        // class/id 的 static 形式由 apply_css_styles 处理；bind 形式静默丢弃
        "class" | "id" => String::new(),
        // style bind 形式不支持（应使用 static style="..."），输出 warning
        "style" => {
            eprintln!(
                "[rml warning] bind form `style={{{}}}` is not supported; \
                 use static form `style=\"...\"` instead. Property will be dropped.",
                expr
            );
            String::new()
        }
        "disabled" | "checked" | "readonly" => {
            format!(".when({}, |el| el)", gen_expr_code(expr, loop_vars, computed))
        }
        _ => {
            // 归一化样式属性 bind 形式不支持（运行时动态样式应走 class= + 主题切换）
            if super::style_attr::is_style_attr(name) {
                eprintln!(
                    "[rml warning] bind form `{}={{{}}}` is not supported for style attribute; \
                     use static form `{}=\"...\"` instead. Property will be dropped.",
                    name, expr, name
                );
                return String::new();
            }
            eprintln!(
                "[rml warning] unknown bind attribute `{}` (expr={:?}); \
                 property will be dropped. Register it in props_registry or add a match arm.",
                name, expr
            );
            String::new()
        }
    }
}

/// 解析 "x,y" 坐标字符串为 `.method_name(gpui::point(gpui::px(x), gpui::px(y)))` 调用
///
/// 支持格式：`"10px,20px"` / `"10,20"` / `"10px 20px"`
fn parse_point_method(method_name: &str, value: &str) -> String {
    let parts: Vec<&str> = value
        .split([',', ' '])
        .filter(|s| !s.trim().is_empty())
        .collect();
    if parts.len() != 2 {
        eprintln!(
            "[rml warning] invalid point value `{}` for {}, expected \"x,y\" (e.g. \"10px,20px\")",
            value, method_name
        );
        return String::new();
    }
    let x = parse_px_value(parts[0].trim());
    let y = parse_px_value(parts[1].trim());
    match (x, y) {
        (Some(xv), Some(yv)) => format!(
            ".{}(gpui::point(gpui::px({}), gpui::px({})))",
            method_name, xv, yv
        ),
        _ => {
            eprintln!(
                "[rml warning] invalid point component in `{}` for {}",
                value, method_name
            );
            String::new()
        }
    }
}

/// 解析 "10px" / "10" / "10.5" 为 f32 值
fn parse_px_value(s: &str) -> Option<f32> {
    let s = s.trim().trim_end_matches("px").trim();
    s.parse::<f32>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_bind_wraps_with_into_content() {
        let code = apply_bind_attr("content", "self.title", &[], &[]);
        // 简单字段访问自动添加 & 前缀（由 IntoContent for &T blanket impl 接管）
        assert_eq!(
            code,
            ".child(rml_core::content::into_content(&self.title, _window, cx))"
        );
    }

    #[test]
    fn content_bind_preserves_complex_expr() {
        let code = apply_bind_attr("content", "self.counter + 1", &[], &[]);
        assert!(code.contains("into_content("));
        assert!(code.contains("self.counter + 1"));
        assert!(!code.contains("&self.counter"));
        assert!(code.contains("_window, cx"));
    }

    #[test]
    fn content_bind_supports_window_cx_scope_vars() {
        // content 表达式可引用 _window/cx（不经 gen_expr_code 解析）
        let code = apply_bind_attr("content", "self.render_card(_window, cx)", &[], &[]);
        assert!(code.contains("self.render_card(_window, cx)"));
        // 方法调用（含括号）不加 & 前缀
        assert!(!code.contains("&self.render_card"));
    }

    #[test]
    fn content_bind_no_borrow_for_loop_vars() {
        // 循环变量已是 &T（来自 .iter()），不加 & 前缀
        let code = apply_bind_attr("content", "item", &["item"], &[]);
        assert!(code.contains("into_content(item,"));
        assert!(!code.contains("&item"));
    }

    #[test]
    fn content_bind_borrow_for_nested_field() {
        // 嵌套字段访问也加 & 前缀
        let code = apply_bind_attr("content", "self.user.name", &[], &[]);
        assert!(code.contains("into_content(&self.user.name,"));
    }
}
