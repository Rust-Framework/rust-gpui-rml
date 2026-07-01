//! Phase B-3 集成测试：验证双向绑定 codegen 生成的代码
//!
//! 验证 `<input model={field}>` 生成的代码包含：
//! - `__rml_get_or_init_input_state` 调用（多 Input 惰性初始化，传引用给 Input::new）
//! - `cx.subscribe` + `InputEvent::Change` 反向绑定（detach 保持订阅存活）
//! - `InputState::set_value` 正向同步（初始值 + 版本号对比）
//! - 类型转换代码（i32 → parse、String → to_string）
//! - `__rml_bump_version` + `cx.notify()` 反向回调
//! - `__rml_input_state_versions` 版本追踪

use rust_rml_engine::compiler::{compile, CodegenCtx};
use std::collections::HashMap;

/// 构造带 field_types 的 CodegenCtx（i32 + String 两个字段）
fn make_ctx_with_field_types() -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["count".to_string(), "name".to_string()],
        version_fields: vec!["count".to_string(), "name".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("count".to_string(), "i32".to_string());
            m.insert("name".to_string(), "String".to_string());
            m
        },
        field_validations: HashMap::new(),
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
    }
}

const RML_SOURCE_WITH_MODEL: &str = r#"
<component>
    <input model={name} placeholder="姓名" />
    <input model={count} placeholder="计数" />
</component>
"#;

#[test]
fn gen_model_input_uses_get_or_init_input_state() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 验证生成代码使用 __rml_get_or_init_input_state（不再使用 self.input_state）
    assert!(
        code.contains("__rml_get_or_init_input_state"),
        "生成代码应包含 __rml_get_or_init_input_state 调用，实际：\n{}",
        code
    );
    assert!(
        !code.contains("self.input_state"),
        "生成代码不应再使用旧的 self.input_state 字段"
    );
    // Input::new 应接收引用（&self.__rml_get_or_init_input_state(...)）
    assert!(
        code.contains("Input::new(&self.__rml_get_or_init_input_state"),
        "Input::new 应接收引用，实际：\n{}",
        code
    );
}

#[test]
fn gen_model_input_generates_type_conversion_for_i32() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // i32 字段应生成 match value.parse::<i32>() { Ok(v) => ..., Err(_) => ... }
    assert!(
        code.contains("match value.parse::<i32>()"),
        "i32 字段应生成 match parse 代码，实际：\n{}",
        code
    );
    assert!(
        code.contains("this.count = v"),
        "i32 字段 parse 成功时应赋值 this.count = v"
    );
    assert!(
        code.contains("Some(\"请输入有效的整数\""),
        "i32 字段 parse 失败时应设置错误消息"
    );
    // 正向绑定时数字类型应 to_string()
    assert!(
        code.contains("self.count.to_string()"),
        "i32 字段正向绑定应调用 to_string()"
    );
}

#[test]
fn gen_model_input_generates_to_string_for_string() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // String 字段反向绑定应生成 value.to_string() 转换代码
    assert!(
        code.contains("this.name = value.to_string()"),
        "String 字段应生成 value.to_string() 转换代码，实际：\n{}",
        code
    );
    // 正向绑定时 String 类型应 clone().into()
    assert!(
        code.contains("self.name.clone().into()"),
        "String 字段正向绑定应调用 clone().into()"
    );
}

#[test]
fn gen_model_input_includes_bump_version_and_notify() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 反向绑定应包含 bump_version 和 notify
    assert!(
        code.contains("__rml_bump_version"),
        "反向绑定应调用 __rml_bump_version"
    );
    assert!(code.contains("cx.notify()"), "反向绑定应调用 cx.notify()");
}

#[test]
fn gen_input_state_impl_generates_helper_method() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 验证生成了 __rml_get_or_init_input_state 方法
    assert!(
        code.contains("fn __rml_get_or_init_input_state"),
        "应生成 __rml_get_or_init_input_state 方法定义"
    );
    // 方法应接收 &mut self、placeholder、window、cx 参数
    assert!(
        code.contains("&mut self")
            && code.contains("placeholder: Option<&'static str>")
            && code.contains("window: &mut gpui::Window")
            && code.contains("gpui::Context<Self>"),
        "方法签名应包含 &mut self、placeholder、window、cx 参数，实际：\n{}",
        code
    );
    // 方法应使用 __rml_input_states HashMap
    assert!(
        code.contains("__rml_input_states"),
        "方法应使用 __rml_input_states 字段"
    );
}

