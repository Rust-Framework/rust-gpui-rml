# Phase B-3.1 双向绑定校验 UI 计划

## Context（背景）

用户运行 demo 后反馈两个问题：
1. **年龄输入数字超出字段类型最大值时变 0**：当前 codegen 生成 `value.parse::<i32>().unwrap_or(0)`，parse 失败（包括溢出）时用 0 覆盖原值，丢失用户输入和原字段值。
2. **校验失败无 UI 表现**：用户期望"双向失败 UI 要表现校验失败的效果"，当前 parse 失败时静默兜底为 0，无任何视觉反馈。

用户确认的设计方向：
- UI 表现：**红色边框 + tooltip 气泡**（hover 时显示错误提示，避免错误文本挤压页面空间）
- 校验机制：使用**宏（Attribute）方案**（类似 C# Attribute），不污染 RML 声明语法（不加 `min`/`max` 属性），后续迭代完善数据校验体系架构
- 本次范围：仅实现**类型校验**（parse 失败 + 类型溢出），业务范围校验（`#[validate]` 宏）留待未来

## 当前问题代码

`crates/engine/src/compiler/codegen.rs` L556-566 `gen_field_assign_expr`：
```rust
fn gen_field_assign_expr(field: &str, ty: &str) -> String {
    match ty {
        "i32" | "u32" | "i64" | "u64" | "isize" | "usize" =>
            format!("this.{} = value.parse::<{}>().unwrap_or(0)", field, ty),
        "f32" => format!("this.{} = value.parse::<f32>().unwrap_or(0.0)", field),
        "f64" => format!("this.{} = value.parse::<f64>().unwrap_or(0.0)", field),
        "bool" => format!("this.{} = !value.is_empty()", field),
        _ => format!("this.{} = value.to_string()", field),
    }
}
```

`unwrap_or(0)` 是问题根源——parse 失败时静默兜底，不保留原值、不通知用户。

## 变更方案

### A. macros 层：注入校验状态字段

**文件**：`crates/macros/src/component.rs` → `inject_tracking_fields` 函数

在现有 `__rml_input_state_versions` 字段后追加：
```rust
let field_errors_field: Field = parse_quote! {
    #[allow(dead_code)]
    __rml_field_errors: std::collections::HashMap<String, Option<gpui::SharedString>>
};
named.named.push(field_errors_field);
```

存储每个字段的校验错误信息：`None` = 校验通过，`Some(msg)` = 校验失败。`SharedString` 与 `InputState::value()` 返回类型一致，便于 codegen 处理。

### B. codegen 层：重写反向赋值逻辑

**文件**：`crates/engine/src/compiler/codegen.rs`

#### B1. 重写 `gen_field_assign_expr`（L556-566）

函数签名不变，但返回值从**单行赋值表达式**改为**完整代码块**（含 parse + 赋值 + 错误处理 + bump_version）。

**整数类型**（i32/u32/i64/u64/isize/usize）：
```rust
format!(r#"match value.parse::<{ty}>() {{
    Ok(v) => {{
        this.{field} = v;
        this.__rml_field_errors.insert({field:?}.to_string(), None);
        this.__rml_bump_version({field:?});
    }}
    Err(_) => {{
        this.__rml_field_errors.insert({field:?}.to_string(), Some("请输入有效的整数".into()));
    }}
}}"#, field = field, ty = ty)
```

**浮点类型**（f32/f64）：
```rust
// 同上，错误消息改为 "请输入有效的数字"
```

**bool 类型**：
```rust
format!(r#"this.{field} = !value.is_empty();
this.__rml_field_errors.insert({field:?}.to_string(), None);
this.__rml_bump_version({field:?});"#, field = field)
```

**String/其他**：
```rust
format!(r#"this.{field} = value.to_string();
this.__rml_field_errors.insert({field:?}.to_string(), None);
this.__rml_bump_version({field:?});"#, field = field)
```

#### B2. 调整 `gen_input_state_impl` 反向闭包生成（L862-870）

当前调用方追加 `this.__rml_bump_version({:?})`，但新 `gen_field_assign_expr` 已包含 `bump_version`，需移除调用方的追加：

```rust
// 旧（L866-869）：
reverse_arms.push_str(&format!(
    "                \"{}\" => {{ {}; this.__rml_bump_version({:?}); }}\n",
    field, assign, field
));

// 新：
reverse_arms.push_str(&format!(
    "                \"{}\" => {{ {} }}\n",
    field, assign
));
```

#### B3. 正向同步清除错误状态（L921-928）

在 `set_value` 后清除该字段的错误状态（VM 值是代码设置的，视为有效）：

```rust
// 在 L926 entity.update(... set_value ...) 后追加：
out.push_str("            self.__rml_field_errors.insert(field.to_string(), None);\n");
```

### C. codegen 层：重写 `gen_model_input` 渲染

**文件**：`crates/engine/src/compiler/codegen.rs` L497-535

当前返回 `rml_ui::Input::new(...)`，改为返回条件包裹结构：

