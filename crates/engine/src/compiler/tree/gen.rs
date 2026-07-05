//! Tree 构造器 codegen —— `Tree::new(self.<state>.as_ref())`。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};
use crate::tags::{self, ComponentTag};

/// 生成 Tree 构造代码
///
/// `Tree::new(self.<state_field>.as_ref())` —— 使用 `as_ref()` 而非 `&` 引用。
/// 属性处理：静态/绑定走公共 setter，事件优先走 Tree 专用 setter（on_activate）。
pub fn gen_tree(
    elem: &Element,
    component: ComponentTag,
    ctx: &CodegenCtx,
    _depth: usize,
    _id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let state_field = match component.kind {
        tags::ComponentKind::Stateful { state_field, .. } => state_field,
        _ => {
            return Err(CodegenError {
                message: "<Tree> component kind mismatch".into(),
            })
        }
    };
    let mut code = format!("{}::new(self.{}.as_ref())", component.ctor_path, state_field);

    let resolved = tags::normalize_component_tag(&elem.tag);
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) =
                    super::super::component::component_static_setter(name, value, &resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, &resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = super::setters::event_setter(name, handler, &resolved) {
                    code.push_str(&s);
                } else if let Some(s) =
                    super::super::component::component_event_setter(name, handler, &resolved)
                {
                    code.push_str(&s);
                }
            }
        }
    }
    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser::ast::{Attribute, Element, EventHandler, Node};
    use crate::parser::Span;
    use crate::tags::{ComponentKind, ComponentTag};

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

    fn tree_component() -> ComponentTag {
        ComponentTag {
            ctor_path: "rml_ui::Tree",
            kind: ComponentKind::Stateful {
                state_field: "tree_state",
                state_ctor: "|_w, c| rml_ui::TreeState::new(c)",
            },
            container: false,
        }
    }

    #[test]
    fn gen_tree_minimal() {
        let elem = make_element("Tree", vec![], vec![]);
        let mut id = 0;
        let code = gen_tree(&elem, tree_component(), &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains("rml_ui::Tree::new"));
        assert!(code.contains("self.tree_state.as_ref()"));
    }

    #[test]
    fn gen_tree_with_on_activate() {
        let elem = make_element(
            "Tree",
            vec![Attribute::Event {
                name: "on_activate".into(),
                handler: EventHandler::Ident("on_activate".into()),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tree(&elem, tree_component(), &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".on_activate_rc("));
        assert!(code.contains("cx.weak_entity()"));
        assert!(code.contains("this.on_activate"));
    }

    #[test]
    fn gen_tree_with_on_select() {
        let elem = make_element(
            "Tree",
            vec![Attribute::Event {
                name: "on_select".into(),
                handler: EventHandler::Ident("on_select".into()),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code = gen_tree(&elem, tree_component(), &ctx(), 0, &mut id, &Vec::new()).unwrap();
        assert!(code.contains(".on_select_rc("));
        assert!(code.contains("cx.weak_entity()"));
        assert!(code.contains("this.on_select"));
    }
}
