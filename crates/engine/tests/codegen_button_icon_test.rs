//! Button icon 属性代码生成测试
//!
//! 验证 `<Button icon="Play" />` 生成 `.icon(rml_ui::Icon::new(rml_ui::IconName::Play))`。

use rust_rml_engine::compiler::CodegenCtx;
use rust_rml_engine::compiler::translator::TranslatorRegistry;
use rust_rml_engine::parser::Span;
use rust_rml_engine::parser::ast::{Attribute, Element, Node};

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

fn r#static(name: &str, value: &str) -> Attribute {
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

#[test]
fn button_icon_static_generates_icon_call() {
    let elem = make_element(
        "Button",
        vec![r#static("icon", "Play"), r#static("label", "启动")],
        vec![],
    );
    let code = gen(&elem);
    assert!(
        code.contains(".icon(rml_ui::Icon::new(rml_ui::IconName::Play))"),
        "icon 映射: {}",
        code
    );
    assert!(code.contains(".label(\"启动\")"), "label 仍生效: {}", code);
}

#[test]
fn button_icon_with_variant_combines() {
    let elem = make_element(
        "Button",
        vec![
            r#static("icon", "Delete"),
            r#static("primary", ""),
            r#static("compact", ""),
        ],
        vec![],
    );
    let code = gen(&elem);
    assert!(
        code.contains(".icon(rml_ui::Icon::new(rml_ui::IconName::Delete))"),
        "icon 映射: {}",
        code
    );
    assert!(code.contains(".primary()"), "variant 仍生效: {}", code);
    assert!(code.contains(".compact()"), "compact 仍生效: {}", code);
}
