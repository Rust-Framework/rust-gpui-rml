# WPF 风格窗口与应用 API 规范化计划（最终收尾版）

> **本计划取代本文件之前的版本**，反映声明式根标签标准化后的当前状态。
>
> 用户的核心诉求（已在前一会话实现大部分）：
> 1. `RmlApplication.main_window` 必须是 `IWindow` 类型的组件（必须是窗口）
> 2. `RmlApplicationExt` 不应存在——`main_window` 是 `RmlApplication` 的内置功能
> 3. 抽象接口 `IComponent` 和 `IWindow`，参考 WPF/MAUI 设计理念
> 4. `IWindow` 自管理窗口通用操作（`open`/`show`/`close`/`state`），不通过扩展
> 5. `#[window]` 和 `#[component]` 不接受任何配置属性；RML 根节点必须是 `<window>`/`<modern_window>`/`<component>`；属性在声明式 `.rml` 中配置
> 6. Demo 改造成 WPF 新建项目模板风格

---

## 一、当前状态分析 Current State

### 已完成（前一会话成果，已通过代码审查确认）

| 范围 | 文件 | 状态 |
|------|------|------|
| Core traits | [component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/component.rs) | ✅ `IComponent: IViewModel`（`rml_template()` + `rml_tag()`） |
| Core traits | [window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/window.rs) | ✅ `IWindow: IComponent`，含 `open/show/close/hide/activate/state` 默认实现 |
| App | [application.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs) | ✅ `RmlApplication<W=NoWindow>` 类型状态模式，内置 `main_window::<W>()`，无 `RmlApplicationExt` |
| UI 内置窗口 | [builtin_window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/builtin_window.rs) | ✅ `Window`/`ModernWindow` 手动 `impl IWindow`，`open()` 调用 `crate::init(cx)` |
| 宏 | [macros/src/window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/window.rs) | ✅ `#[window]` 拒绝参数，添加 `__rml_window_handle` 字段，不再生成 `impl IWindow` |
| 宏 | [macros/src/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs) | ✅ `#[component]` 拒绝参数，模板路径固定 `<snake>.rml` |
| 引擎根标签 | [tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) | ✅ `RootTag` 枚举 + `is_root_tag()` + `root_tag_lookup()` |
| 引擎 codegen | [codegen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs) | ⚠️ 识别根标签 + `gen_window_impl()` + `gen_render_impl_from_children()`，但存在浮点格式化 bug |
| Demo | [main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main_window.rml) | ✅ `<window title="MainWindow" width="800" height="450">` 根 |
| Demo | [main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main_window.rml.rs) | ✅ `#[window] struct MainWindow`（无参数） |
| Demo | [main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs) | ✅ `main_window::<main_window::MainWindow>().run()` |

### 待修复问题（本计划核心）

#### 问题 1：浮点格式化 bug 阻塞 demo 构建

**位置**：[codegen.rs:107,111](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L107-L111)

**错误**：
```text
error[E0308]: mismatched types
  --> out/rml_generated/main_window.rs:10:18
   |
10 |         gpui::px(800)
   |         -------- ^^^ expected `f32`, found integer
```

**根因**：`gen_window_impl()` 中 `width`/`height` 为 `f32`（`"800".parse::<f32>().unwrap()` → `800.0`），但在 `format!()` 中用 `{}` 格式化时 Rust 输出 `"800"`（无小数点），生成的代码 `gpui::px(800)` 被解析为整数字面量，与 `gpui::px(f32)` 签名不匹配。

**修复**：将 `format!` 占位符 `{width}` 改为 `{:?}`，因为 `f32` 的 `Debug` 实现保证输出 `"800.0"` 形式（即包含小数点）。

```rust
// 修改前
gpui::px({width})

// 修改后
gpui::px({width:?})  // 输出 gpui::px(800.0)
```

#### 问题 2：宏 doc 注释过时

