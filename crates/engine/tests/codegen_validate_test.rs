//! Phase B-3.3 集成测试：验证 `#[validate(MyValidator)]` IValidate 接口式校验
//!
//! 测试两类行为：
//! 1. scanner 解析：`#[validate(MyValidator)]` 正确提取 `validator_type`，且与规则式/message 互斥
//! 2. codegen 生成：调用 `MyValidator::default().valid_with_view(value, this as &dyn Any)`
//!    + `message(&result)` 转换 + 失败/成功路径分支

use rust_rml_engine::build::scanner::scan_struct_metadata;
use rust_rml_engine::compiler::{compile, CodegenCtx, ValidationRuleSet};
use std::collections::HashMap;
use std::io::Write;

// ─── Scanner 解析测试 ───

fn write_temp_rml_rs(content: &str) -> std::path::PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "rml_validate_test_{}.rml.rs",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn validator_type_is_extracted_from_attribute() {
    let path = write_temp_rml_rs(
        r#"
#[window]
#[derive(Default)]
pub struct Form {
    #[validate(EmailValidator)]
    pub email: String,
}
        "#,
    );
    let meta = scan_struct_metadata(&path);
    let form = meta.get("Form").expect("Form struct should be scanned");
    let validation = form
        .field_validations
        .get("email")
        .expect("email field should have validation");
    assert_eq!(
        validation.validator_type,
        Some("EmailValidator".to_string()),
        "应提取 IValidate 类型名为 EmailValidator"
    );
    assert!(
        validation.rules.is_empty(),
        "validator_type 模式下 rules 必须为空"
    );
    assert!(
        validation.custom_message.is_none(),
        "validator_type 模式下 custom_message 必须为空"
    );
}

#[test]
fn validator_type_and_rules_are_mutually_exclusive() {
    // 混用 IValidate 类型 + range 规则 → 解析失败（scanner 打印 warning 但不阻塞编译）
    // 验证：field_validations 中不应出现该项（解析失败时不插入）
    let path = write_temp_rml_rs(
        r#"
#[window]
#[derive(Default)]
pub struct Form {
    #[validate(MyValidator, range(min = 0, max = 10))]
    pub age: i32,
}
        "#,
    );
    let meta = scan_struct_metadata(&path);
    let form = meta.get("Form").expect("Form struct should be scanned");
    // 解析失败 → field_validations 不包含 "age"
    assert!(
        !form.field_validations.contains_key("age"),
        "混用 IValidate 类型 + range 规则应解析失败，不应插入 field_validations"
    );
}

#[test]
fn validator_type_and_message_are_mutually_exclusive() {
    let path = write_temp_rml_rs(
        r#"
#[window]
#[derive(Default)]
pub struct Form {
    #[validate(MyValidator, message = "custom")]
    pub name: String,
}
        "#,
    );
    let meta = scan_struct_metadata(&path);
    let form = meta.get("Form").expect("Form struct should be scanned");
    assert!(
        !form.field_validations.contains_key("name"),
        "混用 IValidate 类型 + message 应解析失败"
    );
}

// ─── Codegen 生成测试 ───

