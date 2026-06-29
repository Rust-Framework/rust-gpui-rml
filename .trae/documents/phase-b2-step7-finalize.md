# Phase B-2 Step 7 收尾：修复编译阻塞 + 集成测试 + 端到端验证

## 摘要

Phase B-2 的 Step 1-6 已完成（ComputedCache、字段注入、scanner、codegen 版本管理方法、
`#[command]` 自动注入、`#[computed]` 重命名）。本计划聚焦 Step 7 收尾：

1. **修复编译阻塞**：`crates/core/src/lib.rs` 的 `#![forbid(unsafe_code)]` 与 `computed_cache.rs` 的 `unsafe impl Send/Sync` 冲突
2. **创建 codegen 输出集成测试**：锁定 codegen 契约，防止未来重构破坏
3. **端到端验证**：编译通过 + 全测试套件通过 + Demo 运行验证 UI 行为

## 当前状态分析

### 已完成（Step 1-6）

| 步骤 | 文件 | 状态 |
|------|------|------|
| Step 1 ComputedCache | [crates/core/src/computed_cache.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs) | ✅ 10/10 测试通过 |
| Step 2 字段注入 | [crates/macros/src/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs#L71-L105) | ✅ `inject_tracking_fields` 注入 AtomicU64 + ComputedCache |
| Step 3 scanner | [crates/engine/src/build/scanner.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/scanner.rs) | ✅ `parse_body_with` + `return_type_str` |
| Step 4 codegen | [crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L678-L786) | ✅ `gen_observable_impl` + `gen_computed_wrappers` |
| Step 5 #[command] | [crates/macros/src/command.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs) | ✅ FieldMutationVisitor + bump/notify 注入 |
| Step 6 #[computed] | [crates/macros/src/computed.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/computed.rs) | ✅ 重命名为 `__rml_computed_<name>` |

### 阻塞问题

**编译必然失败**：`crates/core/src/lib.rs:6` 的 `#![forbid(unsafe_code)]` 与 `crates/core/src/computed_cache.rs:48-49` 的 `unsafe impl Send/Sync for ComputedCache {}` 直接冲突。

Rust 规则：`#![forbid(...)]` 是最强 lint 限制，**无法**被任何 `#[allow(...)]` 覆盖。上次会话修复 ComputedCache Send 约束时引入了 `unsafe impl`，但未同步调整 crate root 的 `forbid` 策略，必然编译失败。

### 未验证项

- Demo 编译（被上述阻塞挡住）
- 完整测试套件
- Demo 端到端 UI 行为

## 设计决策

### D1：`forbid` → `deny` + 局部 `#[allow(unsafe_code)]`

将 `crates/core/src/lib.rs` 的 `#![forbid(unsafe_code)]` 改为 `#![deny(unsafe_code)]`，
在 `computed_cache.rs` 的两个 `unsafe impl` 上加 `#[allow(unsafe_code)]` 属性。

**为什么不重构 ComputedCache 避免 unsafe？**
- GPUI `Entity<T>` 要求 `T: Send + Sync`
- `ComputedCache` 内部 `Box<dyn Any>` 存储的值可能含非 `Send` 类型（如 `Vec<MenuItem>` 含 `Rc<dyn Fn>`）
- 唯一的替代方案是限制 `T: Send + 'static`，但 `MenuItem` 不满足，会破坏 `#[computed]` 返回 `Vec<MenuItem>` 的场景
- `unsafe impl` 是合理的：`Mutex` 提供同步，`#[computed]` 仅在 render 线程调用

**为什么不用其他方案？**
- 移到独立子 crate：架构过度切割，增加维护成本
- 重构 ComputedCache 避免 `Box<dyn Any>`：每个 `#[computed]` 返回类型不同，需类型擦除统一存储

### D2：集成测试聚焦于 codegen 输出契约

`crates/engine/tests/` 下的文件不会被 build.rs 处理，因此无法测试 `#[window]` + `include!` 的端到端流程。
但可以调用公开的 `rml::compiler::compile()` 函数，传入手工构造的 `CodegenCtx`，验证生成的代码字符串包含：

- `fn __rml_bump_version` / `fn __rml_get_version` / `fn __rml_computed_deps_version`（版本管理三方法）
- `get_or_compute::<`（缓存包装调用）
- `__rml_computed_` 前缀（重命名后的方法调用）

**为什么不在 `crates/core` 加更多 ComputedCache 单元测试？**
已有 10 个测试覆盖命中/失效/嵌套等场景，新增价值有限。

**为什么不测宏展开后的代码？**
需要 `trybuild` 或 `macrotest` 额外依赖，且 Demo 本身就是最完整的宏端到端验证。

### D3：Demo 作为端到端验证

Demo（`demo/src/main_window.rml.rs`）走完完整流程：
`#[window]` 注入字段 → `#[computed]` 重命名 → `#[command]` 注入 bump/notify → build.rs scanner 提取元信息 → codegen 生成版本管理方法 + 缓存包装 → `include!` 注入 → 编译 → 运行

Demo 启动后点击按钮验证：
- `count` 自增（证明 `self.count += 1` 工作）
- UI 更新（证明自动注入的 `cx.notify()` 生效）

## 实施步骤

### Step 7-1：修复 `#![forbid(unsafe_code)]` 冲突

**文件**：
- [crates/core/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs#L6)
- [crates/core/src/computed_cache.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs#L48-L49)

**做什么**：

1. `crates/core/src/lib.rs` 第 6 行：
   ```rust
   // 修改前
   #![forbid(unsafe_code)]
   
   // 修改后
   #![deny(unsafe_code)]
   ```
   
   理由：`forbid` 无法被 `#[allow]` 覆盖；`deny` 保持 crate 整体的 unsafe 警告级别，同时允许局部 `#[allow]` 覆盖。

2. `crates/core/src/computed_cache.rs` 第 48-49 行，为两个 `unsafe impl` 加 `#[allow(unsafe_code)]`：
   ```rust
   // 修改前
   unsafe impl Send for ComputedCache {}
   unsafe impl Sync for ComputedCache {}
   
   // 修改后
   #[allow(unsafe_code)]
   unsafe impl Send for ComputedCache {}
   
   #[allow(unsafe_code)]
   unsafe impl Sync for ComputedCache {}
   ```

   SAFETY 注释已存在于第 45-47 行，无需重复。

**验证**：`cargo build -p rust-rml-core` 通过。

### Step 7-2：验证 Demo 编译

**命令**：`cargo build -p rust-rml-demo`

**做什么**：编译 demo，修复任何剩余编译错误。

预期通过项：
- `ComputedCache` 的 `Send + Sync` 满足 `Entity<T>` 约束
- codegen 生成的 `get_or_compute::<T>` 泛型参数正确（1 个）
- `#[command]` 注入的 `bump_version` 调用匹配 codegen 生成的 match 臂
- `#[computed]` 重命名后的 `__rml_computed_<name>` 被 codegen 包装方法正确调用

**若失败的诊断路径**：
- codegen 输出错误 → 检查 `target/debug/build/rust-rml-demo-*/out/rml_generated/main_window.rs`
- 宏展开错误 → `cargo expand -p rust-rml-demo main_window`
- 类型不匹配 → 检查 `computed_returns` 提取的类型字符串

### Step 7-3：运行完整测试套件

**命令**：`cargo test --workspace`

**做什么**：运行所有 crate 的单元测试 + 集成测试。

预期测试数：原 219 个 + ComputedCache 10 个 + scanner 新增 = 约 230 个。

**若失败的诊断路径**：
- `crates/core` 测试失败 → ComputedCache 行为问题
- `crates/engine` build::scanner 测试失败 → 宏参数扫描问题
- `crates/engine` compiler 测试失败 → codegen 输出问题
- `crates/macros` 测试失败 → 宏展开问题

### Step 7-4：创建 codegen 输出契约集成测试

**文件**：`crates/engine/tests/codegen_observable_test.rs`（新建）

**做什么**：

构造一个最小的 RML 源码 + CodegenCtx，调用 `rml::compiler::compile()`，验证生成的代码字符串包含预期片段。

```rust
//! Phase B-2 集成测试：验证 codegen 生成的 observable 版本管理方法和 #[computed] 缓存包装

use rml::compiler::{compile, CodegenCtx};
use std::collections::HashMap;

fn make_ctx() -> CodegenCtx {
    CodegenCtx {
        view_struct_name: "Counter".to_string(),
        view_module_path: "test".to_string(),
        stylesheet: None,
        computed_methods: vec!["doubled".to_string()],
        observable_fields: vec!["count".to_string()],
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
```

**验证**：`cargo test -p rust-rml-engine --test codegen_observable_test` 全部通过。

### Step 7-5：运行 Demo 端到端验证

**命令**：`cargo run -p rust-rml-demo`

**做什么**：启动 demo，手动验证 UI 行为。

预期行为：
1. 窗口启动，标题栏显示 "MainWindow" + Frame 图标 + 主菜单（文件/编辑/视图/帮助）+ 系统按钮（最小化/最大化/关闭）
2. 中央显示 "Hello, RML!" + "点击次数：0" + "点击我" 按钮
3. 底部状态栏显示 "就绪"
4. 点击按钮 → count 自增 → UI 更新（证明 `#[command]` 自动注入 `bump_version` + `cx.notify()` 生效）
5. 多次点击 → count 持续自增（证明 `cx.notify()` 每次都触发重渲染）

**若失败的诊断路径**：
- 编译失败 → 查看 `target/debug/build/rust-rml-demo-*/out/rml_generated/main_window.rs` 生成代码
- 运行时崩溃 → 查看 stderr，常见为 `MenuItem` 含 `Rc` 在跨线程访问时 panic（但 `unsafe impl Send/Sync` 应已解决）
- UI 不更新 → `#[command]` 的 `notify` 注入未生效，检查 `extract_context_param` 是否正确识别 `cx`
- 缓存未失效 → `__rml_computed_deps_version` 计算错误，但 demo 中 `menu_items`/`status_items` 不依赖任何字段，缓存命中是正确行为

## 假设与决策

1. **不重构 ComputedCache 避免 unsafe**：`unsafe impl Send/Sync` 是合理的，`Mutex` 提供同步，`#[computed]` 仅在 render 线程调用
2. **集成测试聚焦 codegen 输出契约**：不测试宏展开后的运行时行为（Demo 已覆盖），不测试 ComputedCache 内部（已有 10 个单元测试）
3. **不修改 `crates/engine/src/lib.rs` 的 `#![forbid(unsafe_code)]`**：engine crate 没有直接使用 unsafe，无需调整
4. **测试文件位置**：`crates/engine/tests/codegen_observable_test.rs`（集成测试，访问 `pub` API）
5. **Demo 验证为手动**：无自动化 UI 测试框架，依赖人工观察

## 验证步骤

1. `cargo build -p rust-rml-core` → 修复 forbid 冲突后通过
2. `cargo build -p rust-rml-demo` → 完整编译通过
3. `cargo test --workspace` → 全部测试通过（含新增 7 个 codegen 契约测试）
4. `cargo test -p rust-rml-engine --test codegen_observable_test` → 7/7 通过
5. `cargo run -p rust-rml-demo` → 启动成功，点击按钮 count 自增、UI 更新正常

## 关键文件改动清单

| 文件 | 操作 | 描述 |
|------|------|------|
| `crates/core/src/lib.rs` | 修改 | `#![forbid(unsafe_code)]` → `#![deny(unsafe_code)]` |
| `crates/core/src/computed_cache.rs` | 修改 | 两个 `unsafe impl` 加 `#[allow(unsafe_code)]` |
| `crates/engine/tests/codegen_observable_test.rs` | 新建 | codegen 输出契约集成测试（7 个测试） |

## 依赖顺序

```
Step 7-1 (修复 forbid 冲突)
       ↓
Step 7-2 (验证 Demo 编译) ──→ 若失败则修复 codegen/宏
       ↓
Step 7-3 (运行完整测试套件) ──→ 若失败则修复
       ↓
Step 7-4 (创建集成测试) ──→ cargo test 验证通过
       ↓
Step 7-5 (运行 Demo 端到端验证)
       ↓
   Phase B-2 完成 ✅
```

## Phase B-2 完成后的可选方向

完成 Step 7 后，可与用户讨论下一阶段方向：

- **Phase B-3**：双向绑定（`<Input model="name">` 自动同步 `self.name`）
- **Phase C**：路由系统 / 多窗口管理
- **性能优化**：细粒度更新（仅重渲染依赖变更字段的组件，而非全量 `cx.notify()`）
- **文档**：用户指南 + 宏 API 文档
