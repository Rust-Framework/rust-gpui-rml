# Phase B-3.2 校验体系收尾计划

## 摘要

Phase B-3.2 `#[validate]` 宏的代码实现（Steps A-F）已在上一会话完成。本计划覆盖剩余收尾工作：**运行测试验证** → **Demo 集成** → **文档更新**，完成整个数据校验体系架构。

## 当前状态分析

### 已完成（上一会话）

| 模块 | 文件 | 状态 |
|------|------|------|
| macros | `crates/macros/src/validate.rs` | ✅ `strip_internal_attributes` 实现 |
| macros | `crates/macros/src/lib.rs` | ✅ `#[derive(IModel)]` 声明 `attributes(element, validate)` |
| macros | `crates/macros/src/component.rs` + `window.rs` | ✅ 调用 `strip_internal_attributes` |
| engine | `crates/engine/src/compiler/mod.rs` | ✅ `ValidationRule`/`ValidationRuleSet` 类型定义 + `CodegenCtx.field_validations` 字段 |
| engine | `crates/engine/src/build/scanner.rs` | ✅ `ValidateArgs` 解析器（required/length/range/regex/custom/message） |
| engine | `crates/engine/src/build/mod.rs` | ✅ `field_validations` 传递给 `CodegenCtx` |
| engine | `crates/engine/src/compiler/codegen.rs` | ✅ `gen_field_assign_expr` + 数字/String 校验链生成函数 |
| engine | `crates/engine/src/lib.rs` | ✅ `pub use regex;` re-export |
| engine | `crates/engine/Cargo.toml` | ✅ `regex = "1"` 依赖 |
| tests | `crates/engine/tests/codegen_validation_test.rs` | ✅ 10 个测试已创建（**未运行**） |
| tests | `crates/engine/tests/codegen_two_way_binding_test.rs` | ✅ 添加 `field_validations: HashMap::new()` |

### 待完成

1. **运行测试验证**（Task #116）：运行 10 个新校验测试 + 全工作区回归
2. **Demo 集成**（Task #117）：在 `demo/src/main_window.rml.rs` 的 `age` 字段添加 `#[validate(range(min = 0, max = 150))]`
3. **文档更新**（Task #118）：补充宏 API 文档、双向绑定校验章节、性能开销

## 实现步骤

### Step 1: 运行校验测试（Task #116）

**目的**：验证 10 个新测试用例通过，确认 codegen 生成的校验代码符合契约。

```powershell
cargo test -p rust-rml-engine --test codegen_validation_test
```

**验证点**：
- `range_validation_generates_bounds_check` — range 条件 `v < 0 || v > 150` + 默认消息
- `range_validation_uses_custom_message` — 自定义消息覆盖
- `length_validation_generates_len_check` — `__rml_value.len() < 3 || __rml_value.len() > 20`
- `required_validation_generates_empty_check` — `__rml_value.is_empty()` + "此项为必填"
- `regex_validation_generates_pattern_match` — `rml::regex::Regex::new` + `.is_match`
- `custom_validation_generates_function_call` — `Self::validate_phone(&__rml_value)`
- `multiple_rules_executed_in_order` — required 在 length 之前
- `validation_failure_skips_bump_version` — 失败分支不含 bump_version
- `validation_success_clears_error` — 成功分支赋值 + 清除错误 + bump_version
- `no_validation_falls_back_to_default` — 无规则回退默认 parse 逻辑

**失败处理**：若测试失败，根据断言信息调整 codegen.rs 中的格式化字符串（缩进/占位符/消息文案）。

### Step 2: 全工作区回归测试

```powershell
cargo test --workspace
```

**验证点**：确认所有现有测试（219 个）无回归。

### Step 3: Demo 集成（Task #117）

**文件**：`demo/src/main_window.rml.rs`

**变更**：在 `age` 字段添加 `#[validate]` 属性，演示 range 校验。

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

**验证**：
```powershell
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```

确认：
- 输入 200 → 红色边框 + tooltip 显示"值必须在 0-150 之间"
- 输入 abc → 红色边框 + tooltip 显示"请输入有效的整数"
- 输入 25 → 边框恢复正常，值更新

### Step 4: 文档更新（Task #118）

#### 4.1 `docs/04-code-behind/macros.md` — 新增 `#[validate]` 宏章节

在 `#[command]` 章节后追加 `#[validate]` 章节，包含：
- 语法说明：`#[validate(required, length(min=N, max=M), range(min=N, max=M), regex="...", custom="fn", message="...")]`
- 规则表（required/length/range/regex/custom 各自适用的字段类型）
- 用户使用示例（Form 表单：name 用 required+length，age 用 range，email 用 regex）
- codegen 行为说明（校验失败不赋值、不 bump_version；成功清除错误 + bump_version）

#### 4.2 `docs/03-binding/two-way-binding.md` — 新增"自定义校验规则"章节

在现有"校验失败 UI 表现"章节后追加：
- `#[validate]` 属性声明方式
- 校验规则与字段类型匹配表
- 自定义校验函数签名：`fn(&str) -> Option<SharedString>`
- 错误消息覆盖：`message = "..."` 全局覆盖 vs 各规则默认消息
- 完整示例：Form 表单 + range/length/regex/custom 组合

#### 4.3 `docs/10-advanced/performance.md` — 补充校验规则性能开销

在校验状态查询表后追加"校验规则执行开销"表：
- required：O(n) 字符串 is_empty
- length：O(n) len() 比较
- range：O(1) 数值比较
- regex：O(n) 正则匹配（regex crate 编译期缓存）
- custom：取决于用户函数复杂度
- 整体：校验仅在反向绑定（用户输入）时执行，不影响正向渲染性能

## 假设与决策

1. **测试可能需要微调**：codegen 生成的代码缩进/格式可能与测试断言不完全匹配，运行后根据失败信息调整。
2. **Demo 不添加 name 字段校验**：保持 demo 简洁，仅 age 演示 range 校验。若用户需要更多演示可后续扩展。
3. **文档语言**：中文，与现有文档一致。
4. **不修改 codegen 核心逻辑**：除非测试失败暴露 bug，否则不改动 Steps A-F 已完成的实现。

## 验证步骤

```powershell
# Step 1: 校验测试
cargo test -p rust-rml-engine --test codegen_validation_test

# Step 2: 全工作区回归
cargo test --workspace

# Step 3: Demo 构建 + 运行
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```

## 完成标准

- [x] 10 个校验测试全部通过
- [x] 全工作区测试无回归（219+10=229 通过）
- [x] Demo 编译成功，`age` 字段 range 校验生效
- [x] 3 个文档更新完成（macros + binding + performance）
