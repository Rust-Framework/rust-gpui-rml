# 计划：Observable 解决方案必要性研究 + 后续重构计划

## 摘要

在 RML 当前已落地的「编译期 codegen + AtomicU64 版本号 + `cx.notify()`」机制基础上，开展一项架构师级研究：横向对比 React `setState`、Vue reactive（`ref`/`reactive`/`v-model`）、WPF `DependencyProperty` + `INotifyPropertyChanged`、以及 RML 当前的版本号方案，从「Rust 所有权/Send+Sync 约束」「GPUI 渲染模型」「开发者心智模型」「框架设计哲学」四个维度论证各方案的契合度，并基于结论产出一份可执行的重构计划。

研究以中立视角进行，不预设 WPF 路线对错。最终交付两份文档：

1. **研究文档**：`docs/09-architecture/observable-research.md`（架构师视角的横向对比与论证）
2. **重构计划文档**：`docs/09-architecture/observable-refactor-plan.md`（基于研究结论的决策性重构步骤）

两份文档合并为本计划的单次执行产物；不修改任何框架代码（重构计划本身只产出计划，执行计划属下一轮工作）。

## 当前状态分析（Phase 1 探索结果）

### 已存在的可观察机制

RML 当前并非「无 observable 方案」，而是一套**显式拒绝 wrapper 类型的编译期物化方案**：

| 层       | 文件                                                                                                                                                       | 职责                                                                                                           |
| ------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| 契约      | [crates/core/src/binding.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/binding.rs)                                                             | `BindingPath`/`BindingSegment`；`IBindingContext` **目前仅为 marker trait**（L57-60 注释「MVP 阶段标记，阶段二扩展为完整订阅管理接口」）   |
| 契约      | [crates/core/src/observable.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/observable.rs)                                                       | `ObservableVec<T>`：`RwLock<Vec<T>>` + `AtomicU64` + 可选 `flume::Sender<()>`；仅用于 host Entity 跨线程通知，**非字段级反应式** |
| 契约      | [crates/core/src/computed\_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs)                                              | `ComputedCache`：`Mutex<HashMap<String,(u64, Box<dyn Any>)>>`；core 中唯一 `allow(unsafe_code)` 处                 |
| 宏       | [crates/macros/src/component.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs)                                                     | 为每个 pub 字段注入 `__rml_<field>_version: AtomicU64` + `__rml_computed_cache` + `__rml_input_states`              |
| 宏       | [crates/macros/src/command.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs)                                                         | `syn::visit::Visit` 检测 `self.x =`/`+=`/`push`/`clear` 等，注入 `__rml_bump_version` + `cx.notify()`              |
| 宏       | [crates/macros/src/computed.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/computed.rs)                                                       | 将 `fn xxx` 重命名为 `fn __rml_computed_xxx`，由 codegen 生成缓存包装                                                     |
| codegen | [crates/engine/src/compiler/codegen/observable.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs)                 | 生成 `__rml_bump_version`/`__rml_get_version`/`__rml_computed_deps_version` + `InputState` 惰性同步                |
| codegen | [crates/engine/src/compiler/codegen/binding.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/binding.rs)                       | 双向绑定：parse → validate → assign → bump；失败时不赋值不 bump，仅设错误态                                                     |
| 设计      | [.trae/documents/phase-b2-observable-data-binding-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/phase-b2-observable-data-binding-plan.md) | 明确拒绝 `Observable<i32>` wrapper：「`*self.count += 1` 违反语法不变要求」                                                 |
| 设计      | [.trae/documents/rml-iteration-architecture-analysis.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-architecture-analysis.md)     | §3.1「Rust 所有权 vs WPF 运行时机制」对照表；§3.2 意图 A「心智模型与 Vue 的 mutation-driven 一致」                                     |

### 已识别的痛点与缺口

