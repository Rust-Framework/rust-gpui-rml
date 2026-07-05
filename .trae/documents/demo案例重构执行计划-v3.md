# Demo 案例重构执行计划 v3

> 基于 gpui-component 最佳实践,系统性重构 demo 案例库
> 创建时间: 2026-07-05
> 前置上下文: v2 计划已完成 20 案例 API Table 迁移 + 8 凑数案例升级,但 badge_case.rml 因 `dot` 属性错误阻塞,且 14 案例仍待迁移

## 总览

将 34 个 demo 案例从"凑数式静态展示"重构为"充分发挥 RML 框架能力 + gpui-component 最佳实践"的黄金标准案例库。

**5 个阶段:**
- Phase 1 (P0): 修复阻塞错误 — badge_case `dot` 属性
- Phase 2 (P1): 完成 14 案例 API Table 迁移
- Phase 3 (P1): 升级 6 凑数案例 + 6 半凑数案例
- Phase 4 (P2): 新增 6 个框架能力专项案例
- Phase 5 (P3): CSS 清理 + i18n + mod.rs 注册

**验收标准:**
- `cargo check -p rust-rml-demo` 通过(0 错误)
- 34 个案例 + 6 个新案例全部使用 `<Table>` 渲染 API 文档
- 0 个空 ViewModel 凑数案例
- 每个案例至少演示 1 项 RML 框架能力(`#[computed]`/`#[command]`/`#[validate]`/`if`/`each`/`model`/`<template slot>` 任一)
- `.api-table`/`.api-row`/`.api-prop-*`/`.api-header` CSS 规则全部删除

---

## Phase 1: 修复阻塞错误 (P0)

### 1.1 修复 badge_case.rml 的 `dot` 属性错误

**问题:** Badge 在 RML 中是 `StatelessNoId`,属性注册表中**无 `dot` 条目**,仅继承通用属性(label/variants/size/disabled)。gpui-component 提供 `.dot()`/`.count()`/`.max()`/`.icon()`/`.color()` builder 方法,但 RML 编译器**未映射这些专用属性**。

**修复方案:** 移除所有 `dot` 属性用法,改用 Badge 实际支持的方式:
- 数字徽标: `<Badge>{count}</Badge>` (子节点文本)
- 变体徽标: `<Badge primary="">New</Badge>` (用 variant 替代 dot 语义)

**修改文件:**
- `demo/src/cases/badge_case.rml` — 移除 `<Badge dot="">...</Badge>` 和 `<Badge dot={show_dot}>...</Badge>`,改用 variant + count 子节点
- `demo/src/cases/badge_case.rml.rs` — 移除 `show_dot: bool` 字段和 `on_toggle_dot` 命令,改用 `variant_index: u8` 字段 + `on_cycle_variant` 命令(在 primary/secondary/danger/success 之间循环)

**验证:** `cargo check -p rust-rml-demo` 通过

---

## Phase 2: 完成 14 案例 API Table 迁移 (P1)

### 2.1 迁移模式

统一采用已建立的黄金标准模式:
```rust
// .rml.rs
use rml_ui::{TableColumn, TableRow};
use crate::cases::common::build_api_table;

pub struct XxxCase {
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    // ... 其他字段
}

impl ILifecycle for XxxCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        let (cols, rows) = build_api_table(&[
            ("prop_name", "类型", "说明"),
            // ...
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}
```

```xml
<!-- .rml -->
<Card title="API">
    <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
</Card>
```

### 2.2 待迁移案例清单 (14 个)

**批次 1 — 简单展示型 (6 案例,纯 API 迁移):**
1. `tag_case` — Tag 变体/尺寸 API
2. `progress_case` — Progress value/loading/size API
3. `progress_circle_case` — ProgressCircle value/size API
4. `button_group_case` — ButtonGroup children/vertical API
5. `avatar_group_case` — AvatarGroup limit/ellipsis API
6. `card_case` — Card title/hoverable API

**批次 2 — 状态型 (4 案例,API 迁移 + 验证现有交互):**
7. `checkbox_case` — Checkbox label/checked/disabled API (已有 is_checked/is_disabled)
8. `switch_case` — Switch label/checked/disabled API (已有 is_on/is_disabled)
9. `input_case` — Input placeholder/disabled/value API (已有 input_state)
10. `slider_case` — Slider value/min/max API

**批次 3 — 容器型 (4 案例,API 迁移 + 子节点说明):**
11. `tree_case` — Tree items/expanded API
12. `code_editor_case` — CodeEditor value/language API
13. `title_bar_case` — TitleBar children/title API
14. `native_status_bar_case` — NativeStatusBar align/children API

### 2.3 每案例验证

每完成一个案例后运行 `cargo check -p rust-rml-demo`,确保无新增错误。

---

## Phase 3: 升级凑数案例 (P1)

### 3.1 完全空 ViewModel 升级 (6 案例)

这些案例当前 `pub struct XxxCase {}` 完全无状态,仅有 `code_sample` computed。升级为真实交互:

