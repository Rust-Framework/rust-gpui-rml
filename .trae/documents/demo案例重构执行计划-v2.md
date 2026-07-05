# Demo 案例重构执行计划 v2

## 摘要

用户批评:"demo 的案例代码没有采用最佳组件进行构建,比如 grid 或 table 组件展示 api 说明等,case 中大多数 rml 页面均为了凑数,没有充分发挥 RML 框架的优势和能力。"

本计划是 v1 计划(`demo案例全面重构计划.md`)的延续与细化,基于已完成的 Phase 1 和 Phase 2 起步工作,聚焦剩余 31 个案例的 API 表格迁移、13 个凑数案例升级、8 个框架能力专项演示。

## 当前状态分析

### 已完成
- **Phase 1**: `gen_tag` 子节点 Bug 修复([crates/engine/src/compiler/tag.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/tag.rs) + [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) Tag `container: true`),`cargo check` 通过
- **Phase 2 起步**: 共享工具 [demo/src/cases/common/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/common/mod.rs) 已创建,提供 `build_api_table` 函数;[counter_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml.rs) + [counter_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml) 已迁移为黄金标准
- **table_case**: 已使用 `<Table>` 组件,无需迁移

### 待完成
1. **31 个案例 API 表格迁移**: 仍使用手工 `<div class="api-table">` 拼凑
2. **13 个凑数案例升级**: 空 ViewModel `pub struct XxxCase {}`,无状态/交互/数据绑定
3. **8 个框架能力专项演示**: `#[computed]`/`if`/`each`/键盘事件/具名插槽/`#[validate]`/主题/表达式
4. **CSS 清理**: styles.css 中 7 条 `.api-table`/`.api-row` 系列规则待删除

### 框架能力实勘结论(影响 Phase 4 设计)
- **已实现**: `if`/`else`/`each`/`show`/`ref`/`model` 指令;`#[computed]`/`#[command]`/`#[validate]`(required/length/range/regex);`ObservableVec<T>` 案例层支持;`cx.set_theme()` + CSS `var()`;`<template slot>` + `<slot>`;表达式(算术/比较/逻辑/方法调用/converter `|`)
- **未实现/静默**: `key`/`once`/`html` 指令(parsed 但 codegen 不消费);三元 `?:` 运算符;`Grid` 组件(只有 `h_flex`/`v_flex`);多参数 `WithArgs`(仅用首个参数)
- **事件**: `on-click`/`on-key-down`/`on-key-up`/`on-mouse-*`/`on-wheel`/`on-hover`/`on-mouse-enter`/`on-mouse-leave`(元素级);`on-change`(仅 Input 组件级)

## 改造方案

### Phase 2: API Table 标准化(剩余 31 案例)

#### 迁移模式(已在 counter_case 验证)

ViewModel 端(.rml.rs):
```rust
use rml_ui::{TableColumn, TableRow};
use crate::cases::common::build_api_table;

pub struct XxxCase {
    // 既有字段...
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
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

#### 批次执行顺序(按 mod.rs 注册顺序,每批 5-6 个)

**批次 1**(binding 组 + 早期 components):
- `two_way_case` (order 2)
- `button_case` (order 11) — 已有真实交互,仅需加 api 字段
- `accordion_case` (order 3)
- `tab_bar_case` (order 4)
- `avatar_case` (order 12,凑数) — Phase 3 一并升级
- `i18n_case` (order 21,凑数) — Phase 3 一并升级

**批次 2**(menu 组 + status):
- `menu_context_case` (order 5)
- `menu_dropdown_case` (order 6)
- `menu_editor_case` (order 7)
- `menu_features_case` (order 8)
- `menu_custom_case` (order 9)
- `status_bar_case` (order 12,凑数) — Phase 3 一并升级

**批次 3**(slot + description):
- `slot_case` (order 12,凑数) — Phase 3 一并升级
- `description_list_case` (order 16)
- `badge_case` (order 22,凑数) — Phase 3 一并升级
- `label_case` (order 23,凑数) — Phase 3 一并升级
- `separator_case` (order 24,凑数) — Phase 3 一并升级

**批次 4**(component demos 中段):
- `tag_case` (order 25,凑数) — Phase 3 一并升级
- `progress_case` (order 26)
- `progress_circle_case` (order 27)
- `button_group_case` (order 28,凑数) — Phase 3 一并升级
- `avatar_group_case` (order 29,凑数) — Phase 3 一并升级

**批次 5**(component demos 后段):
- `card_case` (order 30,凑数) — Phase 3 一并升级
- `title_bar_case` (order 31,凑数) — Phase 3 一并升级
- `native_status_bar_case` (order 32,凑数) — Phase 3 一并升级
- `checkbox_case` (order 33)
- `switch_case` (order 34)

**批次 6**(component demos 末段):
- `input_case` (order 35)
- `tree_case` (order 36)
- `slider_case` (order 37)
- `code_editor_case` (order 38)

每批完成后:`cargo check -p rust-rml-demo`

#### CSS 清理(全部迁移后)

**文件**: [demo/assets/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css)

删除第 73 行 `.api-table`(在 `.doc-pane` 组合选择器中)、第 81-113 行 `.api-table`/`.api-row`/`.api-row span`/`.api-prop-name`/`.api-prop-type`/`.api-header` 6 条规则。保留 `.doc-pane .card`/`.code-block`/`.demo-section`。

### Phase 3: 凑数案例升级(13 案例)

#### 设计原则
每个凑数案例添加:至少 1 个 `pub` 字段 + 1 个 `#[command]` 方法 + 1 个 `#[computed]` 方法 + 事件绑定,并演示一项 RML 核心能力。

