# RML 框架五星迭代计划

> 视角：RML 架构师 × 框架设计者
> 主题：将「简洁易用 / 心智负担最低 / 高效发挥 Rust 优势 / 稳定」四维度从当前基线推向 5★
> 基线：v0.1.0，demo/ 41 case + shell + lsp 审查（2026-07-06）

---

## 一、Summary（摘要）

### 1.1 总目标

将 RML 框架在四个核心维度上全部推向 5★，使「纯 Rust 开发者无需理解框架内部即可写出正确的桌面应用」成为可验证的事实，而非宣传语。

### 1.2 四维度评分基线与五星目标

| 维度 | 当前基线 | 五星目标 | 主要责任域 |
|---|---|---|---|
| 简洁易用 | 4★ | 5★ | 模板代码消除、API 表面积收敛 |
| 心智负担最低 | 3★ | 5★ | 框架内部零泄漏、时序复杂性吸收 |
| 高效发挥 Rust 优势 | 5★ | 5★（保持） | 类型安全、零成本抽象、async |
| 稳定 | 4★ | 5★ | 响应式一致性、错误处理、无 panic 路径 |

### 1.3 非目标

- 不重写已有宏系统基础设施
- 不引入新的运行时（继续基于 GPUI）
- 不改变 `.rml` + `.rml.rs` + `build.rs` 三件套闭环
- 不追求向后兼容（RML 是新框架，无历史包袱）

---

## 二、Current State Analysis（现状分析）

### 2.1 已达成部分（保持优势）

- **三件套闭环**：[demo/src/main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs) 5 行 + [demo/src/app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs) 15 行 + [demo/build.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/build.rs) 12 行，启动链路极简
- **MVVM + Contribution 解耦**：[demo/src/cases/counter_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml.rs) 73 行完成完整 case
- **声明式绑定能力**：双向绑定 / 列表渲染 / 条件渲染 / 插槽 / 逃生舱全覆盖
- **响应式数据流**：`ObservableVec` + flume channel + `cx.spawn` + `#[computed]`
- **Demo 覆盖广度**：41 case 覆盖组件 / 指令 / 绑定 / 菜单 / LSP 真实集成

### 2.2 11 个根因（G1-G11）定位

#### 简洁易用 4★ → 5★ 差距

**根因 G1：IContribution impl 是纯样板。** 每个 case 必写 8 行 `impl IContribution for X { fn id() / fn name() }`，仅委托到 `Self::CONTRIBUTION_ID` 与 `t_static(...)`。`#[contribute]` 宏已持有 `label` 信息，理论上可自动生成。
- 锚点：[demo/src/cases/counter_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml.rs) 中 `impl IContribution for CounterCase` 块

