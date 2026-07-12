//! Select/Combobox StateBridge 双向绑定测试

use rust_rml_engine::compiler::{compile, CodegenCtx};
use rust_rml_engine::compiler::translator::TranslatorRegistry;
use rust_rml_engine::parser::ast::{Attribute, Directive, Element, Node};
use rust_rml_engine::parser::Span;
use std::collections::HashMap;

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

fn compile_rml(source: &str, field_types: HashMap<String, String>) -> String {
    let codegen_ctx = CodegenCtx {
        view_struct_name: "TestView".into(),
        view_module_path: "test::view".into(),
        field_types,
        registry: TranslatorRegistry::builtin(),
        ..Default::default()
    };
    let output = compile(source, &codegen_ctx).expect("compile failed");
    output.code
}

#[test]
fn select_value_twoway() {
    let elem = make_element(
        "Select",
        vec![bind("items", "fruits"), bind("value", "selected")],
        vec![],
    );
    let code = gen(&elem);
    assert!(
        code.contains("__rml_get_or_init_select_state"),
        "Select StateBridge 路径: {}",
        code
    );
    assert!(code.contains("rml_ui::Select::new"), "正确构造器: {}", code);
    assert!(
        code.contains("\"selected\"") && code.contains("\"fruits\""),
        "字段名传递: {}",
        code
    );
    assert!(
        !code.contains(".value("),
        "不应生成单向 value setter: {}",
        code
    );
}

#[test]
fn select_value_requires_items() {
    let elem = make_element("Select", vec![bind("value", "selected")], vec![]);
    let registry = TranslatorRegistry::builtin();
    let translator = registry.resolve(&elem).unwrap();
    let mut id_counter = 0;
    let err = translator
        .to_rust(&elem, &ctx(), &mut id_counter, &[], &[])
        .unwrap_err();
    assert!(
        err.message.contains("items"),
        "缺少 items 应报错: {}",
        err.message
    );
}

#[test]
fn select_ref_no_twoway() {
    let elem = Element {
        tag: "Select".into(),
        attributes: vec![bind("items", "fruits"), bind("value", "selected")],
        directives: vec![Directive::Ref {
            name: "my_select".into(),
            span: Span::empty(),
        }],
        children: vec![],
        slot_name: None,
        ..Default::default()
    };
    let code = gen(&elem);
    assert!(
        !code.contains("__rml_get_or_init_select_state"),
        "ref 模式不走 StateBridge: {}",
        code
    );
    assert!(code.contains("get_or_init_ref"), "走 ref 标准路径: {}", code);
}

#[test]
fn select_state_impl_generated() {
    let code = compile_rml(
        r#"<component><Select items={fruits} value={selected} /></component>"#,
        HashMap::new(),
    );
    assert!(
        code.contains("fn __rml_get_or_init_select_state"),
        "生成 select_state 方法: {}",
        code
    );
    assert!(
        code.contains("SelectEvent::Confirm"),
        "订阅 Confirm 事件: {}",
        code
    );
    assert!(
        code.contains("set_selected_value"),
        "正向 set_selected_value: {}",
        code
    );
}

#[test]
fn select_state_impl_forward_reverse() {
    let mut types = HashMap::new();
    types.insert("selected".to_string(), "String".to_string());
    let code = compile_rml(
        r#"<component><Select items={fruits} value={selected} /></component>"#,
        types,
    );
    assert!(
        code.contains("self.selected.clone()"),
        "正向同步: {}",
        code
    );
    assert!(
        code.contains("this.selected = v"),
        "反向赋值: {}",
        code
    );
}

#[test]
fn combobox_value_twoway() {
    let elem = make_element(
        "Combobox",
        vec![bind("items", "tags"), bind("value", "selected_tags")],
        vec![],
    );
    let code = gen(&elem);
    assert!(
        code.contains("__rml_get_or_init_combobox_state"),
        "Combobox StateBridge 路径: {}",
        code
    );
    assert!(code.contains("rml_ui::Combobox::new"), "正确构造器: {}", code);
}

#[test]
fn combobox_state_impl_generated() {
    let code = compile_rml(
        r#"<component><Combobox items={tags} value={selected_tags} /></component>"#,
        HashMap::new(),
    );
    assert!(
        code.contains("fn __rml_get_or_init_combobox_state"),
        "生成 combobox_state 方法: {}",
        code
    );
    assert!(
        code.contains("ComboboxEvent::Change"),
        "订阅 Change 事件: {}",
        code
    );
    assert!(
        code.contains("clear_selection"),
        "正向 clear + add: {}",
        code
    );
}
