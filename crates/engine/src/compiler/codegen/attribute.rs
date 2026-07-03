//! 属性代码生成 —— 静态属性 / CSS 样式 / 内联样式 / 绑定属性
//!
//! 将元素属性转换为 GPUI 构建器方法调用代码。

use crate::css;
use crate::parser::ast::{Attribute, Element};

use super::text::gen_expr_code;

/// 应用静态属性（class/id/style/src/href/type 等字面量属性）
pub(super) fn apply_static_attr(name: &str, value: &str) -> String {
    match name {
        "class" | "id" => String::new(),
        "ref" => String::new(),
        "style" => apply_inline_style(value),
        "src" | "href" => String::new(),
        "type" => String::new(),
        _ => format!(".child({:?})", format!("{}={}", name, value)),
    }
}

/// 从元素的 class/id 属性提取值，匹配 CSS 样式表，返回 GPUI 方法调用代码
pub(super) fn apply_css_styles(elem: &Element, tag: &str, sheet: &css::StyleSheet) -> String {
    let class_value: String = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value } if name == "class" => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();

    let id_value: Option<&str> = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value } if name == "id" => Some(value.as_str()),
            _ => None,
        });

    if class_value.is_empty() && id_value.is_none() {
        return String::new();
    }

    css::styles_for_class(sheet, tag, &class_value, id_value)
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
        "class" | "id" | "style" => String::new(),
        "disabled" | "checked" | "readonly" => {
            format!(".when({}, |el| el)", gen_expr_code(expr, loop_vars, computed))
        }
        _ => format!(".child(format!(\"{{}}\", {}))", gen_expr_code(expr, loop_vars, computed)),
    }
}