**位置**：[macros/src/lib.rs:36-39,47-64](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/lib.rs#L36-L64)

**问题**：`#[component]` 和 `#[window]` 的 doc 注释仍描述旧的参数语法：
- 第 36-39 行：`#[component]` 注释提到 `template = "path"` 参数
- 第 47-54 行：`#[window]` 注释列出 `title = "..."`、`width = N`、`height = N`、`template = "path"` 参数
- 第 57-64 行：示例 `#[window(title = "My App", width = 800, height = 600)]`（已无效，会编译错误）

**修复**：更新为声明式根标签设计：宏不接受参数，窗口/组件属性在 `.rml` 根节点上配置。

#### 问题 3：用户文档过时（12 个文件）

**位置**：`docs/**/*.md`

**搜索结果**（含 `#[view]`、`#[window(...)]`、`#[component(...)]`、`IRmlView`、`RmlApplicationExt`、`MainWindowBuilder` 的文件）：

| 文件 | 主要问题 |
|------|---------|
| `docs/04-code-behind/macros.md` | `#[window(template = "...")]`、`#[component(template = "...")]`、`#[derive(Model)]` |
| `docs/04-code-behind/viewmodel-structure.md` | `#[view]`、`#[component]` 旧参数语法 |
| `docs/06-components/custom-components.md` | `#[view]`、`IRmlView` |
| `docs/06-components/component-props.md` | `#[component]` 旧语法 |
| `docs/06-components/composition.md` | `#[view]` |
| `docs/06-components/slots.md` | `#[view]` |
| `docs/05-events/custom-events.md` | `#[view]` |
| `docs/03-binding/two-way-binding.md` | `#[view]` |
| `docs/08-lifecycle/on-loaded.md` | `#[view]` |
| `docs/08-lifecycle/lifecycle-overview.md` | `#[view]` |
| `docs/09-architecture/solid-principles.md` | `#[view]`、`IRmlView` |
| `docs/07-styling/style-reuse.md` | `#[view]` |

**统一替换规则**：
- `#[view]` → `#[component]`
- `#[window(...)]`（带参数）→ `#[window]`（裸宏）+ 在 `.rml` 根节点 `<window title="..." width="N" height="N">` 配置
- `#[component(...)]`（带参数）→ `#[component]`（裸宏）+ 在 `.rml` 根节点 `<component>` 内定义结构
- `IRmlView` → `IComponent`
- `#[derive(Model)]` → `#[derive(IModel)]`（在 `#[component]`/`#[window]` 之外的独立派生）
- `RmlApplicationExt`、`MainWindowBuilder` → 删除，改用 `RmlApplication::new().main_window::<W>().run()`

---

## 二、实施步骤 Implementation Steps

### Step 1：修复浮点格式化 bug

**文件**：`crates/engine/src/compiler/codegen.rs`

**修改**：`gen_window_impl()` 中 `format!` 占位符

```rust
// 修改前（约 107、111 行）
fn width(&self) -> gpui::Pixels {{
    gpui::px({width})
}}
fn height(&self) -> gpui::Pixels {{
    gpui::px({height})
}}

// 修改后
fn width(&self) -> gpui::Pixels {{
    gpui::px({width:?})
}}
fn height(&self) -> gpui::Pixels {{
    gpui::px({height:?})
}}
```

`format!` 调用处相应更新：`width = width` → `width = width`（变量名不变，由占位符 `{width:?}` 自动应用 `Debug` 格式）。

**验证**：
```bash
cargo build -p rust-rml-engine
cargo build -p rust-rml-demo
```

### Step 2：更新宏 doc 注释

**文件**：`crates/macros/src/lib.rs`

**修改 `#[component]` 注释（约 36-39 行）**：
```rust
/// 标记结构体为 RML 组件（Code-Behind ViewModel）。
///
/// 编译器会为该结构体生成 `Render` trait 实现。
///
/// **不接受任何属性参数**。模板路径固定为 `<snake_case>.rml`，
/// 对应的 `.rml` 根节点必须为 `<component>`。
///
/// 合并自旧 `#[view]` + `#[component]`。
```

**修改 `#[window]` 注释（约 47-64 行）**：
```rust
/// 标记结构体为窗口（顶层 OS 窗口）。
///
/// 在 `#[component]` 基础上额外生成窗口句柄字段（`__rml_window_handle`），
/// `IWindow` trait 实现由 RML 编译器从 `<window>` 根节点属性生成。
///
/// **不接受任何属性参数**。窗口配置（`title`/`width`/`height`）在 `.rml` 根节点上声明式配置：
/// ```text
/// <window title="..." width="N" height="N">...</window>
/// ```
///
/// # 示例
///
/// ```rust,ignore
/// #[window]
/// #[derive(Default)]
/// pub struct MainWindow {
///     pub count: i32,
/// }
/// ```
///
/// 对应 `main_window.rml`：
/// ```text
/// <window title="MainWindow" width="800" height="450">
///     <!-- 子元素 -->
/// </window>
/// ```
```

**验证**：`cargo build -p rust-rml-macros`

### Step 3：更新用户文档（12 个文件）

**范围**：`docs/` 下的 12 个 markdown 文件（见上表）

**统一替换策略**（按文件批量处理）：

1. **`docs/04-code-behind/macros.md`**（最关键，宏参考文档）：
   - 删除 `#[window]` 的"参数"小节（第 35-46 行）
   - 删除 `#[component]` 的参数示例
   - 添加新小节"声明式根节点配置"，说明 `<window>`/`<modern_window>`/`<component>` 根节点及属性配置
   - `#[derive(Model)]` → `#[derive(IModel)]`（或删除，因为 `#[component]`/`#[window]` 已自动生成 `IModel`）

2. **其余 11 个文件**：批量字符串替换
   - `#[view]` → `#[component]`（全局替换）
   - `#[window(title = "...", ...)]` → `#[window]`（删除参数）
   - `#[component(template = "...")]` → `#[component]`（删除参数）
   - `IRmlView` → `IComponent`
   - 删除 `RmlApplicationExt`、`MainWindowBuilder` 相关段落，替换为新 API 示例

**验证**：无编译影响（纯文档），仅人工审查格式正确性。

### Step 4：全量验证

```bash
# 各 crate 编译
cargo build -p rust-rml-core
cargo build -p rust-rml-macros
cargo build -p rust-rml-app
cargo build -p rust-rml-ui
cargo build -p rust-rml-engine
cargo build -p rust-rml-demo

# 全 workspace
cargo build --workspace

# 测试
cargo test -p rust-rml-engine    # 180+ 测试
cargo test -p rust-rml-core      # 24+ 测试

# 运行 demo
cargo run -p rust-rml-demo
```

**通过标准**：
- `cargo build --workspace` 无错误
- 所有测试通过
- demo 启动后显示 `MainWindow` 窗口（800×450，标题 "MainWindow"），点击按钮计数递增

---

## 三、假设与决策 Assumptions & Decisions

### 假设

1. **前一会话的实现是正确的**：核心 traits、`RmlApplication`、宏、根标签支持、demo 文件均通过代码审查，符合设计目标
2. **`f32` 的 `Debug` 格式化保证小数点**：`format!("{:?}", 800.0f32)` 输出 `"800.0"`，符合 Rust 语义
3. **engine 测试无需更新**：测试目录无 `.rml` 文件（180 个测试使用内联片段，不涉及根标签）
4. **docs 更新不影响编译**：纯文档，无 `rustdoc` 测试依赖旧语法

### 决策

1. **使用 `{:?}` 修复浮点格式化**：比 `{:.1}`（强制一位小数）更通用，`800.5` 不会被截断为 `800.5`（已是正确形式），`800.0` 输出 `"800.0"`
2. **宏 doc 注释与宏行为保持一致**：宏拒绝参数后，doc 注释必须同步更新，否则 `cargo doc` 误导用户
3. **docs 统一替换而非重写**：保留文档结构，仅修正过时语法引用；新增"声明式根节点配置"说明在 `macros.md` 中
4. **不更新 `.trae/documents/` 其他历史计划文档**：它们是历史归档，仅本文件作为当前执行计划
5. **不动 `crates/engine/tests/`**：该目录不存在 `.rml` 测试文件，180 个测试通过内联片段测试，根标签不影响

---

## 四、文件变更清单 File Change List

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/engine/src/compiler/codegen.rs` | 修改 | `gen_window_impl()` 浮点占位符 `{width}`/`{height}` → `{width:?}`/`{height:?}` |
| `crates/macros/src/lib.rs` | 修改 | `#[component]` 和 `#[window]` 的 doc 注释更新为声明式根节点设计 |
| `docs/04-code-behind/macros.md` | 修改 | 删除参数小节，新增声明式根节点说明 |
| `docs/04-code-behind/viewmodel-structure.md` | 修改 | `#[view]` → `#[component]`，移除旧参数语法 |
| `docs/06-components/custom-components.md` | 修改 | `#[view]` → `#[component]`，`IRmlView` → `IComponent` |
| `docs/06-components/component-props.md` | 修改 | `#[component(...)]` → `#[component]` |
| `docs/06-components/composition.md` | 修改 | `#[view]` → `#[component]` |
| `docs/06-components/slots.md` | 修改 | `#[view]` → `#[component]` |
| `docs/05-events/custom-events.md` | 修改 | `#[view]` → `#[component]` |
| `docs/03-binding/two-way-binding.md` | 修改 | `#[view]` → `#[component]` |
| `docs/08-lifecycle/on-loaded.md` | 修改 | `#[view]` → `#[component]` |
| `docs/08-lifecycle/lifecycle-overview.md` | 修改 | `#[view]` → `#[component]` |
| `docs/09-architecture/solid-principles.md` | 修改 | `#[view]` → `#[component]`，`IRmlView` → `IComponent` |
| `docs/07-styling/style-reuse.md` | 修改 | `#[view]` → `#[component]` |

**零影响范围**：
- `crates/core/**`、`crates/app/**`、`crates/ui/**`、`crates/engine/src/**`（除 `codegen.rs` 一处修复）、`demo/**`、`crates/engine/tests/`（不存在）

---

## 五、执行顺序 Execution Order

1. **Step 1**：修复 `codegen.rs` 浮点格式化 → `cargo build -p rust-rml-demo` 通过
2. **Step 2**：更新宏 doc 注释 → `cargo build -p rust-rml-macros` + `cargo doc -p rust-rml-macros` 通过
3. **Step 3**：批量更新 12 个 docs 文件
4. **Step 4**：全量验证 → `cargo build --workspace` + `cargo test --workspace` + `cargo run -p rust-rml-demo`

每步完成后立即验证，避免错误累积。

---

## 六、与之前版本的关系 Relationship to Previous Versions

本计划是 [`declarative-root-tag-standardization-plan.md`](./declarative-root-tag-standardization-plan.md)（已批准并执行）和本文件旧版本（描述带参数宏设计）的合并修订版：

| 维度 | 旧版本（本文件） | 声明式根标签版 | 本版本（最终） |
|------|---------------|--------------|--------------|
| `#[window]` 参数 | `title="...", width=N, height=N` | 无参数 | ✅ 无参数 + `<window>` 根配置 |
| `#[component]` 参数 | `template="..."` | 无参数 | ✅ 无参数 + `<component>` 根 |
| 窗口配置位置 | 宏参数 | `.rml` 根节点属性 | ✅ `.rml` 根节点属性 |
| `impl IWindow` 生成 | 宏生成 | 编译器从根节点生成 | ✅ 编译器从根节点生成 |
| `RmlApplicationExt` | 存在（扩展 trait） | 不存在 | ✅ 不存在（内置方法） |

**保持不变**：核心 trait 层次（`IWindow: IComponent: IViewModel: IModel + ILifecycle`）、`RmlApplication<W>` 类型状态模式、双入口模式（声明式 + 命令式）、引擎零影响原则。
