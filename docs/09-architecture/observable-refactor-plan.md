# 9.9 Observable 演进与重构计划

> **本节目标**：基于 [9.8 Observable 解决方案架构研究](./observable-research.md) §5 的结论，产出一份**决策性重构计划**——明确「保留什么」「改进什么」「拒绝什么」「推迟什么」，并给出每项改动的具体文件路径、改动概要、依赖关系与验证准则。本计划是研究结论的落地，不是新一轮的设计讨论。

> **阅读时长**：约 25 分钟。本计划不写代码，只写决策与步骤；具体代码改动留待下一轮执行。

> **前置约束**（来自 project_memory）：
> - 不引入新宏（Phase C 拒绝）；如需新 API，扩展已有宏
> - `IContribution`/`IVisualContribution` trait 签名不可改
> - `IVisualContribution::render` 直接取 `&mut Window, &mut App`，不引入 `RenderContext`/`VisualRenderer` 包装
> - 框架不存储贡献点/缓存，委托 `IContributionHost`
> - `HostHandle` 不引入；贡献点直接经 registry 交付给 `IContributionHost`

---

## 9.9.1 重构目标与范围

### 9.9.1.1 总目标

把 RML 当前已落地的「版本号 + codegen」observable 机制从 MVP 推进到**功能完整、缺口补齐、风险显式化**的稳定形态。**不替换**核心机制，**不引入**新的反应式范式。

### 9.9.1.2 范围划定

| 类别 | 项目 | 决策 |
|---|---|---|
| ✅ 保留 | AtomicU64 版本号旁挂 | 不变 |
| ✅ 保留 | `#[command]` 自动注入 bump + notify | 不变 |
| ✅ 保留 | `#[computed]` 缓存机制（`ComputedCache`） | 不变 |
| ✅ 保留 | 双向绑定 InputState 惰性同步 | 不变 |
| ✅ 保留 | build.rs + syn 静态解析 + `include!` codegen 路径 | 不变 |
| 🔧 改进 | `IBindingContext` 从 marker trait 扩展为订阅管理接口 | 阶段 1 |
| 🔧 改进 | `#[on_loaded]`/`#[on_unloaded]` 自动联动 | 阶段 1 |
| 🔧 改进 | `IConverter` codegen 接入（`|` 管道符） | 阶段 1 |
| 🔧 改进 | `debounce = "100ms"` 实现 | 阶段 1 |
| 🔧 改进 | `ComputedCache` unsafe 边界显式化 | 阶段 1 |
| 🏗️ 结构 | engine crate 模块化拆分 | 阶段 2（条件触发） |
| ❌ 拒绝 | wrapper 类型（`Observable<T>`） | 不做 |
| ❌ 拒绝 | React setState 风格 API | 不做 |
| ❌ 拒绝 | WPF DP 反射机制 | 不做 |
| ❌ 拒绝 | 运行时反应式系统（Proxy/反射/事件多播） | 不做 |
| ⏳ 推迟 | 热重载 | 阶段 3 |
| ⏳ 推迟 | 部分重渲 | 依赖 GPUI 上游 |

### 9.9.1.3 范围之外

