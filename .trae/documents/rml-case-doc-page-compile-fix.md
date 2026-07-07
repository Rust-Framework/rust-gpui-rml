# CaseDocPage RML 改造 —— 编译错误修复收尾

## 背景

续接 [rml-slot-step5-8-continue.md](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-slot-step5-8-continue.md) 的 Step 8。

用户核心诉求："case_doc_page模板如果真的需要，也应该按照.rml规范编写才对" —— CaseDocPage 已改造为 `.rml` + `.rml.rs`，table_case 已改造为 `<CaseDocPage>` + `<template slot="...">` 形式，但 `cargo check -p rust-rml-demo` 编译失败。

## 当前状态分析

### 已完成

| 项目 | 状态 | 依据 |
|------|------|------|
| Step 5: gen_user_component slot 闭包改造 | ✅ | [user_component.rs:74-122](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) `__rml_self_entity` 捕获 + `with_self_alias` + 每 slot 闭包前 clone |
| Step 6: Phase 2 单元测试 | ✅ | 16 个测试通过，819 个 lib 测试无回归 |
| Step 7: CaseDocPage `.rml` + `.rml.rs` | ✅ | [case_doc_page.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rml) + [case_doc_page.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/case_doc_page.rml.rs) |
| Step 8: table_case 改造 | ⏳ 编译失败 | [table_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml) + [table_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml.rs) |

### 编译错误根因