/// 构造带 IValidate 校验的 CodegenCtx
fn make_ctx_with_validator(field: &str, ty: &str, validator_type: &str) -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec![field.to_string()],
        version_fields: vec![field.to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert(field.to_string(), ty.to_string());
            m
        },
        field_validations: {
            let mut m = HashMap::new();
            m.insert(
                field.to_string(),
                ValidationRuleSet {
                    rules: Vec::new(),
                    custom_message: None,
                    validator_type: Some(validator_type.to_string()),
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

#[test]
fn gen_validator_numeric_field_generates_valid_with_view_call() {
    let ctx = make_ctx_with_validator("age", "i32", "AgeValidator");
    let source = r#"
<component>
    <input model={age} placeholder="年龄" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed");

    // 应构造 validator 实例
    assert!(
        code.contains("let __rml_validator = AgeValidator::default();"),
        "应通过 Default::default() 构造 validator，实际：\n{}",
        code
    );
    // 应调用 valid_with_view（数字类型传 value.as_ref()）
    assert!(
        code.contains("__rml_validator.valid_with_view(value.as_ref(), this as &dyn std::any::Any)"),
        "数字字段应调用 valid_with_view(value.as_ref(), this)，实际：\n{}",
        code
    );
    // 应调用 message(&result)
    assert!(
        code.contains("__rml_validator.message(&__rml_result)"),
        "应调用 message(&result) 转换结果，实际：\n{}",
        code
    );
    // 外层应有 match parse::<i32>()
    assert!(
        code.contains("match value.parse::<i32>()"),
        "数字字段外层应有 match parse::<i32>()，实际：\n{}",
        code
    );
}

#[test]
fn gen_validator_string_field_generates_valid_with_view_call() {
    let ctx = make_ctx_with_validator("email", "String", "EmailValidator");
    let source = r#"
<component>
    <input model={email} placeholder="邮箱" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed");

    // 应构造 validator 实例
    assert!(
        code.contains("let __rml_validator = EmailValidator::default();"),
        "应通过 Default::default() 构造 validator，实际：\n{}",
        code
    );
    // String 类型应通过 __rml_value 传引用
    assert!(
        code.contains("__rml_validator.valid_with_view(&__rml_value, this as &dyn std::any::Any)"),
        "String 字段应调用 valid_with_view(&__rml_value, this)，实际：\n{}",
        code
    );
    // 应有 let __rml_value = value.to_string();
    assert!(
        code.contains("let __rml_value = value.to_string()"),
        "String 字段应生成 __rml_value = value.to_string()，实际：\n{}",
        code
    );
    // 不应有 match parse（String 无 parse 阶段）
    assert!(
        !code.contains("match value.parse::<"),
        "String 字段不应有 match parse，实际：\n{}",
        code
    );
}

#[test]
fn gen_validator_calls_message_and_handles_pass() {
    // 成功路径：message 返回 None → 赋值 + 清除错误 + bump_version
    let ctx = make_ctx_with_validator("age", "i32", "PassValidator");
    let source = r#"
<component>
    <input model={age} placeholder="年龄" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed");

    // 提取 validator 的 if-let-Some 块（精确匹配 validator 调用后的 if let）
    let validator_section = code
        .split("let __rml_result = __rml_validator.valid_with_view")
        .nth(1)
        .unwrap_or("");
    // 成功路径在 "} else {" 之后（validator 的 else 分支）
    let else_section = validator_section.split("} else {").nth(1).unwrap_or("");
    let else_block = else_section.split('}').next().unwrap_or("");
    assert!(
        else_block.contains("this.age = v"),
        "成功路径应赋值 this.age = v，实际：\n{}",
        else_section
    );
    assert!(
        else_block.contains("__rml_state.field_errors.insert"),
        "成功路径应清除错误状态，实际：\n{}",
        else_section
    );
    assert!(
        else_block.contains("__rml_bump_version"),
        "成功路径应调用 bump_version，实际：\n{}",
        else_section
    );
}

#[test]
fn gen_validator_calls_message_and_handles_fail() {
    // 失败路径：message 返回 Some(msg) → 仅设置错误，不赋值、不 bump_version
    let ctx = make_ctx_with_validator("age", "i32", "FailValidator");
    let source = r#"
<component>
    <input model={age} placeholder="年龄" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed");

    // 精确提取 validator 的 if-let-Some 块
    let validator_section = code
        .split("let __rml_result = __rml_validator.valid_with_view")
        .nth(1)
        .unwrap_or("");
    // 失败路径在 "if let Some(__rml_err_msg)" 之后、"} else {" 之前
    let fail_section = validator_section
        .split("if let Some(__rml_err_msg) =")
        .nth(1)
        .unwrap_or("");
    let fail_block = fail_section.split("} else {").next().unwrap_or("");
    assert!(
        fail_block.contains("__rml_state.field_errors.insert"),
        "失败路径应设置错误状态，实际：\n{}",
        fail_block
    );
    assert!(
        !fail_block.contains("this.age = v"),
        "失败路径不应赋值，实际：\n{}",
        fail_block
    );
    assert!(
        !fail_block.contains("__rml_bump_version"),
        "失败路径不应调用 bump_version，实际：\n{}",
        fail_block
    );
}

#[test]
fn gen_validator_for_bool_falls_back_to_default() {
    // bool 类型忽略 IValidate（语义不明确），回退到默认逻辑
    let ctx = make_ctx_with_validator("enabled", "bool", "BoolValidator");
    let source = r#"
<component>
    <input model={enabled} placeholder="启用" />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed");

    // 不应生成 validator 调用
    assert!(
        !code.contains("BoolValidator::default()"),
        "bool 类型应忽略 IValidate，实际：\n{}",
        code
    );
    // 应生成默认逻辑：this.enabled = !value.is_empty()
    assert!(
        code.contains("this.enabled = !value.is_empty()"),
        "bool 类型应回退到默认逻辑，实际：\n{}",
        code
    );
}
