//! Phase B-3.2 集成测试：验证 `#[validate]` 宏生成的校验代码
//!
//! 验证 `CodegenCtx.field_validations` 中的规则被正确转换为 codegen 输出：
//! - `range(min, max)` → 数字类型生成 `v < min || v > max` 条件检查
//! - `length(min, max)` → String 类型生成 `.len() < min || .len() > max` 条件检查
//! - `required` → String 类型生成 `.is_empty()` 检查
//! - `regex = "..."` → 生成 `rml::regex::Regex::new(...).is_match(...)` 调用
//! - `custom = "fn"` → 生成 `Self::fn(...)` 调用
//! - `message = "..."` → 覆盖默认错误消息
//! - 校验失败不赋值、不 bump_version；校验通过赋值 + 清除错误 + bump_version

use rust_rml_engine::compiler::{compile, CodegenCtx, ValidationRule, ValidationRuleSet};
use std::collections::HashMap;

/// 构造带 range 校验的 CodegenCtx（age: i32, range 0-150）
fn make_ctx_with_range_validation() -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["age".to_string()],
        version_fields: vec!["age".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("age".to_string(), "i32".to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                "age".to_string(),
                ValidationRuleSet {
                    rules: vec![ValidationRule::Range {
                        min: Some(0.0),
                        max: Some(150.0),
                    }],
                    custom_message: None,
                    validator_type: None,
                },
            );
            m
        },
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    }
}

const RML_SOURCE_WITH_AGE: &str = r#"
<component>
    <input value={age} placeholder="年龄" />
</component>
"#;

#[test]
fn range_validation_generates_bounds_check() {
    let ctx = make_ctx_with_range_validation();
    let code = compile(RML_SOURCE_WITH_AGE, &ctx).expect("compile failed").code;

    // 应生成 range 校验条件：!(0..=150).contains(&v)（失败条件）
    assert!(
        code.contains("!(0..=150).contains(&v)"),
        "range 校验应生成 `!(0..=150).contains(&v)` 条件，实际：\n{}",
        code
    );
    // 应生成默认错误消息
    assert!(
        code.contains("值必须在 0-150 之间"),
        "range 校验应生成默认错误消息，实际：\n{}",
        code
    );
}

#[test]
fn range_validation_uses_custom_message() {
    let mut ctx = make_ctx_with_range_validation();
    ctx.field_validations.get_mut("age").unwrap().custom_message = Some("年龄不合法".to_string());
    let code = compile(RML_SOURCE_WITH_AGE, &ctx).expect("compile failed").code;

    // 应使用自定义消息
    assert!(
        code.contains("年龄不合法"),
        "range 校验应使用自定义消息，实际：\n{}",
        code
    );
    // 不应出现默认消息
    assert!(
        !code.contains("值必须在 0-150 之间"),
        "有自定义消息时不应出现默认消息，实际：\n{}",
        code
    );
}

#[test]
fn length_validation_generates_len_check() {
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["name".to_string()],
        version_fields: vec!["name".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "String".to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                "name".to_string(),
                ValidationRuleSet {
                    rules: vec![ValidationRule::Length {
                        min: Some(3),
                        max: Some(20),
                    }],
                    custom_message: None,
                    validator_type: None,
                },
            );
            m
        },
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    };
    let source = r#"
<component>
    <input value={name} placeholder="姓名" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed").code;

    // 应生成 length 校验条件：__rml_value.len() < 3 || __rml_value.len() > 20
    assert!(
        code.contains("__rml_value.len() < 3 || __rml_value.len() > 20"),
        "length 校验应生成 len 条件，实际：\n{}",
        code
    );
    // 应生成默认错误消息
    assert!(
        code.contains("长度必须在 3-20 之间"),
        "length 校验应生成默认错误消息，实际：\n{}",
        code
    );
}

#[test]
fn required_validation_generates_empty_check() {
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["name".to_string()],
        version_fields: vec!["name".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "String".to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                "name".to_string(),
                ValidationRuleSet {
                    rules: vec![ValidationRule::Required],
                    custom_message: None,
                    validator_type: None,
                },
            );
            m
        },
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    };
    let source = r#"
<component>
    <input value={name} placeholder="姓名" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed").code;

    // 应生成 required 校验：__rml_value.is_empty()
    assert!(
        code.contains("__rml_value.is_empty()"),
        "required 校验应生成 is_empty 检查，实际：\n{}",
        code
    );
    // 应生成默认错误消息
    assert!(
        code.contains("此项为必填"),
        "required 校验应生成默认错误消息，实际：\n{}",
        code
    );
}

#[test]
fn regex_validation_generates_pattern_match() {
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["email".to_string()],
        version_fields: vec!["email".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("email".to_string(), "String".to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                "email".to_string(),
                ValidationRuleSet {
                    rules: vec![ValidationRule::Regex(
                        r"^\w+@\w+\.\w+$".to_string(),
                    )],
                    custom_message: Some("邮箱格式错误".to_string()),
                    validator_type: None,
                },
            );
            m
        },
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    };
    let source = r#"