#### 升级清单

| 案例 | 字段 | 命令 | computed | 交互设计 | 演示能力 |
|---|---|---|---|---|---|
| `badge_case` | `count: i32`, `show_dot: bool` | `on_increment`, `on_toggle_dot` | `badge_label`(>99 显示"99+") | Button 控制 count,Badge 显示 | `#[computed]` 条件逻辑 |
| `label_case` | `text: String`, `weight: u8` | `on_cycle_weight` | `weight_label` | Input model={text} + Label 动态字重 | `model` 双向绑定 |
| `separator_case` | `is_vertical: bool`, `is_dashed: bool` | `on_toggle_orientation`, `on_toggle_dashed` | — | Button 切换方向/样式 | `if` 指令条件渲染 |
| `tag_case` | `tags: Vec<(String,String)>` | `on_add_tag`, `on_cycle_variant`(WithArgs) | — | `each={tag in tags}` 循环渲染 | `each` 列表渲染 |
| `avatar_case` | `name: String`, `size_index: u8` | `on_cycle_size`, `on_update_name` | `size_label` | Input model={name} + Button 切换尺寸 | `model` + `#[computed]` |
| `avatar_group_case` | `members: Vec<String>` | `on_add_member`, `on_remove_member`(WithArgs) | `member_count` | `each` 循环 + 动态增删 | `each` + `ObservableVec` |
| `button_group_case` | `buttons: Vec<(String,String)>` | `on_add_button`, `on_click_button`(WithArgs) | — | `each` 循环动态生成按钮 | `each` + WithArgs |
| `card_case` | `click_count: i32`, `hoverable: bool` | `on_card_click`, `on_toggle_hoverable` | `count_text` | Card 内 Button 点击计数 | `#[computed]` + 事件冒泡 |
| `title_bar_case` | `title: String`, `subtitle: String` | `on_set_title` | — | Input model={title} 修改 TitleBar | `model` 双向绑定 |
| `native_status_bar_case` | `status_text: String`, `item_count: i32` | `on_update_status`, `on_inc_count` | `status_summary` | Input 修改状态栏文本 | `model` + `#[computed]` |
| `i18n_case` | `current_lang: String`, `switch_count: i32` | `on_switch_en`, `on_switch_zh`, `on_toggle_theme` | `lang_label` | Button 切换语言/主题,显示切换次数 | `t()` + 主题系统 |
| `status_bar_case` | `status_message: String`, `last_action: String` | `on_show_ready`, `on_show_case` | — | Button 切换状态栏贡献文本 | status kind 贡献 |
| `slot_case` | `card_title: String`, `card_body: String` | `on_update_content` | — | 自定义 `SlottedCard` 用户组件 + `<template slot>` | 具名插槽 |

#### tag_case 关键代码模式

```rust
pub struct TagCase {
    pub tags: Vec<(String, String)>,  // (label, variant)
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl ILifecycle for TagCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut Context<Self>) {
        self.tags = vec![
            ("Default".into(), "default".into()),
            ("Primary".into(), "primary".into()),
            ("Success".into(), "success".into()),
        ];
        let (cols, rows) = build_api_table(&[
            ("variant", "关联函数", "Tag::primary()/Tag::success() 等"),
            ("closable", "bool", "可关闭标签"),
        ]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl TagCase {
    #[command]
    pub fn on_add_tag(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.tags.push(("New".into(), "default".into()));
    }

    // WithArgs 单参数:传递 tag index
    #[command]
    pub fn on_cycle_variant(&mut self, idx_str: &str, _: &ClickEvent, cx: &mut Context<Self>) {
        if let Ok(idx) = idx_str.parse::<usize>() {
            if let Some(tag) = self.tags.get_mut(idx) {
                let variants = ["default", "primary", "success", "warning", "danger"];
                let cur = variants.iter().position(|v| *v == tag.1).unwrap_or(0);
                tag.1 = variants[(cur + 1) % variants.len()].into();
            }
        }
    }
}
```

