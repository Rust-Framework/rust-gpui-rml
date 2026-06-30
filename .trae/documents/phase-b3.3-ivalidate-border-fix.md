# Phase B-3.3：红色边框修复 + IValidate 接口式校验架构

## 摘要

针对用户两项反馈：
1. **红色边框修复**：当前用 `div().border_1().border_color(red).child(input)` 包裹 Input 是"附加外层边框"，导致与 Input 自身边框效果不一致（双层边框 / 间距错位）。需将 `.border_color()` 移到 Input 自身，wrapper div 仅保留 tooltip。
2. **IValidate 接口式校验架构**：新增 `IValidate` trait，支持 `#[validate(MyValidator)]` 语法，让用户自定义校验逻辑（含跨字段校验）。规则式与接口式互斥。

## 当前状态分析

### 红色边框实现（codegen.rs:534-557）

```rust
let code = format!(
    r#"{{
        let __rml_input = {input_code};
        let __rml_err: Option<gpui::SharedString> = self.__rml_field_errors.get({field:?}).and_then(|e| e.clone());
        if let Some(__rml_err_msg) = __rml_err {{
            gpui::div()
                .id({wrapper_id:?})
                .border_1()                              // ← 问题点 1
                .border_color(gpui::rgb(0xff0000))      // ← 问题点 2
                .child(__rml_input)
                .tooltip(move |window, cx| rml_ui::Tooltip::new(__rml_err_msg.clone()).build(window, cx))
                .into_any_element()
        }} else {{
            __rml_input.into_any_element()
        }}
    }}"#,
    ...
);
```

**问题**：wrapper div 的 `.border_1().border_color(red)` 形成外层边框，而 Input 内部仍有主题色边框（input.rs:391 `this.border_color(border_color).border_1()`），导致：
- 视觉上看到两层边框
- wrapper 边框的 padding 与 Input 自身边框不重合，间距错位

### Input Styled 实现验证（gpui-component input.rs:238-242, 387-401）

```rust
impl Styled for Input {
    fn style(&mut self) -> &mut StyleRefinement {
        &mut self.style
    }
}

// render 内部：
.when(self.appearance, |this| {
    this.bg(bg).rounded(cx.theme().radius)
        .when(self.bordered, |this| {
            this.border_color(border_color)   // 主题色边框（先）
                .border_1()
                ...
        })
})
.items_center()
.gap(gap_x)
.refine_style(&self.style)                    // ← 用户样式（后），覆盖主题色
```

**结论**：`refine_style(&self.style)` 在主题边框之后应用，因此对 Input 调用 `.border_color(red)` 会存入 `self.style.border_color`，render 时覆盖主题色，实现单层红色边框。

### 现有 ValidationRuleSet（compiler/mod.rs:38-44）

```rust
#[derive(Debug, Clone, Default)]
pub struct ValidationRuleSet {
    pub rules: Vec<ValidationRule>,
    pub custom_message: Option<String>,
}
```

### 现有 ValidateArgs parser（scanner.rs:322-407）

`other` 分支返回 `Err`，拒绝未知标识符。需扩展为识别类型名。

## 拟定变更

### 1. 修复红色边框（codegen.rs:497-560）

**文件**：[crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L497-L560)

**变更**：将 `.border_color()` 从 wrapper div 移到 Input 自身，wrapper div 仅保留 `.id()` + `.child()` + `.tooltip()`。

**新的 codegen 输出**：
```rust
{
    let __rml_input = rml_ui::Input::new(&self.__rml_get_or_init_input_state(...))
        .disabled(false);
    let __rml_err: Option<gpui::SharedString> = self.__rml_field_errors.get("field").and_then(|e| e.clone());
    if let Some(__rml_err_msg) = __rml_err {
        // 直接修改 Input 自身边框颜色（覆盖主题色），wrapper div 仅承载 tooltip
        let __rml_input = __rml_input.border_color(gpui::rgb(0xff0000));
        gpui::div()
            .id("rml_input_err:field")
            .child(__rml_input)
            .tooltip(move |window, cx| rml_ui::Tooltip::new(__rml_err_msg.clone()).build(window, cx))
            .into_any_element()
    } else {
        __rml_input.into_any_element()
    }
}
```

**关键点**：
- `__rml_input.border_color(gpui::rgb(0xff0000))` 通过 Styled trait 修改 Input 自身 `style.border_color`
- 不再调用 `.border_1()` —— Input 内部已 `border_1()`（默认 `bordered=true`）
- wrapper div 移除 `.border_1()` 和 `.border_color()`

### 2. 创建 IValidate trait（新文件）

