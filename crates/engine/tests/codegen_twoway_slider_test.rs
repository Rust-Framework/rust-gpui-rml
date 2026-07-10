//! PascalCase Slider StateBridge 双向绑定测试（C3/C4）
//!
//! 验证 `<Slider value={field}>` 自动路由到 gen_model_state_bridge，
//! 以及 gen_state_bridge_impl 生成正向/反向同步代码。

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

// ── 元素级：gen_model_state_bridge 路由 ──────────────────────────────

#[test]
fn slider_value_twoway() {
    let elem = make_element("Slider", vec![bind("value", "volume")], vec![]);
    let code = gen(&elem);
    assert!(
        code.contains("__rml_get_or_init_slider_state"),
        "Slider StateBridge 路径: {}",
        code
    );
    assert!(
        code.contains("rml_ui::Slider::new"),
        "正确构造器: {}",
        code
    );
    assert!(
        code.contains("\"volume\""),
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
fn slider_value_disabled() {
    let elem = make_element(
        "Slider",
        vec![bind("value", "volume"), static_attr("disabled", "true")],
        vec![],
    );
    let code = gen(&elem);
    assert!(
        code.contains(".disabled(true)"),
        "disabled 属性: {}",
        code
    );
    assert!(
        code.contains("__rml_get_or_init_slider_state"),
        "StateBridge 路径: {}",
        code
    );
}

// ── ref 模式不走 StateBridge ───────────────────────────────────

#[test]
fn slider_ref_no_twoway() {
    let elem = Element {
        tag: "Slider".into(),
        attributes: vec![],
        directives: vec![Directive::Ref {
            name: "my_slider".into(),
            span: Span::empty(),
        }],
        children: vec![],
        slot_name: None,
        ..Default::default()
    };
    let code = gen(&elem);
    assert!(
        !code.contains("__rml_get_or_init_slider_state"),
        "ref 模式不走 StateBridge: {}",
        code
    );
    assert!(
        code.contains("get_or_init_ref"),
        "走 ref 标准 Stateful 路径: {}",
        code
    );
}

// ── 无 value 绑定走标准路径 ───────────────────────────────────

#[test]
fn slider_without_value_no_twoway() {
    let elem = make_element("Slider", vec![static_attr("disabled", "true")], vec![]);
    let code = gen(&elem);
    assert!(
        !code.contains("__rml_get_or_init_slider_state"),
        "无 value 绑定不走 StateBridge: {}",
        code
    );
    assert!(
        code.contains("rml_ui::Slider::new"),
        "标准 Stateful 构造: {}",
        code
    );
}

// ── 全管线：gen_state_bridge_impl 生成 ─────────────────────────

fn compile_slider(source: &str, field_types: HashMap<String, String>) -> String {
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
fn slider_state_impl_generated() {
    let code = compile_slider(
        r#"<component><Slider value={volume} /></component>"#,
        HashMap::new(),
    );
    assert!(
        code.contains("fn __rml_get_or_init_slider_state"),
        "生成 slider_state 方法: {}",
        code
    );
    assert!(
        code.contains("rml_ui::SliderState::new()"),
        "创建 SliderState: {}",
        code
    );
    assert!(
        code.contains("rml_ui::SliderEvent::Change"),
        "订阅 Change 事件: {}",
        code
    );
}

#[test]
fn slider_state_impl_forward_sync() {
    // f32 字段 → self.volume as f32
    let mut types = HashMap::new();
    types.insert("volume".to_string(), "f32".to_string());
    let code = compile_slider(
        r#"<component><Slider value={volume} /></component>"#,
        types,
    );
    assert!(
        code.contains("self.volume as f32"),
        "f32 正向同步: {}",
        code
    );
    assert!(
        code.contains("SliderValue::Single"),
        "包装为 Single: {}",
        code
    );
}

#[test]
fn slider_state_impl_forward_sync_i32() {
    // i32 字段 → self.volume as f32
    let mut types = HashMap::new();
    types.insert("count".to_string(), "i32".to_string());
    let code = compile_slider(
        r#"<component><Slider value={count} /></component>"#,
        types,
    );
    assert!(
        code.contains("self.count as f32"),
        "i32 正向同步 as f32: {}",
        code
    );
}

#[test]
fn slider_state_impl_reverse_sync() {
    // 反向：SliderEvent::Change → this.field = v as <type>
    let mut types = HashMap::new();
    types.insert("volume".to_string(), "f32".to_string());
    let code = compile_slider(
        r#"<component><Slider value={volume} /></component>"#,
        types,
    );
    assert!(
        code.contains("this.volume = v"),
        "f32 反向赋值（无 cast）: {}",
        code
    );
    assert!(
        code.contains("__rml_bump_version"),
        "bump_version 调用: {}",
        code
    );
}

#[test]
fn slider_state_impl_reverse_sync_i32() {
    let mut types = HashMap::new();
    types.insert("count".to_string(), "i32".to_string());
    let code = compile_slider(
        r#"<component><Slider value={count} /></component>"#,
        types,
    );
    assert!(
        code.contains("this.count = v as i32"),
        "i32 反向赋值（as i32）: {}",
        code
    );
}

#[test]
fn slider_state_impl_version_tracking() {
    let code = compile_slider(
        r#"<component><Slider value={volume} /></component>"#,
        HashMap::new(),
    );
    assert!(
        code.contains("set_state_bridge_version"),
        "版本追踪（通用 StateBridge 版本设置）: {}",
        code
    );
    assert!(
        code.contains("get_state_bridge_version"),
        "版本读取（通用 StateBridge 版本获取）: {}",
        code
    );
    assert!(
        code.contains("__rml_get_version"),
        "版本读取: {}",
        code
    );
}

#[test]
fn slider_state_impl_no_slider_no_method() {
    // 无 <Slider value={field}> 时不生成 __rml_get_or_init_slider_state
    let code = compile_slider(
        r#"<component><div /></component>"#,
        HashMap::new(),
    );
    assert!(
        !code.contains("__rml_get_or_init_slider_state"),
        "无 Slider 不生成方法: {}",
        code
    );
}

#[test]
fn slider_state_impl_multiple_fields() {
    let mut types = HashMap::new();
    types.insert("volume".to_string(), "f32".to_string());
    types.insert("count".to_string(), "i32".to_string());
    let code = compile_slider(
        r#"<component>
            <Slider value={volume} />
            <Slider value={count} />
        </component>"#,
        types,
    );
    assert!(
        code.contains("\"volume\"") && code.contains("\"count\""),
        "多字段匹配臂: {}",
        code
    );
    assert!(
        code.contains("this.count = v as i32"),
        "i32 反向: {}",
        code
    );
    assert!(
        code.contains("self.volume as f32"),
        "f32 正向: {}",
        code
    );
}