| 痛点                                                            | 位置                                                                                          | 性质                                                                                                    | <br /> |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------- | :----- |
| `IBindingContext` 仍为 marker trait，无细粒度订阅                      | [binding.rs:57-60](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/binding.rs)          | 缺口                                                                                                    | <br /> |
| `cx.notify()` 触发 Entity 全量重渲，无部分重渲                            | GPUI 渲染模型                                                                                   | 限制                                                                                                    | <br /> |
| `#[command]` 不追踪指针间接修改（`let p = &mut self.x; *p = 1;`）        | [command.rs:25 注释](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs)        | 限制                                                                                                    | <br /> |
| `Subscription` 非 `Sync`，`.detach()` 后无法运行期取消                  | [component.rs:78-80 注释](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs) | 限制                                                                                                    | <br /> |
| `ComputedCache` `unsafe Send/Sync` 依赖「仅 render 线程调用」约定，无运行期强制 | [computed\_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs) | 风险                                                                                                    | <br /> |
| `#[on_loaded]`/`#[on_unloaded]` 仍 pass-through，未自动联动          | [lifecycle.rs:16](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/lifecycle.rs)       | 缺口                                                                                                    | <br /> |
| `debounce = "100ms"` 已解析但未实现                                  | [command.rs:50](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs)           | 缺口                                                                                                    | <br /> |
| `IConverter` codegen 接入缺失（\`                                  | \` 管道符未在 codegen 中处理）                                                                       | [converter/trait\_def.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/converter/trait_def.rs) | 缺口     |
| engine crate 体量过大（parser/compiler/build/runtime/css 全在一处）     | [crates/engine/src/](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/)                | 结构债                                                                                                   | <br /> |

### 设计哲学的已写明取向

来自 [phase-b2-observable-data-binding-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/phase-b2-observable-data-binding-plan.md) 与 [rml-iteration-architecture-analysis.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-architecture-analysis.md)：

1. **「语法不变」是硬约束**：`self.count += 1` 不得变成 `*self.count += 1`（拒绝 wrapper 类型）
2. **「最少样板」是设计目标**：所有 pub 字段默认 observable，不引入 `#[observable]` 属性
3. **「WPF 心智 + Rust 编译期优势」是设计意图**：用 codegen 替代反射、用版本号替代事件回调、用 `WeakEntity` 替代 `WeakReference`
4. **「不与 Rust 约束对抗」是设计原则**：把 `Send+Sync`/`Entity<T>: T: Send+Sync`/`Subscription 非 Sync` 转化为更安全的设计
5. **「心智模型对齐 Vue mutation-driven」是开发者视角目标**（§3.2 意图 A）

## 提议变更

### 交付物 1：研究文档 `docs/09-architecture/observable-research.md`

**做什么**：撰写一份架构师视角的横向对比研究文档，结构如下（章节顺序与覆盖维度已确定）：

1. **研究背景与问题陈述**

   * 当前 RML 已有版本号方案的事实

   * 用户提出的核心问题：是否需要追求 WPF 依赖属性通知方案？是否应转向 React/Vue 的 setState/reactive 模式？

2. **四方方案客观陈述**（不预设优劣）

   * **2.1 React** **`setState`** **/ hooks**：不可变状态 + 显式 setter + fiber diff；`useState`/`useReducer`；批量更新机制

   * **2.2 Vue reactive**：`ref`/`reactive` 基于 Proxy（Vue 3）或 getter/setter（Vue 2）；`v-model` 双向绑定；watch/computed 自动依赖收集

   * **2.3 WPF** **`DependencyProperty`** **+** **`INotifyPropertyChanged`**：DP 元数据系统 + 属性变更事件 + binding engine + IValueConverter + ValidationRule

   * **2.4 RML 当前**：编译期 codegen + AtomicU64 + `cx.notify()` + `#[computed]` ComputedCache + 惰性 InputState

