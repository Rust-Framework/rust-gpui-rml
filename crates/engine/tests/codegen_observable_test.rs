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
        contribution_bindings: false,
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
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed");
    assert!(
        code.contains("fn __rml_bump_version"),
        "missing __rml_bump_version method\n{}",
        code
    );
}

#[test]
fn generates_get_version_method() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed");
    assert!(
        code.contains("fn __rml_get_version"),
        "missing __rml_get_version method\n{}",
        code
    );
}

#[test]
fn generates_computed_deps_version_method() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed");
    assert!(
        code.contains("fn __rml_computed_deps_version"),
        "missing __rml_computed_deps_version method\n{}",
        code
    );
}

#[test]
fn bump_version_targets_count_field() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed");
    assert!(
        code.contains("\"count\" =>"),
        "missing count match arm\n{}",
        code
    );
    assert!(
        code.contains("__rml_count_version"),
        "missing __rml_count_version field access\n{}",
        code
    );
}

#[test]
fn computed_deps_sums_count_version() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed");
    assert!(
        code.contains("self.__rml_get_version(\"count\")"),
        "missing computed deps sum expression\n{}",
        code
    );
}

#[test]
fn contributehost_emits_attach_contribution_bindings() {
    let mut ctx = make_ctx();
    ctx.view_struct_name = "Shell".to_string();
    ctx.contribution_bindings = true;
    let code = compile(RML_SOURCE, &ctx).expect("compile");
    assert!(
        code.contains("__rml_attach_contribution_bindings"),
        "missing __rml_attach_contribution_bindings for contributehost with bindings\n{}",
        code
    );
}

#[test]
fn generates_computed_wrapper_for_doubled() {
    let code = compile(RML_SOURCE, &make_ctx()).expect("compile failed");
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
    let code = compile(RML_SOURCE, &ctx).expect("compile failed");
    assert!(
        code.contains("fn __rml_bump_version"),
        "missing __rml_bump_version even with empty fields\n{}",
        code
    );
}
