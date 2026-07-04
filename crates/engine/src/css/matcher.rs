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

/// 父元素信息（用于后代/子选择器匹配）
///
/// 使用 owned `String` 以避免跨层级生命周期复杂度。
/// 由 codegen 在递归遍历 AST 时构建，从根到直接父元素排列。
#[derive(Debug, Clone)]
pub struct ParentInfo {
    pub tag: String,
    pub classes: Vec<String>,
    pub id: Option<String>,
}

/// 元素匹配上下文：当前元素的 tag、class 列表、id + 父链
#[derive(Debug, Clone)]
pub struct ElementContext<'a> {
    pub tag: &'a str,
    pub classes: Vec<&'a str>,
    pub id: Option<&'a str>,
    /// 父元素链（从根到直接父元素，不含当前元素）
    pub parents: Vec<ParentInfo>,
}

impl<'a> ElementContext<'a> {
    pub fn new(tag: &'a str, classes: Vec<&'a str>, id: Option<&'a str>) -> Self {
        Self {
            tag,
            classes,
            id,
            parents: Vec::new(),
        }
    }

    /// 从 `class="a b c"` 字符串构建 classes 列表
    pub fn from_class_attr(tag: &'a str, class_value: &'a str, id: Option<&'a str>) -> Self {
        let classes: Vec<&str> = class_value.split_whitespace().collect();
        Self::new(tag, classes, id)
    }

    /// 带父链构建
    pub fn with_parents(
        tag: &'a str,
        classes: Vec<&'a str>,
        id: Option<&'a str>,
        parents: Vec<ParentInfo>,
    ) -> Self {
        Self {
            tag,
            classes,
            id,
            parents,
        }
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
        // 后代选择器 `A B`：B 匹配当前元素，A 匹配任一祖先
        Selector::Descendant(ancestor, descendant) => {
            if !matches_selector(descendant, ctx) {
                return false;
            }
            matches_in_parents(ancestor, &ctx.parents)
        }
        // 子选择器 `A > B`：B 匹配当前元素，A 匹配直接父元素
        Selector::Child(parent, child) => {
            if !matches_selector(child, ctx) {
                return false;
            }
            match ctx.parents.last() {
                Some(immediate) => {
                    let ancestors = &ctx.parents[..ctx.parents.len() - 1];
                    matches_on_parent(parent, immediate, ancestors)
                }
                None => false,
            }
        }
    }
}

/// 在父链中查找任一匹配给定选择器的元素
///
/// `parents` 从根到直接父元素排列；从直接父元素向根方向遍历。
fn matches_in_parents(sel: &Selector, parents: &[ParentInfo]) -> bool {
    for i in (0..parents.len()).rev() {
        let ancestors = &parents[..i];
        if matches_on_parent(sel, &parents[i], ancestors) {
            return true;
        }
    }
    false
}