#[test]
fn gen_model_input_supports_multiple_inputs() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 验证两个 input 都生成了独立的 __rml_get_or_init_input_state 调用
    let count_calls = code.matches("__rml_get_or_init_input_state").count();
    // 至少 2 次调用（render 内 2 次）+ 1 次方法定义 = 至少 3 次
    assert!(
        count_calls >= 3,
        "应至少有 3 次 __rml_get_or_init_input_state 出现（2 次调用 + 1 次定义），实际 {}",
        count_calls
    );

    // 验证两个字段名都被传入
    assert!(
        code.contains(r#""name""#) && code.contains(r#""count""#),
        "应分别传入 name 和 count 字段名"
    );
}

#[test]
fn gen_model_input_preserves_placeholder_attribute() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // placeholder 作为 Some("...") 参数传入 __rml_get_or_init_input_state
    assert!(
        code.contains("Some(\"姓名\""),
        "placeholder 应作为 Some(\"姓名\") 传入 helper，实际：\n{}",
        code
    );
    assert!(
        code.contains("Some(\"计数\""),
        "placeholder 应作为 Some(\"计数\") 传入 helper"
    );
}

#[test]
fn gen_model_input_floating_point_types() {
    let ctx = CodegenCtx {
        view_struct_name: "TestView".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: Vec::new(),
        observable_fields: vec!["score".to_string()],
        version_fields: vec!["score".to_string()],
        computed_deps: HashMap::new(),
        computed_returns: HashMap::new(),
        field_types: {
            let mut m = HashMap::new();
            m.insert("score".to_string(), "f64".to_string());
            m
        },
        field_validations: HashMap::new(),
        model_fields: Vec::new(),
        user_components: HashMap::new(),
        is_contributehost: false,
    };
    let source = r#"
<component>
    <input model={score} />
</component>
"#;
    let code = compile(source, &ctx).expect("compile failed");

    // f64 应生成 match value.parse::<f64>() { Ok(v) => ..., Err(_) => ... }
    assert!(
        code.contains("match value.parse::<f64>()"),
        "f64 字段应生成 match parse 代码，实际：\n{}",
        code
    );
    assert!(
        code.contains("Some(\"请输入有效的数字\""),
        "f64 字段 parse 失败时应设置错误消息"
    );
    assert!(
        code.contains("self.score.to_string()"),
        "f64 字段正向绑定应调用 to_string()"
    );
}

#[test]
fn gen_input_state_impl_includes_subscribe() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 应使用 cx.subscribe 订阅 InputEvent::Change
    assert!(
        code.contains("cx.subscribe(&entity"),
        "应使用 cx.subscribe 订阅 InputState 事件，实际：\n{}",
        code
    );
    assert!(
        code.contains("rml_ui::InputEvent::Change"),
        "应匹配 InputEvent::Change 事件"
    );
    // subscription 应调用 detach() 而非存储在 Vec 中
    assert!(
        code.contains(".detach()"),
        "subscription 应调用 detach() 保持订阅存活"
    );
    assert!(
        !code.contains("__rml_input_subscriptions"),
        "不应存储 Vec<Subscription>（Sync 约束问题）"
    );
}

#[test]
fn gen_input_state_impl_includes_set_value() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 应使用 set_value 进行正向同步（初始值 + 版本号变化时）
    assert!(
        code.contains("state.set_value(initial_value, window, cx)"),
        "应调用 set_value 设置初始值，实际：\n{}",
        code
    );
    assert!(
        code.contains("state.set_value(value, window, cx)"),
        "应调用 set_value 进行正向同步"
    );
    // 应使用 InputState::new(window, cx) 创建 entity（非 default）
    assert!(
        code.contains("rml_ui::InputState::new(window, cx)"),
        "应使用 InputState::new(window, cx) 创建 entity"
    );
    assert!(
        !code.contains("InputState::default()"),
        "不应使用不存在的 InputState::default()"
    );
}

