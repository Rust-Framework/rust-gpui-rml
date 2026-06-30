# Phase B-3 校验体系完整实现计划

## 摘要

用户反馈两个核心问题（已在上一会话完成代码实现）：
1. 数字超出字段类型最大值时变 0 → 已修复为 parse 失败保留原值 + 设置错误状态
2. 双向失败时 UI 需表现校验失败效果 → 已实现红色边框 + tooltip 气泡

用户明确指示："MVVM 的设计思路是数据驱动，且 RML 要对齐 HTML+CSS，尽量不要污染声明语法，使用类似 C# 的 Attribute 方案最合适（rust 中的宏），后续迭代要进一步完善数据校验体系架构。"

本计划分两阶段推进：
- **阶段 1（Phase B-3.1 收尾）**：完成上一会话批准的 Step E 剩余文档更新
- **阶段 2（Phase B-3.2 校验体系架构）**：实现 `#[validate]` 宏（C# Attribute 风格），支持 range/length/required/regex/custom 等校验规则

## 当前状态分析

### 已完成（Phase B-3.1 代码实现）

| 文件 | 状态 | 内容 |
|------|------|------|
| `crates/macros/src/component.rs` L134-141 | ✅ | 已注入 `__rml_field_errors: HashMap<String, Option<SharedString>>` 字段 |
| `crates/engine/src/compiler/codegen.rs` L585-628 | ✅ | `gen_field_assign_expr` 已实现 match parse + Ok/Err 分支 + 默认错误消息 |
| `crates/engine/src/compiler/codegen.rs` L534-557 | ✅ | `gen_model_input` 已实现条件包裹 div + tooltip |
| `crates/engine/src/compiler/codegen.rs` L987 | ✅ | `gen_input_state_impl` 正向同步后清除错误 |
| `crates/engine/tests/codegen_two_way_binding_test.rs` | ✅ | 16 个测试通过（含 6 个 Phase B-3.1 校验 UI 测试） |

### 待完成（文档收尾）

| 文件 | 待办 |
|------|------|
| `docs/03-binding/two-way-binding.md` | 编号冲突：两个 "3.3.9"（校验失败 UI + 性能），后续编号错位 |
| `docs/04-code-behind/macros.md` | 注入字段表缺少 `__rml_field_errors`；无 `#[validate]` 宏说明 |
| `docs/10-advanced/performance.md` | 未提及 `__rml_field_errors.get()` 查询开销 |

### 关键架构约束

- **scanner 模式**：宏信息必须能被 `crates/engine/src/build/scanner.rs` 通过 syn 静态提取，再经 `CodegenCtx` 传给 codegen。宏在用户 crate 编译期展开，codegen 在 build.rs 中运行，二者不能跨阶段直接通信。
- **`__rml_field_errors` 复用**：字段校验错误状态已由 `#[component]`/`#[window]` 注入，UI 包裹机制（红色边框 + tooltip）已就绪。`#[validate]` 宏只需把规则传递到 codegen，由 codegen 生成校验代码写入此字段。
- **与 `#[command]` 零冲突**：`#[command]` 自动注入 `bump_version`，`#[validate]` 不破坏此机制——校验在 parse 成功后、赋值前执行，校验通过才赋值 + `bump_version`。
- **C# Attribute 风格**：作为字段属性标注规则，不污染 RML 声明语法（`<input model={age} />` 保持不变），校验规则在 Rust 端声明。

## 阶段 1：Phase B-3.1 文档收尾

### 1.1 修正 `docs/03-binding/two-way-binding.md` 编号冲突

**文件**：`d:\GitCode\RF\rust-gpui-rml\docs\03-binding\two-way-binding.md`

**当前状态**：第 332 行 `## 3.3.9 校验失败 UI` 与第 382 行 `## 3.3.9 双向绑定的性能` 编号冲突，后续 `3.3.10 常见陷阱`、`3.3.11 小结` 错位。

**修正方案**：
- 第 332 行保持 `## 3.3.9 校验失败 UI`
- 第 382 行 `## 3.3.9 双向绑定的性能` → `## 3.3.10 双向绑定的性能`
- 后续 `3.3.10 常见陷阱` → `3.3.11 常见陷阱`
- 后续 `3.3.11 小结` → `3.3.12 小结`

