//! KeyBinding 构造代码生成
//!
//! ## 构造器
//!
//! `KeyBinding::new()` —— 无 ElementId、无 cx 参数（RenderOnce 组件）。
//!
//! ## 子节点处理
//!
//! KeyBinding 实现 `ParentElement`，子节点通过 `.child()` / `.children()` 注入。
//!
//! ## 属性
//!
//! - `key="Ctrl+S"` (static) → `.key("Ctrl+S")`
//! - `when={cond}` (bind) → `.when(cond)`
//! - `on-press={handler}` (event) → entity 捕获模式

use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

use super::setters;

/// 生成 KeyBinding 外壳（属性 + 样式，不含子节点）
pub fn gen_key_binding_shell(
    elem: &Element,
    ctx: &CodegenCtx,
    _id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let mut code = "rml_ui::KeyBinding::new()".to_string();

    append_css_class_styles(
        &mut code,
        elem,
        "KeyBinding",
        ctx.stylesheet.as_ref(),
        parents,
    );

    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(s) = setters::static_setter(name, value) {
                    code.push_str(&s);
                } else if let Some(s) =
                    crate::compiler::setters::component_static_setter(name, value, "KeyBinding")
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(s) = setters::bind_setter(name, expr, &lv, &computed) {
                    code.push_str(&s);
                } else if let Some(s) = crate::compiler::setters::component_bind_setter(
                    name,
                    expr,
                    &lv,
                    &computed,
                    "KeyBinding",
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(s) = setters::event_setter(name, handler) {
                    code.push_str(&s);
                    continue;
                }
                if let Some(s) =
                    crate::compiler::setters::component_event_setter(name, handler, "KeyBinding")
                {
                    code.push_str(&s);
                }
            }
        }
    }

    Ok(code)
}

/// 生成 KeyBinding 构造代码
pub fn gen_key_binding(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let mut code = gen_key_binding_shell(elem, ctx, id_counter, loop_vars, parents)?;
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
    use crate::parser::ast::{Attribute, Element, EventHandler, Node};
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
            ..Default::default()
        }
    }

    #[test]
    fn gen_key_binding_minimal() {
        let elem = make_element("KeyBinding", vec![], vec![]);
        let code = gen_key_binding(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::KeyBinding::new()"));
    }

    #[test]
    fn gen_key_binding_with_key() {
        let elem = make_element(
            "KeyBinding",
            vec![Attribute::Static {
                name: "key".into(),
                value: "Ctrl+S".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_key_binding(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".key(\"Ctrl+S\")"));
    }

    #[test]
    fn gen_key_binding_with_when() {
        let elem = make_element(
            "KeyBinding",
            vec![Attribute::Bind {
                name: "when".into(),
                expr: "is_active".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let code = gen_key_binding(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".when(self.is_active)"));
    }

    #[test]
    fn gen_key_binding_with_on_press() {
        let elem = make_element(
            "KeyBinding",
            vec![
                Attribute::Static {
                    name: "key".into(),
                    value: "Ctrl+S".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_press".into(),
                    handler: EventHandler::Ident("handle_save".into()),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let code = gen_key_binding(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains(".key(\"Ctrl+S\")"));
        assert!(code.contains(".on_press("));
        assert!(code.contains("let __entity = cx.entity()"));
        assert!(code.contains("this.handle_save(cx)"));
    }

    #[test]
    fn gen_key_binding_rejects_children() {
        let elem = make_element(
            "KeyBinding",
            vec![Attribute::Static {
                name: "key".into(),
                value: "Ctrl+S".into(),
                span: Span::empty(),
            }],
            vec![Node::Text("Content".into())],
        );
        let err = gen_key_binding(&elem, &ctx(), &mut 1, &Vec::new(), &[]);
        // gen_key_binding 仍生成子节点；外层包裹由 KeyBindingTranslator 编译期拒绝
        assert!(err.is_ok());
        let code = err.unwrap();
        assert!(code.contains(".child("));
    }

    #[test]
    fn gen_key_binding_full_example() {
        let elem = make_element(
            "KeyBinding",
            vec![
                Attribute::Static {
                    name: "key".into(),
                    value: "Escape".into(),
                    span: Span::empty(),
                },
                Attribute::Bind {
                    name: "when".into(),
                    expr: "dialog_open".into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_press".into(),
                    handler: EventHandler::Ident("close_dialog".into()),
                    span: Span::empty(),
                },
            ],
            vec![Node::Text("Modal content".into())],
        );
        let code = gen_key_binding(&elem, &ctx(), &mut 1, &Vec::new(), &[]).unwrap();
        assert!(code.contains("rml_ui::KeyBinding::new()"));
        assert!(code.contains(".key(\"Escape\")"));
        assert!(code.contains(".when(self.dialog_open)"));
        assert!(code.contains(".on_press("));
        assert!(code.contains("this.close_dialog(cx)"));
        assert!(code.contains(".child("));
    }
}
