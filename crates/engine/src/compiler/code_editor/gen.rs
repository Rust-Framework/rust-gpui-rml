//! CodeEditor 构造器 codegen
//!
//! 生成 `Input::new(self.editor_state.as_ref().expect(...))`
//!     `.font_family(cx.theme().mono_font_family.clone())`
//!     `.text_size(cx.theme().mono_font_size)`
//!     `.size_full()`
//!
//! 支持事件属性（onchange 等），委托到 Input 事件 setter。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element};
use crate::tags::{self, ComponentTag};

/// 生成 CodeEditor 构造代码
pub fn gen_code_editor(
    elem: &Element,
    component: ComponentTag,
    ctx: &CodegenCtx,
    _depth: usize,
    _id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let state_field = match component.kind {
        tags::ComponentKind::Stateful { state_field } => state_field,
        _ => {
            return Err(CodegenError {
                message: "<CodeEditor> component kind mismatch".into(),
            })
        }
    };

    let mut code = format!(
        "{ctor}::new(self.{field}.as_ref().expect(\"init {field} in on_loaded\"))\n            \
         .font_family(cx.theme().mono_font_family.clone())\n            \
         .text_size(cx.theme().mono_font_size)\n            \
         .size_full()",
        ctor = component.ctor_path,
        field = state_field
    );

    let resolved = tags::normalize_component_tag(&elem.tag);
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                if let Some(s) =
                    super::super::component::component_static_setter(name, value, &resolved)
                {
                    code.push_str(&s);
                }
            }
            Attribute::Bind { name, expr } => {
                if let Some(s) = super::super::component::component_bind_setter(
                    name, expr, &lv, &computed, &resolved,
                ) {
                    code.push_str(&s);
                }
            }
            Attribute::Event { name, handler } => {
                // CodeEditor 事件委托到 Input 事件 setter（onchange 等）
                if let Some(s) = super::super::input::event_setter(name, handler, &resolved) {
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

    fn code_editor_component() -> ComponentTag {
        ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "editor_state",
            },
            container: false,
        }
    }

    #[test]
    fn gen_code_editor_minimal() {
        let elem = make_element("CodeEditor", vec![], vec![]);
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains("rml_ui::Input::new"));
        assert!(code.contains("self.editor_state.as_ref().expect"));
        assert!(code.contains(".font_family(cx.theme().mono_font_family.clone())"));
        assert!(code.contains(".text_size(cx.theme().mono_font_size)"));
        assert!(code.contains(".size_full()"));
    }

    #[test]
    fn gen_code_editor_with_on_change() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Event {
                name: "on_change".into(),
                handler: EventHandler::Ident("on_editor_change".into()),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".on_change("));
        assert!(code.contains("this.on_editor_change"));
    }
}