3. **四维评估矩阵**（核心章节，每维列出所有方案的具体表现，不评判）

   * **3.1 Rust 所有权与** **`Send+Sync`** **契合度**

     * React：基于不可变与 diff，需在 Rust 中转化为 `Rc<RefCell>` 或 `Arc<Mutex>`；`setState` 闭包捕获问题

     * Vue：Proxy 在 Rust 中无对应物；`ref`/`reactive` 等价于 wrapper 类型（与 RML 「语法不变」硬约束冲突）

     * WPF：`DependencyProperty` 依赖运行时反射，Rust 无对应物；`INotifyPropertyChanged` 事件回调跨 Send+Sync 边界困难

     * RML：版本号 + codegen 满足 `Send+Sync`；但 `Subscription` 非 Sync 限制取消订阅

   * **3.2 GPUI 渲染模型契合度**

     * React/Vue：假设虚拟 DOM diff；GPUI 无 vdom，`cx.notify()` 触发整 Entity 重渲

     * WPF：假设可视化树局部失效；GPUI 不暴露部分重渲 API

     * RML：版本号机制只能跳过 `#[computed]` 重算，无法跳过整 Entity 重渲；这是 GPUI 限制而非 RML 选择

   * **3.3 开发者心智模型契合度**

     * React：函数式 + 不可变；Rust 开发者熟悉但与「字段赋值」直觉有距离

     * Vue：mutation-driven；与 Rust 字段赋值最接近

     * WPF：XAML + DataContext + Binding；面向 C# 开发者

     * RML：Vue 心智 + WPF 命令/校验/转换器词汇

   * **3.4 框架设计哲学契合度**

     * React：runtime diff，编译期优化靠 JSX transform

     * Vue：runtime reactive + 编译期 hints（patchFlag）

     * WPF：runtime 反射 + 元数据注册

     * RML：编译期 codegen + 零运行时反射（与 Rust 编译期优势一致）

4. **关键张力点深度分析**（中立呈现两难）

   * **4.1 「语法不变」vs「显式 setState」**：RML 选择前者（`self.x += 1` 自动触发），代价是 `#[command]` 宏无法追踪指针间接修改；React setState 选择后者，代价是用户必须写 `setCount(c => c + 1)`

   * **4.2 「全量重渲」vs「细粒度订阅」**：GPUI 限制决定 RML 即使有 `IBindingContext` 订阅也无法跳过 Entity 重渲；只能靠 `#[computed]` 缓存降低重算成本

   * **4.3 「编译期 codegen」vs「运行时反射」**：codegen 提供编译期正确性保证，代价是 `.rml` 修改需重编译（无热重载）；WPF 反射支持运行时设计时数据 but 编辑后立即可见

   * **4.4 「wrapper 类型」vs「字段旁挂版本号」**：wrapper（`Observable<i32>`）可在 Rust 中实现真正的字段级订阅，但破坏 `pub x: i32` 惯用法与外部 API 互操作；版本号旁挂保持语法不变但只能粗粒度追踪

5. **架构师结论**（基于四维分析得出，非预设）

   * 章节标题预留：「5.1 WPF 依赖属性路线对 RML 的契合度评估」「5.2 React/Vue setState 路线对 RML 的契合度评估」「5.3 当前版本号方案的不可替代性与可改进点」「5.4 推荐的演进方向」

   * 结论将明确回答用户三个问题：

     * (a) observable 解决方案的必要性如何？

     * (b) React/Vue setState 路线是否更适合 RML？

     * (c) WPF 依赖属性通知方案是否不适合 RML？

   * **结论撰写原则**：基于 §3、§4 的事实与权衡推导，不引用主观偏好；如某方案在某维度表现差，必须给出具体场景与证据

**为什么**：用户要求研究 + 重构计划，研究是重构的依据；中立立场意味着结论必须由分析支撑而非预设

**怎么做**：

* 严格基于 Phase 1 探索的事实（文件路径、行号、注释）撰写，不臆测

* 引用 [phase-b2-observable-data-binding-plan.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/phase-b2-observable-data-binding-plan.md) 与 [rml-iteration-architecture-analysis.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-architecture-analysis.md) 中已写明的设计决策

* 必要时引用 React/Vue/WPF 公开文档的行为事实（无需联网，基于公开知识）

* 文档遵循项目 docs 渐进式披露原则；不创建额外 SKILL.md（已存在 docs 体系）

### 交付物 2：重构计划文档 `docs/09-architecture/observable-refactor-plan.md`

**做什么**：基于研究文档 §5 的结论，产出一份**决策性重构计划**。结构如下：

1. **重构目标与范围**：明确「保留什么」「改进什么」「拒绝什么」
2. **决策矩阵**：每个识别出的痛点/缺口给出决策（保留 / 改进 / 拒绝 / 推迟）
3. **分阶段重构步骤**（按决策落地）：

   * **阶段 0（不改动）**：列出明确「保留不变」的机制及理由

   * **阶段 1（小改进）**：低风险补全，如 `IBindingContext` 真正实现细粒度订阅、`#[on_loaded]` 自动联动、`IConverter` codegen 接入、`debounce` 实现

   * **阶段 2（结构性）**：如研究结论支持，可能涉及拆分 engine crate、引入显式 setState API 作为 `#[command]` 的补充（非替代）

   * **阶段 3（远期）**：热重载、部分重渲（依赖 GPUI 上游）