```xml
<div each={tag in tags} h-flex="" gap-2="">
    <Tag primary={tag.1 == "primary"} success={tag.1 == "success"} 
        warning={tag.1 == "warning"} danger={tag.1 == "danger"}>
        {tag.0}
    </Tag>
    <Button label="切换 variant" on-click={on_cycle_variant, idx} />
</div>
```

> 注意:each 循环中无法直接获取 index,需用 `enumerate` 模式或在外层 Vec 中存储 index。实际实现时可能需要调整数据结构为 `Vec<(usize, String, String)>` 或使用单独的 idx 字段。

### Phase 4: 新增框架能力演示案例(8 案例)

#### 4.1 新增案例分组

在 [demo/src/cases/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/mod.rs) 新增 `framework` 组案例注册。

在 [demo/assets/i18n/zh-CN.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json) + [en-US.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/en-US.json) 添加键:
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

#### 4.2 新增案例清单

| 案例 | order | 演示能力 | 字段/命令设计 | 框架能力实勘调整 |
|---|---|---|---|---|
| `computed_case` | 50 | `#[computed]` 依赖追踪+缓存 | `first_name`/`last_name`/`base_price`/`tax_rate` → computed 链: `full_name` → `display_text`, `tax_amount` → `total_price` | ✅ 直接可用 |
| `conditional_case` | 51 | `if`/`else`/`show` 指令 | `count: i32`/`show_detail: bool` → `if={count > 10}` 显示不同内容; `show={show_detail}` 控制显隐 | ✅ 直接可用 |
| `list_case` | 52 | `each` 循环 + `ObservableVec` | `todos: ObservableVec<String>` → `each={todo in todos}` 渲染 + WithArgs 删除 | ⚠️ 不用 `key` 指令(codegen 不消费),用 index 变量 |
| `keyboard_case` | 53 | `on-key-down`/`on-key-up` | `last_key: String`/`key_count: i32` → div 监听键盘事件显示按键 | ✅ 元素级事件已支持 |
| `slot_template_case` | 54 | 用户组件具名插槽 | `#[component(slots=["header","body","footer"])]` + `<template slot>` + `<slot name>` | ✅ 直接可用 |
| `validation_case` | 55 | `#[validate]` 系统 | `#[validate(required)] name`/`#[validate(length(min=3,max=20))] username`/`#[validate(range(min=0,max=150))] age`/`#[validate(regex="...")] code` | ✅ 所有规则可用 |
| `theme_case` | 56 | `cx.set_theme` + CSS `var()` | `current_theme: String` → Button 切换主题,色块用 `style="background: var(--primary)"` | ✅ 直接可用 |
| `expression_case` | 57 | 表达式运算符 + converter 链 | `a: i32`/`b: i32`/`flag: bool`/`price: f64` → `{a + b}`/`{!flag}`/`{items.len()}`/`{price \| Currency}` | ⚠️ 不用三元 `?:`,用 `if`/`else` 或 computed 替代 |

#### 4.3 关键案例设计

**list_case**(避免 `key` 指令):
```rust
pub struct ListCase {
    pub todos: ObservableVec<String>,
    pub new_todo: String,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
}

impl ListCase {
    #[command]
    pub fn on_add_todo(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if !self.new_todo.trim().is_empty() {
            self.todos.push(self.new_todo.trim().to_string());
            self.new_todo.clear();
        }
    }

    // WithArgs 单参数:传递 todo index
    #[command]
    pub fn on_remove_todo(&mut self, idx_str: &str, _: &ClickEvent, cx: &mut Context<Self>) {
        if let Ok(idx) = idx_str.parse::<usize>() {
            self.todos.remove_where(|_, i| i == idx);
        }
    }
}
```

```xml
<div each={todo in todos} h-flex="" gap-2="">
    <span>{todo}</span>
    <Button label="删除" danger="" on-click={on_remove_todo, idx} />
</div>
<!-- each 循环中 idx 变量需要框架支持,若不支持则改用 Vec<(String, usize)> 结构 -->
```

> **风险**: `each` 循环中是否提供 index 变量需验证。若不提供,`list_case` 改用 `Vec<(String, usize)>` 结构,循环变量为 `(todo, idx)` 元组。

