# Demo 案例重构剩余任务执行计划

## 摘要

基于已批准的 v3 计划,继续执行 demo 案例库的系统性重构。Phase 1(badge/label/card 修复)和 Phase 2+3 批次 A(6 个空 ViewModel 升级)已完成。本计划聚焦剩余三批工作:批次 B(6 个半凑数案例)、批次 C(2 个纯 API 迁移)、Phase 4(6 个新框架能力案例)、Phase 5(CSS 清理 + i18n + mod.rs 注册)。

**核心目标**:每个案例都用 `<Table>` 组件展示 API,并至少演示 1 项 RML 框架能力(model 双向绑定 / if 条件渲染 / each 列表 / computed 计算属性 / command 命令)。

***

## 当前状态分析

### 已完成(稳定,cargo check 通过)

| 案例                        | 状态                           | 框架能力演示                                      |
| ------------------------- | ---------------------------- | ------------------------------------------- |
| badge\_case               | ✅ size + if 条件渲染             | if={size\_index == 0}                       |
| label\_case               | ✅ 子节点形式                      | 文本插值                                        |
| slot\_case                | ✅ Card title bind codegen 修复 | slot 插槽                                     |
| tag\_case                 | ✅ model + if 条件渲染 7 变体       | model={tag\_text}, if={variant\_index == N} |
| card\_case                | ✅ model + hoverable 切换       | model={card\_title}                         |
| title\_bar\_case          | ✅ model 双向绑定                 | model={title}                               |
| native\_status\_bar\_case | ✅ #\[command] 事件             | on-show-ready 等命令                           |
| avatar\_group\_case       | ✅ if 条件渲染动态 Avatar           | if={avatar\_count > N}                      |
| button\_group\_case       | ✅ if 条件渲染动态 Button           | if={button\_count > N}                      |

### 待修改(批次 B — 6 个半凑数案例)

| 案例                     | 当前状态                            | 缺失项                                      |
| ---------------------- | ------------------------------- | ---------------------------------------- |
| progress\_case         | 仅 current: f32,无命令              | api 字段 + loading + 命令 + .api-table→Table |
| progress\_circle\_case | 仅 current: f32,无命令              | api 字段 + loading + 命令 + .api-table→Table |
| checkbox\_case         | 有 is\_checked/is\_disabled + 命令 | api 字段 + .api-table→Table                |
| switch\_case           | 有 is\_on/is\_disabled + 命令      | api 字段 + .api-table→Table                |
| input\_case            | 有 input\_state                  | api 字段 + .api-table→Table                |
| slider\_case           | 有 slider\_state/disabled\_state | api 字段 + .api-table→Table                |

### 待修改(批次 C — 2 个纯 API 迁移)

| 案例                 | 当前状态                            | 缺失项                       |
| ------------------ | ------------------------------- | ------------------------- |
| tree\_case         | 有 tree\_state + on\_activate 命令 | api 字段 + .api-table→Table |
| code\_editor\_case | 有 editor\_state                 | api 字段 + .api-table→Table |

### 共享工具(已就绪)

* `demo/src/cases/common/mod.rs` 的 `build_api_table(&[(prop, type, desc)])` 函数

* 返回 `(Vec<TableColumn>, Vec<TableRow>)`,所有案例统一调用

***

## 实施步骤

### 批次 B — 6 个半凑数案例增强

**统一模式**(对每个案例):

1. 添加 `use rml_ui::{TableColumn, TableRow}` + `use crate::cases::common::build_api_table`
2. 添加 `api_columns: Vec<TableColumn>` + `api_rows: Vec<TableRow>` 字段
3. `ILifecycle::on_loaded` 中调用 `build_api_table` 填充 api 字段
4. `.rml` 中替换 `<div class="api-table">...</div>` 为 `<Table columns={api_columns} rows={api_rows} bordered="" stripe="" />`

**案例 1: progress\_case**(order 26)

* 文件:`demo/src/cases/progress_case.rml.rs` + `.rml`

* 新增字段:`loading: bool`

* 新增命令:`on_increase`(current += 10.0,上限 100)、`on_decrease`(current -= 10.0,下限 0)、`on_toggle_loading`(loading = !loading)

* API 表格:value / loading / size

* .rml 新增"交互演示"Card:三个 Button + if 条件渲染 loading 状态的 Progress

**案例 2: progress\_circle\_case**(order 27)

* 文件:`demo/src/cases/progress_circle_case.rml.rs` + `.rml`