```rust
// 生成的代码结构（以 field="name" 为例）：
{
    let __rml_input = rml_ui::Input::new(&self.__rml_get_or_init_input_state("name", Some("姓名"), _window, cx));
    let __rml_err: Option<gpui::SharedString> = self.__rml_field_errors.get("name").and_then(|e| e.clone());
    if let Some(__rml_err_msg) = __rml_err {
        gpui::div()
            .id("rml_input_err:name")
            .border_1()
            .border_color(gpui::rgb(0xff0000))
            .child(__rml_input)
            .tooltip(move |window, cx| rml_ui::Tooltip::new(__rml_err_msg.clone()).build(window, cx))
            .into_any_element()
    } else {
        __rml_input.into_any_element()
    }
}
```

**关键点**：
- `.id("rml_input_err:<field>")` 使 div 成为 `StatefulInteractiveElement`，从而可调用 `.tooltip()`
- `.tooltip(move |window, cx| ...)` 闭包接收 `&mut Window, &mut App`，捕获 `__rml_err_msg`（`SharedString` 是 `Clone`）
- `.into_any_element()` 统一 if/else 分支返回类型为 `AnyElement`
- 需在 codegen 顶部 `use` 中确认 `IntoElement` trait 在作用域内（已在 L172-173 import）

**disabled 属性处理**：当前在 `gen_model_input` 末尾追加 `.disabled(bool)`。新方案中 disabled 应加在 `__rml_input` 上（包裹前），调整代码顺序。

### D. 测试更新

**文件**：`crates/engine/tests/codegen_two_way_binding_test.rs`

#### D1. 更新现有测试断言
- `gen_model_input_generates_type_conversion_for_i32`：断言 `match value.parse::<i32>()` + `Ok(v) =>` + `Err(_) =>`
- `gen_model_input_generates_to_string_for_string`：断言 `this.name = value.to_string()` + `__rml_field_errors.insert`
- `gen_model_input_includes_bump_version_and_notify`：断言 `bump_version` 在 `Ok` 分支内（非外部追加）

#### D2. 新增校验测试
- `gen_field_assign_generates_error_handling_for_i32`：验证 i32 生成 `match` + `Err(_)` 分支 + "请输入有效的整数"
- `gen_field_assign_preserves_old_value_on_error`：验证 parse 失败时不覆盖原值（`Err` 分支无 `this.field = ...`）
- `gen_model_input_wraps_with_error_div`：验证生成 `__rml_err` 检查 + `div().id().border_1().border_color().tooltip()`
- `gen_model_input_includes_tooltip_closure`：验证 `Tooltip::new(...).build(window, cx)` + `.into_any_element()`
- `gen_input_state_impl_clears_error_on_forward_sync`：验证正向同步部分包含 `__rml_field_errors.insert(field, None)`

### E. 文档更新

**文件**：`docs/03-binding/two-way-binding.md`

- 3.3.3 字段类型表：补充"校验失败行为"列（parse 失败 → 保留原值 + 红色边框 + tooltip）
- 3.3.8 循环防护：补充"校验失败时不 bump_version，不触发正向同步"
- 新增 3.3.X **校验失败 UI**：说明红色边框 + tooltip 机制 + 错误消息语言 + 未来 `#[validate]` 宏预留

**文件**：`docs/04-code-behind/macros.md`

- `#[component]` 注入字段表：补充 `__rml_field_errors: HashMap<String, Option<SharedString>>`
- 新增说明：未来 `#[validate]` 宏（C# Attribute 风格）将基于此字段实现业务校验

### F. Demo 验证

`demo/src/main_window.rml.rs` + `demo/src/main_window.rml` 无需修改——`<input model={age}>` 自动获得校验 UI。

## 不做的事项

- **不**实现 `#[validate]` 宏（用户明确为未来迭代）
- **不**支持 `min`/`max` RML 属性（用户要求不污染声明语法）
- **不**实现自定义校验消息（默认中文消息，未来 `#[validate]` 宏支持自定义）
- **不**修改 `gen_field_value_expr`（正向同步逻辑不变）
- **不**修改 `crates/ui/src/lib.rs`（`Tooltip` 已 re-export，L68）

## 验证步骤

### 1. 编译验证
```bash
cargo build -p rust-rml-engine
cargo build -p rust-rml-demo
```

### 2. 测试验证
```bash
cargo test -p rust-rml-engine --test codegen_two_way_binding_test
cargo test --workspace
```

### 3. 运行时验证
```bash
cargo run -p rust-rml-demo
```

验证场景：
- 在"年龄"输入框输入 `abc` → Input 显示红色边框，hover 显示"请输入有效的整数"，ViewModel 的 `age` 保留原值
- 在"年龄"输入框输入 `99999999999999999999`（超出 i32 范围）→ 同上校验失败
- 在"年龄"输入框输入 `25` → 校验通过，红色边框消失
- 点击"+1"按钮 → `count` 字段变化，`profile_summary` 更新，不影响 `age` 的校验状态

## 执行顺序

1. macros 注入 `__rml_field_errors` 字段（A）
2. codegen 重写 `gen_field_assign_expr`（B1）
3. codegen 调整反向闭包生成（B2）
4. codegen 正向同步清除错误状态（B3）
5. codegen 重写 `gen_model_input` 渲染（C）
6. 编译验证（cargo build）
7. 更新测试（D）
8. 测试验证（cargo test）
9. 运行时验证（cargo run）
10. 文档更新（E）