<component>
    <input value={email} placeholder="邮箱" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed").code;

    // 应生成 regex 编译 + is_match 调用
    assert!(
        code.contains("rml::regex::Regex::new"),
        "regex 校验应生成 rml::regex::Regex::new 调用，实际：\n{}",
        code
    );
    assert!(
        code.contains(".is_match(&__rml_value)"),
        "regex 校验应生成 is_match 调用，实际：\n{}",
        code
    );
    // 应使用自定义消息
    assert!(
        code.contains("邮箱格式错误"),
        "regex 校验应使用自定义消息，实际：\n{}",
        code
    );
}

#[test]
fn custom_validation_generates_function_call() {
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["phone".to_string()],
        version_fields: vec!["phone".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("phone".to_string(), "String".to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                "phone".to_string(),
                ValidationRuleSet {
                    rules: vec![ValidationRule::Custom("validate_phone".to_string())],
                    custom_message: None,
                    validator_type: None,
                },
            );
            m
        },
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    };
    let source = r#"
<component>
    <input value={phone} placeholder="手机号" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed").code;

    // 应生成 Self::validate_phone 调用
    assert!(
        code.contains("Self::validate_phone(&__rml_value)"),
        "custom 校验应生成 Self::validate_phone 调用，实际：\n{}",
        code
    );
    // 应生成 if let Some(__rml_err) = ... 模式
    assert!(
        code.contains("if let Some(__rml_err) = Self::validate_phone"),
        "custom 校验应生成 if let Some 模式，实际：\n{}",
        code
    );
}

#[test]
fn multiple_rules_executed_in_order() {
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["name".to_string()],
        version_fields: vec!["name".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("name".to_string(), "String".to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                "name".to_string(),
                ValidationRuleSet {
                    rules: vec![
                        ValidationRule::Required,
                        ValidationRule::Length {
                            min: Some(3),
                            max: Some(20),
                        },
                    ],
                    custom_message: None,
                    validator_type: None,
                },
            );
            m
        },
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    };
    let source = r#"
<component>
    <input value={name} placeholder="姓名" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed").code;

    // 应同时生成 required 和 length 校验
    assert!(
        code.contains("__rml_value.is_empty()"),
        "应生成 required 校验，实际：\n{}",
        code
    );
    assert!(
        code.contains("__rml_value.len() < 3 || __rml_value.len() > 20"),
        "应生成 length 校验，实际：\n{}",
        code
    );
    // required 应在 length 之前（按声明顺序）
    let required_pos = code.find("__rml_value.is_empty()").unwrap_or(0);
    let length_pos = code
        .find("__rml_value.len() < 3")
        .unwrap_or(0);
    assert!(
        required_pos < length_pos,
        "required 应在 length 之前（required_pos={}, length_pos={}）",
        required_pos,
        length_pos
    );
}

#[test]
fn validation_failure_skips_bump_version() {
    let ctx = make_ctx_with_range_validation();
    let code = compile(RML_SOURCE_WITH_AGE, &ctx).expect("compile failed").code;

    // 提取 range 校验失败分支（if !(0..=150).contains(&v) { ... }）
    let fail_section = code.split("if !(0..=150).contains(&v)").nth(1).unwrap_or("");
    let fail_block = fail_section.split("} else").next().unwrap_or("");

    // 失败分支不应包含 bump_version
    assert!(
        !fail_block.contains("__rml_bump_version"),
        "校验失败分支不应调用 bump_version，实际：\n{}",
        fail_block
    );
    // 失败分支应设置错误状态
    assert!(
        fail_block.contains("__rml_state.field_errors.insert"),
        "校验失败分支应设置错误状态，实际：\n{}",
        fail_block
    );
}

#[test]
fn validation_success_clears_error() {
    let ctx = make_ctx_with_range_validation();
    let code = compile(RML_SOURCE_WITH_AGE, &ctx).expect("compile failed").code;

    // 成功分支（最后的 else 块）应包含赋值 + 清除错误 + bump_version
    let success_section = code.split("} else {").last().unwrap_or("");
    assert!(
        success_section.contains("this.age = v"),
        "成功分支应赋值，实际：\n{}",
        success_section
    );
    assert!(
        success_section.contains("__rml_state.field_errors.insert"),
        "成功分支应清除错误状态，实际：\n{}",
        success_section
    );
    assert!(
        success_section.contains("__rml_bump_version"),
        "成功分支应调用 bump_version，实际：\n{}",
        success_section
    );
}

#[test]
fn no_validation_falls_back_to_default() {
    // 无校验规则时应回退到默认类型校验逻辑
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["age".to_string()],
        version_fields: vec!["age".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("age".to_string(), "i32".to_string());
            m
        },
        field_validations: HashMap::new(), // 无校验规则
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
        ..Default::default()
    };
    let code = compile(RML_SOURCE_WITH_AGE, &ctx).expect("compile failed").code;

    // 应生成默认的 match parse 逻辑（无 range 校验）
    assert!(
        code.contains("match value.parse::<i32>()"),
        "无校验规则应回退到默认 parse 逻辑，实际：\n{}",
        code
    );
    // 不应生成 range 校验条件
    assert!(
        !code.contains("v < 0 || v > 150"),
        "无校验规则不应生成 range 条件，实际：\n{}",
        code
    );
    // 应生成默认错误消息
    assert!(
        code.contains("请输入有效的整数"),
        "无校验规则应使用默认类型错误消息，实际：\n{}",
        code
    );
}
