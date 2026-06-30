//! CSS 选择器匹配器
//!
//! 将元素信息（tag/classes/id）与 `StyleSheet` 中的规则匹配，
//! 收集适用的声明并交给 `mapper` 生成 GPUI 方法调用代码。
//!
//! 详见文档 §7.2.3 支持的选择器。
//!
//! ## 匹配优先级
//!
//! 收集所有匹配规则的声明（按规则出现顺序），后出现的同名属性覆盖前者。
//! 这与文档 §7.1 描述的「全局样式表 < class 属性 < inline style」一致：
//! 本模块只负责「全局样式表 + class 属性」层级，inline style 由 codegen 直接处理。

use super::ast::*;
use super::mapper;

/// 元素匹配上下文：当前元素的 tag、class 列表、id
#[derive(Debug, Clone)]
pub struct ElementContext<'a> {
    pub tag: &'a str,
    pub classes: Vec<&'a str>,
    pub id: Option<&'a str>,
}

impl<'a> ElementContext<'a> {
    pub fn new(tag: &'a str, classes: Vec<&'a str>, id: Option<&'a str>) -> Self {
        Self { tag, classes, id }
    }

    /// 从 `class="a b c"` 字符串构建 classes 列表
    pub fn from_class_attr(tag: &'a str, class_value: &'a str, id: Option<&'a str>) -> Self {
        let classes: Vec<&str> = class_value.split_whitespace().collect();
        Self::new(tag, classes, id)
    }
}

/// 匹配单个选择器是否命中当前元素
pub fn matches_selector(sel: &Selector, ctx: &ElementContext) -> bool {
    match sel {
        Selector::Universal => true,
        Selector::Tag(name) => ctx.tag == name.as_str(),
        Selector::Class(name) => ctx.classes.iter().any(|c| *c == name.as_str()),
        Selector::Id(name) => ctx.id == Some(name.as_str()),
        Selector::Compound(parts) => parts.iter().all(|p| matches_selector(p, ctx)),
        // 后代/子选择器需要父元素上下文，此处简化为只匹配末端选择器
        // 完整 DOM 树匹配需要 codegen 传递父链，留待后续增强
        Selector::Descendant(_, descendant) => matches_selector(descendant, ctx),
        Selector::Child(_, child) => matches_selector(child, ctx),
    }
}

/// 收集所有匹配规则的声明（按规则出现顺序）
///
/// 后出现的规则中同名属性会覆盖先前的，由 `HashMap` 去重保留最后值。
pub fn collect_matching_declarations<'a>(
    sheet: &'a StyleSheet,
    ctx: &ElementContext,
) -> Vec<&'a Declaration> {
    let mut result = Vec::new();
    for rule in &sheet.rules {
        let hit = rule.selectors.iter().any(|sel| matches_selector(sel, ctx));
        if hit {
            for decl in &rule.declarations {
                result.push(decl);
            }
        }
    }
    result
}

/// 生成元素的全部样式代码（合并所有匹配规则的声明）
///
/// 返回形如 `.p(gpui::px(10)).bg(gpui::rgb(0xff0000))` 的字符串，
/// 直接拼接到元素构造调用之后。
pub fn generate_styles(sheet: &StyleSheet, ctx: &ElementContext) -> String {
    let decls = collect_matching_declarations(sheet, ctx);
    let decl_refs: Vec<Declaration> = decls.iter().map(|d| (*d).clone()).collect();
    mapper::map_declarations(&decl_refs, &sheet.variables)
}