**expression_case**(避免三元运算符):
```xml
<!-- 不使用三元 ?: -->
<p>加法: {a + b}</p>
<p>逻辑非: {!flag}</p>
<p>方法调用: {todos.len()}</p>
<p>converter 链: {price | Currency}</p>

<!-- 条件显示用 if 指令替代三元 -->
<div if={flag}>
    <p>flag 为 true 时显示</p>
</div>
<div else="">
    <p>flag 为 false 时显示</p>
</div>
```

## 假设与决策

1. **已有计划延续**: Phase 1 已完成(gen_tag 修复 + counter_case 迁移),本计划从 Phase 2 剩余 31 案例开始执行
2. **框架能力边界**: 基于实勘结论调整 Phase 4 设计 — 不使用 `key`/`once`/`html` 指令、三元 `?:`、多参数 WithArgs
3. **批次粒度**: 每批 5-6 个案例,每批后 `cargo check`,避免一次性大量改动导致编译错误堆积
4. **Phase 2/3 交错**: 凑数案例(13 个)在迁移 API 表格的同时升级为真实交互,不分离两次改动
5. **CSS 清理时机**: 31 个案例全部迁移后统一删除 styles.css 中 `.api-table` 系列规则
6. **i18n 同步**: zh-CN.json + en-US.json 必须同步添加 Phase 4 新案例的键
7. **each 循环 index**: 待验证框架是否提供循环 index 变量;若不提供,用 Vec 元组结构绕过

## 验证步骤

### Phase 2 完成
```powershell
# 确认无残留手工 API 表格
grep -r "api-table" demo/src/cases/  # 应返回 0 结果
cargo check -p rust-rml-demo
```

### Phase 3 完成
```powershell
# 确认无空 ViewModel
grep -r "pub struct.*Case {}" demo/src/cases/  # 应返回 0 结果
cargo check -p rust-rml-demo
```

### Phase 4 完成
```powershell
cargo check -p rust-rml-demo
cargo run -p rust-rml-demo  # 目视确认 8 个新案例显示与交互
```

### 全部完成
- `grep -r "api-table" demo/src/cases/` → 0 结果
- `grep -r "pub struct.*Case {}" demo/src/cases/` → 0 结果
- styles.css 中 `.api-table`/`.api-row`/`.api-prop-*`/`.api-header` 规则已删除
- demo/src/cases/mod.rs 新增 8 个 framework 组案例
- zh-CN.json + en-US.json 同步新增 framework 组 i18n 键
- `cargo check -p rust-rml-demo` 0 errors
- `cargo run -p rust-rml-demo` 目视确认所有案例显示与交互正常

## 风险与缓解

1. **each 循环 index 变量**: 框架可能不提供循环 index。缓解:用 `Vec<(T, usize)>` 元组结构,或在 on_loaded 中预计算 index
2. **Tag variant 关联函数 codegen**: `<Tag primary={cond}>` 的布尔绑定是否正确生成 `Tag::primary()`?需验证 `gen_tag` 对 variant 属性的处理
3. **ObservableVec 案例层版本追踪**: scanner 对 `#[component]` 结构的 `ObservableVec<T>` 字段应自动生成 `__rml_get_version` 分支,但需实际编译验证
4. **CSS 清理副作用**: 删除 `.api-table` 规则后,若仍有案例未迁移会样式丢失。缓解:严格按批次执行,最后一批迁移完再清理 CSS
5. **Phase 4 新案例 group 注册**: `framework` 组需要在 welcome_case 的 `compute_grouped_items` 中识别(已有 `binding`/`components`/`i18n`/`menu` 分支),需添加 `framework` 分支

## 执行顺序

```
Phase 2 批次 1 (6 案例) → cargo check
    ↓
Phase 2 批次 2 (6 案例) → cargo check
    ↓
Phase 2 批次 3 (5 案例,含 3 个凑数 → 同步 Phase 3 升级) → cargo check
    ↓
Phase 2 批次 4 (5 案例,含 3 个凑数 → 同步 Phase 3 升级) → cargo check
    ↓
Phase 2 批次 5 (5 案例,含 3 个凑数 → 同步 Phase 3 升级) → cargo check
    ↓
Phase 2 批次 6 (4 案例) → cargo check
    ↓
Phase 2 CSS 清理 → cargo check
    ↓
Phase 3 剩余凑数案例升级(批次 1-2 中的凑数案例) → cargo check
    ↓
Phase 4 新增 8 个 framework 案例 → cargo check + cargo run
```

> **优化**: Phase 2 和 Phase 3 在批次 3-5 中交错执行(凑数案例同时迁移 API 表格 + 升级交互),减少重复改动。
