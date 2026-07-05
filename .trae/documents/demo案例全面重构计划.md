# Demo 案例全面重构计划

## Context

用户批评:"demo 的案例代码没有采用最佳组件进行构建,比如 grid 或 table 组件展示 api 说明等,case 中大多数 rml 页面均为了凑数,没有充分发挥 RML 框架的优势和能力。"

经系统调研确认三大问题:

1. **32 个案例的 API 表格用手工 div 拼凑** — 只有 `table_case` 用了 `<Table>` 组件,严重损害框架自举可信度
2. **13 个案例是凑数**(空 ViewModel `{}` + 静态展示) — 占 38%,无状态/交互/数据绑定
3. **18 项 RML 核心能力零覆盖** — `#[computed]`、`if`/`each` 指令、`ObservableVec`、键盘事件、命名插槽、`#[validate]` 校验、主题系统等

用户批准:**全面重构** + **凑数案例升级为真实交互** + **新增专项能力演示案例**。

## Phase 1: 编译验证与修复 (P0)

### 前置验证

运行 `cargo check -p rust-rml-demo` 确认当前编译状态。若存在错误(可能在 tag\_case/progress\_case/progress\_circle\_case/input\_case),按以下方式修复:

### 1.1 修复 `gen_tag` 子节点丢失 Bug

**文件**: [crates/engine/src/compiler/tag.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tag.rs)

**问题**: `gen_tag` 函数(行 67)在处理完构造器和属性后直接 `return Ok(code)`,不处理 `elem.children`。导致 `<Tag>Default</Tag>` 的文本 "Default" 被静默丢弃。

**修复**: 在属性处理循环后、`return` 前,参考 `gen_component` 的子节点处理逻辑,添加文本/元素子节点的 `.child(...)` 生成。