#[test]
fn gen_input_state_impl_includes_version_tracking() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 应使用 __rml_input_state_versions 追踪同步版本
    assert!(
        code.contains("__rml_input_state_versions"),
        "应使用 __rml_input_state_versions 追踪同步版本，实际：\n{}",
        code
    );
    // 应对比 current_version 和 last_synced
    assert!(
        code.contains("current_version") && code.contains("last_synced"),
        "应对比 current_version 和 last_synced 决定是否正向同步"
    );
    // 反向闭包内应更新版本号标记
    assert!(
        code.contains("this.__rml_input_state_versions.insert(field.to_string(), v)"),
        "反向闭包内应更新版本号标记"
    );
}

// ─── Phase B-3.1 校验失败 UI 测试 ───

#[test]
fn gen_field_assign_generates_error_handling_for_i32() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // i32 应生成 match parse + Err 分支 + 错误消息
    assert!(
        code.contains("match value.parse::<i32>()"),
        "i32 字段应生成 match parse 代码"
    );
    assert!(
        code.contains("Err(_) =>"),
        "i32 字段应生成 Err 分支处理 parse 失败"
    );
    assert!(
        code.contains("Some(\"请输入有效的整数\""),
        "i32 字段 parse 失败时应设置中文错误消息"
    );
}

#[test]
fn gen_field_assign_preserves_old_value_on_error() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // Err 分支不应包含 this.count = ...（不覆盖原值）
    // 提取 i32 的 Err 分支内容验证
    let err_section = code.split("Err(_) =>").nth(1).unwrap_or("");
    let err_block = err_section.split("}").next().unwrap_or("");
    assert!(
        !err_block.contains("this.count ="),
        "Err 分支不应覆盖原值，实际 Err 块：\n{}",
        err_block
    );
    // Err 分支应设置错误状态
    assert!(
        err_block.contains("__rml_field_errors.insert"),
        "Err 分支应设置错误状态"
    );
}

#[test]
fn gen_model_input_applies_red_border_to_input() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 应检查 __rml_field_errors 获取错误状态
    assert!(
        code.contains("__rml_field_errors.get("),
        "应检查 __rml_field_errors 获取错误状态"
    );
    // Phase B-3.3：红色边框应直接应用到 Input 自身（通过 Styled trait .border_color()），
    // 而非附加在外层 wrapper div 上（避免双层边框 / 间距错位）
    assert!(
        code.contains("let __rml_input = __rml_input.border_color(gpui::rgb(0xff0000))"),
        "Input 自身应被设置红色边框，实际：\n{}",
        code
    );
    // wrapper div 不应再附加 .border_1().border_color(...) 链
    assert!(
        !code.contains(".border_1().border_color(gpui::rgb(0xff0000))"),
        "wrapper div 不应再附加 border，实际：\n{}",
        code
    );
    // wrapper div 仍需 id 承载 tooltip
    assert!(
        code.contains("rml_input_err:"),
        "wrapper div 应有 id（rml_input_err:<field>）以承载 tooltip"
    );
}

#[test]
fn gen_model_input_includes_tooltip_closure() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 应包含 tooltip 闭包 + Tooltip::new + build + into_any_element
    assert!(
        code.contains(".tooltip(move |window, cx|"),
        "应使用 .tooltip() 闭包"
    );
    assert!(
        code.contains("rml_ui::Tooltip::new("),
        "闭包内应创建 Tooltip::new()"
    );
    assert!(
        code.contains(".build(window, cx)"),
        "应调用 .build(window, cx) 构建 tooltip"
    );
    assert!(
        code.contains(".into_any_element()"),
        "应使用 into_any_element() 统一返回类型"
    );
}

#[test]
fn gen_input_state_impl_clears_error_on_forward_sync() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    // 正向同步部分（set_value 后）应清除错误状态
    let forward_section = code.split("state.set_value(value, window, cx)").nth(1).unwrap_or("");
    assert!(
        forward_section.contains("__rml_field_errors.insert(field.to_string(), None)"),
        "正向同步 set_value 后应清除错误状态，实际：\n{}",
        forward_section
    );
}