/// 便捷入口：给定 class 属性值，返回样式代码
pub fn styles_for_class(sheet: &StyleSheet, tag: &str, class_value: &str, id: Option<&str>) -> String {
    let ctx = ElementContext::from_class_attr(tag, class_value, id);
    generate_styles(sheet, &ctx)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn sheet_with(rule: Rule) -> StyleSheet {
        StyleSheet {
            rules: vec![rule],
            variables: HashMap::new(),
        }
    }

    fn class_rule(class: &str, prop: &str, value: Value) -> StyleSheet {
        sheet_with(Rule {
            selectors: vec![Selector::Class(class.into())],
            declarations: vec![Declaration {
                property: prop.into(),
                value,
            }],
        })
    }

    #[test]
    fn match_class_selector() {
        let sheet = class_rule("card", "padding", Value::Length(10.0, Unit::Px));
        let ctx = ElementContext::from_class_attr("div", "card", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".p(gpui::px(10"));
    }

    #[test]
    fn match_tag_selector() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Tag("div".into())],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(8.0, Unit::Px),
            }],
        });
        let ctx = ElementContext::from_class_attr("div", "", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".p(gpui::px(8"));
    }

    #[test]
    fn match_id_selector() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Id("main".into())],
            declarations: vec![Declaration {
                property: "background".into(),
                value: Value::Color(Color::rgb(255, 255, 255)),
            }],
        });
        let ctx = ElementContext::from_class_attr("div", "", Some("main"));
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".bg(gpui::rgb("));
    }

    #[test]
    fn match_universal_selector() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Universal],
            declarations: vec![Declaration {
                property: "margin".into(),
                value: Value::Length(0.0, Unit::Px),
            }],
        });
        let ctx = ElementContext::from_class_attr("span", "", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".m(gpui::px(0"));
    }

    #[test]
    fn match_compound_selector() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Compound(vec![
                Selector::Class("button".into()),
                Selector::Class("primary".into()),
            ])],
            declarations: vec![Declaration {
                property: "background".into(),
                value: Value::Color(Color::rgb(0, 123, 255)),
            }],
        });
        // 同时有 button 和 primary 类 → 匹配
        let ctx = ElementContext::from_class_attr("button", "button primary", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".bg(gpui::rgb("));

        // 只有 button 类 → 不匹配
        let ctx2 = ElementContext::from_class_attr("button", "button", None);
        let code2 = generate_styles(&sheet, &ctx2);
        assert!(code2.is_empty());
    }

    #[test]
    fn match_multiple_classes() {
        let mut sheet = StyleSheet::default();
        sheet.rules.push(Rule {
            selectors: vec![Selector::Class("card".into())],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(10.0, Unit::Px),
            }],
        });
        sheet.rules.push(Rule {
            selectors: vec![Selector::Class("shadow".into())],
            declarations: vec![Declaration {
                property: "background".into(),
                value: Value::Color(Color::rgb(0, 0, 0)),
            }],
        });
        let ctx = ElementContext::from_class_attr("div", "card shadow", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".p(gpui::px(10"));
        assert!(code.contains(".bg(gpui::rgb("));
    }

    #[test]
    fn later_rule_overrides_earlier() {
        let mut sheet = StyleSheet::default();
        sheet.rules.push(Rule {
            selectors: vec![Selector::Class("btn".into())],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(10.0, Unit::Px),
            }],
        });
        sheet.rules.push(Rule {
            selectors: vec![Selector::Class("btn-lg".into())],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(20.0, Unit::Px),
            }],
        });
        let ctx = ElementContext::from_class_attr("button", "btn btn-lg", None);
        let code = generate_styles(&sheet, &ctx);
        // 两个 .p() 都会生成（不去重），后者在 GPUI 中覆盖前者
        assert!(code.contains(".p(gpui::px(10"));
        assert!(code.contains(".p(gpui::px(20"));
    }

    #[test]
    fn no_match_returns_empty() {
        let sheet = class_rule("card", "padding", Value::Length(10.0, Unit::Px));
        let ctx = ElementContext::from_class_attr("div", "other", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.is_empty());
    }

    #[test]
    fn descendant_matches_last_selector() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Descendant(
                Box::new(Selector::Class("container".into())),
                Box::new(Selector::Class("title".into())),
            )],
            declarations: vec![Declaration {
                property: "font-size".into(),
                value: Value::Length(24.0, Unit::Px),
            }],
        });
        // 简化匹配：只检查末端 .title
        let ctx = ElementContext::from_class_attr("h1", "title", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".text_size(gpui::px(24"));
    }

    #[test]
    fn color_var_generates_runtime_theme_query() {
        // 颜色属性的 var() 生成运行时主题查询(不构建期内联)
        let mut sheet = StyleSheet::default();
        sheet.variables.insert(
            "--primary".to_string(),
            Value::Color(Color::rgb(0, 123, 255)),
        );
        sheet.rules.push(Rule {
            selectors: vec![Selector::Class("btn".into())],
            declarations: vec![Declaration {
                property: "background".into(),
                value: Value::Var("--primary".into(), None),
            }],
        });
        let ctx = ElementContext::from_class_attr("button", "btn", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(
            code.contains("rml::theme::color(\"--primary\")"),
            "expected runtime theme query, got: {}",
            code
        );
        assert!(code.contains(".bg("));
    }
}