**同步修复**: [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 中 Tag 的 `container` 标志需改为 `true`(当前为 `false`),否则即使 gen\_tag 处理了子节点也不会生成 `.child(...)`。

### 1.2 验证 progress/input 编译

* `progress_case`: 确认 `component_static_setter` 中 `"loading"` 分支生成 `.loading(true)`(非无参 `.loading()`)

* `input_case`: 确认 `Input` 走 `Stateful` + `ref` 分支,`on_change` 由 `compiler/input/event.rs` 专用模块处理

### 验证

```powershell
cargo check -p rust-rml-demo  # 0 errors
```

***

## Phase 2: API Table 标准化 (P1)

### 2.1 创建共享工具模块

**新文件**: `demo/src/cases/common/mod.rs`

```rust
use rml_ui::{TableColumn, TableRow};

/// 构建 API 文档表格的列定义和行数据。
/// props 是 (属性名, 类型, 说明) 三元组切片。
pub fn build_api_table(props: &[(&str, &str, &str)]) -> (Vec<TableColumn>, Vec<TableRow>) {
    let columns = vec![
        TableColumn::new("prop", "属性"),
        TableColumn::new("type", "类型"),
        TableColumn::new("desc", "说明"),
    ];
    let rows = props.iter().map(|(p, t, d)| {
        TableRow::new()
            .cell("prop", *p)
            .cell("type", *t)
            .cell("desc", *d)
    }).collect();
    (columns, rows)
}
```

**注册**: 在 `demo/src/cases/mod.rs` 添加 `pub mod common;`

### 2.2 逐案例迁移(32 个案例)

**改造模式**:

ViewModel 端(.rml.rs):

```rust
// 改造前
pub struct XxxCase {}

// 改造后
pub struct XxxCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl ILifecycle for XxxCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("value", "f32", "进度值 0-100"),
            ("loading", "布尔标志", "加载中状态"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}
```

模板端(.rml):

```xml
<!-- 改造前 -->
<Card title="API">
    <div class="api-table">
        <div class="api-row api-header"><span class="api-prop-name">属性</span>...</div>
        <div class="api-row"><span class="api-prop-name">value</span>...</div>
    </div>
</Card>

<!-- 改造后 -->
<Card title="API">
    <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
</Card>
```

**参考黄金标准**: [demo/src/cases/table\_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/table_case.rml.rs) 第 15-60 行

**执行顺序**: 按 `mod.rs` 注册顺序分批改造,每批 4-5 个案例,每批后 `cargo check`。

### 2.3 CSS 清理

**文件**: [demo/assets/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css)

32 个案例全部迁移后,删除 `.api-table`/`.api-row`/`.api-prop-name`/`.api-prop-type`/`.api-header` 系列规则。保留 `.doc-pane .card`/`.code-block`/`.demo-section` 等其他规则。

### 验证

```powershell
# 确认无残留手工 API 表格
grep -r "api-table" demo/src/cases/  # 应返回 0 结果
cargo check -p rust-rml-demo
```

***

## Phase 3: 凑数案例升级 (P1)

### 设计原则

每个凑数案例添加:至少 1 个 `pub` 字段 + 1 个 `#[command]` 方法 + 1 个 `#[computed]` 方法 + 事件绑定。

### 升级清单

| 案例                        | 字段                                               | 命令                                                  | computed                  | 交互设计                                                                       |
| ------------------------- | ------------------------------------------------ | --------------------------------------------------- | ------------------------- | -------------------------------------------------------------------------- |
| badge\_case               | `count: i32`, `show_dot: bool`                   | `on_increment`, `on_toggle_dot`                     | `badge_label`(>99显示"99+") | Button 控制 count,Badge 显示                                                   |
| label\_case               | `text: String`, `weight: u8`                     | `on_cycle_weight`                                   | `weight_label`            | input model={text},Label 动态字重                                              |
| separator\_case           | `is_vertical: bool`, `is_dashed: bool`           | `on_toggle_orientation`, `on_toggle_dashed`         | —                         | Button 切换方向/样式,演示 `if` 指令                                                  |
| tag\_case                 | `tags: Vec<(String,String)>`                     | `on_add_tag`, `on_cycle_variant`                    | —                         | `each={tag in tags}` 循环渲染(需先修 gen\_tag Bug)                                |
| progress\_case            | `value: f32`, `is_loading: bool`                 | `on_increment`, `on_decrement`, `on_toggle_loading` | `progress_label`          | Button 组控制 value                                                           |
| progress\_circle\_case    | `value: f32`, `auto_mode: bool`                  | `on_toggle_auto`, `on_reset`                        | —                         | 自动递增模式                                                                     |
| button\_group\_case       | `buttons: Vec<(String,String)>`                  | `on_add_button`, `on_click_button`(WithArgs)        | —                         | `each` 循环动态生成按钮                                                            |
| avatar\_group\_case       | `members: Vec<String>`                           | `on_add_member`, `on_remove_member`                 | —                         | `each` 循环,动态增删                                                             |
| card\_case                | `click_count: i32`, `hoverable: bool`            | `on_card_click`, `on_toggle_hoverable`              | `count_text`              | Card 内 Button 点击计数                                                         |
| title\_bar\_case          | `title: String`                                  | `on_set_title`                                      | —                         | input model={title} 修改标题                                                   |
| native\_status\_bar\_case | `status_text: String`, `item_count: i32`         | `on_update_status`                                  | —                         | 动态状态文本                                                                     |
| input\_case               | `#[validate(length(min=2,max=20))] name: String` | `on_submit`                                         | `greeting`                | model 双向绑定 + 验证                                                            |
| slot\_case                | 自定义 `SlottedCard` 用户组件                           | —                                                   | —                         | `#[component(slots=["header","body","footer"])]` + `<template slot="...">` |

### 关键修复

**tag\_case 前置修复**: Phase 1 的 `gen_tag` 子节点 Bug 必须先修复,否则 `<Tag>Default</Tag>` 文本丢失。

### 验证

```powershell
# 确认无空 ViewModel
grep -r "pub struct.*Case {}" demo/src/cases/  # 应返回 0 结果
cargo check -p rust-rml-demo
```

***

## Phase 4: 新增框架能力演示案例 (P2)

### 4.1 新增案例分组

在 `demo/src/cases/mod.rs` 中新增 `framework` 组,与现有 `binding`/`components`/`i18n`/`menu` 并列。

i18n 新增键(zh-CN.json + en-US.json 同步):

```json
"tree.group.framework": "框架能力",
"case.computed.title": "计算属性 #[computed]",
"case.conditional.title": "条件渲染 if/show",
"case.list.title": "列表渲染 each",
"case.keyboard.title": "键盘事件",
"case.slot_template.title": "具名插槽",
"case.validation.title": "数据验证 #[validate]",
"case.theme.title": "主题切换",
"case.expression.title": "表达式与转换器"
```

### 4.2 新增案例清单(8 个)

| 案例                   | 演示能力                         | 字段/命令设计                                                                                                                                          | order |
| -------------------- | ---------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ | ----- |
| computed\_case       | `#[computed]` 依赖追踪+缓存        | `first_name`/`last_name`/`base_price`/`tax_rate` → computed 链: `full_name` → `display_text`, `tax_amount` → `total_price`                        | 50    |
| conditional\_case    | `if`/`else`/`show` 指令        | `count: i32` → `if={count>10}` 显示不同内容; `show={detail}` 控制显隐                                                                                      | 51    |
| list\_case           | `each` 循环 + `ObservableVec`  | `todos: ObservableVec<String>` → `each={todo in todos}` 渲染 + WithArgs 删除                                                                         | 52    |
| keyboard\_case       | `on-key-down`/`on-key-up`    | `last_key: String`/`key_count: i32` → div 监听键盘事件显示按键                                                                                             | 53    |
| slot\_template\_case | 用户组件具名插槽                     | `#[component(slots=["header","body","footer"])]` + `<template slot="...">` + `<slot name="...">`                                                 | 54    |
| validation\_case     | `#[validate]` 系统             | `#[validate(required)] name`/`#[validate(length(min=3,max=20))] username`/`#[validate(range(min=0,max=150))] age`/`#[validate(regex(...))] code` | 55    |
| theme\_case          | `cx.set_theme` + CSS `var()` | `current_theme: String` → Button 切换主题,色块用 `style="background: var(--primary)"`                                                                   | 56    |
| expression\_case     | 表达式运算符 + converter 链         | `a: i32`/`b: i32`/`flag: bool`/`price: f64` → `{a+b}`/`{!flag}`/`{user.name}`/`{items.len()}`/`{price\|Currency}`                                | 57    |

### 4.3 注意事项

* **`key`/`once`/`html`** **指令**: codegen 未实现,`list_case` 不使用 `key` 指令,用 index 变量替代

* **`ObservableVec`**: 确认 scanner/codegen 对案例层 `ObservableVec<T>` 字段的版本追踪完整

* **事件支持**: `event.rs` 已支持 `on_key_down/up`、`on_mouse_down/up/move`、`on_hover/enter/leave`、`on_wheel`,keyboard\_case 可直接使用

* **slot\_template\_case vs slot\_case**: Phase 3 的 slot\_case 做基础具名插槽,Phase 4 的 slot\_template\_case 做高级(默认插槽+作用域参数)

### 验证

```powershell
cargo check -p rust-rml-demo
cargo run -p rust-rml-demo  # 目视确认 8 个新案例显示与交互
```

***

## 阶段依赖与执行顺序

```
Phase 1(编译修复) ── 必须先完成,gen_tag Bug 修复是 tag_case 升级的前置
      │
      ▼
Phase 2(API Table) ── 独立于 Phase 3/4,可并行
      │
Phase 3(凑数升级) ── tag_case 依赖 Phase 1 的 gen_tag 修复
      │                 slot_case 可与 Phase 4 的 slot_template_case 协调设计
      ▼
Phase 4(新案例) ──── list_case 注意 key 指令未实现
```

**推荐执行顺序**: Phase 1 → Phase 2 → Phase 3 → Phase 4

Phase 2 和 Phase 3 可交错:改到某个凑数案例时,顺便迁移它的 API 表到 `<Table>`。

## 风险

1. **gen\_tag 修复影响**: 修改 `container` 标志为 `true` 后,所有 `<Tag>` 标签都会处理子节点,需确认无副作用
2. **ObservableVec 案例层使用**: 仅 shell 层使用过,案例层需验证版本追踪完整性
3. **CSS 清理时机**: 必须 32 个案例全部迁移后才能删除 `.api-table` 样式
4. **i18n 文件同步**: zh-CN.json 和 en-US.json 必须同步添加键

## 验证策略

1. **编译验证**: 每个 Phase 完成后 `cargo check -p rust-rml-demo`
2. **静态检查**:

   * `grep -r "api-table" demo/src/cases/` → 0 结果(Phase 2 完成)

   * `grep -r "pub struct.*Case {}" demo/src/cases/` → 0 结果(Phase 3 完成)
3. **运行时验证**: `cargo run -p rust-rml-demo` 目视确认所有案例显示与交互
4. **生成代码审查**: 检查 `target/debug/build/rust-rml-demo-*/out/rml_generated/` 中生成代码正确性

