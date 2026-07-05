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

use super::text::gen_expr_code;

/// 应用静态属性（class/id/style/src/href/type 等字面量属性）
///
/// 未知属性输出 warning 并返回空字符串（不生成错误代码）。
pub(super) fn apply_static_attr(name: &str, value: &str) -> String {
    match name {
        "class" | "id" => String::new(),
        "ref" => String::new(),
        "style" => apply_inline_style(value),
        "src" | "href" => String::new(),
        "type" => String::new(),
        _ => {
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
pub(super) fn apply_css_styles(
    elem: &Element,
    tag: &str,
    sheet: &css::StyleSheet,
    parents: &[css::ParentInfo],
) -> String {
    let class_value: String = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "class" => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let id_value: Option<&str> = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "id" => Some(value.as_str()),
            _ => None,
        });

    if class_value.is_empty() && id_value.is_none() {
        return String::new();
    }

    css::styles_for_class_with_parents(sheet, tag, &class_value, id_value, parents)
}

/// 解析 inline style 属性（如 `style="padding: 10px; color: red;"`）
fn apply_inline_style(style_str: &str) -> String {
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
/// - `content={expr}`：直接嵌入表达式作为 child（支持 AnyElement/impl IntoElement）
/// - `value={expr}`：格式化为文本 child
/// - `class`/`id`：由 `apply_css_styles` 处理 static 形式，bind 形式输出 warning
/// - `style={expr}`：输出 warning（bind 形式不支持，应使用 static `style="..."`）
/// - `disabled`/`checked`/`readonly`：条件 `.when(...)` 包装
/// - 未知属性：输出 warning + 返回空字符串
pub(super) fn apply_bind_attr(
    name: &str,
    expr: &str,
    loop_vars: &[&str],
    computed: &[&str],
) -> String {
    match name {
        // content={expr}：直接嵌入表达式作为 child（支持 AnyElement/impl IntoElement）
        // 表达式可引用 _window/cx（render 方法作用域内可用），不经 gen_expr_code 解析
        "content" => format!(".child({})", expr),
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
            eprintln!(
                "[rml warning] unknown bind attribute `{}` (expr={:?}); \
                 property will be dropped. Register it in props_registry or add a match arm.",
                name, expr
            );
            String::new()
        }
    }
}