**文件**：[crates/core/src/validate.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/validate.rs)（新建）

**内容**：
```rust
//! 校验接口（Phase B-3.3）
//!
//! 用户可通过 `#[validate(MyValidator)]` 引用实现 `IValidate` 的类型，
//! 自定义校验逻辑（含跨字段校验）。规则式（range/length/required/regex/custom）
//! 与接口式（IValidate）互斥。

use gpui::SharedString;

/// 校验结果
///
/// - `Pass`：通过
/// - `Fail(msg)`：失败，附带默认错误消息
#[derive(Debug, Clone)]
pub enum ValidResult {
    Pass,
    Fail(SharedString),
}

/// 校验接口
///
/// 实现此 trait 的类型可作为 `#[validate(MyValidator)]` 引用。
/// 类型必须实现 `Default`（codegen 通过 `MyValidator::default()` 构造实例）。
///
/// # 方法
/// - `valid(&self, value: &str) -> ValidResult`：简单校验（仅根据 value 判断）
/// - `valid_with_view(&self, value: &str, view: &dyn Any) -> ValidResult`：带视图上下文校验
///   （默认委托给 `valid`，重写后可访问 view 的其他字段进行跨字段校验）
/// - `message(&self, result: &ValidResult) -> Option<SharedString>`：结果→消息转换
///   （默认从 `Fail(msg)` 提取，可重写以实现 i18n 或自定义映射）
///
/// # view 参数说明
///
/// `valid_with_view` 的 `view: &dyn Any` 是视图结构体引用（即 `&self`）。
/// 实现者需 `view.downcast_ref::<MyView>()` 取回具体类型。此设计让 validator
/// 无需自行获取外部状态，所有依赖由 codegen 注入。
///
/// # 示例
///
/// ```rust,ignore
/// use rml_core::validate::{IValidate, ValidResult};
/// use gpui::SharedString;
///
/// #[derive(Default)]
/// struct EmailValidator;
///
/// impl IValidate for EmailValidator {
///     fn valid(&self, value: &str) -> ValidResult {
///         if value.contains('@') {
///             ValidResult::Pass
///         } else {
///             ValidResult::Fail("邮箱格式错误".into())
///         }
///     }
/// }
/// ```
pub trait IValidate: Default + Send + Sync {
    /// 简单校验：仅根据 value 判断
    fn valid(&self, value: &str) -> ValidResult {
        let _ = value;
        ValidResult::Pass
    }

    /// 带视图上下文的校验：可访问 view 的其他字段
    ///
    /// 默认实现委托给 `valid`。重写后可通过 `view.downcast_ref::<MyView>()` 访问跨字段。
    fn valid_with_view(&self, value: &str, view: &dyn std::any::Any) -> ValidResult {
        let _ = view;
        self.valid(value)
    }

    /// 将校验结果转换为错误消息
    ///
    /// - 返回 `None`：校验通过（不显示错误）
    /// - 返回 `Some(msg)`：校验失败，UI 显示红色边框 + tooltip(msg)
    fn message(&self, result: &ValidResult) -> Option<SharedString> {
        match result {
            ValidResult::Pass => None,
            ValidResult::Fail(msg) => Some(msg.clone()),
        }
    }
}
```

**注册模块**：[crates/core/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs#L8-L20)

在第 18 行（`pub mod model;` 之前）插入 `pub mod validate;`。

### 3. 扩展 ValidationRuleSet（compiler/mod.rs:38-44）

**文件**：[crates/engine/src/compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L38-L44)

**变更**：添加 `validator_type: Option<String>` 字段。

```rust
#[derive(Debug, Clone, Default)]
pub struct ValidationRuleSet {
    /// 规则列表（按声明顺序）
    pub rules: Vec<ValidationRule>,
    /// 自定义错误消息（覆盖默认消息）
    pub custom_message: Option<String>,
    /// IValidate 类型名（Phase B-3.3：`#[validate(MyValidator)]`）
    ///
    /// 为 Some 时，rules 必须为空（规则式与接口式互斥）。
    /// codegen 通过 `MyValidator::default().valid_with_view(value, this)` 调用。
    pub validator_type: Option<String>,
}
```

### 4. 扩展 ValidateArgs parser（scanner.rs:322-407）

**文件**：[crates/engine/src/build/scanner.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/scanner.rs#L322-L407)

**变更**：
- `ValidateArgs` 结构添加 `validator_type: Option<String>` 字段
- `other` 分支不再返回 Err，而是捕获标识符为 `validator_type`
- 解析完成后校验：`validator_type` 与 `rules` 互斥；`validator_type` 与 `custom_message` 互斥

```rust
struct ValidateArgs {
    rules: Vec<ValidationRule>,
    custom_message: Option<String>,
    validator_type: Option<String>,
}

