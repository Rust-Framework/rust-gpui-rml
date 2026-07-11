//! Phase B-2 集成测试：验证 codegen 生成的 observable 版本管理方法和 #[computed] 缓存包装
//!
//! 调用公开的 `rml::compiler::compile()` 函数，传入手工构造的 `CodegenCtx`，
//! 验证生成的代码字符串包含预期的版本管理三方法和缓存包装方法。

use rust_rml_engine::compiler::{compile, CodegenCtx};
use std::collections::HashMap;

fn make_ctx() -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "Counter".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: vec!["doubled".to_string()],
        observable_fields: vec!["count".to_string()],
        version_fields: vec!["count".to_string()],
        computed_deps: {
            let mut m = HashMap::new();
            m.insert("doubled".to_string(), vec!["count".to_string()]);
            m
        },
        computed_returns: {
            let mut m = HashMap::new();
            m.insert("doubled".to_string(), "i32".to_string());
            m
        },
        field_types: HashMap::new(),
        field_validations: HashMap::new(),
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    }
}

const RML_SOURCE: &str = r#"
<component>
    <div>{count}</div>
</component>
"#;

#[test]
fn generates_bump_version_method() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("fn __rml_bump_version"),
        "missing __rml_bump_version method\n{}",
        code
    );
}

#[test]
fn generates_get_version_method() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("fn __rml_get_version"),
        "missing __rml_get_version method\n{}",
        code
    );
}

#[test]
fn generates_computed_deps_version_method() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("fn __rml_computed_deps_version"),
        "missing __rml_computed_deps_version method\n{}",
        code
    );
}

#[test]
fn bump_version_delegates_to_rml_state() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("fn __rml_bump_version(&mut self, field: &str)"),
        "missing bump_version method with &mut self signature\n{}",
        code
    );
    assert!(
        code.contains("self.__rml_state.bump_version(field)"),
        "missing RmlState::bump_version delegation\n{}",
        code
    );
}

#[test]
fn computed_deps_sums_count_version() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("self.__rml_get_version(\"count\")"),
        "missing computed deps sum expression\n{}",
        code
    );
}

#[test]
fn generates_computed_wrapper_for_doubled() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("pub fn doubled(&self) -> i32"),
        "missing doubled wrapper method\n{}",
        code
    );
    assert!(
        code.contains("get_or_compute::<i32>(\"doubled\""),
        "missing get_or_compute call for doubled\n{}",
        code
    );
    assert!(
        code.contains("self.__rml_computed_doubled()"),
        "missing __rml_computed_doubled call\n{}",
        code
    );
}

#[test]
fn empty_observable_fields_still_generates_match() {
    // 即使无 observable 字段，也应生成空 match（带 _ => {} 兜底）
    let mut ctx = make_ctx();
    ctx.observable_fields.clear();
    let code = compile(RML_SOURCE, &ctx).expect("compile failed").code;
    assert!(
        code.contains("fn __rml_bump_version"),
        "missing __rml_bump_version even with empty fields\n{}",
        code
    );
}

#[test]
fn observable_vec_fields_route_to_self_version() {
    // ObservableVec<T> 字段应路由到 self.field.version()
    let mut ctx = make_ctx();
    ctx.field_types.insert(
        "workbenches".to_string(),
        "ObservableVec < Arc < dyn IWorkbench > >".to_string(),
    );
    ctx.field_types.insert(
        "menus".to_string(),
        "ObservableVec<MenuViewModel>".to_string(),
    );
    // 非 ObservableVec 字段不应路由
    ctx.field_types.insert("count".to_string(), "i32".to_string());
    let code = compile(RML_SOURCE, &ctx).expect("compile failed").code;
    assert!(
        code.contains(r#""workbenches" => self.workbenches.version()"#),
        "missing ObservableVec version route for workbenches\n{}",
        code
    );
    assert!(
        code.contains(r#""menus" => self.menus.version()"#),
        "missing ObservableVec version route for menus\n{}",
        code
    );
    // count 是 i32，不应路由
    assert!(
        !code.contains(r#""count" => self.count.version()"#),
        "i32 field should not route to .version()\n{}",
        code
    );
    // 默认分支仍存在
    assert!(
        code.contains("_ => self.__rml_state.get_version(field)"),
        "missing default arm for non-ObservableVec fields\n{}",
        code
    );
}

#[test]
fn generates_notify_wrapper_methods() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    for method in [
        "__rml_notify_info",
        "__rml_notify_success",
        "__rml_notify_warning",
        "__rml_notify_error",
    ] {
        assert!(
            code.contains(&format!("fn {}", method)),
            "missing {} method\n{}",
            method,
            code
        );
    }
}

#[test]
fn notify_wrappers_delegate_to_rml_state() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("self.__rml_state.notify_info(message)"),
        "missing notify_info delegation\n{}",
        code
    );
    assert!(
        code.contains("self.__rml_state.notify_success(message)"),
        "missing notify_success delegation\n{}",
        code
    );
    assert!(
        code.contains("self.__rml_state.notify_warning(message)"),
        "missing notify_warning delegation\n{}",
        code
    );
    assert!(
        code.contains("self.__rml_state.notify_error(message)"),
        "missing notify_error delegation\n{}",
        code
    );
}

#[test]
fn render_drains_pending_notifications() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed").code;
    assert!(
        code.contains("self.__rml_state.drain_notifications(_window, cx)"),
        "missing drain_notifications call in render\n{}",
        code
    );
}