4. **关键文件改动清单**：每个步骤对应的文件路径与改动概要
5. **风险与回滚策略**
6. **验证步骤**

**为什么**：用户明确要「研究 + 后续重构计划」，重构计划是研究的落地；决策性而非建议性，可执行

**怎么做**：

* 重构计划必须遵循 project\_memory 的硬约束：

  * 不引入新宏（Phase C 拒绝）；如需新 API，扩展已有宏

  * `IContribution`/`IVisualContribution` trait 签名不可改

  * `IVisualContribution::render` 直接取 `&mut Window, &mut App`

  * 框架不存储贡献点/缓存，委托 `IContributionHost`

* 文档语言遵循用户偏好：科学、专业、面向架构师；分析可行性、步骤、评估准则、关键软件工程原则

* 不写代码，只写计划；具体代码改动留待下一轮执行

## 假设与决策

1. **不修改任何框架代码**：本轮只产出两份文档，文档内可包含代码片段示意但不实际改动 crates
2. **研究立场中立**：§3、§4 不预设结论；§5 结论由前文事实推导
3. **重构计划决策性**：不是「可以考虑 A 或 B」，而是「采用 A，理由 X；拒绝 B，理由 Y」
4. **遵循项目硬约束**：重构计划不引入新宏、不改 IContribution 签名、不引入框架层缓存
5. **文档路径**：放在 `docs/09-architecture/` 下与 [responsibility.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/09-architecture/responsibility.md) 同级，便于与其他架构文档互引
6. **引用现成分析**：尽量复用 [rml-iteration-architecture-analysis.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/documents/rml-iteration-architecture-analysis.md) §3、§5 已有的对比表，避免重复劳动
7. **不创建 SKILL.md 索引**：本次只新增 2 个文档，不涉及大量参考文件聚合

## 验证步骤

1. **文档存在性**：`docs/09-architecture/observable-research.md` 与 `docs/09-architecture/observable-refactor-plan.md` 均创建成功
2. **事实正确性**：研究文档中所有引用的文件路径、行号、注释与 Phase 1 探索结果一致
3. **结论可追溯性**：研究文档 §5 每个结论都能在 §3 或 §4 找到支撑事实
4. **决策可执行性**：重构计划每个步骤都给出文件路径、改动概要、依赖前置步骤
5. **约束遵循**：重构计划不违反 project\_memory 中的硬约束（无新宏、签名不改、框架不存储）
6. **链接有效性**：所有 markdown 链接使用 `file:///` 绝对路径协议，可点击跳转
7. **篇幅合理**：研究文档约 600-900 行，重构计划文档约 300-500 行（与研究复杂度匹配，不过度工程化）

## 依赖顺序

```
Phase 1 探索（已完成）
    ↓
Phase 2 澄清（已完成，确认中立立场 + 研究+重构计划交付物）
    ↓
Phase 3 撰写计划（当前步骤）
    ↓
Phase 4 通知用户 → 用户确认 → 执行：
    ↓
执行步骤 A：撰写研究文档 observable-research.md
    ↓ （研究结论驱动）
执行步骤 B：撰写重构计划 observable-refactor-plan.md
    ↓
返回最终响应（不调用 NotifyUser）
```

## 关键文件清单

| 文件                                                 | 操作 | 说明               |
| -------------------------------------------------- | -- | ---------------- |
| `docs/09-architecture/observable-research.md`      | 新建 | 架构师视角的四方方案对比研究文档 |
| `docs/09-architecture/observable-refactor-plan.md` | 新建 | 基于研究结论的决策性重构计划   |

## 范围之外

* 实际修改 crates/\* 下的任何代码（属下一轮工作）

* 创建 SKILL.md 索引或调整 docs 目录结构

* 联网检索 React/Vue/WPF 最新文档（基于公开知识与项目内已写明的对比）

* 评估 GPUI 上游是否会引入部分重渲（属 GPUI 团队范畴）