impl syn::parse::Parse for ValidateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rules = Vec::new();
        let mut custom_message = None;
        let mut validator_type = None;

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "required" => rules.push(ValidationRule::Required),
                "length" | "range" => { /* 不变 */ }
                "regex" | "custom" | "message" => { /* 不变 */ }
                other => {
                    // 未知标识符：识别为 IValidate 类型名
                    if validator_type.is_some() {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("duplicate validator type: {}", other),
                        ));
                    }
                    validator_type = Some(other.to_string());
                }
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        // 互斥校验
        if validator_type.is_some() {
            if !rules.is_empty() {
                return Err(syn::Error::new(
                    ident.span(),
                    "cannot mix IValidate type with rule-based validators (required/length/range/regex/custom)",
                ));
            }
            if custom_message.is_some() {
                return Err(syn::Error::new(
                    ident.span(),
                    "cannot mix IValidate type with message override (use IValidate::message() instead)",
                ));
            }
        }

        Ok(ValidateArgs { rules, custom_message, validator_type })
    }
}

impl From<ValidateArgs> for ValidationRuleSet {
    fn from(args: ValidateArgs) -> Self {
        ValidationRuleSet {
            rules: args.rules,
            custom_message: args.custom_message,
            validator_type: args.validator_type,
        }
    }
}
```

### 5. 新增 codegen 函数 gen_field_assign_with_validator

**文件**：[crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L588-L607)

**变更**：
- 在 `gen_field_assign_expr` 顶部添加 `validator_type` 优先路由
- 新增 `gen_field_assign_with_validator` 函数

**路由逻辑**：
```rust
fn gen_field_assign_expr(
    field: &str,
    ty: &str,
    validation: Option<&crate::compiler::ValidationRuleSet>,
) -> String {
    // 优先级 1：IValidate 接口式
    if let Some(v) = validation {
        if let Some(validator_type) = &v.validator_type {
            return gen_field_assign_with_validator(field, ty, validator_type);
        }
    }
    // 优先级 2：规则式（原有逻辑不变）
    let rules = match validation { ... };
    ...
}
```

**新函数 gen_field_assign_with_validator**：

数字类型生成代码：
```rust
match value.parse::<i32>() {
    Ok(v) => {
        let __rml_validator = MyValidator::default();
        let __rml_result = __rml_validator.valid_with_view(value.as_ref(), this as &dyn std::any::Any);
        if let Some(__rml_err_msg) = __rml_validator.message(&__rml_result) {
            this.__rml_field_errors.insert("field".to_string(), Some(__rml_err_msg));
        } else {
            this.field = v;
            this.__rml_field_errors.insert("field".to_string(), None);
            this.__rml_bump_version("field");
        }
    }
    Err(_) => {
        this.__rml_field_errors.insert("field".to_string(), Some("请输入有效的整数".into()));
    }
}
```

String 类型生成代码：
```rust
{
    let __rml_value = value.to_string();
    let __rml_validator = MyValidator::default();
    let __rml_result = __rml_validator.valid_with_view(&__rml_value, this as &dyn std::any::Any);
    if let Some(__rml_err_msg) = __rml_validator.message(&__rml_result) {
        this.__rml_field_errors.insert("field".to_string(), Some(__rml_err_msg));
    } else {
        this.field = __rml_value;
        this.__rml_field_errors.insert("field".to_string(), None);
        this.__rml_bump_version("field");
    }
}
```

bool 类型：忽略 IValidate（语义不明确），使用默认 `gen_field_assign_expr_default`。

### 6. 更新测试

#### 6.1 修改现有边框断言（codegen_two_way_binding_test.rs:344-365）

**文件**：[crates/engine/tests/codegen_two_way_binding_test.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/tests/codegen_two_way_binding_test.rs#L344-L365)

**变更**：
- 移除 `border_1()` 在 wrapper div 上的断言
- 改为断言 Input 自身 `.border_color(gpui::rgb(0xff0000))`
- 保留 `rml_input_err:` id 断言（wrapper 仍需 id 以调用 tooltip）

```rust
#[test]
fn gen_model_input_applies_red_border_to_input() {
    let ctx = make_ctx_with_field_types();
    let code = compile(RML_SOURCE_WITH_MODEL, &ctx).expect("compile failed");

    assert!(code.contains("__rml_field_errors.get("), "应检查错误状态");
    // Input 自身应直接 border_color 覆盖主题色（而非 wrapper div）
    assert!(
        code.contains("let __rml_input = __rml_input.border_color(gpui::rgb(0xff0000))"),
        "Input 自身应被设置红色边框，实际：\n{}",
        code
    );
    // wrapper div 不应再有 border_1
    assert!(
        !code.contains(".border_1().border_color(gpui::rgb(0xff0000))"),
        "wrapper div 不应再附加 border，实际：\n{}",
        code
    );
    assert!(code.contains("rml_input_err:"), "wrapper div 仍需 id 承载 tooltip");
}
```

#### 6.2 新增 IValidate 测试

**文件**：[crates/engine/tests/codegen_validate_test.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/tests/codegen_validate_test.rs)（新建）

**测试用例**：
1. `validator_type_is_extracted_from_attribute`：`#[validate(MyValidator)]` → `field_validations["field"].validator_type == Some("MyValidator")`
2. `validator_type_and_rules_are_mutually_exclusive`：`#[validate(MyValidator, range(min=0, max=10))]` → 解析失败
3. `validator_type_and_message_are_mutually_exclusive`：`#[validate(MyValidator, message="...")]` → 解析失败
4. `gen_validator_numeric_field_generates_valid_with_view_call`：数字字段 → 生成 `MyValidator::default().valid_with_view(value.as_ref(), this as &dyn std::any::Any)`
5. `gen_validator_string_field_generates_valid_with_view_call`：String 字段 → 同上
6. `gen_validator_calls_message_and_handles_pass`：成功路径 → 赋值 + 清除错误 + bump_version
7. `gen_validator_calls_message_and_handles_fail`：失败路径 → 设置 __rml_field_errors，不赋值