### 1.2 补充 `docs/04-code-behind/macros.md` 注入字段表

**文件**：`d:\GitCode\RF\rust-gpui-rml\docs\04-code-behind\macros.md`

**位置**：`#[component]` 章节的注入字段表（约 L84-89）

**变更**：在现有 4 个注入字段后追加第 5 个：
```
| `__rml_field_errors` | `HashMap<String, Option<SharedString>>` | Phase B-3.1 校验状态（None=通过，Some(msg)=失败） |
```

### 1.3 补充 `docs/10-advanced/performance.md` 校验查询开销

**文件**：`d:\GitCode\RF\rust-gpui-rml\docs\10-advanced\performance.md`

**位置**：10.1.2 "双向绑定的性能特征" 表后

**新增条目**：
```
| `__rml_field_errors.get(field)` | O(1) HashMap 查询 | 每次 render 调用一次 | 可忽略（HashMap 查询常数时间） |
```

## 阶段 2：`#[validate]` 宏实现（Phase B-3.2）

### 2.1 设计 `#[validate]` 宏 API（C# Attribute 风格）

#### 用户使用示例

```rust
#[window]
#[derive(Default)]
pub struct Form {
    #[validate(required, length(min = 3, max = 20))]
    pub name: String,

    #[validate(range(min = 0, max = 150))]
    pub age: i32,

    #[validate(regex = r"^\w+@\w+\.\w+$", message = "邮箱格式错误")]
    pub email: String,

    #[validate(custom = "validate_phone")]
    pub phone: String,
}

impl Form {
    fn validate_phone(value: &str) -> Option<String> {
        if value.starts_with("1") && value.len() == 11 {
            None
        } else {
            Some("手机号格式错误".into())
        }
    }
}
```

#### 支持的校验规则

| 规则 | 语法 | 适用类型 | 说明 |
|------|------|---------|------|
| `required` | `required` | 所有 | 非空校验（String 非空、数字非零、bool 为 true） |
| `length` | `length(min = N, max = M)` | String | 字符串长度范围（min/max 任一可省略） |
| `range` | `range(min = N, max = M)` | 数值类型 | 数值范围（min/max 任一可省略） |
| `regex` | `regex = "pattern"` | String | 正则匹配（使用 `regex` crate） |
| `custom` | `custom = "fn_name"` | 所有 | 自定义校验函数 `fn(&T) -> Option<String>` |
| `message` | `message = "..."` | 所有 | 自定义错误消息（可附在任何规则后） |

#### 校验执行顺序

1. 类型校验（parse）失败 → 设置默认错误消息，短路
2. 用户规则按声明顺序依次执行：
   - `required` → `length` → `range` → `regex` → `custom`
3. 任一规则失败 → 写入 `__rml_field_errors`，**不赋值、不 bump_version**
4. 全部通过 → 赋值 + 清除错误 + `bump_version`

### 2.2 实现步骤

#### Step A：macros 侧创建 `validate.rs` 模块

**新文件**：`d:\GitCode\RF\rust-gpui-rml\crates\macros\src\validate.rs`

**内容**：
- `ValidationRule` 枚举（需 Clone, Debug）：
  ```rust
  pub enum ValidationRule {
      Required,
      Length { min: Option<i64>, max: Option<i64> },
      Range { min: Option<f64>, max: Option<f64> },
      Regex(String),
      Custom(String),
  }

  pub struct ValidationRuleSet {
      pub rules: Vec<ValidationRule>,
      pub custom_message: Option<String>,
  }
  ```

- `ValidateArgs` 解析器（实现 `syn::parse::Parse`）：
  - 逗号分隔的规则列表
  - 支持 `required`（标识符）、`length(min = N, max = M)`（函数调用语法）、`range(...)`、`regex = "..."`、`custom = "..."`、`message = "..."`
  - 解析为 `ValidationRuleSet`

- `strip_validate_attributes(fields: &mut Vec<Field>) -> HashMap<String, ValidationRuleSet>` 函数：
  - 遍历 `Vec<Field>`，对每个字段检查 `field.attrs`
  - 找到 `#[validate(...)]` 时解析参数，存入 HashMap（key=字段名）
  - 从 `field.attrs` 中移除该属性（避免编译期未识别属性警告）

