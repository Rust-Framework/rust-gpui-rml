//! PascalCase Input/TextInput InputStateBridge 双向绑定测试
//!
//! 验证 Phase C2：`<Input value={field}>` / `<TextInput value={field}>`
//! 自动路由到 gen_model_input，复用 InputState 双向同步机制。

use rust_rml_engine::compiler::translator::TranslatorRegistry;
use rust_rml_engine::compiler::CodegenCtx;
use rust_rml_engine::parser::ast::{Attribute, Directive, Element, EventHandler, Node};
use rust_rml_engine::parser::Span;

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

fn bind(name: &str, expr: &str) -> Attribute {
    Attribute::Bind {
        name: name.into(),
        expr: expr.into(),
        span: Span::empty(),
    }
}

fn event(name: &str, handler: &str) -> Attribute {
    Attribute::Event {
        name: name.into(),
        handler: EventHandler::Ident(handler.into()),
        span: Span::empty(),
    }
}

fn static_attr(name: &str, value: &str) -> Attribute {
    Attribute::Static {
        name: name.into(),
        value: value.into(),
        span: Span::empty(),
    }
}

fn gen(elem: &Element) -> String {
    let registry = TranslatorRegistry::builtin();
    let translator = registry.resolve(elem).expect("no translator found for tag");
    let mut id_counter = 0;
    let codegen_ctx = ctx();
    let (code, _) = translator
        .to_rust(elem, &codegen_ctx, &mut id_counter, &[], &[])
        .expect("codegen failed");
    code
}

// ── Input ─────────────────────────────────────────────────────

#[test]
fn pascal_input_value_twoway() {
    let elem = make_element("Input", vec![bind("value", "name")], vec![]);
    let code = gen(&elem);
    assert!(code.contains("__rml_get_or_init_input_state"), "InputStateBridge 路径: {}", code);
    assert!(code.contains("rml_ui::Input::new"), "正确构造器: {}", code);
    assert!(code.contains("\"name\""), "字段名传递: {}", code);
    assert!(!code.contains(".value("), "不应生成单向 value setter: {}", code);
}

#[test]
fn pascal_input_value_with_placeholder() {
    let elem = make_element(
        "Input",
        vec![bind("value", "name"), static_attr("placeholder", "用户名")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains("__rml_get_or_init_input_state"), "InputStateBridge: {}", code);
    assert!(code.contains("Some(\"用户名\")"), "placeholder 传递: {}", code);
}

#[test]
fn pascal_input_value_disabled() {
    let elem = make_element(
        "Input",
        vec![bind("value", "name"), static_attr("disabled", "true")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains(".disabled(true)"), "disabled 属性: {}", code);
}

// ── TextInput ─────────────────────────────────────────────────

#[test]
fn pascal_textinput_value_twoway() {
    let elem = make_element("TextInput", vec![bind("value", "email")], vec![]);
    let code = gen(&elem);
    assert!(code.contains("__rml_get_or_init_input_state"), "TextInput InputStateBridge: {}", code);
    assert!(code.contains("rml_ui::Input::new"), "TextInput 复用 Input 构造器: {}", code);
    assert!(code.contains("\"email\""), "字段名: {}", code);
    assert!(!code.contains(".value("), "不应单向 value: {}", code);
}

// ── ref 模式不走双向绑定 ───────────────────────────────────────

#[test]
fn pascal_input_ref_no_twoway() {
    let elem = Element {
        tag: "Input".into(),
        attributes: vec![],
        directives: vec![Directive::Ref { name: "my_input".into(), span: Span::empty() }],
        children: vec![],
        slot_name: None,
        ..Default::default()
    };
    let code = gen(&elem);
    assert!(!code.contains("__rml_get_or_init_input_state"), "ref 模式不走 InputStateBridge: {}", code);
    assert!(code.contains("get_or_init_ref"), "走 ref 标准 Stateful 路径: {}", code);
}

// ── Converter ─────────────────────────────────────────────────

#[test]
fn pascal_input_value_with_converter() {
    let elem = make_element("Input", vec![bind("value", "price | Currency")], vec![]);
    let code = gen(&elem);
    assert!(code.contains("__rml_get_or_init_input_state"), "InputStateBridge: {}", code);
    assert!(code.contains("\"price\""), "converter 剥离后字段名: {}", code);
    assert!(!code.contains("Currency"), "converter 不出现在元素代码中: {}", code);
}

// ── on_change 合并 ────────────────────────────────────────────

#[test]
fn pascal_input_value_with_on_change() {
    let elem = make_element(
        "Input",
        vec![bind("value", "name"), event("on_change", "on_name_change")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains("__rml_get_or_init_input_state"), "InputStateBridge: {}", code);
    assert!(!code.contains("cx.subscribe"), "on_change 由 InputState 内部处理，不在元素代码中 subscribe: {}", code);
}

// ── 无 value 绑定走标准路径 ───────────────────────────────────

#[test]
fn pascal_input_without_value_no_twoway() {
    let elem = make_element("Input", vec![static_attr("disabled", "true")], vec![]);
    let code = gen(&elem);
    assert!(!code.contains("__rml_get_or_init_input_state"), "无 value 绑定不走 InputStateBridge: {}", code);
    assert!(code.contains("rml_ui::Input::new"), "标准 Stateful 构造: {}", code);
}