### 7. 更新文档

#### 7.1 macros.md

**文件**：[docs/04-code-behind/macros.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/04-code-behind/macros.md)

**变更**：
- 在 4.2.9 节末尾添加 `### IValidate 接口式校验` 子节
- 包含：IValidate trait 定义、`valid`/`valid_with_view`/`message` 方法说明、`#[validate(MyValidator)]` 语法、跨字段校验示例、与规则式互斥说明
- 概览表 4.2.1 的 `#[validate]` 行 description 更新为 `range/length/required/regex/custom/IValidate`

#### 7.2 two-way-binding.md

**文件**：[docs/03-binding/two-way-binding.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/03-binding/two-way-binding.md)

**变更**：
- 在 3.3.10 节末尾添加 `### IValidate 接口式校验` 子节
- 包含：声明方式、与规则式校验的差异、跨字段校验示例、codegen 行为
- 更新 3.3.9 节的"红色边框"描述，从"包裹 div 边框"改为"Input 自身边框"

## 假设与决策

### 假设
1. **Input 默认 bordered=true**：因此 codegen 不需要再调用 `.border_1()`，仅需 `.border_color()` 覆盖颜色
2. **validator_type 仅支持单标识符**：不支持 `crate::validators::MyValidator` 路径形式，用户需 `use` 导入（与 `custom = "fn_name"` 一致）
3. **IValidate 实例无状态**：要求 `Default`，通过 `MyValidator::default()` 构造。需要状态参数时使用规则式
4. **`view: &dyn Any` 是视图结构体引用**：codegen 生成 `this as &dyn std::any::Any`，validator 通过 `downcast_ref::<MyView>()` 取回

### 决策
1. **规则式与接口式互斥**：parser 拒绝 `#[validate(MyValidator, range(...))]` 混用。IValidate 已封装完整校验逻辑，无需额外规则
2. **`validator_type` 优先于 `rules`**：codegen 先检查 `validator_type`，再走规则式分支
3. **bool 类型忽略 IValidate**：与规则式一致，bool 语义不明确
4. **保持 wrapper div 承载 tooltip**：Input 的 `RenderOnce` 消费 self 返回内部 div，无法在 Input struct 上调用 `.tooltip()`；wrapper div 仅承载 id + tooltip，不再附加边框

## 验证步骤

1. **运行校验相关测试**：
   ```sh
   cargo test -p rust-rml-engine --test codegen_validate_test
   cargo test -p rust-rml-engine --test codegen_two_way_binding_test
   ```

2. **运行 scanner 单元测试**：
   ```sh
   cargo test -p rust-rml-engine --lib build::scanner
   ```

3. **全工作区回归**：
   ```sh
   cargo test --workspace
   ```

4. **Demo 编译验证**（红色边框修复不破坏 demo）：
   ```sh
   cargo build -p rust-rml-demo
   ```

5. **手动验证**：在 demo 中创建 IValidate 实现并标注 `#[validate(MyValidator)]`，运行 demo 输入触发校验，确认红色边框为单层（Input 自身）而非双层（wrapper + Input）
