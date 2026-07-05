//! CodeEditor 构造器 codegen
//!
//! CodeEditor 基于 Input，自动应用 `font_family(mono)` + `text_size(mono)` + `size_full()`。
//!
//! ## Input 事件架构
//!
//! CodeEditor 同 Input/TextInput，事件通过 `InputState: EventEmitter<InputEvent>` +
//! `cx.subscribe` 订阅（Input element 无 `.on_change()` 方法）。事件属性经 `is_input_event`
//! 检测后由 block 表达式包装构造器，详见 `input/event.rs`。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler};
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
    let (state_field, state_ctor) = match component.kind {
        tags::ComponentKind::Stateful { state_field, state_ctor } => (state_field, state_ctor),
        _ => {
            return Err(CodegenError {
                message: "<CodeEditor> component kind mismatch".into(),
            })
        }
    };

    let resolved = tags::normalize_component_tag(&elem.tag);
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        crate::parser::ast::Directive::Ref(name) => Some(name.as_str()),
        _ => None,
    });

    // 收集 Input 事件处理器（on_change/on_enter/on_focus/on_blur）
    // 这些事件不走 setter 链路（component_event_setter 返回 None），
    // 由 block 表达式中的 cx.subscribe 统一处理
    let input_event_handlers: Vec<(&str, &EventHandler)> = elem
        .attributes
        .iter()
        .filter_map(|attr| {
            if let Attribute::Event { name, handler, .. } = attr {
                if super::super::input::is_input_event(name, &resolved) {
                    Some((name.as_str(), handler))
                } else {
                    None
                }
            } else {
                None
            }
        })
        .collect();

    // CodeEditor 额外应用的样式链
    let style_chain = ".font_family(cx.theme().mono_font_family.clone())\n            \
         .text_size(cx.theme().mono_font_size)\n            \
         .size_full()";

    let ctor_expr = if !input_event_handlers.is_empty() {
        // block 表达式：({ let __rml_entity = ...; <subscribe>; Input::new(&__rml_entity) })
        let entity_expr = if let Some(name) = ref_name {
            format!(
                "self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {})",
                name, state_ctor
            )
        } else {
            // no-ref + Input 事件：字段类型为 Option<Entity<T>>，需 as_ref().expect 取出
            format!(
                "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
                state_field, state_field
            )
        };
        let ref_key = ref_name.unwrap_or(state_field);
        let subscribe_code: String = input_event_handlers
            .iter()
            .map(|(event_name, handler)| {
                super::super::input::gen_input_event_subscribe(ref_key, event_name, handler)
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            "({{ let __rml_entity = {entity_expr}; {subscribe_code} {}::new(&__rml_entity){style_chain} }})",
            component.ctor_path
        )
    } else if let Some(name) = ref_name {
        format!(
            "{}::new(&self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {})){style_chain}",
            component.ctor_path, name, state_ctor
        )
    } else {
        // no-ref 路径：字段类型为 Option<Entity<T>>，需 as_ref().expect 取出
        format!(
            "{}::new(self.{}.as_ref().expect(\"init {} in on_loaded\")){style_chain}",
            component.ctor_path, state_field, state_field
        )
    };

    let mut code = ctor_expr;

    // 非事件属性的 setter 链（事件属性由 block 表达式处理，跳过）
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
                // 非 Input 事件委托到通用 component_event_setter
                if !super::super::input::is_input_event(name, &resolved) {
                    if let Some(s) =
                        super::super::component::component_event_setter(name, handler, &resolved)
                    {
                        code.push_str(&s);
                    }
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

    fn code_editor_component() -> ComponentTag {
        ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "editor_state",
                state_ctor: "|w, c| rml_ui::InputState::new(w, c)",
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
        // no-ref 路径：Option<Entity<T>> 字段需 as_ref().expect 取出
        assert!(code.contains("rml_ui::Input::new(self.editor_state.as_ref().expect(\"init editor_state in on_loaded\"))"));
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
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        // block 表达式包装
        assert!(code.contains("({ let __rml_entity"));
        assert!(code.contains("is_event_subscribed"));
        assert!(code.contains("cx.subscribe(&__rml_entity"));
        assert!(code.contains("InputEvent::Change"));
        assert!(code.contains("this.on_editor_change(entity.read(cx), cx)"));
        assert!(code.contains("detach()"));
        assert!(code.contains("mark_event_subscribed"));
        assert!(code.contains("Input::new(&__rml_entity)"));
        // 仍应包含样式链
        assert!(code.contains(".font_family(cx.theme().mono_font_family.clone())"));
        assert!(code.contains(".size_full()"));
    }
}