#### Step B：macros 侧集成到 `#[window]`/`#[component]`

**文件**：`d:\GitCode\RF\rust-gpui-rml\crates\macros\src\component.rs`

**变更**：
1. 在 `inject_tracking_fields` 之前调用 `validate::strip_validate_attributes(&mut named.named)`，剥离 `#[validate]` 属性
2. 剥离的规则集暂存（但因为宏在用户 crate 编译期运行，无法直接传递给 build.rs，需要通过 scanner 重新提取）

**关键决策**：宏仅剥离属性 + 让字段保持原样，不生成校验代码。校验规则由 scanner 重新从源码提取（与 `#[computed]` 的依赖扫描模式一致）。

#### Step C：scanner 侧扩展元数据扫描

**文件**：`d:\GitCode\RF\rust-gpui-rml\crates\engine\src\build\scanner.rs`

**变更**：
1. 将 `ValidationRule` 和 `ValidationRuleSet` 类型移到 `crates/engine/src/build/scanner.rs`（或共享模块），避免 macros → engine 反向依赖
2. `StructMetadata` 新增字段：
   ```rust
   pub field_validations: HashMap<String, ValidationRuleSet>,
   ```
3. `scan_struct_metadata` 第一遍扫描时，读取 `field.attrs`，遇到 `path.is_ident("validate")` 时：
   - 调用 `attr.parse_args::<ValidateArgs>()` 解析（复用 macros 的解析逻辑，或独立实现一份）
   - 转换为 `ValidationRuleSet` 存入 `field_validations`

**关键约束**：scanner 通过 syn 静态提取，不能依赖运行时编译产物。`ValidateArgs` 解析逻辑需要可在 build.rs 中调用（无 proc_macro 依赖）。

#### Step D：CodegenCtx 新增 `field_validations` 字段

**文件**：`d:\GitCode\RF\rust-gpui-rml\crates\engine\src\compiler\mod.rs`

**变更**：
1. `CodegenCtx` 新增字段：
   ```rust
   pub field_validations: HashMap<String, ValidationRuleSet>,
   ```
2. 派生 `Clone`（如未派生）
3. `crates/engine/src/build/mod.rs` 的 `Builder::build()` 中，将 `StructMetadata.field_validations` 映射到 `CodegenCtx.field_validations`

#### Step E：codegen 侧 `gen_field_assign_expr` 应用校验规则

**文件**：`d:\GitCode\RF\rust-gpui-rml\crates\engine\src\compiler\codegen.rs`（约 L580-650）

**变更**：函数签名从 `gen_field_assign_expr(field: &str, ty: &str)` 改为 `gen_field_assign_expr(field: &str, ty: &str, validation: Option<&ValidationRuleSet>)`。

**生成代码逻辑**：

```rust
// 无校验规则时（向后兼容）
match value.parse::<i32>() {
    Ok(v) => {
        this.age = v;
        this.__rml_field_errors.insert("age", None);
        this.__rml_bump_version("age");
    }
    Err(_) => {
        this.__rml_field_errors.insert("age", Some("请输入有效的整数".into()));
    }
}

// 有 range(min = 0, max = 150) 校验规则时
match value.parse::<i32>() {
    Ok(v) => {
        if v >= 0 && v <= 150 {
            this.age = v;
            this.__rml_field_errors.insert("age", None);
            this.__rml_bump_version("age");
        } else {
            this.__rml_field_errors.insert("age", Some("年龄必须在 0-150 之间".into()));
        }
    }
    Err(_) => {
        this.__rml_field_errors.insert("age", Some("请输入有效的整数".into()));
    }
}

// 有 length(min = 3, max = 20) + required 校验规则时（String 字段）
{
    let __rml_value = value.to_string();
    if __rml_value.is_empty() {
        this.__rml_field_errors.insert("name", Some("此项为必填".into()));
    } else if __rml_value.len() < 3 || __rml_value.len() > 20 {
        this.__rml_field_errors.insert("name", Some("长度必须在 3-20 之间".into()));
    } else {
        this.name = __rml_value;
        this.__rml_field_errors.insert("name", None);
        this.__rml_bump_version("name");
    }
}

// 有 regex 校验规则时（String 字段）
{
    let __rml_value = value.to_string();
    let __rml_re = regex::Regex::new(r"^\w+@\w+\.\w+$").unwrap();
    if !__rml_re.is_match(&__rml_value) {
        this.__rml_field_errors.insert("email", Some("邮箱格式错误".into()));
    } else {
        this.email = __rml_value;
        this.__rml_field_errors.insert("email", None);
        this.__rml_bump_version("email");
    }
}

// 有 custom 校验规则时
{
    let __rml_value = value.to_string();
    if let Some(__rml_err) = Form::validate_phone(&__rml_value) {
        this.__rml_field_errors.insert("phone", __rml_err);
    } else {
        this.phone = __rml_value;
        this.__rml_field_errors.insert("phone", None);
        this.__rml_bump_version("phone");
    }
}
```