/// 匹配选择器是否命中指定父元素（带其祖先链）
///
/// 处理后代/子组合器在父元素层级的递归匹配。
fn matches_on_parent(sel: &Selector, parent: &ParentInfo, ancestors: &[ParentInfo]) -> bool {
    match sel {
        Selector::Universal => true,
        Selector::Tag(name) => parent.tag == name.as_str(),
        Selector::Class(name) => parent.classes.iter().any(|c| c == name),
        Selector::Id(name) => parent.id.as_deref() == Some(name.as_str()),
        Selector::Compound(parts) => parts.iter().all(|p| matches_on_parent(p, parent, ancestors)),
        Selector::Descendant(ancestor, descendant) => {
            if !matches_on_parent(descendant, parent, ancestors) {
                return false;
            }
            matches_in_parents(ancestor, ancestors)
        }
        Selector::Child(par, child) => {
            if !matches_on_parent(child, parent, ancestors) {
                return false;
            }
            match ancestors.last() {
                Some(immediate) => {
                    let grand_ancestors = &ancestors[..ancestors.len() - 1];
                    matches_on_parent(par, immediate, grand_ancestors)
                }
                None => false,
            }
        }
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

/// 便捷入口：给定 class 属性值，返回样式代码（无父链）
pub fn styles_for_class(sheet: &StyleSheet, tag: &str, class_value: &str, id: Option<&str>) -> String {
    let ctx = ElementContext::from_class_attr(tag, class_value, id);
    generate_styles(sheet, &ctx)
}

/// 便捷入口：带父链返回样式代码
///
/// `parents` 从根到直接父元素排列。用于后代/子选择器匹配。
pub fn styles_for_class_with_parents(
    sheet: &StyleSheet,
    tag: &str,
    class_value: &str,
    id: Option<&str>,
    parents: &[ParentInfo],
) -> String {
    let classes: Vec<&str> = class_value.split_whitespace().collect();
    let ctx = ElementContext::with_parents(tag, classes, id, parents.to_vec());
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
    fn descendant_matches_with_parent() {
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
        // 有 .container 父元素 → 匹配
        let parents = vec![ParentInfo {
            tag: "div".into(),
            classes: vec!["container".into()],
            id: None,
        }];
        let ctx = ElementContext::with_parents("h1", vec!["title"], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".text_size(gpui::px(24"));
    }

    #[test]
    fn descendant_does_not_match_without_parent() {
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
        // 无父链 → 不匹配（严格父链匹配，不再退化到末端选择器）
        let ctx = ElementContext::from_class_attr("h1", "title", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.is_empty(), "expected no match without .container parent");
    }

    #[test]
    fn descendant_matches_ancestor_not_immediate_parent() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Descendant(
                Box::new(Selector::Class("root".into())),
                Box::new(Selector::Class("leaf".into())),
            )],
            declarations: vec![Declaration {
                property: "margin".into(),
                value: Value::Length(4.0, Unit::Px),
            }],
        });
        // .root 是祖父，不是直接父元素 → 后代选择器仍应匹配
        let parents = vec![
            ParentInfo {
                tag: "div".into(),
                classes: vec!["root".into()],
                id: None,
            },
            ParentInfo {
                tag: "div".into(),
                classes: vec!["middle".into()],
                id: None,
            },
        ];
        let ctx = ElementContext::with_parents("span", vec!["leaf"], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".m(gpui::px(4"));
    }

    #[test]
    fn child_selector_matches_immediate_parent() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Child(
                Box::new(Selector::Class("list".into())),
                Box::new(Selector::Class("item".into())),
            )],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(8.0, Unit::Px),
            }],
        });
        // 直接父元素是 .list → 匹配
        let parents = vec![ParentInfo {
            tag: "ul".into(),
            classes: vec!["list".into()],
            id: None,
        }];
        let ctx = ElementContext::with_parents("li", vec!["item"], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".p(gpui::px(8"));
    }

    #[test]
    fn child_selector_does_not_match_grandparent() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Child(
                Box::new(Selector::Class("list".into())),
                Box::new(Selector::Class("item".into())),
            )],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(8.0, Unit::Px),
            }],
        });
        // .list 是祖父，不是直接父元素 → 子选择器不匹配
        let parents = vec![
            ParentInfo {
                tag: "div".into(),
                classes: vec!["list".into()],
                id: None,
            },
            ParentInfo {
                tag: "div".into(),
                classes: vec!["wrapper".into()],
                id: None,
            },
        ];
        let ctx = ElementContext::with_parents("li", vec!["item"], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.is_empty(), "child selector should not match grandparent");
    }

    #[test]
    fn child_selector_no_parent_does_not_match() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Child(
                Box::new(Selector::Class("list".into())),
                Box::new(Selector::Class("item".into())),
            )],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(8.0, Unit::Px),
            }],
        });
        // 无父链 → 不匹配
        let ctx = ElementContext::from_class_attr("li", "item", None);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.is_empty());
    }

    #[test]
    fn nested_combinator_descendant_of_child() {
        // .root > .mid .leaf —— Descendant(Child(.root, .mid), .leaf)
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Descendant(
                Box::new(Selector::Child(
                    Box::new(Selector::Class("root".into())),
                    Box::new(Selector::Class("mid".into())),
                )),
                Box::new(Selector::Class("leaf".into())),
            )],
            declarations: vec![Declaration {
                property: "margin".into(),
                value: Value::Length(2.0, Unit::Px),
            }],
        });
        // 父链: [.root, .mid] → .mid 是 .root 的直接子元素，.leaf 是 .mid 的后代 → 匹配
        let parents = vec![
            ParentInfo {
                tag: "div".into(),
                classes: vec!["root".into()],
                id: None,
            },
            ParentInfo {
                tag: "div".into(),
                classes: vec!["mid".into()],
                id: None,
            },
        ];
        let ctx = ElementContext::with_parents("span", vec!["leaf"], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".m(gpui::px(2"));
    }

    #[test]
    fn nested_combinator_descendant_of_child_no_match() {
        // .root > .mid .leaf —— Descendant(Child(.root, .mid), .leaf)
        // 不匹配场景：.mid 的直接父是 .other 而非 .root → Child(.root, .mid) 失败
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Descendant(
                Box::new(Selector::Child(
                    Box::new(Selector::Class("root".into())),
                    Box::new(Selector::Class("mid".into())),
                )),
                Box::new(Selector::Class("leaf".into())),
            )],
            declarations: vec![Declaration {
                property: "margin".into(),
                value: Value::Length(2.0, Unit::Px),
            }],
        });
        // 父链 [.root, .other, .mid]：.mid 的直接父是 .other，不是 .root
        let parents = vec![
            ParentInfo {
                tag: "div".into(),
                classes: vec!["root".into()],
                id: None,
            },
            ParentInfo {
                tag: "div".into(),
                classes: vec!["other".into()],
                id: None,
            },
            ParentInfo {
                tag: "div".into(),
                classes: vec!["mid".into()],
                id: None,
            },
        ];
        let ctx = ElementContext::with_parents("span", vec!["leaf"], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(
            code.is_empty(),
            "should not match: .mid's immediate parent is .other, not .root"
        );
    }

    #[test]
    fn tag_child_selector_matches() {
        // ul > li —— Child(Tag(ul), Tag(li))
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Child(
                Box::new(Selector::Tag("ul".into())),
                Box::new(Selector::Tag("li".into())),
            )],
            declarations: vec![Declaration {
                property: "padding".into(),
                value: Value::Length(4.0, Unit::Px),
            }],
        });
        // 直接父是 ul → 匹配
        let parents = vec![ParentInfo {
            tag: "ul".into(),
            classes: vec![],
            id: None,
        }];
        let ctx = ElementContext::with_parents("li", vec![], None, parents);
        let code = generate_styles(&sheet, &ctx);
        assert!(code.contains(".p(gpui::px(4"));
    }

    #[test]
    fn styles_for_class_with_parents_convenience() {
        let sheet = sheet_with(Rule {
            selectors: vec![Selector::Child(
                Box::new(Selector::Class("parent".into())),
                Box::new(Selector::Class("child".into())),
            )],
            declarations: vec![Declaration {
                property: "margin".into(),
                value: Value::Length(6.0, Unit::Px),
            }],
        });
        let parents = vec![ParentInfo {
            tag: "div".into(),
            classes: vec!["parent".into()],
            id: None,
        }];
        let code = styles_for_class_with_parents(&sheet, "span", "child", None, &parents);
        assert!(code.contains(".m(gpui::px(6"));
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