从生成代码 [target/.../rml_generated/table_case.rs:18-23](file:///d:/GitCode/RF/rust-gpui-rml/target/debug/build/rust-rml-demo-44b49bea0090a75e/out/rml_generated/table_case.rs) 可见：

```rust
let __rml_entity = self.case_doc_page.as_ref().expect("init CaseDocPage in on_loaded").clone();
let __rml_self_entity = cx.entity();
__rml_entity.update(cx, |this, _cx| { this.title = (cx.t("case.table.title")).into(); });  // ← E0502
__rml_entity.update(cx, |this, _cx| { this.description = "...".into(); });
__rml_entity.update(cx, |this, _cx| { this.code_rml = (self.rml_sample()).into(); });
__rml_entity.update(cx, |this, _cx| { this.code_rust = (self.rust_sample()).into(); });
```

**E0502 借用冲突**：`__rml_entity.update(cx, ...)` 对 `cx` 可变借用，闭包内 `cx.t("case.table.title")` 对 `cx` 不可变借用，两者冲突。

**根因**：[user_component.rs:138-175](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 的 `gen_prop_assign` 对绑定属性生成 `__rml_entity.update(cx, |this, _cx| { this.<field> = (<rust_expr>).into(); })`，当 `<rust_expr>` 引用 `cx`（如 `cx.t(...)`）时即冲突。

## 修复方案

### 核心思路

将绑定表达式的计算移到 `update` 闭包**外**，避免闭包内借用 `cx`。

**当前生成**（冲突）：
```rust
__rml_entity.update(cx, |this, _cx| { this.title = (cx.t("case.table.title")).into(); });
```

**目标生成**（无冲突）：
```rust
{ let __rml_value_title = cx.t("case.table.title"); __rml_entity.update(cx, |this, _cx| { this.title = (__rml_value_title).into(); }); }
```

`cx.t(...)` 在 `update` 闭包外计算，借用在闭包外释放，不与 `update(cx, ...)` 冲突。

### 适用范围

- **绑定属性**（`PropValue::Bind`）：统一采用"闭包外计算"模式。无论表达式是否引用 `cx`，都先 `let __rml_value_<field> = <expr>;` 再在闭包内赋值。
  - 引用 `cx` 的表达式（`cx.t(...)`）：避免借用冲突 ✅
  - 引用 `self` 的表达式（`self.field` / `self.computed()`）：闭包外计算同样合法（`self` 与 `cx` 不冲突）
  - 引用 `loop_var` 的表达式：闭包外计算同样合法
- **静态属性**（`PropValue::Static`）：不涉及 `cx`，保持原 `__rml_entity.update(cx, |this, _cx| { this.<field> = ...; });` 格式不变。

## 实施步骤

### 步骤 1：修改 `gen_prop_assign` 的 `PropValue::Bind` 分支

**文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs)

**修改位置**：`gen_prop_assign` 函数（line 138-175）

**当前代码**（line 163-174）：
```rust
let assign_expr = match attr_value {
    PropValue::Static(value) => gen_static_assign(name, value, field_type)?,
    PropValue::Bind(expr) => {
        let rust_expr = component_bind_rust_expr(expr, &loop_vars_slice, &computed_slice);
        gen_bind_assign(name, &rust_expr, field_type)
    }
};

Ok(Some(format!(
    "__rml_entity.update(cx, |this, _cx| {{ {} }});",
    assign_expr
)))
```

**改为**：
```rust
match attr_value {
    PropValue::Static(value) => {
        let assign_expr = gen_static_assign(name, value, field_type)?;
        Ok(Some(format!(
            "__rml_entity.update(cx, |this, _cx| {{ {} }});",
            assign_expr
        )))
    }
    PropValue::Bind(expr) => {
        let rust_expr = component_bind_rust_expr(expr, &loop_vars_slice, &computed_slice);
        // 在 update 闭包外计算表达式值，避免 cx 借用冲突
        // （如 cx.t(...) 与 update(cx, ...) 冲突）
        let value_var = format!("__rml_value_{}", name);
        let assign_expr = gen_bind_assign(name, &value_var, field_type);
        Ok(Some(format!(
            "{{ let {} = {}; __rml_entity.update(cx, |this, _cx| {{ {} }}); }}",
            value_var, rust_expr, assign_expr
        )))
    }
}
```

**同步更新文档注释**（line 129-137）：在 `gen_prop_assign` 的 doc comment 中补充绑定属性的新生成格式：
```rust
/// - 绑定属性 `sample={sample}` → `{ let __rml_value_sample = self.sample(); __rml_entity.update(cx, |this, _cx| { this.sample = (__rml_value_sample).into(); }); }`
///   （在 update 闭包外计算表达式值，避免 cx.t(...) 等引用 cx 的表达式与 update(cx, ...) 借用冲突）
```

### 步骤 2：更新现有测试断言

**文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 测试模块

绑定属性的生成格式变化，需更新 3 个测试用例的断言：

**test_bind_field_prop**（line 422-432）：
```rust
// 旧断言：code.contains("this.title = (self.title).into();")
// 新断言：
assert!(
    code.contains("let __rml_value_title = self.title;"),
    "expected bind value pre-computation, got: {}",
    code
);
assert!(
    code.contains("this.title = (__rml_value_title).into();"),
    "expected bind field assignment via value var, got: {}",
    code
);
```

**test_bind_computed_prop**（line 434-445）：
```rust
// 旧断言：code.contains("this.sample = (self.sample()).into();")
// 新断言：
assert!(
    code.contains("let __rml_value_sample = self.sample();"),
    "expected computed method pre-computation, got: {}",
    code
);
assert!(
    code.contains("this.sample = (__rml_value_sample).into();"),
    "expected computed assignment via value var, got: {}",
    code
);
```

**test_bind_numeric_field**（line 447-457）：
```rust
// 旧断言：code.contains("this.count = self.count;")
// 新断言：
assert!(
    code.contains("let __rml_value_count = self.count;"),
    "expected numeric field pre-computation, got: {}",
    code
);
assert!(
    code.contains("this.count = __rml_value_count;"),
    "expected numeric assignment via value var, got: {}",
    code
);
```

### 步骤 3：新增 cx 借用冲突场景测试

**文件**：[crates/engine/src/compiler/user_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) 测试模块

新增测试用例，验证 `cx.t(...)` 等 i18n 调用的绑定属性生成正确格式：

```rust
#[test]
fn test_bind_i18n_call_prop() {
    // title={t("case.table.title")} 应生成闭包外计算 + update 闭包内赋值
    let info = make_info("MyComp", &[("title", "SharedString")]);
    let elem = make_element("MyComp", vec![bind_attr("title", "t(\"case.table.title\")")], vec![]);
    let code = gen(&info, &elem, &CodegenCtx::default());
    assert!(
        code.contains("let __rml_value_title = cx.t(\"case.table.title\");"),
        "expected i18n call pre-computation outside update closure, got: {}",
        code
    );
    assert!(
        code.contains("this.title = (__rml_value_title).into();"),
        "expected i18n value assignment via value var, got: {}",
        code
    );
    // 确保闭包内不再直接调用 cx.t(...)
    let update_closure_start = code.find("__rml_entity.update(cx,").unwrap();
    let update_closure_end = code[update_closure_start..].find("});").unwrap();
    let closure_body = &code[update_closure_start..update_closure_start + update_closure_end];
    assert!(
        !closure_body.contains("cx.t("),
        "cx.t(...) should be outside update closure, but found inside: {}",
        closure_body
    );
}
```

### 步骤 4：编译验证

**步骤 4.1**：单元测试通过
```bash
cargo test -p rust-rml-engine --lib user_component
```
预期：所有 user_component 测试通过（含新增的 `test_bind_i18n_call_prop`）。

**步骤 4.2**：全量测试无回归
```bash
cargo test -p rust-rml-engine --lib
```
预期：所有 lib 测试通过，无回归。

**步骤 4.3**：demo 编译通过
```bash
cargo check -p rust-rml-demo
```
预期：table_case 编译通过，无 E0502 借用冲突错误。

### 步骤 5：运行时验证（可选）

```bash
cargo run -p rust-rml-demo
```

导航到 table case，验证：
- 标题 + 描述正确显示
- 演示区 6 个 Table 示例渲染正常
- 代码区 Tab 切换正常（.rml / .rml.rs）
- API 区 Table 渲染正常

## 关键设计决策

### 决策 1：统一对所有绑定属性采用"闭包外计算"模式

不区分表达式是否引用 `cx`，统一用 `let __rml_value_<field> = <expr>;` 包裹。

**理由**：
- 简化代码生成逻辑，无需分析表达式是否引用 `cx`
- 对不引用 `cx` 的表达式（如 `self.field`）无副作用，仅多一个 let 绑定
- 性能影响可忽略（编译期优化会消除多余绑定）

### 决策 2：变量名用字段名作后缀

`__rml_value_<field_name>`（如 `__rml_value_title`、`__rml_value_code_rml`）。

**理由**：
- 避免多个绑定属性之间的变量名冲突
- 字段名在 struct 内唯一，保证变量名唯一
- 可读性好，便于调试

### 决策 3：静态属性保持原格式

静态属性 `title="..."` 不涉及 `cx`，保持 `__rml_entity.update(cx, |this, _cx| { this.title = "...".into(); });` 不变。

**理由**：
- 静态属性在闭包内直接赋值，无借用冲突风险
- 减少生成代码变化范围，降低测试更新成本

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| `__rml_value_<field>` 与用户代码变量名冲突 | `__rml_` 前缀是 RML 框架保留前缀，用户代码不应使用 |
| 绑定表达式有副作用，闭包外计算改变执行顺序 | RML 绑定表达式应为纯表达式（字段访问 / computed 方法 / i18n 调用），无副作用 |
| 生成代码行数增加 | 每个绑定属性多一行 `let`，可接受 |

## 验证清单

- [ ] 步骤 1：`gen_prop_assign` Bind 分支改造完成
- [ ] 步骤 2：3 个现有测试断言更新完成
- [ ] 步骤 3：新增 `test_bind_i18n_call_prop` 测试用例
- [ ] 步骤 4.1：`cargo test -p rust-rml-engine --lib user_component` 通过
- [ ] 步骤 4.2：`cargo test -p rust-rml-engine --lib` 无回归
- [ ] 步骤 4.3：`cargo check -p rust-rml-demo` 编译通过
- [ ] 步骤 5（可选）：`cargo run -p rust-rml-demo` 运行验证 table case 渲染

## 实施顺序

严格按步骤 1 → 2 → 3 → 4 顺序执行。步骤 5 可选，由用户决定是否运行时验证。
