//! CodeEditor 构造器 codegen
//!
//! CodeEditor 基于 Input，仅注入代码编辑器**行为**语义（`code_editor(language)`、`multi_line`、
//! `focus_bordered`/`bordered` 等）。布局与视觉（width/height/font/padding/background 等）不由
//! codegen 硬编码，由使用方通过 RML 样式属性、`class` + CSS 变量、或主题覆盖层定制。
//!
//! ## Input 事件架构
//!
//! CodeEditor 同 Input/TextInput，事件通过 `InputState: EventEmitter<InputEvent>` +
//! `cx.subscribe` 订阅（Input element 无 `.on_change()` 方法）。事件属性经 `is_input_event`
//! 检测后由 block 表达式包装构造器，详见 `input/event.rs`。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, EventHandler};
use crate::tags::{self, ComponentTag};

/// gen_code_editor 内联处理的属性（须在 props_registry 登记，供反向校验）
pub const HANDLED_PROPS: &[&str] = &[
    "value",
    "language",
    "bordered",
    "focus_bordered",
    "context_menu",
    "on_change",
    "on_enter",
    "on_focus",
    "on_blur",
];

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
                span: Some(elem.span),
            })
        }
    };

    let resolved = tags::normalize_component_tag(&elem.tag);
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        crate::parser::ast::Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    });

    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 声明式 value 属性：绑定或静态字符串，用于 InputState::default_value
    let value_expr: Option<String> = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Bind { name, expr, .. } if name == "value" => {
            Some(crate::compiler::codegen::gen_expr_code(expr, &lv, &computed))
        }
        Attribute::Static { name, value, .. } if name == "value" => {
            Some(format!("{:?}.to_string()", value))
        }
        _ => None,
    });

    // 声明式 language 属性：静态字符串，默认 "rml"
    let language: &str = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "language" => Some(value.as_str()),
            _ => None,
        })
        .unwrap_or("rml");

    // 声明式 bordered 属性：控制 Input 的外边框（CodeEditor 默认 false，避免与容器边框重叠）。
    // 未指定时生成 .bordered(false)；指定时按布尔值生成 .bordered(<bool>)。
    let bordered: Option<bool> = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name, value, .. } if name == "bordered" => {
            Some(value.is_empty() || value.eq_ignore_ascii_case("true"))
        }
        _ => None,
    });

    // 声明式 focus_bordered 属性：控制聚焦边框（CodeEditor 默认 false）。
    // 未指定时用默认 false；指定时按布尔值生成 .focus_bordered(<bool>)。
    let focus_bordered: Option<bool> = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name, value, .. } if name == "focus_bordered" => {
            Some(value.is_empty() || value.eq_ignore_ascii_case("true"))
        }
        _ => None,
    });

    // 声明式 context-menu 属性：指定右键菜单构建方法名
    // 方法签名：fn(&self, NativeMenu, &mut Window, &mut Context<Self>) -> NativeMenu
    // 注意：parser 已将 kebab-case `context-menu` 规范化为 snake_case `context_menu`
    let context_menu_method: Option<&str> = elem.attributes.iter().find_map(|attr| match attr {
        Attribute::Static { name, value, .. } if name == "context_menu" && !value.is_empty() => {
            Some(value.as_str())
        }
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

    // CodeEditor 行为默认值：focus_bordered / bordered（Input API 语义，非布局 CSS）。
    // width/height/font/padding 等视觉样式由 RML 属性、class+CSS、主题覆盖层负责。
    let style_chain = {
        let mut s = String::new();
        match focus_bordered {
            Some(b) => s.push_str(&format!("\n            .focus_bordered({})", b)),
            None => s.push_str("\n            .focus_bordered(false)"),
        }
        match bordered {
            Some(b) => s.push_str(&format!("\n            .bordered({})", b)),
            None => s.push_str("\n            .bordered(false)"),
        }
        s
    };

    let ctor_expr = if let Some(value_code) = &value_expr {
        // 声明式 value：内联创建 InputState，无需 editor_state 字段或 on_loaded 初始化
        let ref_key = ref_name.unwrap_or(state_field);
        let ctor_code = format!(
            "move |w, c| rml_ui::InputState::new(w, c).code_editor({:?}).multi_line(true).default_value(&__code)",
            language
        );
        if !input_event_handlers.is_empty() {
            let subscribe_code: String = input_event_handlers
                .iter()
                .map(|(event_name, handler)| {
                    super::super::input::gen_input_event_subscribe(ref_key, event_name, handler)
                })
                .collect::<Vec<_>>()
                .join(" ");
            format!(
                "{{ let __code = {}; let __rml_entity = self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {}); {} {}::new(&__rml_entity){style_chain} }}",
                value_code, ref_key, ctor_code, subscribe_code, component.ctor_path
            )
        } else {
            format!(
                "{{ let __code = {}; {}::new(&self.__rml_state.get_or_init_ref(\"{}\", _window, &mut *cx, {})){style_chain} }}",
                value_code, component.ctor_path, ref_key, ctor_code
            )
        }
    } else if !input_event_handlers.is_empty() {
        // block 表达式：{ let __rml_entity = ...; <subscribe>; Input::new(&__rml_entity) }
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
            "{{ let __rml_entity = {entity_expr}; {subscribe_code} {}::new(&__rml_entity){style_chain} }}",
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

    // context-menu 属性：生成 .context_menu(closure) 调用
    // 闭包通过 cx.entity().update() 桥接 &mut App → &mut Context<Self>
    if let Some(method) = context_menu_method {
        code.push_str(&format!(
            "\n            .context_menu({{\n                \
             let __view = cx.entity();\n                \
             move |menu, w, c| __view.update(c, |this, cx| this.{method}(menu, w, cx))\n            }})"
        ));
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
        assert!(code.contains("rml_ui::Input::new(self.editor_state.as_ref().expect(\"init editor_state in on_loaded\"))"));
        assert!(code.contains(".focus_bordered(false)"));
        assert!(code.contains(".bordered(false)"));
        // 不硬编码布局/视觉样式
        assert!(!code.contains(".font_family("));
        assert!(!code.contains(".w_full()"));
        assert!(!code.contains(".h(gpui::px("));
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
        assert!(code.contains("{ let __rml_entity"));
        assert!(code.contains("cx.subscribe(&__rml_entity"));
        assert!(code.contains("Input::new(&__rml_entity)"));
        assert!(code.contains(".focus_bordered(false)"));
        assert!(!code.contains(".h(gpui::px("));
    }

    #[test]
    fn gen_code_editor_with_value_bind() {
        let mut c = ctx();
        c.computed_methods = vec!["code_sample".into()];
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Bind {
                name: "value".into(),
                expr: "code_sample".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &c, 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains("let __code = self.code_sample();"));
        assert!(code.contains(".code_editor(\"rml\").multi_line(true).default_value(&__code)"));
        assert!(!code.contains("as_ref().expect"));
        assert!(!code.contains(".h(gpui::px("));
    }

    #[test]
    fn gen_code_editor_h_full_deprecated_drops_attribute() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "h_full".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(!code.contains(".h_full()"));
        assert!(!code.contains(".h(gpui::px("));
    }

    #[test]
    fn gen_code_editor_no_hardcoded_height_when_height_attr_set() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "height".into(),
                value: "full".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(!code.contains(".h(gpui::px("));
    }

    #[test]
    fn gen_code_editor_no_hardcoded_font_when_font_family_set() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "font_family".into(),
                value: "Fira Code".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(!code.contains(".font_family(cx.theme()"));
    }

    #[test]
    fn gen_code_editor_no_hardcoded_padding_when_padding_set() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "padding".into(),
                value: "8px".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(!code.contains(".p_0()"));
    }

    #[test]
    fn gen_code_editor_focus_bordered_true() {
        // <CodeEditor focus_bordered="true" /> → .focus_bordered(true)
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "focus_bordered".into(),
                value: "true".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".focus_bordered(true)"));
        assert!(!code.contains(".focus_bordered(false)"));
    }

    #[test]
    fn gen_code_editor_focus_bordered_false_explicit() {
        // <CodeEditor focus_bordered="false" /> → .focus_bordered(false)（与默认一致，但显式生成）
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "focus_bordered".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".focus_bordered(false)"));
    }

    #[test]
    fn gen_code_editor_bordered_false() {
        // <CodeEditor bordered="false" /> → .bordered(false)
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "bordered".into(),
                value: "false".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".bordered(false)"));
    }

    #[test]
    fn gen_code_editor_bordered_empty_defaults_true() {
        // <CodeEditor bordered /> → .bordered(true)（空值按真）
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "bordered".into(),
                value: "".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".bordered(true)"));
    }

    #[test]
    fn gen_code_editor_no_bordered_attr_defaults_false() {
        // 未指定 bordered 时默认生成 .bordered(false) 避免边框重叠
        let elem = make_element("CodeEditor", vec![], vec![]);
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".bordered(false)"));
    }

    #[test]
    fn gen_code_editor_with_value_and_language() {
        let mut c = ctx();
        c.computed_methods = vec!["code_sample".into()];
        let elem = make_element(
            "CodeEditor",
            vec![
                Attribute::Bind {
                    name: "value".into(),
                    expr: "code_sample".into(),
                    span: Span::empty(),
                },
                Attribute::Static {
                    name: "language".into(),
                    value: "rust".into(),
                    span: Span::empty(),
                },
            ],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &c, 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".code_editor(\"rust\").multi_line(true).default_value(&__code)"));
    }

    #[test]
    fn gen_code_editor_with_value_static() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "value".into(),
                value: "let x = 1;".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains("let __code = \"let x = 1;\".to_string();"));
        assert!(code.contains(".default_value(&__code)"));
    }

    #[test]
    fn gen_code_editor_context_menu() {
        let elem = make_element(
            "CodeEditor",
            vec![Attribute::Static {
                name: "context_menu".into(),
                value: "build_editor_menu".into(),
                span: Span::empty(),
            }],
            vec![],
        );
        let mut id = 0;
        let code =
            gen_code_editor(&elem, code_editor_component(), &ctx(), 0, &mut id, &Vec::new())
                .unwrap();
        assert!(code.contains(".context_menu("));
        assert!(code.contains("this.build_editor_menu(menu, w, cx)"));
    }
}