| 案例 | 升级方案 |
|------|----------|
| `card_case` | 添加 `card_title: String` + `card_body: String` + `hoverable: bool` + `on_toggle_hoverable` 命令 + model 双向绑定演示 |
| `title_bar_case` | 添加 `title: String` + `theme: String` + `on_switch_theme` 命令(在 light/dark 之间切换) + 演示 TitleBar 内嵌 Button |
| `native_status_bar_case` | 添加 `status_text: String` + `align: String` + `on_show_ready`/`on_show_warning` 命令 + 演示状态栏动态消息 |
| `avatar_group_case` | 添加 `avatar_count: u8` + `limit: u8` + `on_add_avatar`/`on_increase_limit` 命令 + each 指令动态生成 Avatar 子节点 |
| `button_group_case` | 添加 `is_vertical: bool` + `button_count: u8` + `on_toggle_orientation`/`on_add_button` 命令 + each 指令动态生成 Button |
| `tag_case` | 添加 `variant_index: u8` + `tag_text: String` + `on_cycle_variant` 命令 + model 双向绑定 + 计算属性 variant_label |

### 3.2 半凑数案例增强 (6 案例)

这些案例已有部分状态,但交互不充分:

| 案例 | 当前状态 | 增强方案 |
|------|----------|----------|
| `progress_case` | 有 `current: f32` 但无命令 | 添加 `on_increase`/`on_decrease`/`on_toggle_loading` 命令 + `loading: bool` + 进度条动态控制 |
| `progress_circle_case` | 待验证 | 同 progress_case 模式 |
| `checkbox_case` | 有 is_checked/is_disabled + 命令 | 添加 `items: Vec<CheckboxItem>` + each 指令渲染多项 + 全选/反选命令 |
| `switch_case` | 有 is_on/is_disabled + 命令 | 添加 `settings: Vec<SettingItem>` + each 指令渲染多个 Switch 配置项 |
| `input_case` | 有 input_state | 添加 `history: Vec<String>` + `on_submit` 命令 + each 指令渲染历史记录 + `#[validate]` 演示 |
| `slider_case` | 待验证 | 添加 `current_value: f32` + `on_change` 命令 + 数值实时显示 |

### 3.3 验证

每完成一个案例后运行 `cargo check -p rust-rml-demo`,确保:
- 0 编译错误
- 案例打开后能交互(命令触发状态变化)
- model/each/if 指令正确工作

---

## Phase 4: 新增框架能力专项案例 (P2)

### 4.1 设计原则

每个新案例聚焦演示 1-2 项 RML 框架核心能力,使用 gpui-component 最佳实践组件作为载体。

### 4.2 新案例清单 (6 个)

#### 4.2.1 `validation_case` — 表单验证演示 (order 40)

**演示能力:** `#[validate]` 宏 + `model` 双向绑定 + 实时校验反馈

**ViewModel:**
```rust
pub struct ValidationCase {
    pub username: String,      // required + length 3-20
    pub email: String,         // required + regex
    pub password: String,      // required + length 8+
    pub confirm_password: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// #[validate] 字段: username (required, length 3-20), email (required, regex), password (required, length 8+)
// #[computed] is_valid: 所有字段有效
// #[command] on_submit: 校验通过时显示成功消息
```

**RML 模板:** Form 布局 + Input + 校验错误提示 + 提交按钮(disabled={!is_valid})

#### 4.2.2 `theme_case` — 主题切换演示 (order 41)

**演示能力:** `cx.set_theme()` + CSS `var()` 主题令牌 + `#[command]` 全局状态变更

**ViewModel:**
```rust
pub struct ThemeCase {
    pub current_theme: String,  // "light" / "dark"
    pub accent_color: String,   // "blue" / "green" / "purple"
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// #[command] on_switch_theme: 切换 light/dark
// #[command] on_change_accent: 切换强调色
// #[computed] theme_label: 当前主题描述
```

**RML 模板:** 主题预览卡片 + 颜色色板 + 切换按钮组

#### 4.2.3 `list_case` — 列表渲染演示 (order 42)

**演示能力:** `each` 指令 + `ObservableVec<T>` + 增删改查

**ViewModel:**
```rust
pub struct ListCase {
    pub todos: Vec<TodoItem>,  // 或 ObservableVec
    pub new_todo: String,
    pub filter: String,        // "all" / "active" / "completed"
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// #[command] on_add: 添加新待办
// #[command] on_remove(idx): 删除指定项
// #[command] on_toggle(idx): 切换完成状态
// #[computed] filtered_todos: 按 filter 过滤
// #[computed] active_count: 未完成数量
```

**RML 模板:** Input + 添加按钮 + each 渲染列表 + 过滤按钮组

#### 4.2.4 `conditional_case` — 条件渲染演示 (order 43)

**演示能力:** `if`/`else`/`show` 指令 + `#[computed]` 条件逻辑

**ViewModel:**
```rust
pub struct ConditionalCase {
    pub status: String,        // "loading" / "empty" / "loaded" / "error"
    pub item_count: u8,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// #[command] on_set_loading/on_set_empty/on_set_loaded/on_set_error
// #[command] on_add_item
// #[computed] status_message: 状态描述文本
```