**自定义消息支持**：若 `validation.custom_message` 为 `Some(msg)`，所有失败分支使用此消息替代默认消息。

**调用方修改**：`gen_input_state_impl` 中调用 `gen_field_assign_expr` 时传入 `ctx.field_validations.get(field)`。

#### Step F：添加 `regex` crate 依赖

**文件**：`d:\GitCode\RF\rust-gpui-rml\crates\engine\Cargo.toml`

**变更**：`[dependencies]` 新增 `regex = "1"`。

**codegen 生成代码说明**：用户 crate 需要自行添加 `regex` 依赖（或框架通过 `rml::runtime::regex` re-export）。推荐 re-export 方式：在 `crates/core/src/lib.rs` 或 `crates/ui/src/lib.rs` 中 `pub use regex;`，codegen 生成代码使用 `rml::regex::Regex::new(...)`。

#### Step G：测试

**新文件**：`d:\GitCode\RF\rust-gpui-rml\crates\engine\tests\codegen_validation_test.rs`

**测试用例**（每个用例构造带 `field_validations` 的 CodegenCtx，调用 `compile` 验证生成代码）：

1. `range_validation_generates_bounds_check`：range(min=0, max=150) → 验证生成 `v >= 0 && v <= 150`
2. `range_validation_uses_custom_message`：range + message → 验证使用自定义消息
3. `length_validation_generates_len_check`：length(min=3, max=20) → 验证生成 `.len() < 3 || .len() > 20`
4. `required_validation_generates_empty_check`：required → 验证生成 `.is_empty()` 检查
5. `regex_validation_generates_pattern_match`：regex="..." → 验证生成 `regex::Regex::new(...)` + `is_match`
6. `custom_validation_generates_function_call`：custom="fn" → 验证生成 `Form::fn(&value)` 调用
7. `multiple_rules_executed_in_order`：required + length + range → 验证多个规则按顺序生成
8. `validation_failure_skips_bump_version`：任一规则失败 → 验证 Err 分支不含 `__rml_bump_version`
9. `validation_success_clears_error`：校验通过 → 验证 `__rml_field_errors.insert(field, None)`
10. `no_validation_falls_back_to_default`：无 validation → 验证回退到原默认类型校验逻辑

**回归测试**：运行 `cargo test --workspace` 确保 244 个现有测试无回归。

#### Step H：Demo 集成

**文件**：`d:\GitCode\RF\rust-gpui-rml\demo\src\main_window.rml.rs`

**变更**：为 `age` 字段添加 range 校验：
```rust
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
    pub name: String,
    #[validate(range(min = 0, max = 150))]
    pub age: i32,
}
```

**验证**：`cargo run -p rust-rml-demo`，输入年龄 200 应显示红色边框 + tooltip"年龄必须在 0-150 之间"。

#### Step I：文档更新

**文件1**：`d:\GitCode\RF\rust-gpui-rml\docs\04-code-behind\macros.md`

**新增章节 4.2.X `#[validate]`**：
- 用途：声明式字段校验规则
- 规则列表（required/length/range/regex/custom/message）
- 使用示例
- 与 `#[command]` 协作说明
- 自定义校验函数签名要求

**文件2**：`d:\GitCode\RF\rust-gpui-rml\docs\03-binding\two-way-binding.md`

**新增章节 3.3.X "自定义校验规则"**：
- `#[validate]` 宏使用示例
- 规则执行顺序说明
- 自定义消息覆盖
- 与默认类型校验的关系

