//! PascalCase 组件自动双向绑定代码生成测试
//!
//! 验证 Phase C1：Stateless 表单组件的 `checked={field}` / `value={field}` /
//! `selected_index={field}` 绑定自动生成 on_click 反向回写。

use rust_rml_engine::parser::ast::{Attribute, Element, EventHandler, Node};
use rust_rml_engine::parser::Span;
use rust_rml_engine::compiler::translator::TranslatorRegistry;
use rust_rml_engine::compiler::CodegenCtx;

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

// ── Checkbox ──────────────────────────────────────────────────

#[test]
fn checkbox_checked_twoway_generates_on_click() {
    let elem = make_element("Checkbox", vec![bind("checked", "agree")], vec![]);
    let code = gen(&elem);
    assert!(code.contains(".selected(self.agree)"), "正向同步: {}", code);
    assert!(code.contains(".on_click("), "on_click 注入: {}", code);
    assert!(code.contains("checked: &bool"), "bool 载荷: {}", code);
    assert!(code.contains("this.agree = *checked"), "反向回写: {}", code);
    assert!(code.contains("this.__rml_bump_version(\"agree\")"), "版本追踪: {}", code);
    assert!(code.contains("cx.notify()"), "通知重渲染: {}", code);
}

#[test]
fn checkbox_twoway_with_user_handler_merges() {
    let elem = make_element(
        "Checkbox",
        vec![bind("checked", "agree"), event("on_click", "on_agree_change")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains("this.agree = *checked"), "自动回写: {}", code);
    assert!(code.contains("this.on_agree_change(checked, cx)"), "用户回调: {}", code);
    assert!(code.contains("cx.notify()"), "通知: {}", code);
    // 不应出现两个 on_click
    let on_click_count = code.matches(".on_click(").count();
    assert_eq!(on_click_count, 1, "合并为单个 on_click: {}", code);
}

#[test]
fn checkbox_without_twoway_no_on_click() {
    let elem = make_element("Checkbox", vec![], vec![Node::Text("同意".into())]);
    let code = gen(&elem);
    assert!(!code.contains(".on_click("), "无双向绑定不应注入 on_click: {}", code);
}

// ── Switch ────────────────────────────────────────────────────

#[test]
fn switch_checked_twoway_generates_on_click() {
    let elem = make_element("Switch", vec![bind("checked", "enabled")], vec![]);
    let code = gen(&elem);
    assert!(code.contains(".checked(self.enabled)"), "正向: {}", code);
    assert!(code.contains("this.enabled = *checked"), "反向: {}", code);
    assert!(code.contains("this.__rml_bump_version(\"enabled\")"), "版本: {}", code);
}

// ── Rating ────────────────────────────────────────────────────

#[test]
fn rating_value_twoway_generates_on_click() {
    let elem = make_element("Rating", vec![bind("value", "score")], vec![]);
    let code = gen(&elem);
    assert!(code.contains(".value(self.score.clone())"), "正向: {}", code);
    assert!(code.contains("value: &usize"), "usize 载荷: {}", code);
    assert!(code.contains("this.score = *value"), "反向: {}", code);
    assert!(code.contains("this.__rml_bump_version(\"score\")"), "版本: {}", code);
}

#[test]
fn rating_twoway_with_user_handler_merges() {
    let elem = make_element(
        "Rating",
        vec![bind("value", "score"), event("on_click", "on_score_change")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains("this.score = *value"), "自动回写: {}", code);
    assert!(code.contains("this.on_score_change(value, cx)"), "用户回调: {}", code);
    let on_click_count = code.matches(".on_click(").count();
    assert_eq!(on_click_count, 1, "合并为单个 on_click: {}", code);
}

// ── RadioGroup ────────────────────────────────────────────────

#[test]
fn radio_group_selected_index_twoway_generates_on_click() {
    let elem = make_element("RadioGroup", vec![bind("selected_index", "radio_idx")], vec![]);
    let code = gen(&elem);
    assert!(code.contains(".selected_index(Some(self.radio_idx))"), "正向: {}", code);
    assert!(code.contains("idx: &usize"), "usize 载荷: {}", code);
    assert!(code.contains("this.radio_idx = *idx"), "反向: {}", code);
    assert!(code.contains("this.__rml_bump_version(\"radio_idx\")"), "版本: {}", code);
}

#[test]
fn radio_group_twoway_with_user_handler_merges() {
    let elem = make_element(
        "RadioGroup",
        vec![bind("selected_index", "radio_idx"), event("on_click", "on_radio_change")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains("this.radio_idx = *idx"), "自动回写: {}", code);
    assert!(code.contains("this.on_radio_change(idx, cx)"), "用户回调: {}", code);
    let on_click_count = code.matches(".on_click(").count();
    assert_eq!(on_click_count, 1, "合并为单个 on_click: {}", code);
}

// ── Stepper ───────────────────────────────────────────────────

#[test]
fn stepper_selected_index_twoway_generates_on_click() {
    let elem = make_element("Stepper", vec![bind("selected_index", "step_idx")], vec![]);
    let code = gen(&elem);
    assert!(code.contains(".selected_index(self.step_idx)"), "正向: {}", code);
    assert!(code.contains("idx: &usize"), "usize 载荷: {}", code);
    assert!(code.contains("this.step_idx = *idx"), "反向: {}", code);
    assert!(code.contains("this.__rml_bump_version(\"step_idx\")"), "版本: {}", code);
}

#[test]
fn stepper_twoway_with_user_handler_merges() {
    let elem = make_element(
        "Stepper",
        vec![bind("selected_index", "step_idx"), event("on_click", "on_step_change")],
        vec![],
    );
    let code = gen(&elem);
    assert!(code.contains("this.step_idx = *idx"), "自动回写: {}", code);
    assert!(code.contains("this.on_step_change(idx, cx)"), "用户回调: {}", code);
    let on_click_count = code.matches(".on_click(").count();
    assert_eq!(on_click_count, 1, "合并为单个 on_click: {}", code);
}

// ── 边界情况 ──────────────────────────────────────────────────

#[test]
fn checkbox_twoway_only_bind_no_user_handler() {
    let elem = make_element("Checkbox", vec![bind("checked", "agree")], vec![]);
    let code = gen(&elem);
    assert!(code.contains("this.agree = *checked"), "自动回写: {}", code);
    assert!(!code.contains("this.on_"), "不应有用户回调: {}", code);
}

#[test]
fn rating_without_value_bind_no_on_click() {
    let elem = make_element("Rating", vec![], vec![]);
    let code = gen(&elem);
    assert!(!code.contains(".on_click("), "无双向绑定不应注入 on_click: {}", code);
}

#[test]
fn checkbox_static_selected_no_twoway() {
    let elem = make_element(
        "Checkbox",
        vec![Attribute::Static {
            name: "selected".into(),
            value: "true".into(),
            span: Span::empty(),
        }],
        vec![],
    );
    let code = gen(&elem);
    assert!(!code.contains(".on_click("), "静态属性不应触发双向: {}", code);
    assert!(code.contains(".selected(true)"), "静态选中: {}", code);
}