**RML 模板:** 状态切换按钮组 + if/else 渲染不同状态卡片

#### 4.2.5 `slot_case` — 插槽模板演示 (order 44, 复用现有 slot_case 名)

**注意:** 现有 `slot_case` 已存在,本案例命名为 `template_slot_case` 避免冲突。

**演示能力:** `<template slot="...">` 自定义渲染 + Scoped Slot 参数

**ViewModel:**
```rust
pub struct TemplateSlotCase {
    pub users: Vec<UserRow>,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// 用户数据 + each 渲染 + 自定义 cell 模板
```

**RML 模板:** Table + `<template slot="header">` + `<template slot="cell" field="...">` + `<template slot="footer">`

#### 4.2.6 `expression_case` — 表达式与转换器演示 (order 45)

**演示能力:** 算术/比较/逻辑表达式 + converter `|` 管道 + 方法调用

**ViewModel:**
```rust
pub struct ExpressionCase {
    pub price: f32,
    pub quantity: u32,
    pub tax_rate: f32,       // 0.0-1.0
    pub currency: String,    // "CNY" / "USD"
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}
// #[computed] subtotal: price * quantity
// #[computed] tax: subtotal * tax_rate
// #[computed] total: subtotal + tax
// #[command] on_increase_quantity/on_decrease_quantity/on_change_currency
```

**RML 模板:** Input + 表达式实时计算显示 + converter 格式化货币

### 4.3 新案例注册

**修改文件:**
- `demo/src/cases/mod.rs` — 添加 6 个新模块声明
- `demo/src/cases/welcome_case.rml.rs` — `compute_grouped_items` 添加 `"framework"` 分支
- `demo/assets/i18n/zh-CN.json` + `en-US.json` — 添加 6 个新案例的 i18n 键
- `demo/src/cases/common/mod.rs` — 无需修改(build_api_table 已通用)

**分组策略:** 新案例 `group = "framework"`,在 welcome 页面单独分组展示。

---

## Phase 5: CSS 清理 + i18n + 注册 (P3)

### 5.1 删除过期 CSS 规则

**修改文件:** `demo/assets/styles.css`

**删除行:** 第 81-113 行的 7 条规则:
- `.api-table`
- `.api-row`
- `.api-row span`
- `.api-prop-name`
- `.api-prop-type`
- `.api-header`

**保留:** `.doc-pane .card, .doc-pane .code-block, .doc-pane .demo-section` 规则(从 `.api-table` 移除后,`.doc-pane .api-table` 选择器自然失效,但需移除 `.api-table` 从第 75 行的选择器列表)

### 5.2 i18n 键添加

为 Phase 4 的 6 个新案例添加 i18n 键:
- `case.validation.title` / `case.theme.title` / `case.list.title` / `case.conditional.title` / `case.template_slot.title` / `case.expression.title`
- `tree.group.framework` — 框架能力分组标签

### 5.3 最终验证

- `cargo check -p rust-rml-demo` 通过
- `cargo test -p rust-rml-engine` 通过(无回归)
- 启动 demo,逐个打开案例验证交互

---

## 执行顺序与依赖

```
Phase 1 (修复 badge) ──┐
                       ├──> Phase 2 (14 案例 API 迁移) ──┐
                       │                                  │
                       └──> Phase 3 (12 凑数升级) ────────┤
                                                          ├──> Phase 5 (清理)
                       Phase 4 (6 新案例) ────────────────┘
```

- Phase 1 必须先完成(阻塞编译)
- Phase 2/3/4 可并行(不同案例文件)
- Phase 5 必须最后(依赖前 4 阶段完成)

## 风险与缓解

| 风险 | 缓解措施 |
|------|----------|
| Badge 不支持 `dot` 等专用属性 | Phase 1 已明确改用 variant + 子节点 |
| ObservableVec 案例层支持未验证 | Phase 4 `list_case` 先用 `Vec<T>` + `__rml_bump_version`,如不支持再降级 |
| `cx.set_theme()` API 签名未知 | Phase 4 `theme_case` 实施前先探查 `crates/engine/src/` 中的主题切换实现 |
| 新案例 i18n 键遗漏 | Phase 5 统一添加,使用 `t_static` 编译期校验 |
| 凑数案例升级后行为变化 | 每案例修改后立即 `cargo check`,逐个验证 |

## 决策记录

1. **不使用 Grid 组件** — RML 当前无 `<Grid>` 组件,仅有 `h_flex`/`v_flex`。所有布局用 flex + Card 组合。
2. **不引入新宏** — 遵循用户硬约束:"Phase C is rejected; new macros should not be added"
3. **不修改 IContribution/IVisualContribution trait** — 遵循硬约束
4. **新案例 group="framework"** — 独立分组,与现有 "components"/"binding"/"menu"/"i18n" 并列
5. **Phase 4 案例命名避免冲突** — `template_slot_case` 而非 `slot_case`(已存在)
6. **build_api_table 共享工具复用** — 不修改 `common/mod.rs`,所有案例统一调用
