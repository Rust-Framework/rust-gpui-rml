# WPF 风格窗口 API — 收尾完成计划

> 本计划承接 `wpf-style-window-api-final-implementation-plan.md`，聚焦剩余的 Phase 6（docs 文档批量更新）与 Phase 7（全量验证）。
>
> 用户核心要求（来自原始 `/plan` 指令）**已在前序会话中全部实现**：
> 1. `RmlApplication.main_window` 必须是 `IWindow` 类型的组件（必须是窗口）✅
> 2. `RmlApplicationExt` 不应存在——`main_window` 是 `RmlApplication` 的内置功能 ✅
> 3. 定义抽象接口 `IComponent` 和 `IWindow`，参考 WPF/MAUI 设计理念 ✅
> 4. `IWindow` 自管理窗口通用操作（`open`/`show`/`close`/`state`），不通过扩展 ✅
> 5. 充分发挥 Rust + GPUI + gpui-component 的优秀特性 ✅

---

## 一、当前状态分析 Current State

### 已完成（Phases 1-5）

| 文件 | 状态 | 关键内容 |
|------|------|---------|
| `crates/core/src/window.rs` | ✅ | `IWindow` trait，`close/show/hide/activate/state` 均为默认实现（基于 `handle()`）|
| `crates/core/src/component.rs` | ✅ | `IComponent` trait（合并自 `IRmlView`）|
| `crates/app/src/application.rs` | ✅ | `RmlApplication<W>` 类型状态，内置 `main_window::<W>()` |
| `crates/macros/src/window.rs` | ✅ | `#[window]` 宏精简，只生成核心方法 |
| `crates/ui/src/window/builtin_window.rs` | ✅ | 内置 `Window` / `ModernWindow` IWindow 实现 |
| `crates/app/src/lifecycle.rs` | ✅ | 源码注释已清理 |
| `crates/ui/src/window/types.rs` | ✅ | `<ModernWindow>` → `<ModernWindowShell>` |
| `crates/macros/Cargo.toml` | ✅ | description 已更新 |
| `crates/core/src/command.rs` | ✅ | 注释已更新 |
| 各 crate `README.md`（core/macros/app/ui/demo）| ✅ | 已同步新 API |

### 待完成

| 项目 | 范围 | 说明 |
|------|------|------|
| Phase 6 | `docs/**` | 14 个文件共 **58 处 `#[view]`** 需改为 `#[component]`；`docs/09-architecture/responsibility.md:229` 的 run-pattern 需更新 |
| Phase 7 | workspace | `cargo build --workspace` + `cargo test --workspace` + `cargo run -p rust-rml-demo` |

### 探索确认（2026-06-26）

通过 Grep 验证：
- `#[view]` 在 `docs/` 中分布：**58 处 / 14 文件**，全部为裸 `#[view]`（无 `#[view(...)]` 参数化形式）
- `IRmlView` / `rml_view` 在 `docs/` 中：**0 处**（已清理完毕）
- `RmlApplication::new().run::<MyViewModel>()` 在 `docs/` 中：**1 处**（`responsibility.md:229`）
- 所有 `#[view]` 标注的结构体（如 `SearchView`/`UserListView`/`DataView` 等）均**非窗口**（无 title/size 字段在 `#[view]` 标注上），故全部应转为 `#[component]`，无任何 `#[window]` 转换

---

## 二、实施步骤 Implementation Steps

### Phase 6: docs/** 批量更新

**目标**：将 `docs/` 下所有过时的 `#[view]` 引用替换为 `#[component]`，并修正 `responsibility.md` 中的 run-pattern。

#### 6.1 Batch A — `docs/05-events/`（3 文件，7 处）

| 文件 | 出现次数 |
|------|---------|
| `docs/05-events/event-objects.md` | 1 |
| `docs/05-events/debounce-throttle.md` | 4 |
| `docs/05-events/custom-events.md` | 2 |

**操作**：每个文件先 `Read` 再 `Edit`，使用 `replace_all=true` 将 `#[view]` → `#[component]`。

#### 6.2 Batch B — `docs/06-components/`（4 文件，11 处）

| 文件 | 出现次数 |
|------|---------|
| `docs/06-components/slots.md` | 1 |
| `docs/06-components/custom-components.md` | 2 |
| `docs/06-components/composition.md` | 5 |
| `docs/06-components/component-props.md` | 3 |

**操作**：同上，`replace_all` `#[view]` → `#[component]`。

#### 6.3 Batch C — `docs/07-styling/` + `docs/08-lifecycle/`（6 文件，31 处）

| 文件 | 出现次数 |
|------|---------|
| `docs/07-styling/theming.md` | 1 |
| `docs/08-lifecycle/lifecycle-overview.md` | 3 |
| `docs/08-lifecycle/on-loaded.md` | 8 |
| `docs/08-lifecycle/on-unloaded.md` | 6 |
| `docs/08-lifecycle/async-tasks.md` | 5 |
| `docs/08-lifecycle/resource-management.md` | 8 |

