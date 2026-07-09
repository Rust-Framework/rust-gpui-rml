//! RadioGroup 组件代码生成
//!
//! RadioGroup 构造器为 `RadioGroup::vertical(id)` 或 `RadioGroup::horizontal(id)`
//! （`new(id)` 为私有，必须通过关联函数 vertical/horizontal 创建）。
//!
//! ## 布局选择
//!
//! - 默认（无属性） → `RadioGroup::vertical(id)`
//! - `horizontal=""` 或 `horizontal="true"` → `RadioGroup::horizontal(id)`
//! - `layout="horizontal"` → `RadioGroup::horizontal(id)`
//! - `layout="vertical"` → `RadioGroup::vertical(id)`（显式指定，与默认一致）
//!
//! ## 子节点
//!
//! RadioGroup 不实现 `ParentElement`，但有 `.child(impl Into<Radio>)` 方法。
//! 子节点（`<Radio>`）通过标准 `gen_node` 生成 `Radio::new(...)` 代码，
//! 由 `.child(...)` 注入。`From<&'static str>`/`From<String>`/`From<SharedString>`
//! 也已为 Radio 实现，因此纯文本子节点会经 `.label(...)` 路径由 Radio 的 codegen 处理。
//!
//! ## 事件
//!
//! `on_click` 签名为 `Fn(&usize, &mut Window, &mut App)`（选中索引），
//! 由 `component_event_setter` 中 RadioGroup 专属分支处理。

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Directive, Element};

/// 生成 RadioGroup 构造代码
pub fn gen_radio_group(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let resolved = "RadioGroup";
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. ElementId（参考 component.rs 的 ref 处理）
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    });
    let id_code = if let Some(name) = ref_name {
        format!("\"rml_ref:{}\"", name)
    } else {
        let id_val = *id_counter;
        *id_counter += 1;
        format!("(\"rml_el\", {}usize)", id_val)
    };

    // 2. 构造器选择：horizontal 或 layout="horizontal" → horizontal(id)，否则 vertical(id)
    let mut is_horizontal = false;
    for attr in &elem.attributes {
        if let Attribute::Static { name, value, .. } = attr {
            if name == "horizontal" && (value.is_empty() || value.eq_ignore_ascii_case("true")) {
                is_horizontal = true;
            }
            if name == "layout" && value.eq_ignore_ascii_case("horizontal") {
                is_horizontal = true;
            }
        }
    }
    let ctor = if is_horizontal {
        format!("rml_ui::RadioGroup::horizontal({})", id_code)
    } else {
        format!("rml_ui::RadioGroup::vertical({})", id_code)
    };

    let mut code = ctor;

    // 3. 处理其他属性（跳过 horizontal/layout/vertical，已用于构造器选择）
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if name == "horizontal" || name == "layout" || name == "vertical" {
                    continue;
                }
                if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if name == "horizontal" || name == "layout" || name == "vertical" {
                    continue;
                }
                if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name, expr, &lv, &computed, resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, resolved)
                {
                    code.push_str(&s);
                }
            }
        }
    }

    // 4. 子节点：<Radio> → .child(Radio::new(...))，文本 → .child(Radio::new(...).label(...))
    // RadioGroup 有 .child(impl Into<Radio>) 和 .children(impl IntoIterator<Item = impl Into<Radio>>)
    for child in &elem.children {
        let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
        if is_iter {
            code.push_str(&format!("\n            .children({})", child_code));
        } else {
            code.push_str(&format!("\n            .child({})", child_code));
        }
    }

    Ok(code)
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

    fn make_element(attrs: Vec<Attribute>, children: Vec<Node>) -> Element {
        Element {
            tag: "RadioGroup".into(),
            attributes: attrs,
            directives: vec![],
            children,
            slot_name: None,
            ..Default::default()
        }
    }

    #[test]
    fn gen_radio_group_default_vertical() {
        let elem = make_element(vec![], vec![]);
        let mut id = 0;
        let code = gen_radio_group(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::RadioGroup::vertical("));
        assert!(!code.contains("horizontal("));
    }

    #[test]
    fn gen_radio_group_horizontal_attr() {
        let elem = make_element(
            vec![Attribute::Static {
                name: "horizontal".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_radio_group(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::RadioGroup::horizontal("));
        assert!(!code.contains("vertical("));
    }

    #[test]
    fn gen_radio_group_layout_horizontal() {
        let elem = make_element(
            vec![Attribute::Static {
                name: "layout".into(),
                value: "horizontal".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_radio_group(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::RadioGroup::horizontal("));
    }

    #[test]
    fn gen_radio_group_selected_index_static() {
        let elem = make_element(
            vec![Attribute::Static {
                name: "selected_index".into(),
                value: "2".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_radio_group(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".selected_index(Some(2usize))"));
    }

    #[test]
    fn gen_radio_group_skips_layout_in_setter() {
        // layout 属性已用于构造器选择，不应再生成 setter
        let elem = make_element(
            vec![Attribute::Static {
                name: "layout".into(),
                value: "horizontal".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_radio_group(&elem, &ctx(), &mut id, &Vec::new()).unwrap();
        // 不应出现 .layout(...) setter 调用
        assert!(!code.contains(".layout("));
    }
}