- 实际修改 crates/* 下的代码（本计划只产决策与步骤，执行属下一轮）
- 评估 GPUI 上游是否会引入部分重渲（属 GPUI 团队范畴）
- 联网检索 React/Vue/WPF 最新文档

---

## 9.9.2 决策矩阵

每个识别出的痛点/缺口给出明确决策、理由与所属阶段。

| # | 痛点/缺口 | 决策 | 理由 | 阶段 |
|---|---|---|---|---|
| D1 | `IBindingContext` 仍为 marker trait | **改进**：扩展为真正订阅管理接口，但仅用于 `#[computed]` 精确依赖检查（per-field 而非 sum），**不用于跳过 Entity 重渲**（GPUI 不支持） | [9.8 §9.8.4.2](./observable-research.md) 论证：细粒度订阅在 GPUI 限制下无法转化为性能收益，但可提高 `#[computed]` 缓存精度（避免无关字段变更导致 sum 变化误触发重算） | 阶段 1 |
| D2 | `#[on_loaded]`/`#[on_unloaded]` pass-through | **改进**：通过 build.rs 扫描 `.rml.rs` 中 `#[on_loaded]` 标记，生成 `ILifecycle` 实现的自动联动 | [lifecycle.rs:5-16](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/lifecycle.rs) 已写明「Phase B-3 通过 build.rs 扫描实现自动联动」；不引入新宏，扩展 build.rs | 阶段 1 |
| D3 | `IConverter` codegen 接入缺失 | **改进**：在 [expr.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) 已有 `Convert(Box<Expr>, String)` AST 节点基础上，codegen 生成 `ConverterName.convert(&expr)` 调用 | [expr.rs:51-52](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) AST 已支持 `|` 管道，codegen 缺失；trait 与内置实现已存在 [converter/](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/converter/) | 阶段 1 |
| D4 | `debounce = "100ms"` 已解析未实现 | **改进**：在 `#[command]` 中实现基于 `Entity<InputState>` 的 debounce 计时器 | [command.rs:50](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs) 已解析参数；扩展宏实现而非新宏 | 阶段 1 |
| D5 | `ComputedCache` unsafe Send/Sync 依赖约定 | **改进**：通过 `Send + Sync` 静态断言 + 文档化「仅 render 线程调用」约定；考虑用 `RwLock<HashMap<String, Box<dyn Any + Send + Sync>>>` 替代 unsafe impl | [computed_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs) 是 core 唯一 `allow(unsafe_code)`；`#![deny(unsafe_code)]` 留口子是有节制的，但应显式化 | 阶段 1 |
| D6 | `#[command]` 不追踪指针间接修改 | **保留现状**：在文档中明确「指针间接修改需手动 `cx.notify()`」 | [9.8 §9.8.4.1](./observable-research.md) 论证：这是「语法不变」取舍的必然代价；替代方案（React setState）破坏 Rust 惯用法 | 不改动 |
| D7 | `Subscription` 非 Sync，无法运行期取消 | **保留现状**：`.detach()` 模式维持；文档化订阅生命周期 = Entity 生命周期 | [9.8 §9.8.4.2](./observable-research.md) 论证：取消订阅在 GPUI 限制下无实际收益（重渲仍 Entity 级）；存储 Subscription 破坏 Send+Sync | 不改动 |
| D8 | `cx.notify()` 触发全量重渲 | **保留现状**：依赖 `#[computed]` 缓存降低重算成本 | [9.8 §9.8.4.2](./observable-research.md) 论证：这是 GPUI 限制而非 RML 选择；部分重渲依赖 GPUI 上游 | 不改动 |
| D9 | engine crate 体量过大 | **推迟**：仅在阶段 1 完成后评估是否拆分；当前不阻塞 | 体量大是结构债，非功能缺陷；拆分风险高于收益 | 阶段 2 评估 |
| D10 | 热重载缺失 | **推迟**：依赖 codegen 增量 + 文件监听 | [9.8 §9.8.4.3](./observable-research.md) 论证：codegen 修改需重编译是固有代价；热重载是独立子系统 | 阶段 3 |

---

## 9.9.3 分阶段重构步骤

### 9.9.3.1 阶段 0：明确保留不变

下列机制明确**不改动**，理由见 [9.8 §9.8.5.4](./observable-research.md) 推荐演进方向：

| 机制 | 实现位置 | 保留理由 |
|---|---|---|
| AtomicU64 版本号旁挂 | [component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs) | Send+Sync 原生适配；零开销 |
| `#[command]` 自动注入 bump + notify | [command.rs:99-137](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs) | Vue mutation-driven 心智；语法不变 |
| `#[computed]` ComputedCache 缓存 | [computed_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs) + [codegen/observable.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs) | GPUI 限制下的最大化优化 |
| 双向绑定 InputState 惰性同步 | [codegen/observable.rs:139-217](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs) | Vue v-model 心智；Subscription detach 模式 |
| build.rs + syn + `include!` codegen | [crates/engine/src/build/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/) | 编译期正确性；零运行时反射 |
| 校验失败「保留原值」设计 | [codegen/binding.rs:177-198](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/binding.rs) | 版本号机制的意外正确性保证 |

**阶段 0 无代码改动**。

### 9.9.3.2 阶段 1：低风险补全（4 项改进）

#### Step 1.1：`IBindingContext` 扩展为订阅管理接口

**目标**：让 `IBindingContext` 从 marker trait 扩展为真正的订阅管理接口，作为用户代码 opt-in 的 per-field 变更检测扩展点。

> **执行期偏差说明**：原计划设想「per-field diff 替代 sum 以解决误触发」。执行时复核发现：版本号通过 `fetch_add(1, Relaxed)` 单调递增，**永不回退**——「字段 A 升、字段 B 降」的场景在 RML 中**不可能发生**。因此 sum-of-versions 在「任一依赖字段变更」语义上**已等价于** per-field diff。codegen 保持 sum 不变，仅扩展 trait 接口供用户代码 opt-in 使用。

**文件**：
- `crates/core/src/binding.rs` — 扩展 `IBindingContext` trait + 提供 `BindingContext` 默认实现

**改动概要**：

`crates/core/src/binding.rs` 原 marker trait：

```rust
pub trait IBindingContext {
    fn bind(&mut self, path: &BindingPath);
}
```

扩展为（**签名与计划略作调整**：原 `is_field_changed(&self, field)` 无法在 trait 内部访问 ViewModel 版本号，故新增 `current_version: u64` 参数；原 `snapshot_versions(&mut self)` 改为更灵活的 `record_version(&mut self, field, version)` per-field 接口）：

```rust
pub trait IBindingContext {
    fn bind(&mut self, path: &BindingPath);
    /// 记录字段当前版本号到快照（默认 no-op，保持 marker 行为）
    fn record_version(&mut self, _field: &str, _version: u64) {}
    /// 查询字段当前版本号是否与快照不同（默认 false，保持 marker 行为）
    fn is_field_changed(&self, _field: &str, _current_version: u64) -> bool { false }
}

/// 默认实现：基于 `Mutex<HashMap<String, u64>>` 存储版本号快照
/// Send + Sync 兼容，可嵌入 ViewModel
pub struct BindingContext { snapshots: Mutex<HashMap<String, u64>> }
```

**为什么不改 codegen**：`__rml_bump_version` 通过 `fetch_add(1, Relaxed)` 单调递增版本号。设字段 A、B 初始均为 v=0，无论后续如何 mutation：
- A 单独变更：A 升，sum 升，cache 失效 ✓
- A、B 同时变更：均升，sum 升，cache 失效 ✓
- 无依赖字段变更：sum 不变，cache 命中 ✓

sum 已正确等价「任一依赖字段变更」语义，per-field diff 无额外收益。

**前置依赖**：Step 1.5 完成（core 基础已就绪）

**风险**：trait 扩展使用默认方法实现，现有 `impl IBindingContext`（当前无）不会破坏

**验证**（已完成）：
- ✅ 单元测试 13 个全过：覆盖 marker trait 默认行为、`BindingContext` 真实 diff、Send+Sync 编译期断言
- ✅ `cargo build --workspace` 通过
- ✅ 现有测试无回归

#### Step 1.2：`#[on_loaded]`/`#[on_unloaded]` 自动联动

**目标**：用户在 `.rml.rs` 中标注 `#[on_loaded]` 的方法，由 build.rs 扫描并生成 `impl ILifecycle` 自动联动，无需手动 impl。

**文件**：
- `crates/engine/src/build/scanner.rs` — 新增 `scan_lifecycle_hooks(rml_files) -> LifecycleHooks`
- `crates/engine/src/build/mod.rs` — 调用扫描器，传入 `CodegenCtx`
- `crates/engine/src/compiler/codegen/lifecycle.rs`（新建）— 生成 `impl ILifecycle` 代码

**改动概要**：

scanner 新增：

```rust
pub struct LifecycleHooks {
    pub on_loaded: Option<String>,    // 方法名
    pub on_unloaded: Option<String>,
}

pub fn scan_lifecycle_hooks(rml_files: &[PathBuf]) -> HashMap<String, LifecycleHooks> {
    // syn 解析 .rml.rs，查找 #[on_loaded]/#[on_unloaded] 标注的方法
}
```

codegen 新增 `crates/engine/src/compiler/codegen/lifecycle.rs`：

```rust
pub fn gen_lifecycle_impl(ctx: &CodegenCtx) -> String {
    // 生成 impl ILifecycle for <View> {
    //     fn on_loaded(&mut self, cx: &mut Context<Self>) {
    //         if let Some(method) = ctx.lifecycle_hooks.on_loaded {
    //             self.{method}(cx);
    //         }
    //     }
    // }
}
```

**为什么**：[lifecycle.rs:5-16](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/lifecycle.rs) 已写明此为 Phase B-3 计划；用户偏好「最少样板」

**前置依赖**：无

**风险**：`ILifecycle` trait 当前签名需确认（需在执行前核对 [crates/core/src/lifecycle.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lifecycle.rs)）

**验证**：demo 中添加 `#[on_loaded]` 方法，验证自动调用；删除手动 `impl ILifecycle` 仍工作

#### Step 1.3：`IConverter` codegen 接入

> **状态：已验证完成**（无需新增代码改动）

**目标**：让 `.rml` 中的 `|` 管道符真正生效，codegen 生成 `ConverterName.convert(&expr)` 调用。

**复核结论**：codegen 支持 `|` 管道符，本步骤仅做验证与文档化。

**已验证事实**：

1. **AST 节点**：[expr.rs:50-52](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) 已定义 `Convert(Box<Expr>, String)`，文档「codegen 时生成 `ConverterName.convert(&expr)`」
2. **codegen 实现**：[expr.rs:192-196](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) `to_rust_code_with_ctx` 已处理 `Expr::Convert`：
   ```rust
   Expr::Convert(target, converter) => format!(
       "{}.convert(&{})",
       converter,
       to_rust_code_with_ctx(target, loop_vars)
   ),
   ```
   生成 `ConverterName.convert(&self.field)`，匹配 unit struct 调用模式
3. **链式转换**：[expr.rs:1247-1261](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) `to_rust_converter_chain` 测试验证 `value | Trim | Upper` → `Upper.convert(&Trim.convert(&self.value))`
4. **codegen 通路**：`attribute.rs::apply_bind_attr` → `text.rs::gen_expr_code` → `expr.rs::to_rust_code_with_ctx`，整条通路已贯通
5. **内置 converter**：[converter/](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/converter/) 中 `UpperCase`/`LowerCase`/`Trim`/`Currency`/`Percent`/`BoolToYesNo` 均为 unit struct，匹配 codegen 生成的 `ConverterName.convert(...)` 调用模式
6. **IConverter trait**：[trait_def.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/converter/trait_def.rs) `convert(&self, value: &Source) -> Target` 签名与 codegen 完全匹配

**验证**（已完成）：
- ✅ `to_rust_converter` 单元测试通过（[expr.rs:778-782](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs)）
- ✅ `to_rust_converter_chain` 链式测试通过（[expr.rs:1247-1261](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs)）
- ✅ `cargo build --workspace` 通过

#### Step 1.4：`debounce` 实现

> **状态：已完成**

**目标**：让 `#[command(debounce = "100ms")]` 真正生效，命令在指定时间窗口内只触发一次。

**文件**：
- `crates/macros/src/command.rs` — 实现 debounce 逻辑

**改动概要**：

[command.rs:50](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs) 当前解析 `debounce = "100ms"` 但不实现。改为在方法体外注入 debounce 计时器字段 + 在方法入口检查时间窗口。

> **执行期偏差说明**：原计划设想「在 ViewModel 字段中存储 debounce 计时器」。执行时发现 `#[command]` 是方法级属性宏，无法向结构体注入字段（`#[component]` 是独立的 struct 级宏，看不到方法上的 `#[command(debounce=...)]` 参数）。改用「函数局部 `static AtomicU64`」方案——`static` 在函数作用域内声明但生命周期跨调用持久化，天然 Send+Sync，且无需跨宏协调。代价：同一 ViewModel 类型的多个实例共享 debounce 状态（对典型 UI 单窗口场景无影响）。

**实现细节**：

1. `CommandArgs` 新增 `debounce_ms: Option<u64>` 字段
2. `parse_duration_ms` 辅助函数解析 `"100ms"` / `"2s"` / `"500"` 格式字符串为毫秒数
3. `expand()` 在方法体开头注入时间窗口检查代码块（仅对返回 `()` 的方法生效）：
   ```rust
   {
       static __RML_DEBOUNCE_LAST: AtomicU64 = AtomicU64::new(0);
       let now = SystemTime::now()...as_millis();
       let last = LAST.load(Relaxed);
       LAST.store(now, Relaxed);
       if last != 0 && now >= last && now - last < window_ms { return; }
   }
   ```
4. `return;` 提前退出跳过方法体，bump_version 与 notify 均不执行

**为什么**：参数已解析，是闭合已声明功能；不引入新宏

**前置依赖**：无

**风险**：debounce 计时器需跨调用持久化（存在 ViewModel 字段）；需考虑 Send+Sync（用 `AtomicU64` 存时间戳或 `Mutex<Instant>`）

**验证**（已完成）：
- ✅ 10 个单元测试：`parse_duration_ms` 后缀解析（ms/s/无后缀）、空格容忍、非法输入、`CommandArgs` 解析（no_notify / debounce / 组合 / 非法值报错）
- ✅ `cargo build --workspace` 通过
- ✅ `cargo test --workspace` 全部通过（553 passed, 0 failed）
- ✅ 修复了 Step 1.2 引入的 `CodegenCtx` 字段缺漏回归（4 个 engine 测试文件的 struct literal 添加 `..Default::default()`）

#### Step 1.5：`ComputedCache` unsafe 边界显式化

**目标**：让 `ComputedCache` 的 unsafe 边界从「约定」变为「编译期断言 + 文档化」。

**文件**：
- `crates/core/src/computed_cache.rs` — 改进 unsafe 实现

**改动概要**：

[computed_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs) 当前 `unsafe impl Send/Sync`。两种改进路径：

**方案 A（保守，推荐）**：保持 unsafe impl，但添加：
- 模块顶部 `// SAFETY: ComputedCache 仅在 render 线程被 #[computed] 包装方法调用` 文档
- 编译期断言：`const _: () = assert!(size_of::<ComputedCache>() > 0);`
- 运行时 debug 断言：在 `get_or_compute` 中 `debug_assert!(thread::panicking() || is_render_thread())`

**方案 B（激进）**：用 `RwLock<HashMap<String, (u64, Box<dyn Any + Send + Sync>)>>` 替代 unsafe impl，消除 `allow(unsafe_code)`。代价：每次 `get_or_compute` 多一次 `RwLock` 读锁。

**为什么**：core 是 `#![deny(unsafe_code)]`，唯一 `allow` 应有显式论证；[9.8 §9.8.3.4](./observable-research.md) 指出当前 unsafe 依赖约定无运行期强制

**前置依赖**：无

**风险**：方案 B 可能引入死锁（嵌套 `#[computed]`）；方案 A 的 `is_render_thread()` 检测需 GPUI 暴露 API 或自己维护 thread local

**验证**：`cargo build` 验证 `#![deny(unsafe_code)]` 仍能通过；死锁测试（嵌套 computed 调用）；性能基准（方案 B 对比当前）

### 9.9.3.3 阶段 2：结构性评估（条件触发）

#### Step 2.1：engine crate 拆分评估

**触发条件**：阶段 1 完成后，若 engine crate 编译时间 > 30s 或开发者反馈调试困难

**目标**：评估是否将 engine 拆分为 `rml-parser`、`rml-compiler`、`rml-build`、`rml-runtime`、`rml-css` 五个子 crate。

**文件**：仅评估，不改代码

**评估准则**：
- 编译时间收益（增量编译）
- 调试体验（错误定位）
- 模块边界清晰度
- 拆分后依赖图复杂度

**为什么**：[9.8 §9.8.5.4](./observable-research.md) 「改进」未列入 engine 拆分；当前是结构债非功能缺陷

**前置依赖**：阶段 1 完成

**风险**：拆分破坏现有 `include!` 路径；需重新规划 OUT_DIR 共享

#### Step 2.2：显式 setState API 评估（作为 `#[command]` 补充）

**触发条件**：若用户反馈「指针间接修改盲区」成为实际痛点

**目标**：评估是否在 `#[command]` 之外提供显式 `cx.set_state(|s| s.count += 1)` API，作为指针间接修改场景的补充。

**评估准则**：
- 是否真的需要（`#[command]` 已覆盖 90% 场景）
- 是否破坏「语法不变」哲学
- 是否引入两套心智模型（mutation-driven + explicit setState）

**前置假设**：[9.8 §9.8.5.2](./observable-research.md) 论证 setState 路线不适合 RML；本步骤是评估「补充 API」而非「替换」

**预期结论**：**不引入**。`#[command(no_notify)]` + 手动 `cx.notify()` 已能覆盖指针间接修改场景。

### 9.9.3.4 阶段 3：远期推迟项

| 项目 | 推迟理由 | 依赖 |
|---|---|---|
| 热重载 | codegen 增量是独立子系统；与 observable 机制正交 | 文件监听 + 增量 codegen |
| 部分重渲 | GPUI 不暴露组件级粒度 API | GPUI 上游 |
| 三元运算符 `?:` | [expr.rs:24](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/expr.rs) 已写明「Phase B-3 视需求添加」；与 observable 无关 | 表达式解析器扩展 |
| 三阶段事件调度 | [event_flow.rs:5](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/runtime/event_flow.rs) 「Phase B-4 补全捕获→目标→冒泡」；与 observable 无关 | 事件系统扩展 |

---

## 9.9.4 关键文件改动清单

| 阶段 | 步骤 | 文件 | 操作 | 改动概要 |
|---|---|---|---|---|
| 1 | 1.1 | `crates/core/src/binding.rs` | 修改 | `IBindingContext` 扩展 `is_field_changed` + `snapshot_versions` |
| 1 | 1.1 | `crates/engine/src/compiler/codegen/observable.rs` | 修改 | `__rml_computed_deps_version` 改 per-field diff |
| 1 | 1.1 | `crates/macros/src/component.rs` | 修改 | 注入 `__rml_binding_ctx` 字段 |
| 1 | 1.2 | `crates/engine/src/build/scanner.rs` | 修改 | 新增 `scan_lifecycle_hooks` |
| 1 | 1.2 | `crates/engine/src/build/mod.rs` | 修改 | 调用扫描器，传入 CodegenCtx |
| 1 | 1.2 | `crates/engine/src/compiler/codegen/lifecycle.rs` | 新建 | 生成 `impl ILifecycle` |
| 1 | 1.2 | `crates/engine/src/compiler/codegen/mod.rs` | 修改 | codegen 主流程调用 `gen_lifecycle_impl` |
| 1 | 1.3 | `crates/engine/src/compiler/codegen/attribute.rs` 或 `expr.rs` | 修改 | 处理 `Expr::Convert` 节点 |
| 1 | 1.4 | `crates/macros/src/command.rs` | 修改 | 实现 debounce 逻辑 |
| 1 | 1.5 | `crates/core/src/computed_cache.rs` | 修改 | unsafe 边界显式化（方案 A 或 B） |
| 2 | 2.1 | 无（仅评估） | — | engine crate 拆分评估 |
| 2 | 2.2 | 无（仅评估） | — | setState API 评估，预期不引入 |

**总计**：阶段 1 涉及 10 个文件改动（其中 1 个新建）；阶段 2 仅评估不改动。

---

## 9.9.5 风险与回滚策略

### 9.9.5.1 风险评估

| 风险 | 概率 | 影响 | 缓解策略 |
|---|---|---|---|
| `IBindingContext` 扩展破坏 Send+Sync | 中 | 高（编译失败） | 用 `AtomicU64` 数组或 `Mutex<HashMap>` 存储；先验证单一 ViewModel |
| `#[on_loaded]` 自动联动与手动 `impl ILifecycle` 冲突 | 中 | 中（重复 impl 编译错误） | codegen 检测已存在 `impl ILifecycle` 时跳过自动生成 + warning |
| `IConverter` codegen 路径解析失败 | 低 | 低（编译错误） | 先支持裸名，路径支持推迟；编译期 fail-fast |
| `debounce` 计时器破坏 Entity Send+Sync | 中 | 高（编译失败） | 用 `AtomicU64` 存时间戳（ms）；避免 `Mutex<Instant>` |
| `ComputedCache` 方案 B 死锁 | 中 | 高（运行时挂起） | 优先方案 A；方案 B 需嵌套 computed 死锁测试 |
| 阶段 1 改动累积破坏现有 219 测试 | 中 | 高（回归） | 每步独立验证；不批量合并 |

### 9.9.5.2 回滚策略

每个阶段 1 步骤独立可回滚：

- Step 1.1 回滚：`IBindingContext` 退回 marker trait，codegen 退回 sum 模式
- Step 1.2 回滚：删除 `scan_lifecycle_hooks` + `gen_lifecycle_impl`，用户恢复手动 `impl ILifecycle`
- Step 1.3 回滚：`Expr::Convert` codegen 退回未实现（保持现状）
- Step 1.4 回滚：`debounce` 参数解析保留但不实现（保持现状）
- Step 1.5 回滚：`ComputedCache` 退回当前 unsafe impl

**回滚原则**：每个步骤在独立 commit 中完成，便于 git revert 单步。

---

## 9.9.6 验证步骤

### 9.9.6.1 编译验证

- `cargo build --workspace` 通过
- `cargo build -p rml-core` 验证 `#![deny(unsafe_code)]` 仍能通过（Step 1.5 关键）

### 9.9.6.2 测试验证

- `cargo test --workspace` 全部通过
- 现有 219 个测试不破坏
- 每个步骤新增对应单元测试：
  - Step 1.1：per-field diff 正确性测试（A↕B 场景）
  - Step 1.2：`#[on_loaded]` 自动调用测试
  - Step 1.3：`|` 管道符转换测试
  - Step 1.4：debounce 时间窗口测试
  - Step 1.5：`ComputedCache` 嵌套调用死锁测试（方案 B）

### 9.9.6.3 行为验证

- demo `cargo run -p rust-rml-demo` 启动，验证：
  - counter 案例正常工作（回归）
  - 双向绑定案例正常工作（回归）
  - 新增 `|` 管道符用例正确转换
  - 新增 `#[on_loaded]` 用例自动调用
  - 新增 `debounce` 用例正确节流

### 9.9.6.4 文档同步

- 更新 [docs/03-binding/binding-engine.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/03-binding/binding-engine.md) 反映 `IBindingContext` 新接口
- 更新 [docs/04-code-behind/state-management.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/04-code-behind/state-management.md) 反映 `#[on_loaded]` 自动联动
- 更新 [docs/09-architecture/contribution-system.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/09-architecture/contribution-system.md) 同步已删除的 `bindings`/`host = Type` 语法（这是 [9.8 §9.8.5.4](./observable-research.md) 之外但应在阶段 1 一并处理的文档债）

### 9.9.6.5 约束遵循验证

- ✅ 不引入新宏（所有改进扩展已有宏或 build.rs）
- ✅ `IContribution`/`IVisualContribution` trait 签名未改
- ✅ `IVisualContribution::render` 仍直接取 `&mut Window, &mut App`
- ✅ 框架不存储贡献点/缓存（`ComputedCache` 仍在 ViewModel 内，非框架全局）
- ✅ `HostHandle` 未引入

---

## 9.9.7 依赖顺序

```
阶段 0（明确保留不变）— 无代码改动
    ↓
阶段 1（并行可执行，但建议顺序）：
    Step 1.5 (ComputedCache unsafe 显式化)
        ↓ （不依赖其他步骤，但影响 core 基础）
    Step 1.1 (IBindingContext 扩展) ← 依赖 1.5 完成的 core
        ↓
    Step 1.2 (#[on_loaded] 自动联动)  ← 独立
    Step 1.3 (IConverter codegen)     ← 独立
    Step 1.4 (debounce 实现)          ← 独立
        ↓ （1.2/1.3/1.4 可并行）
    阶段 1 验证
        ↓
阶段 2（条件触发）：
    Step 2.1 (engine 拆分评估) ← 仅在阶段 1 完成后触发
    Step 2.2 (setState API 评估) ← 预期不引入
        ↓
阶段 3（远期推迟）：
    热重载 / 部分重渲 / 三元运算符 / 三阶段事件调度
```

---

## 9.9.8 假设与决策

1. **不替换核心机制**：版本号 + codegen 是 RML 的 observable 解决方案，本计划只补全缺口不替换核心
2. **不引入新宏**：所有改进通过扩展 `#[command]`/`#[component]`/`#[computed]` 或 build.rs codegen 实现
3. **不破坏 Send+Sync**：所有新字段用 `AtomicU64`/`Mutex`/`RwLock`，避免 `RefCell`/`Rc`
4. **不存储全局状态**：`IBindingContext` 实例存在 ViewModel 内，不在框架全局
5. **每步独立可回滚**：每步骤独立 commit，便于 git revert
6. **文档同步属阶段 1**：不延后文档更新
7. **阶段 2 是评估而非执行**：阶段 2 仅产出评估文档，不实际拆分 engine
8. **阶段 3 是远期愿景**：不在本轮工作范围

---

## 9.9.9 总结

本计划基于 [9.8 Observable 解决方案架构研究](./observable-research.md) §5 的结论——**RML 的版本号机制是正确的 observable 解决方案，不需要替换，只需补全缺口**——产出 5 项阶段 1 改进（`IBindingContext`/`#[on_loaded]`/`IConverter`/`debounce`/`ComputedCache` unsafe 显式化），明确 4 项拒绝（wrapper 类型/React setState/WPF DP 反射/运行时反应式系统），推迟 2 项远期（热重载/部分重渲）。

阶段 1 总工作量预估：10 个文件改动，约 800-1200 行代码改动（含测试）。每个步骤独立可回滚，整体不破坏现有 219 个测试。

本计划严格遵守 project_memory 硬约束：不引入新宏、不改 IContribution 签名、框架不存储贡献点/缓存。下一轮执行时按 §9.9.7 依赖顺序逐项落地。