* 同 progress\_case 模式:新增 loading + 三命令 + 交互 Card

**案例 3: checkbox\_case**(order 33)

* 文件:`demo/src/cases/checkbox_case.rml.rs` + `.rml`

* 已有 is\_checked/is\_disabled + on\_toggle\_checked/on\_toggle\_disabled + status\_text computed

* 仅添加 api 字段 + ILifecycle impl + 替换 .api-table

* API 表格:label / checked / disabled / size

**案例 4: switch\_case**(order 34)

* 文件:`demo/src/cases/switch_case.rml.rs` + `.rml`

* 已有 is\_on/is\_disabled + on\_toggle/on\_toggle\_disabled + status\_text computed

* 仅添加 api 字段 + ILifecycle impl + 替换 .api-table

* API 表格:label / checked / disabled / size

**案例 5: input\_case**(order 35)

* 文件:`demo/src/cases/input_case.rml.rs` + `.rml`

* 已有 input\_state(在 on\_loaded 初始化)

* 仅添加 api 字段 + 替换 .api-table

* **不添加 on\_change 事件**(subscribe 架构属于 M1' 任务范围)

* API 表格:placeholder / default\_value / disabled(注明通过 InputState builder 设置)

**案例 6: slider\_case**(order 37)

* 文件:`demo/src/cases/slider_case.rml.rs` + `.rml`

* 已有 slider\_state + disabled\_state

* 仅添加 api 字段 + 替换 .api-table

* API 表格:disabled / SliderState::min/max/step/default\_value

**批次 B 验证**:`cargo check -p rust-rml-demo` 通过

### 批次 C — 2 个纯 API 迁移

**案例 7: tree\_case**(order 36)

* 文件:`demo/src/cases/tree_case.rml.rs` + `.rml`

* 已有 tree\_state + on\_activate 命令 + status\_text computed

* 添加 api 字段 + ILifecycle impl + 替换 .api-table

* API 表格:on-activate / on-select / TreeState::items

**案例 8: code\_editor\_case**(order 38)

* 文件:`demo/src/cases/code_editor_case.rml.rs` + `.rml`

* 已有 editor\_state

* 添加 api 字段 + ILifecycle impl + 替换 .api-table

* API 表格:InputState::code\_editor / multi\_line / default\_value

**批次 C 验证**:`cargo check -p rust-rml-demo` 通过

### Phase 4 — 6 个新框架能力专项案例

**目标**:每个案例专注演示 1 项 RML 框架核心能力,放在 "framework" 分组下。

**案例 9: expression\_case**(framework, order 41)

* 文件:新建 `demo/src/cases/expression_case.rml.rs` + `.rml`

* 演示:表达式绑定 + #\[computed] 计算属性

* 内容:输入两个数字(model 双向绑定),实时显示和/积/商(computed)

* API 表格:无(纯框架能力演示,展示 expression 语法)

**案例 10: conditional\_case**(framework, order 42)

* 文件:新建 `demo/src/cases/conditional_case.rml.rs` + `.rml`

* 演示:if 条件渲染(多分支)

* 内容:tab\_index 字段 + 3 个 Button 切换 + if={tab\_index == 0/1/2} 渲染不同 Card

* API 表格:if 指令语法说明

**案例 11: list\_case**(framework, order 43)

* 文件:新建 `demo/src/cases/list_case.rml.rs` + `.rml`

* 演示:each 列表渲染

* 内容:items: Vec<String> 字段 + each={item in items} 渲染 Tag 列表 + 添加/删除项命令

* API 表格:each 指令语法说明

**案例 12: template\_slot\_case**(framework, order 44)

* 文件:新建 `demo/src/cases/template_slot_case.rml.rs` + `.rml`

* 演示:slot 模板插槽

* 内容:定义带 slot 的 Card 模板 + 不同插槽内容复用

* API 表格:slot 指令语法说明

**案例 13: validation\_case**(framework, order 45)

* 文件:新建 `demo/src/cases/validation_case.rml.rs` + `.rml`

* 演示:model 双向绑定 + 表单验证

* 内容:email 字段 + model 绑定 + computed 验证邮箱格式 + 错误提示

* API 表格:model 指令语法说明

**案例 14: theme\_case**(framework, order 46)

* 文件:新建 `demo/src/cases/theme_case.rml.rs` + `.rml`

* 演示:主题切换

* 内容:theme\_index 字段 + 切换 Button + if 条件渲染不同主题色的 Badge/Tag

* API 表格:主题相关 API

**Phase 4 验证**:`cargo check -p rust-rml-demo` 通过

### Phase 5 — CSS 清理 + i18n + mod.rs 注册

**任务 5.1: CSS 清理**

* 文件:`demo/assets/styles.css`

* 删除第 81-113 行的 7 条 .api-table 相关 CSS 规则(.api-table / .api-row / .api-row span / .api-prop-name / .api-prop-type / .api-header)

* 原因:所有案例已迁移到 `<Table>` 组件,这些 CSS 不再被引用

**任务 5.2: mod.rs 注册 6 个新模块**

* 文件:`demo/src/cases/mod.rs`

* 添加:

  ```rust
  // Phase 4:6 个框架能力专项案例
  #[path = "expression_case.rml.rs"]
  pub mod expression_case;
  #[path = "conditional_case.rml.rs"]
  pub mod conditional_case;
  #[path = "list_case.rml.rs"]
  pub mod list_case;
  #[path = "template_slot_case.rml.rs"]
  pub mod template_slot_case;
  #[path = "validation_case.rml.rs"]
  pub mod validation_case;
  #[path = "theme_case.rml.rs"]
  pub mod theme_case;
  ```

**任务 5.3: welcome\_case.rs 添加 "framework" 分组**

* 文件:`demo/src/cases/welcome_case.rml.rs`

* 在 `compute_grouped_items` 的 label match 中添加:

  ```rust
  Some("framework") => t_static("tree.group.framework"),
  ```

**任务 5.4: i18n 条目添加**

* 文件:`demo/assets/locales/zh-CN.json` + `en-US.json`

* 添加 6 个新案例的 title 条目(case.expression.title / case.conditional.title / case.list.title / case.template\_slot.title / case.validation.title / case.theme.title)

* 添加 1 个新分组条目(tree.group.framework = "框架能力" / "Framework")

**Phase 5 验证**:

* `cargo check -p rust-rml-demo` 通过

* 全局搜索 `.api-table` 确认 0 引用

* 全局搜索 `api_columns` 确认所有案例都已迁移

***

## 假设与决策

### 假设

1. **codegen 已支持** **`<Table>`** **组件**:批次 A 已验证通过,Table 组件的 columns/rows/bordered/stripe 属性 codegen 正确
2. **`loading={expr}`** **绑定安全**:codegen 修复后,`loading={true}` 生成 `.loading(true)`,`loading={loading}` 生成 `.loading(self.loading)`,后者对 bool 字段安全
3. **Input on\_change 事件不在本计划范围**:subscribe 架构属于 M1' 任务,本计划仅做 API 迁移
4. **framework 分组的 i18n key**:使用 `tree.group.framework`,与现有 binding/components/i18n/menu 分组命名一致

### 决策

1. **批次 B 范围**:progress/progress\_circle 添加交互命令(loading + 增减),checkbox/switch/input/slider 仅做 API 迁移(已有交互或属于 M1' 范围)
2. **Phase 4 案例分组**:6 个新案例放在 "framework" 分组,与 "components" 分组区分,突出框架能力演示
3. **Phase 4 case order**:使用 41-46,避免与现有 22-38 组件案例冲突
4. **不修改 codegen**:本计划仅修改 demo 案例,不触及 engine codegen 代码(除非发现新 bug)

### 风险与缓解

| 风险                        | 缓解措施                                       |
| ------------------------- | ------------------------------------------ |
| Phase 4 新案例触发 codegen bug | 先用最小化 .rml 验证 codegen,再丰富内容;遇到 bug 记录并单独修复 |
| 表达式语法可能与现有 codegen 不兼容    | expression\_case 优先实现,验证表达式绑定语法            |
| each 指令可能未完全实现            | list\_case 验证 each 语法,若不支持则降级为 if + 索引渲染   |

***

## 验证步骤

### 每批次验证

```bash
cargo check -p rust-rml-demo
```

### 最终验证(Phase 5 完成后)

1. `cargo check -p rust-rml-demo` 编译通过
2. `cargo test -p rust-rml-engine` 测试通过(确认 codegen 修改未破坏)
3. 全局搜索 `.api-table` → 0 引用(所有案例已迁移到 Table)
4. 全局搜索 `api_columns` → 所有案例都有该字段
5. 启动 demo,确认:

   * 所有案例可正常打开

   * API 表格用 `<Table>` 渲染(带边框 + 斑马纹)

   * framework 分组下有 6 个新案例

   * 交互按钮响应正常

