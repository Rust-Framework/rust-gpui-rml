//! KeyBinding 附着到焦点宿主（Input 等）的 codegen 辅助
//!
//! ## 唯一写法（声明式子节点）
//!
//! ```rml
//! <Input ref="demo_input" placeholder="...">
//!   <KeyBinding key="Ctrl+S" on-press={on_save} />
//!   <KeyBinding key="Escape" on-press={on_clear} />
//! </Input>
//! ```
//!
//! `KeyBinding` 子节点为声明式元数据，不渲染为 Input 的视觉子节点；
//! codegen 将宿主包裹在 KeyBinding 链中。不支持外层 `<KeyBinding>…</KeyBinding>` 包裹写法。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Element, Node};
use crate::tags;

use super::gen::gen_key_binding_shell;

/// 支持 `<KeyBinding>` 声明式子节点的焦点宿主标签
pub fn is_key_binding_host_tag(tag: &str) -> bool {
    matches!(
        tags::canonical_tag(tag).as_str(),
        "Input" | "TextInput" | "NumberInput" | "CodeEditor" | "Textarea" | "textarea"
    )
}

/// 将子节点分为 KeyBinding 声明（元数据）与其余节点
pub fn partition_key_binding_children(children: &[Node]) -> (Vec<&Element>, Vec<&Node>) {
    let mut key_bindings = Vec::new();
    let mut others = Vec::new();
    for child in children {
        if let Node::Element(elem) = child {
            if tags::canonical_tag(&elem.tag) == "KeyBinding" {
                key_bindings.push(elem);
                continue;
            }
        }
        others.push(child);
    }
    (key_bindings, others)
}

/// 校验焦点宿主的子节点：仅允许 KeyBinding 声明式子节点
pub fn validate_key_binding_host_children(
    host_tag: &str,
    children: &[Node],
    span: crate::parser::Span,
) -> Result<(), CodegenError> {
    if !is_key_binding_host_tag(host_tag) {
        return Ok(());
    }
    let (_, others) = partition_key_binding_children(children);
    if others.is_empty() {
        return Ok(());
    }
    Err(CodegenError {
        message: format!(
            "<{}> 仅接受 <KeyBinding> 作为声明式子节点；\
             请使用 <Input>...<KeyBinding/></Input> 写法，勿混入其他子元素",
            tags::canonical_tag(host_tag)
        ),
        span: Some(span),
    })
}

/// 用 KeyBinding 链包裹已生成的宿主元素代码（由内向外嵌套）
pub fn wrap_with_key_bindings(
    inner_code: String,
    key_bindings: Vec<&Element>,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    if key_bindings.is_empty() {
        return Ok(inner_code);
    }
    let mut wrapped = inner_code;
    for kb in key_bindings.iter().rev() {
        if !kb.children.is_empty() {
            return Err(CodegenError {
                message: "<KeyBinding> 作为焦点宿主子节点时不应再包含子元素；请使用自闭合形式"
                    .into(),
                span: Some(kb.span),
            });
        }
        let shell = gen_key_binding_shell(kb, ctx, id_counter, loop_vars, parents)?;
        wrapped = format!("{shell}.child({wrapped})");
    }
    Ok(wrapped)
}

/// 若宿主含 KeyBinding 子节点，则包裹并返回最终代码
pub fn apply_key_bindings_to_host(
    host_elem: &Element,
    inner_code: String,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    validate_key_binding_host_children(&host_elem.tag, &host_elem.children, host_elem.span)?;
    let (key_bindings, _) = partition_key_binding_children(&host_elem.children);
    wrap_with_key_bindings(inner_code, key_bindings, ctx, id_counter, loop_vars, parents)
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

    fn kb_elem(key: &str, handler: &str) -> Element {
        Element {
            tag: "KeyBinding".into(),
            attributes: vec![
                Attribute::Static {
                    name: "key".into(),
                    value: key.into(),
                    span: Span::empty(),
                },
                Attribute::Event {
                    name: "on_press".into(),
                    handler: EventHandler::Ident(handler.into()),
                    span: Span::empty(),
                },
            ],
            directives: vec![],
            children: vec![],
            slot_name: None,
            span: Span::empty(),
        }
    }

    fn input_with_key_bindings() -> Element {
        Element {
            tag: "Input".into(),
            attributes: vec![Attribute::Static {
                name: "placeholder".into(),
                value: "demo".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![
                Node::Element(kb_elem("Ctrl+S", "on_save")),
                Node::Element(kb_elem("Escape", "on_clear")),
            ],
            slot_name: None,
            span: Span::empty(),
        }
    }

    #[test]
    fn partition_splits_key_bindings() {
        let elem = input_with_key_bindings();
        let (kbs, others) = partition_key_binding_children(&elem.children);
        assert_eq!(kbs.len(), 2);
        assert!(others.is_empty());
    }

    #[test]
    fn wrap_input_with_two_key_bindings() {
        let elem = input_with_key_bindings();
        let (kbs, _) = partition_key_binding_children(&elem.children);
        let inner = "rml_ui::Input::new(&entity)".to_string();
        let code = wrap_with_key_bindings(inner, kbs, &ctx(), &mut 1, &[], &[]).unwrap();
        assert!(code.contains("rml_ui::KeyBinding::new()"));
        assert!(code.contains(".key(\"Ctrl+S\")"));
        assert!(code.contains(".key(\"Escape\")"));
        assert!(code.contains(".on_press("));
        assert!(code.contains(".child(rml_ui::Input::new(&entity))"));
        // 两层 KeyBinding 嵌套
        assert_eq!(code.matches("KeyBinding::new()").count(), 2);
    }

    #[test]
    fn reject_mixed_children_on_input_host() {
        let elem = Element {
            tag: "Input".into(),
            attributes: vec![],
            directives: vec![],
            children: vec![Node::Text("bad".into())],
            slot_name: None,
            span: Span::empty(),
        };
        let err = validate_key_binding_host_children(&elem.tag, &elem.children, elem.span)
            .unwrap_err();
        assert!(err.message.contains("仅接受 <KeyBinding>"));
    }

    #[test]
    fn reject_wrapper_key_binding_with_children() {
        use crate::compiler::translator::component::key_binding::KeyBindingTranslator;
        use crate::compiler::translator::IRmlTranslator;
        let elem = Element {
            tag: "KeyBinding".into(),
            attributes: vec![Attribute::Static {
                name: "key".into(),
                value: "Ctrl+S".into(),
                span: Span::empty(),
            }],
            directives: vec![],
            children: vec![Node::Text("bad".into())],
            slot_name: None,
            span: Span::empty(),
        };
        let translator = KeyBindingTranslator;
        let err = translator
            .to_rust(&elem, &ctx(), &mut 1, &[], &[])
            .unwrap_err();
        assert!(err.message.contains("不支持包裹子元素"));
    }
}