**根因 G2：Input/CodeEditor 配置时序缺失。** on_loaded 阶段 `ref_entities` 未填充，`placeholder` / `default_value` 等 InputState builder 属性无法在声明式模板中设置，必须用 `ElementRef.with_mut` 命令式操作。`on_rendered` 钩子尚未实现（M5' 待办）。
- 锚点：[demo/src/cases/input_case.rml.rs#L42-L44](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml.rs#L42-L44) 注释 "应在首次 render 后通过 ElementRef.with_mut 设置"
- 锚点：[demo/src/cases/code_editor_case.rml.rs#L21-L23](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/code_editor_case.rml.rs#L21-L23)

**根因 G3：`<component content={...}>` 逃生舱被频繁使用。** `welcome_case.render_group`、`list_case.render_item`、`key_case.render_item`、`main_window.active_view/render_menu_bar/render_status_bar` 均通过命令式渲染绕过声明式能力。
- 锚点：[demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) 中 `active_view` / `render_menu_bar` / `render_status_bar` 方法

#### 心智负担最低 3★ → 5★ 差距

**根因 G4：`__rml_bump_version` 字段名字符串化。** `Vec<T>` 字段修改后必须手动 `self.__rml_bump_version("items")`，字段名以字符串字面量传递，重命名无编译期检查。`ObservableVec<T>` 自动 bump，但 `Vec<T>` 不自动 —— 不一致。
- 锚点：[demo/src/cases/welcome_case.rml.rs#L84-L85](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/welcome_case.rml.rs#L84-L85)
- 锚点：[demo/src/cases/list_case.rml.rs#L79](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/list_case.rml.rs#L79)
- 锚点：[demo/src/shell/main_window.rml.rs#L411](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L411)

**根因 G5：`cx.notify()` 调用时机不统一。** `counter_case::on_click` 不调用，`expression_case::on_increase_a` 调用，`icon_case::on_rotate_icon` 调用 —— 用户从代码无法推理规则。
- 锚点：[demo/src/cases/counter_case.rml.rs#L59](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml.rs#L59)（不调用 cx.notify）
- 锚点：[demo/src/cases/expression_case.rml.rs#L68](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/expression_case.rml.rs#L68)（调用 cx.notify）
- 锚点：[demo/src/cases/icon_case.rml.rs#L123](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/icon_case.rml.rs#L123)（调用 cx.notify）

**根因 G6：ability cast 注册样板。** `StatusReady` / `LspStatusItem` 等仅 `#[contribute]` 无 `#[component]` 的视觉贡献，需手写 `static XXX_REGISTERED: Once = Once::new();` + `ensure_xxx_registered()` + 在 MainWindow::on_loaded 手动调用。源于 Rust trait object 不可 upcast 的限制。
- 锚点：[demo/src/cases/status_bar_case.rml.rs#L141-L150](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml.rs#L141-L150)
- 锚点：[demo/src/lsp/lsp_status.rs#L72-L82](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_status.rs#L72-L82)

**根因 G7：re-entrancy 陷阱需用户手动 defer。** `welcome_case` 首次 render 时读取 MainWindow 会触发 re-entrant panic，用户必须知道用 `cx.defer_in` 绕开。这是 GPUI 内部时序复杂性直接泄漏。
- 锚点：[demo/src/cases/welcome_case.rml.rs#L46-L53](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/welcome_case.rml.rs#L46-L53)

**根因 G8：`__rml_state` 字段必须出现在用户 `Default` impl。** `MainWindow::default` 必须显式 `__rml_state: Default::default()`，宏注入字段未对用户隐藏。
- 锚点：[demo/src/shell/main_window.rml.rs#L86](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L86)

#### 稳定 4★ → 5★ 差距

**根因 G9：响应式刷新模型不一致。** G4 + G5 共同导致：相同字段变更可能触发不同刷新行为，用户难以预测。

**根因 G10：LSP 集成中存在 `unwrap()` 路径。** `code_editor_tab.rml.rs` 多处 `.unwrap_or_default()`，部分 `parse().unwrap()` 在异常输入下可能 panic。
- 锚点：[demo/src/lsp/code_editor_tab.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml.rs)（21KB，最大文件）

**根因 G11：错误处理不统一。** LSP 失败用 `log::warn!`，IO 失败用 `unwrap_or_default()`，配置失败用 `log::warn!` 优雅降级 —— 缺少统一错误传播策略。

#### 架构问题

**根因 G12：MainWindow 逐步变成 God Object。** 570 行，承担 7 个职责：ILifecycle（7 个 init_* 方法）/ IWorkbenchManager / IContributionHost / ViewModel 投影 / 视图构建 / 8 个 #[command] 处理器 / LSP 启动。
- 锚点：[demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)（570 行）

**根因 G13：ViewModel 三兄弟重复。** `CaseViewModel` / `MenuViewModel` / `StatusViewModel` 三个文件结构高度相似：`from_contribution` 过滤 slot → 提取元数据 → `build_*_view_models` 排序。
- 锚点：[demo/src/shell/case_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_view_model.rs)
- 锚点：[demo/src/shell/menu_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs)
- 锚点：[demo/src/shell/status_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/status_view_model.rs)

**根因 G14：命名与风格不一致。** `<Button>` / `<dropdown-menu>` / `<menu-item>` / `<Accordion>` 大小写混用；`primary=""` / `ghost=""` 布尔属性空字符串语法怪异。

---

## 三、五星目标定义（可验证标准）

每个维度的 5★ 必须是**可量化、可验证**的，而非主观判断。

### 3.1 简洁易用 5★

| 指标 | 5★ 标准 | 验证方法 |
|---|---|---|
| 简单 case 行数 | `.rml.rs` ≤ 50 行（含 imports） | counter/button/expression case 行数审计 |
| 中等 case 行数 | `.rml.rs` ≤ 100 行 | input/tree/tab_bar case 行数审计 |
| 复杂 case 行数 | `.rml.rs` ≤ 300 行 | LSP CodeEditorTab 行数审计 |
| 模板代码占比 | 每 case 必写样板 ≤ 5 行 | 统计 imports + 属性 + IContribution impl |
| 声明式覆盖率 | 90% UI 需求可用 `.rml` 表达，无 `<component content={...}>` 逃生舱 | case 中 `content={self.xxx(...)}` 出现频次 ≤ 10% |
| 最小可运行 demo | ≤ 30 行达成 hello world + 按钮 + 状态 | 新建最小项目验证 |

### 3.2 心智负担最低 5★

| 指标 | 5★ 标准 | 验证方法 |
|---|---|---|
| `__rml_*` 出现频次 | 用户代码 0 次 | grep `__rml_` 排除宏生成代码 |
| `cx.notify()` 手动调用 | 用户代码 0 次（除显式跨 entity 通知） | grep `cx.notify` 在 `demo/src/cases/` |
| `ensure_*_registered` 函数 | 用户代码 0 个 | grep `ensure_.*_registered` |
| `Once::new()` ability 注册 | 用户代码 0 处 | grep `register_visual_ability\|register_contribution_ability` 在 demo |
| `cx.defer_in` 手动调用 | 用户代码 0 次（框架自动处理） | grep `defer_in` 在 `demo/src/cases/` |
| 概念术语表 | 用户需学习的框架概念 ≤ 10 个 | 见附录 B |

### 3.3 高效发挥 Rust 优势 5★（保持）

| 指标 | 5★ 标准 | 验证方法 |
|---|---|---|
| 类型安全 | 所有绑定类型在编译期检查 | 编译期错误验证 |
| 零成本抽象 | 宏展开后无运行时反射 | 检查宏展开代码 |
| 所有权语义 | 共享状态用 `Arc<RwLock<T>>`，无 `Rc<RefCell>` | grep `Rc<RefCell>` |
| async/await | LSP/IO 异步任务用 `cx.spawn` | 既有标准保持 |

### 3.4 稳定 5★

| 指标 | 5★ 标准 | 验证方法 |
|---|---|---|
| 响应式一致性 | 相同字段变更触发相同刷新行为 | 见 §9.3 回归测试套件 |
| 无 panic 路径 | demo 全代码无 `unwrap()` / `expect()`（除 `parse().unwrap()` URI 等明显安全场景） | grep `unwrap()` 排除白名单 |
| 错误优雅降级 | LSP/IO 失败显示日志，不阻塞 UI | LSP 服务不可用时验证 |
| re-entrancy 安全 | 跨 entity 读取零 panic | 见 §9.2 验证 case |

---

## 四、迭代路线总览

按优先级分四阶段，每阶段有明确退出标准。沿用 `.trae/documents/` 主流的 M1/M2/M3 命名。

| 阶段 | 名称 | 周期 | 退出标准 |
|---|---|---|---|
| **M1** | 基础傻瓜化 | 4-6 周 | §3.1 前 3 项指标 + §3.2 前 3 项指标达成 |
| **M2** | 核心心智负担消除 | 6-8 周 | §3.2 全部指标 + §3.4 响应式一致性达成 |
| **M3** | 架构优化 | 4-6 周 | MainWindow 拆分 + 声明式覆盖率达成 |
| **M4+** | 精雕细琢 | 持续 | §3.4 全部指标 + 文档完整性 |

依赖关系：M1 与 M2 部分可并行（不同宏模块）；M3 依赖 M2 完成（拆分前需先消除内部泄漏）；M4+ 在 M3 后持续进行。

---

## 五、M1：基础傻瓜化

### 5.1 M1-1：`#[command]` 宏自动注入 `cx.notify()`

#### 目标

消除用户对「何时调用 `cx.notify()`」的思考。所有 `#[command]` 方法末尾自动注入 `cx.notify()`。

#### 软件工程原理

- **响应式系统的自动失效传播**：类比 SolidJS 的细粒度响应式，状态变更应自动触发依赖重算，无需用户手动通知。
- **迪米特法则（最小知识原则）**：用户不应知道框架刷新机制的存在。
- **不变式封装**：「状态变更 → 通知刷新」是框架不变式，应由框架强制保证。

#### 设计方案

宏在方法体首行注入 `NotifyGuard`，RAII 在 drop 时自动 `cx.notify()`，覆盖 early return 路径：

```rust
// 用户书写
#[command]
pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
}

// 宏展开后
pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    let _guard = rml_core::NotifyGuard::new(cx);
    self.count += 1;
    // guard drop 时自动 cx.notify()
}
```

`NotifyGuard` 实现：

```rust
pub struct NotifyGuard<'a, T> {
    cx: &'a mut Context<T>,
    armed: bool,
}

impl<'a, T> NotifyGuard<'a, T> {
    pub fn new(cx: &'a mut Context<T>) -> Self {
        Self { cx, armed: true }
    }
}

impl<'a, T> Drop for NotifyGuard<'a, T> {
    fn drop(&mut self) {
        if self.armed {
            self.cx.notify();
        }
    }
}
```

#### 实施步骤

1. 在 `crates/core` 中实现 `NotifyGuard<T>` 类型
2. 修改 `crates/macros` 中 `#[command]` 宏展开逻辑：
   - 解析方法签名，确认包含 `cx: &mut Context<Self>` 参数
   - 在方法体首行注入 `let _rml_notify_guard = rml_core::NotifyGuard::new(cx);`
3. 提供 `cx.notify()` 显式调用的 deprecation 警告（lint 级别）
4. 删除 demo 所有 case 中的显式 `cx.notify()` 调用
5. 验证所有 case 仍正常工作

#### 验证标准

- `grep -r "cx.notify()" demo/src/cases/` 返回 0 行（除跨 entity 通知）
- 所有 case 行为与改进前一致
- 新增 early return 路径的 case 不需要特殊处理

#### 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| 用户在 command 中调用异步任务，guard 提前 drop | 中 | 文档说明：异步任务用 `cx.spawn`，spawn 内部 update 闭包自动 notify |
| 性能影响（无变更仍 notify） | 低 | GPUI notify 是脏标记，无变更时重渲是空操作；可加 `dirty` 标志优化 |
| 与现有显式 notify 重复调用 | 低 | 重复 notify 幂等，无副作用 |

#### 影响范围

- [crates/core/](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/) 新增 `NotifyGuard` 类型
- [crates/macros/](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/) 修改 `#[command]` 宏
- [demo/src/cases/](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/) 删除显式 `cx.notify()`

---

### 5.2 M1-2：`#[contribute]` 宏自动生成 `IContribution` impl

#### 目标

消除每个 case 必写的 `impl IContribution for X { fn id() / fn name() }` 8 行样板。

#### 软件工程原理

- **DRY（Don't Repeat Yourself）**：`label` 信息已在 `#[contribute(label = "...")]` 中，不应在 impl 中重复。
- **Convention over Configuration**：默认行为由约定生成，仅在需要 override 时手写。
- **信息源唯一性**：标签字符串只在一个地方声明。

#### 设计方案

`#[contribute]` 宏读取 `label` 参数，自动生成 `IContribution` impl：

```rust
// 用户书写
#[contribute(
    host_id = "demo.shell",
    id = "binding.counter",
    kind = "case",
    group = "binding",
    order = 1,
    label = "case.counter.title"
)]
#[component]
#[derive(Default)]
pub struct CounterCase {
    pub count: i32,
}
// 不再手写 impl IContribution

// 宏展开后自动生成
impl IContribution for CounterCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.counter.title") }
}
```

**Override 机制**：用户可手写 `impl IContribution` 覆盖默认行为（如动态名称）。宏检测到用户已手写时跳过自动生成。

#### 实施步骤

1. 修改 `#[contribute]` 宏：
   - 解析 `label = "..."` 参数
   - 生成默认 `IContribution` impl
   - 检测是否已存在手写 impl（通过 syn 解析 AST）
2. 处理边缘情况：
   - `label` 缺失：编译期错误，提示必填
   - `label` 是字面量 vs 表达式：仅支持字面量字符串
   - 用户手写 impl：宏跳过自动生成
3. 删除 demo 所有 case 中的 `impl IContribution` 样板
4. 验证所有 case 仍正常工作

#### 验证标准

- `grep -A 5 "impl IContribution for" demo/src/cases/*.rml.rs` 返回 0 行（除非用户主动 override）
- 每 case 平均减少 8 行样板代码
- 简单 case `.rml.rs` 行数降至 50 行以内

#### 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| 用户需要动态 name（如带变量的标题） | 中 | 保留 override 机制：手写 impl 优先 |
| 宏展开冲突（auto impl 与手写 impl 共存） | 低 | syn 解析检测，已存在则跳过 |
| `label` 字符串与 i18n key 解耦不彻底 | 低 | 保持现状：label 即 i18n key |

#### 影响范围

- [crates/macros/](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/) 修改 `#[contribute]` 宏
- [demo/src/cases/](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/) 删除 `impl IContribution` 样板

---

### 5.3 M1-3：`<Input placeholder="..." />` codegen 透传到 InputState builder

#### 目标

让最常见的 Input 用法（`placeholder` / `default_value`）可声明式设置，消除「必须用 ElementRef.with_mut 配置」的命令式缺口。

#### 软件工程原理

- **声明式优先，命令式兜底**：能用模板表达的，不应强制命令式。
- **最小惊讶原则**：`<Input placeholder="姓名" />` 在所有 UI 框架中都是标准用法，用户期待它直接工作。
- **配置时序透明化**：用户不应感知「Entity 何时创建」的内部时序。

#### 设计方案

**方案 A（推荐）：编译期识别字符串属性，生成 builder 配置代码。**

Input compiler 识别 `placeholder` / `default-value` 等属性，在 `__rml_populate_refs` 注入 Entity 后立即调用 builder：

```rust
// 用户书写
<Input ref="input_state" placeholder="姓名" default-value="张三" />

// 宏展开后（伪代码）
let entity = cx.new(|cx| {
    let mut state = InputState::new(window, cx);
    state = state.placeholder("姓名".into());
    state = state.default_value("张三");
    state
});
self.input_state.set(entity);
```

**方案 B：增加 `on_rendered` 钩子。** 用户在 `on_rendered` 中通过 `ElementRef.with_mut` 配置。这是兜底方案，不是首选。

#### 实施步骤

1. 在 InputState builder API 上确认 `placeholder` / `default_value` 方法签名
2. 修改 Input compiler：
   - 识别白名单字符串属性（placeholder / default-value / disabled 等）
   - 在 Entity 创建闭包内生成 builder 调用链
3. 处理动态绑定：`placeholder={some_field}` 需生成 `set_placeholder` 调用
4. 扩展到 CodeEditor：`language` / `value` / `multi_line` 等属性同样透传
5. 重写 `input_case` / `code_editor_case` 验证声明式用法

#### 验证标准

- `<Input placeholder="姓名" />` 直接显示占位文本，无需任何 .rs 代码
- `<CodeEditor value={code} language="rust" />` 直接显示代码
- [demo/src/cases/input_case.rml.rs#L42-L44](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml.rs#L42-L44) 不再出现 "应在首次 render 后通过 ElementRef.with_mut 设置" 注释

#### 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| InputState builder API 不支持某些属性 | 中 | 优先支持 placeholder/default_value/disabled，其他保持命令式 |
| 动态绑定（`placeholder={field}`）需响应式更新 | 中 | 生成 `__rml_bind_placeholder` 监听 field 变化调用 set_placeholder |
| 与 `ref` 指令交互复杂 | 低 | ref 仍创建 Entity，属性透传在创建闭包内 |

#### 影响范围

- [crates/engine/src/compiler/input/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/input/) 识别字符串属性
- [crates/engine/src/compiler/code_editor/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/) 同上
- [demo/src/cases/input_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/input_case.rml.rs) 简化为纯声明式
- [demo/src/cases/code_editor_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/code_editor_case.rml.rs) 简化为纯声明式

---

## 六、M2：核心心智负担消除

### 6.1 M2-1：`#[observable]` 字段属性替代 `__rml_bump_version`

#### 目标

消除 `__rml_bump_version("field_name")` 字符串字面量调用，让字段变更自动触发版本递增 + 通知。

#### 软件工程原理

- **封装不变式**：「字段变更 → 版本递增 → 通知」是框架不变式，应由类型系统强制。
- **类型安全优先**：字段名应在编译期可知，避免字符串字面量重命名失效。
- **统一响应式模型**：`Vec<T>` 与 `ObservableVec<T>` 应有相同的变更通知语义，差异由宏生成代码吸收。

#### 设计方案

引入 `#[observable]` 字段属性，`#[component]` 宏识别后生成类型安全的 setter：

```rust
// 用户书写
#[component]
pub struct ListCase {
    #[observable]
    pub items: Vec<SharedString>,
}

// 用户操作（保持原语法）
self.items.push("New".into());
// 或
self.items = new_vec;

// 宏生成（隐式）
impl ListCase {
    pub fn set_items(&mut self, value: Vec<SharedString>, cx: &mut Context<Self>) {
        self.items = value;
        self.__rml_bump_version_internally("items");  // 内部 API，编译期字段名
        cx.notify();
    }
}
```

**对于 `Vec<T>`**：宏生成 `VecObservable<T>` 包装类型，提供 `push` / `pop` / `clear` / `set` 方法，每个方法内部 bump + notify：

```rust
// 用户书写
#[observable]
pub items: Vec<SharedString>,

// 宏生成（字段类型替换）
pub items: VecObservable<SharedString>,
```

用户代码 `self.items.push(x)` 直接生效，无需手动 bump。

**对于简单字段（i32 / String / bool）**：宏生成 setter，赋值时自动 bump + notify。用户直接 `self.count += 1` 通过运算符重载或 Deref 透明生效。

#### 实施步骤

1. 设计 `#[observable]` 字段属性语法
2. 在 `crates/core` 中实现 `VecObservable<T>` / `Observable<T>` 包装类型
3. 修改 `#[component]` 宏：
   - 识别 `#[observable]` 字段
   - 替换字段类型为 Observable 包装
   - 生成 setter 方法
4. 处理 `#[derive(Default)]` 兼容性：Observable 包装需实现 Default
5. 删除 demo 中所有 `__rml_bump_version` 调用
6. 验证响应式行为一致

#### 验证标准

- `grep -r "__rml_bump_version" demo/` 返回 0 行
- 字段重命名触发编译期错误（而非运行时静默失效）
- `Vec<T>` 与 `ObservableVec<T>` 行为统一

#### 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| Observable 包装破坏 `&self.field` 借用模式 | 高 | 实现 `Deref<Target=T>`，对外透明 |
| 用户直接 `self.items = new_vec` 绕过 setter | 中 | 字段仍为 pub，但赋值通过 `Deref` 不触发；需 `set_items` 方法引导 |
| 性能开销（Observable 内部 RwLock） | 低 | Observable 内部用 AtomicUsize 版本号，无需 RwLock |
| 与第三方库交互（如 `Vec::new()` 期望） | 中 | 提供 `Observable::new()` 与 `From<Vec<T>>` |

#### 影响范围

- [crates/core/](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/) 新增 `Observable<T>` / `VecObservable<T>` 类型
- [crates/macros/](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/) 修改 `#[component]` 宏识别 `#[observable]`
- [demo/src/cases/welcome_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/welcome_case.rml.rs) 删除 `__rml_bump_version` 调用
- [demo/src/cases/list_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/list_case.rml.rs) 同上
- [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) 同上

---

### 6.2 M2-2：视觉 ability 自动注册

#### 目标

消除 `ensure_*_registered` 函数 + `Once::new()` 静态变量 + MainWindow 手动调用的样板。

#### 软件工程原理

- **依赖注入自动化**：能力注册是框架职责，不应由用户手动触发。
- **生命周期透明**：contribution 注册时机由框架决定，用户不感知。
- **迪米特法则**：用户不应知道「ability cast」机制的存在。

#### 设计方案

`#[contribute]` 宏在生成 registration 代码时，自动调用 `register_visual_ability::<T>()` / `register_command_ability::<T>()`：

```rust
// 用户书写
#[contribute(host_id = "demo.shell", id = "status.ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;

impl IVisual for StatusReady {
    fn render(&self, _w: &mut Window, _cx: &mut App) -> AnyElement { /* ... */ }
}

// 宏展开后（隐式生成）
#[ctor::ctor]
fn _register_status_ready_abilities() {
    register_contribution_ability::<StatusReady>();
    register_visual_ability::<StatusReady>();  // 检测到 IVisual impl 自动注册
}
```

宏通过 syn 解析检测是否 impl `IVisual` / `ICommand`，自动注册对应 ability。

#### 实施步骤

1. 修改 `#[contribute]` 宏：
   - 解析模块内 `impl IVisual for X` / `impl ICommand for X` 语句
   - 生成 `#[ctor::ctor]` 函数调用对应 register
2. 删除 demo 中所有 `ensure_*_registered` 函数 + `Once::new()` 静态变量
3. 删除 MainWindow::on_loaded 中的手动调用
4. 验证 ability 注册时序正确（在 contribution bootstrap 前）

#### 验证标准

- `grep -r "ensure_.*_registered" demo/` 返回 0 行
- `grep -r "register_visual_ability\|register_contribution_ability" demo/` 返回 0 行
- 状态栏 / 菜单贡献正常工作

#### 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| `#[ctor::ctor]` 注册时序早于 contribution host 创建 | 中 | register 仅填充全局表，bootstrap 时查询；时序无关 |
| 模块未导入导致 ctor 不执行 | 高 | 文档说明：contribution 模块必须在 `mod.rs` 中显式声明 |
| 宏检测 `impl IVisual` 误判（如泛型 impl） | 低 | 限定为 `impl IVisual for X` 精确匹配 |

#### 影响范围

- [crates/macros/](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/) 修改 `#[contribute]` 宏
- [demo/src/cases/status_bar_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml.rs) 删除 `ensure_status_ready_registered`
- [demo/src/lsp/lsp_status.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/lsp_status.rs) 删除 `ensure_lsp_status_item_registered`
- [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) 删除 `init_contribution_host` 中手动调用
- [demo/src/shell/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) 删除 `register_workbench_abilities` 函数

---

### 6.3 M2-3：`on_loaded_deferred` 钩子 + re-entrancy 自动吸收

#### 目标

让用户无需理解 GPUI re-entrancy 规则即可写出正确的跨 entity 读取代码。

#### 软件工程原理

- **框架吸收复杂性**：底层运行时的时序约束应由框架层封装，不泄漏到业务代码。
- **最小惊讶原则**：用户在 on_loaded 中读取其他 entity 应是安全操作。
- **生命周期分层**：on_loaded（自身就绪）vs on_loaded_deferred（环境就绪）。

#### 设计方案

在 `ILifecycle` trait 增加默认方法 `on_loaded_deferred`，框架在 `on_loaded` 后用 `cx.defer_in` 自动调用：

```rust
pub trait ILifecycle {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
    
    /// 在 on_loaded 后延迟执行，安全读取其他 entity。
    /// 框架自动用 cx.defer_in 包裹，避免 re-entrancy panic。
    fn on_loaded_deferred(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
}
```

框架层（`#[component]` 宏生成的 `__rml_lifecycle` 调用）：

```rust
// 宏生成的生命周期分发
fn __rml_invoke_lifecycle(this: &mut Self, window: &mut Window, cx: &mut Context<Self>) {
    this.on_loaded(window, cx);
    let window_handle = window.window_handle();
    cx.defer_in(window, |this, window, cx| {
        this.on_loaded_deferred(window, cx);
    });
}
```

#### 实施步骤

1. 在 `ILifecycle` trait 增加 `on_loaded_deferred` 默认方法
2. 修改 `#[component]` 宏生成的生命周期分发代码
3. 重写 `welcome_case`：将 `refresh_items` 从 `on_loaded` + 手动 `defer_in` 迁移到 `on_loaded_deferred`
4. 文档说明：跨 entity 读取放在 `on_loaded_deferred`，自身初始化放在 `on_loaded`

#### 验证标准

- `grep -r "defer_in" demo/src/cases/` 返回 0 行
- welcome_case 首次渲染正常显示案例列表
- 文档明确区分 on_loaded vs on_loaded_deferred 使用场景

#### 风险与缓解

| 风险 | 概率 | 缓解 |
|---|---|---|
| defer 导致首帧闪烁（数据未就绪） | 中 | 框架在首帧渲染占位符（loading state），defer 后填充 |
| 用户在 on_loaded 中仍读取其他 entity | 高 | 文档 + lint 警告：跨 entity 读取应放 on_loaded_deferred |
| defer 时序不确定 | 低 | GPUI defer_in 在当前 effect cycle 结束后执行，时序可预测 |

#### 影响范围

- [crates/core/](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/) `ILifecycle` trait 增加 `on_loaded_deferred`
- [crates/macros/](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/) `#[component]` 宏生成生命周期分发
- [demo/src/cases/welcome_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/welcome_case.rml.rs) 迁移到 `on_loaded_deferred`

---

## 七、M3：架构优化

### 7.1 M3-1：MainWindow 拆分

#### 目标

将 570 行的 MainWindow 拆分为 3 个职责单一的类型，提升可维护性。

#### 设计方案

```
MainWindow (壳)
├── WorkbenchManager     (IWorkbenchManager impl, workbenches + activated)
├── ShellHost            (IContributionHost impl, entries 存储)
├── ShellViewModels      (cases/menus/status/activities + 投影逻辑)
└── MainWindowServices   (LSP / ActivityBar / i18n observer)
```

MainWindow 仅持有这三个组件的引用，自身只实现 ILifecycle（协调初始化顺序）。

#### 实施步骤

1. 抽取 `WorkbenchManager`：封装 `ObservableVec<Arc<dyn IWorkbench>>` + `activated` + `IWorkbenchManager` impl
2. 抽取 `ShellHost`：封装 `entries: Arc<RwLock<Vec<ContribEntry>>>` + `IContributionHost` impl
3. 抽取 `ShellViewModels`：封装 `cases/menus/status/activities` + `project_entries` + `rebuild_i18n_dependent`
4. MainWindow 持有 `WorkbenchManager` + `ShellHost` + `ShellViewModels`，协调初始化
5. 渲染方法（`active_view` / `render_menu_bar` / `render_status_bar`）委托到对应组件

#### 验证标准

- [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) 行数 ≤ 200 行
- 每个抽取类型行数 ≤ 150 行
- 每个类型单一职责（用一句话描述功能）

---

### 7.2 M3-2：`on_rendered` 钩子（M5'）

#### 目标

提供首次渲染后的生命周期钩子，让 ElementRef 场景有干净的声明式方案。

#### 设计方案

```rust
pub trait ILifecycle {
    fn on_loaded(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
    fn on_loaded_deferred(&mut self, window: &mut Window, cx: &mut Context<Self>) {}
    
    /// 首次渲染完成后调用，ElementRef 已填充。
    fn on_rendered(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}
}
```

框架在首次 `__rml_populate_refs` 后调用 `on_rendered`。

#### 验证标准

- `input_case` / `code_editor_case` 在 `on_rendered` 中配置 ElementRef
- 替代方案 A（codegen 透传）仍优先；on_rendered 是兜底

---

### 7.3 M3-3：布尔属性语法统一

#### 目标

消除 `primary=""` 的怪异语法，统一为 `primary` 或 `primary={true}`。

#### 设计方案

RML 解析器识别三种布尔属性语法：

```xml
<Button primary />              <!-- 简写：等同 primary="true" -->
<Button primary="true" />       <!-- 显式 true -->
<Button primary={is_primary} /> <!-- 绑定表达式 -->
```

`primary=""` 不再解析为布尔 true（保持向后不兼容，RML 无历史包袱）。

#### 实施步骤

1. 修改 RML 解析器：识别无值属性（`primary`）为布尔 true
2. 修改 codegen：生成 `.primary(true)` 而非 `.primary("")`
3. 更新所有 case 模板：`primary=""` → `primary`
4. 更新文档：语法说明

#### 验证标准

- `grep 'primary=""' demo/` 返回 0 行
- 所有布尔属性支持简写 + 绑定两种语法

---

### 7.4 M3-4：IWorkbenchProvider 注册为 contribution

#### 目标

让 `LspWorkbenchProvider` 等 workbench 工厂通过 contribution 注册，新增 schema 不需修改 MainWindow。

#### 设计方案

```rust
#[contribute_provider(schema = "lsp", host_id = "demo.shell")]
pub struct LspWorkbenchProvider { ... }
```

MainWindow::build_workbench 改为查询全局 provider registry：

```rust
fn build_workbench(&self, uri: &Uri) -> Option<Arc<dyn IWorkbench>> {
    let schema = uri.scheme();
    cx.get_service::<WorkbenchProviderRegistry>()
        .and_then(|r| r.get(schema))
        .map(|p| p.render(uri))
}
```

---

## 八、M4+：精雕细琢（P3 改进项，持续）

### 8.1 ContributionSlot 泛型化

#### 目标

消除 `CaseViewModel` / `MenuViewModel` / `StatusViewModel` 三个近重复类型的代码冗余。

#### 设计方案

引入泛型 slot 抽象：

```rust
pub struct ContributionSlot<S: Slot> {
    contributions: Vec<(Arc<dyn IContribution>, ContributionOptions)>,
    _marker: PhantomData<S>,
}

pub trait Slot {
    const KIND: &'static str;
    fn filter(c: &IContribution, o: &ContributionOptions) -> bool {
        o.effective_slot() == Some(Self::KIND)
    }
}

pub struct CaseSlot;
impl Slot for CaseSlot { const KIND: &'static str = "case"; }
// 类似 MenuSlot / StatusSlot / ActivitySlot
```

具体 ViewModel（如 `CaseViewModel`）仅包含 slot 特有字段 + render 委托。

#### 影响范围

- [demo/src/shell/case_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_view_model.rs)
- [demo/src/shell/menu_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs)
- [demo/src/shell/status_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/status_view_model.rs)

---

### 8.2 命名规范统一

#### 目标

统一 PascalCase vs kebab-case 标签命名规则。

#### 设计方案

- **内置组件**：PascalCase（`Button` / `Card` / `TabBar`）
- **容器/语义标签**：kebab-case（`dropdown-menu` / `context-menu` / `menu-item`）
- **HTML 兼容标签**：小写（`div` / `span` / `p` / `input`）
- **组件别名**：仅 PascalCase 为标准，kebab-case 为兼容（如 `accordion` 兼容 `Accordion`）

文档明确分类，lint 检查非标准命名。

---

### 8.3 错误处理与稳定性强化

#### 目标

消除 demo 中所有 `unwrap()` / `expect()` panic 路径，统一错误处理策略。

#### 设计方案

- **LSP 失败**：`log::warn!` + 状态栏显示错误摘要（已部分实现）
- **IO 失败**：返回 `Result`，调用方决定降级策略
- **URI 解析**：保留 `parse().unwrap()` 仅限编译期常量 URI，运行时 URI 用 `?` 传播
- **配置失败**：`log::warn!` 优雅降级（已实现）

引入 `RmlResult<T>` 类型别名 + `RmlError` 枚举统一错误类型。

#### 验证标准

- `grep -r "unwrap()" demo/src/` 仅剩白名单（编译期常量 URI 等）
- LSP 服务不可用时 demo 不 panic，状态栏显示 "LSP unavailable"

#### 影响范围

- [demo/src/lsp/code_editor_tab.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/lsp/code_editor_tab.rml.rs) 替换 unwrap 路径

---

## 九、Verification（验证标准）

### 9.1 量化指标脚本

建立评估脚本 `scripts/audit-five-star.ps1`：

```powershell
# 检查 __rml_* 出现频次
$observableCount = (Select-String -Path "demo/src/**/*.rs" -Pattern "__rml_bump_version").Count
Write-Host "Observable violations: $observableCount (target: 0)"

# 检查 cx.notify 显式调用
$notifyCount = (Select-String -Path "demo/src/cases/*.rs" -Pattern "cx\.notify\(\)").Count
Write-Host "Manual notify: $notifyCount (target: 0)"

# 检查 ensure_*_registered
$ensureCount = (Select-String -Path "demo/src/**/*.rs" -Pattern "ensure_.*_registered").Count
Write-Host "Manual ability registration: $ensureCount (target: 0)"

# 检查 case 平均行数
$caseLines = (Get-ChildItem demo/src/cases/*.rml.rs | ForEach-Object { (Get-Content $_).Count } | Measure-Object -Average).Average
Write-Host "Average case lines: $caseLines (target: <= 80)"
```

### 9.2 验证 case 映射表

每个改进项配套一个验证 case：

| 改进项 | 验证 case | 验证方法 |
|---|---|---|
| M1-1 | counter_case | 删除 cx.notify() 后仍刷新 |
| M1-2 | counter_case | 删除 impl IContribution 后仍注册 |
| M1-3 | input_case | placeholder 直接显示 |
| M2-1 | list_case | 删除 __rml_bump_version 后仍刷新 |
| M2-2 | status_bar_case | 删除 ensure_*_registered 后仍渲染 |
| M2-3 | welcome_case | 删除 defer_in 后无 panic |
| M3-1 | main_window | 行数 ≤ 200 |
| M3-3 | 所有 case | primary 简写工作 |

### 9.3 回归测试套件

建立 `crates/core/tests/five_star_regression.rs`：

```rust
#[test]
fn no_user_code_uses_rml_internal_api() {
    // 扫描 demo/src，确保无 __rml_* 调用
}

#[test]
fn all_commands_auto_notify() {
    // 反射 #[command] 方法，验证 guard 注入
}

#[test]
fn observable_fields_type_checked() {
    // 验证字段重命名触发编译错误
}
```

---

## 十、里程碑与时间线

### 10.1 里程碑表

| 里程碑 | 周期 | 退出标准 | 阶段产出 |
|---|---|---|---|
| **M0** | Week 0 | 评估基线建立 | 完成现状审计报告 |
| **M1** | Week 1-3 | M1-1 + M1-2 完成 | case 平均行数 ≤ 50 |
| **M2** | Week 4-6 | M1-3 完成 | input/code_editor 声明式 |
| **M3** | Week 7-10 | M2-1 + M2-2 完成 | __rml_* 零出现 |
| **M4** | Week 11-14 | M2-3 + M3-1 完成 | re-entrancy 零手动 defer |
| **M5** | Week 15-20 | M3-2 + M3-3 + M3-4 完成 | MainWindow ≤ 200 行 |
| **M6+** | 持续 | M4+ 持续改进 | 稳定性指标达成 |

### 10.2 关键路径

M1-1 → M2-1（响应式模型统一）→ M3-1（架构拆分）是关键路径，决定整体进度。

M1-2、M1-3、M2-2、M2-3 可并行。

### 10.3 退出标准

每个里程碑退出时需满足：

1. 对应改进项的验证 case 通过
2. 量化指标脚本输出达标
3. 文档同步更新
4. 回归测试套件全绿
5. 性能基准无回退（±5% 内）

---

## 附录 A：关键软件工程原理对照表

| 原理 | 应用改进项 |
|---|---|
| **DRY** | M1-2（IContribution 自动生成） |
| **Convention over Configuration** | M1-2、M2-2 |
| **封装不变式** | M1-1（notify guard）、M2-1（Observable） |
| **迪米特法则** | M1-1、M2-2、M2-3 |
| **响应式自动失效传播** | M1-1、M2-1 |
| **声明式优先，命令式兜底** | M1-3、M3-2 |
| **最小惊讶原则** | M1-3、M3-3 |
| **框架吸收复杂性** | M2-3 |
| **类型安全优先** | M2-1 |
| **生命周期分层** | M2-3、M3-2 |
| **单一职责原则** | M3-1 |
| **开闭原则** | M3-4 |
| **依赖注入自动化** | M2-2 |

---

## 附录 B：术语表（用户需学习的概念）

5★ 标准要求用户需学习的框架概念 ≤ 10 个：

| 术语 | 简述 |
|---|---|
| contribution | 贡献点，向 shell 注册功能 |
| component | RML 组件，`.rml` + `.rml.rs` 文件对 |
| command | `#[command]` 方法，处理事件 |
| computed | `#[computed]` 方法，缓存计算属性 |
| lifecycle | `ILifecycle` trait，on_loaded 等钩子 |
| ref | `ref="name"` 指令，元素引用 |
| each | `each={x in xs}` 指令，列表渲染 |
| if | `if={expr}` 指令，条件渲染 |
| model | `model={field}` 指令，双向绑定 |
| slot | `<template slot="...">` 插槽 |

**目标**：用户掌握上述 10 个术语即可写出 90% 的应用，无需理解 ability cast、bump_version、defer_in、re-entrancy 等内部概念。

---

## 附录 C：改进前后对比（counter_case 示例）

### 改进前（73 行）

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{TableColumn, TableRow};
use crate::cases::common::build_api_table;

#[contribute(host_id = "demo.shell", id = "binding.counter", kind = "case", group = "binding", order = 1)]
#[component]
#[derive(Default)]
pub struct CounterCase {
    pub count: i32,
    pub step: i32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl IContribution for CounterCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.counter.title") }
}

impl ILifecycle for CounterCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[/* ... */]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CounterCase {
    #[computed]
    pub fn counter_text(&self) -> String { format!("点击次数：{}", self.count) }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += self.step.max(1);
        cx.notify();  // ← 改进后删除
    }
    // ...
}
```

### 改进后（约 35 行）

```rust
use rml::prelude::*;
use rml_ui::{TableColumn, TableRow};
use crate::cases::common::build_api_table;

#[contribute(host_id = "demo.shell", id = "binding.counter", kind = "case", group = "binding", order = 1, label = "case.counter.title")]
#[component]
#[derive(Default)]
pub struct CounterCase {
    pub count: i32,
    pub step: i32,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// ← 改进后：无 impl IContribution（宏自动生成）

impl ILifecycle for CounterCase {
    fn on_loaded(&mut self, _w: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[/* ... */]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl CounterCase {
    #[computed]
    pub fn counter_text(&self) -> String { format!("点击次数：{}", self.count) }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, _cx: &mut Context<Self>) {
        self.count += self.step.max(1);
        // ← 改进后：无 cx.notify()（宏自动注入）
    }
}
```

**减少 38 行（52%）**，且无框架内部 API 泄漏。

---

## 修订历史

| 日期 | 版本 | 修订内容 |
|---|---|---|
| 2026-07-06 | v1.0 | 初版：基于 demo 全面审查建立五星迭代计划 |