**文件3**：`d:\GitCode\RF\rust-gpui-rml\docs\10-advanced\performance.md`

**补充**：校验规则性能开销（regex 编译开销、自定义函数调用开销）。

## 假设与决策

### 假设

1. **`regex` crate 已可用**：若 `Cargo.lock` 已包含 regex 依赖（gpui-component 间接依赖），可直接使用；否则需在 engine/Cargo.toml 显式添加。
2. **scanner 可解析 `#[validate(...)]` 属性**：syn 2.x 的 `attr.parse_args::<T>()` 可解析任意属性参数。
3. **`ValidationRuleSet` 可跨 crate 共享**：类型定义放在 engine 的 build 模块中，macros 不依赖此类型（macros 仅剥离属性，不解析规则）。

### 决策

1. **宏不解析校验规则**：`#[validate]` 宏仅剥离属性，规则由 scanner 重新提取。理由：保持宏简单，与 `#[computed]` 的扫描模式一致。
2. **类型校验优先于规则校验**：parse 失败时直接设置类型错误消息，不执行用户规则。
3. **校验失败不赋值、不 bump_version**：保留原值，仅更新错误状态。与现有"parse 失败保留原值"策略一致。
4. **自定义消息全局覆盖**：`message = "..."` 应用于该字段所有失败分支，覆盖默认消息。
5. **`custom` 函数签名**：`fn(&str) -> Option<SharedString>`（接收字符串值，返回错误消息）。通过 `TypeName::fn_name` 调用（需 codegen 知道视图类型名，已有 `ctx.view_struct_name`）。
6. **regex 模式编译**：在 codegen 中生成 `regex::Regex::new(...).unwrap()`，每次校验时编译（性能开销可接受，因校验频率低；后续可优化为 `lazy_static!` 或 `once_cell::sync::Lazy`）。

## 验证步骤

### 阶段 1 验证

```powershell
# 文档收尾后无编译影响，仅检查链接和格式
cargo build -p rust-rml-engine
cargo test -p rust-rml-engine --test codegen_two_way_binding_test
```

### 阶段 2 验证

```powershell
# Step E 后：编译验证
cargo build -p rust-rml-engine

# Step G 后：新增测试 + 回归测试
cargo test -p rust-rml-engine --test codegen_validation_test
cargo test --workspace

# Step H 后：Demo 运行时验证（手动）
cargo run -p rust-rml-demo
# 输入年龄 200 → 应显示红色边框 + tooltip "年龄必须在 0-150 之间"
# 输入年龄 50 → 应正常显示，无错误
```

### 完成标志

- [ ] 阶段 1：3 个文档文件更新完成，无编号冲突
- [ ] 阶段 2 Step A-F：代码实现完成，编译通过
- [ ] 阶段 2 Step G：10 个新测试通过，244 个回归测试无失败
- [ ] 阶段 2 Step H：Demo 运行时校验 UI 正常
- [ ] 阶段 2 Step I：3 个文档文件更新完成
- [ ] 更新 `crates/macros/src/README.md`（如有）和 `crates/engine/src/README.md`（如有）

## 风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| `regex` crate 未在 workspace | 显式添加 `regex = "1"` 到 `crates/engine/Cargo.toml`，并 re-export |
| `custom` 函数路径解析复杂 | 限制为 `TypeName::fn_name` 形式（同模块静态方法），不支持完整路径 |
| scanner 解析 `#[validate]` 属性失败 | 提供 fallback：解析失败时跳过该校验规则 + 编译期警告（不阻塞编译） |
| `ValidationRuleSet` 类型跨 crate 复用 | 放在 `crates/engine/src/build/scanner.rs`，macros 不依赖此类型 |
| 校验规则组合爆炸 | 限制每次最多 5 个规则，超出报编译期错误 |

## 后续展望

- **Phase B-3.3 异步校验**：支持 `custom_async` 调用异步函数（如服务端校验用户名是否已存在）
- **Phase B-3.4 跨字段校验**：`#[validate(compare = "password_confirm", op = "eq")]` 支持字段间比较
- **Phase B-3.5 校验聚合**：`Form::validate() -> Result<(), Vec<ValidationError>>` 一次性校验所有字段
- **Phase C 路由/多窗口**：见 project_memory.md 中规划