**操作**：同上，`replace_all` `#[view]` → `#[component]`。

#### 6.4 Batch D — `docs/09-architecture/responsibility.md`（1 文件，9 + 1 处）

**两步操作**：

1. `replace_all` `#[view]` → `#[component]`（处理 9 处 `#[view]`）
2. 定向 `Edit` 第 229 行 run-pattern：
   - old: `` - 通过 `RmlApplication::new().run::<MyViewModel>()` 启动根视图 ``
   - new: `` - 通过 `RmlApplication::new().main_window::<MyWindow>().run()` 启动根窗口 ``

#### 6.5 Phase 6 验证

执行两个 Grep 调用，遍历 `e:\GitCode\RF\rust-gpui-rml\docs`：
- `#\[view\]` → 期望返回 **0 文件**
- `IRmlView` → 期望返回 **0 文件**（回归校验）

---

### Phase 7: 全量验证

按顺序执行，任一步骤失败立即停止排查：

```bash
# 1. 工作区编译
cargo build --workspace

# 2. 工作区测试
cargo test --workspace

# 3. Demo 运行（手动验证窗口启动）
cargo run -p rust-rml-demo
```

**预期验证点**：
- `IWindow` trait 默认实现编译通过
- `#[window]` 宏精简后编译通过（依赖 trait 默认实现）
- 内置 `Window`/`ModernWindow` 编译通过
- `#[component]` 宏生成的代码与 RML 模板 `include!` 协同正常
- Demo 启动后显示 Counter 窗口，`+`/`-` 按钮可交互

---

## 三、假设与决策 Assumptions & Decisions

### 决策 1: 所有剩余 `#[view]` 均转为 `#[component]`

- **理由**：通过 Grep 校验，14 个文件中 58 处 `#[view]` 标注的结构体（`SearchView`/`UserListView`/`DataView` 等）均非窗口（无 title/width/height 字段在 `#[view]` 标注上）。根据原计划 Rule 1 默认规则，统一转为 `#[component]`。
- **影响**：无需逐个判断 component vs window，可用 `replace_all` 批量处理。

### 决策 2: `replace_all` 是安全的

- **理由**：`#[view]` 是唯一 token（裸形式），无 `#[view(...)]` 参数化形式会被误伤。
- **影响**：每个文件一次 `Edit` 即可完成替换。

### 决策 3: 不修改 `quick-start.md` 中的 `mod views;`

- **理由**：`mod views;` 是复数形式的 Rust 模块名（指向 `views/` 目录），与 Rule 3 的 `mod view`（单数）不同，属误报。
- **影响**：保持原样不动。

### 决策 4: `responsibility.md:229` 类型参数 `MyViewModel` → `MyWindow`

- **理由**：新 API 的入口点类型必须是窗口（`IWindow + Default`），类型参数语义已变。
- **影响**：仅此一处定向 Edit。

### 决策 5: 不更新 `.trae/documents/**` 历史计划文档

- **理由**：历史计划文档（如 `wpf-style-window-and-application-api-plan.md`、`wpf-style-window-api-refined-plan.md` 等）作为历史归档保留。本计划文件除外。
- **影响**：仅更新本计划文件，不动其他 `.trae/documents/*.md`。

---

## 四、执行顺序 Execution Order

1. **Phase 6 Batch A**（3 文件）：并行 `Read` → 并行 `Edit`（`replace_all`）
2. **Phase 6 Batch B**（4 文件）：并行 `Read` → 并行 `Edit`（`replace_all`）
3. **Phase 6 Batch C**（6 文件）：并行 `Read` → 并行 `Edit`（`replace_all`）
4. **Phase 6 Batch D**（1 文件）：`Read` → `Edit`（`replace_all`）→ `Edit`（定向 run-pattern）
5. **Phase 6 验证**：两个 Grep 调用，确认零残留
6. **Phase 7 编译验证**：`cargo build --workspace`
7. **Phase 7 测试验证**：`cargo test --workspace`
8. **Phase 7 运行验证**：`cargo run -p rust-rml-demo`

每批次完成后立即进入下一批次，无需中间验证（Phase 6 末尾统一 Grep 验证）。

---

## 五、验证清单 Verification Checklist

- [ ] Phase 6 Batch A: `docs/05-events/` 3 文件 `#[view]` → `#[component]`
- [ ] Phase 6 Batch B: `docs/06-components/` 4 文件 `#[view]` → `#[component]`
- [ ] Phase 6 Batch C: `docs/07-styling/` + `docs/08-lifecycle/` 6 文件 `#[view]` → `#[component]`
- [ ] Phase 6 Batch D: `docs/09-architecture/responsibility.md` `#[view]` → `#[component]` + run-pattern 更新
- [ ] Phase 6 验证: `docs/` 中 `#[view]` 与 `IRmlView` 均为 0 处
- [ ] Phase 7: `cargo build --workspace` 通过
- [ ] Phase 7: `cargo test --workspace` 通过
- [ ] Phase 7: `cargo run -p rust-rml-demo` 正常启动
