# RML Demo 案例官方文档化增强计划

## 摘要

将 12 个已注册 RML 组件的 demo 案例对齐 gpui-component 官方文档标准。每个案例页面教会开发者：组件能做什么、RML 怎么声明、业务场景怎么用。每个功能小节、每条示例代码、每行 API 说明都必须有实际教学价值，拒绝"为了做而做"。

## 当前状态分析

### 12 个组件当前状态（基于实际代码扫描）

| # | 组件 | 模式 | `<code>` 标签 | 现有 section 数 | 工作量 |
|---|---|---|---|---|---|
| 1 | Input | Legacy | 是 | 1 | **极大**：迁移+大幅补全 |
| 2 | Tree | Legacy | 否（无） | 1 | **极大**：迁移+大幅补全 |
| 3 | Accordion | Legacy | 是 | 6 | 中：纯迁移+去 code |
| 4 | Alert | Legacy | 是 | 7 | 中：纯迁移+去 code |
| 5 | Avatar | Legacy | 是 | 4 | 中：迁移+小补全 |
| 6 | Popover | Legacy | 是 | 3 | 中：迁移+小补全 |
| 7 | Table | Canonical | 是（1 处） | 6 | 小：仅去 1 处 code |
| 8 | Button | Canonical | 否 | 9 | 小：核对准确性 |
| 9 | Badge | Canonical | 否 | 5 | 小：核对 |
| 10 | Checkbox | Canonical | 否 | 5 | 小：核对 |
| 11 | Icon | Canonical | 否 | 6 | 小：核对 |
| 12 | Tooltip | Canonical | 否 | 4 | 小：核对 |

### 模式判定依据

- **Canonical（规范）**：含 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段 + `include_str!` + 无 `code_tab` 字段
- **Legacy（遗留）**：含 `code_tab: usize` 字段 + 手写 `rml_sample`/`rust_sample` 字符串 + 无 `case_doc_page` 字段

### 已规范案例清单（无需迁移）

badge_case / button_case / button_group_case / checkbox_case / icon_case / kbd_case / label_case / link_case / separator_case / spinner_case / switch_case / tag_case / table_case / tooltip_case

### 需迁移案例清单（6 个）

input_case / accordion_case / alert_case / avatar_case / tree_case / popover_case

## RML 框架能力验证（Phase 1 关键产出）

### Input 组件真实能力（通过源码 `crates/ui/src/input/input.rs` 验证）

`rml_ui::Input`（实际为 `gpui_component::input::Input`）的 builder 方法：

| 方法 | RML 是否暴露 | 备注 |
|---|---|---|
| `prefix(element)` | ✗ 未在 codegen 映射 | 仅 Tabs/TabBar/Tab 有 prefix/suffix 映射 |
| `suffix(element)` | ✗ 未在 codegen 映射 | 同上 |
| `h_full()` / `h(height)` | ✓ 走 style 属性 `height` | CodeEditor 专属支持 |
| `appearance(bool)` | ✗ 未在 codegen 映射 | - |
| `bordered(bool)` | ✗ 未在 codegen 映射 | CodeEditor 专属 `.focus_bordered(false)` |
| `focus_bordered(bool)` | ✗ 未在 codegen 映射 | - |
| `cleanable(bool)` | ✗ 未在 codegen 映射 | - |
| `mask_toggle()` | ✗ 未在 codegen 映射 | - |
| `disabled(bool)` | ✓ 通用静态/bind 属性 | - |
| `tab_index(isize)` | ✗ 未在 codegen 映射 | - |
| `size` (Sizable) | ✓ 通用静态属性 | - |
| `selected` (Selectable) | ✓ 通用静态/bind 属性 | - |
| `placeholder` | ⚠ 错误映射 | codegen 生成 `.placeholder()`，但 Input 无此方法（仅 InputState 有） |
| `default_value` | ✗ InputState builder 专属 | RML 不暴露 |
| `masked` | ✗ InputState builder 专属 | RML 不暴露 |
| `validate` / `pattern` / `mask_pattern` | ✗ InputState builder 专属 | RML 不暴露 |

### InputState 通过 ElementRef 可访问的方法

`rml_ui::InputState` builder 方法（需在 `state_ctor` 中调用，或通过 `ElementRef.with_mut` 在首次渲染后修改）：

- `placeholder(text)` - 占位文本
- `default_value(text)` - 默认值
- `masked(bool)` - 密码掩码
- `cleanable(bool)` / `clean_on_escape()` - 可清空
- `validate(closure)` / `pattern(regex)` - 校验
- `mask_pattern(pattern)` - 掩码格式
- `set_placeholder(text, cx)` / `set_value(text, cx)` - 运行时修改

### Input 事件（通过 `cx.subscribe` 订阅 `InputEvent`）

- `on_change` → `InputEvent::Change`
- `on_enter` → `InputEvent::PressEnter`
- `on_focus` → `InputEvent::Focus`
- `on_blur` → `InputEvent::Blur`

回调签名：`fn(&mut self, &InputState, &mut Context<Self>)`（通过 `entity.read(cx)` 取得 state）

### 两种 Input 声明模式

1. **PascalCase Stateful 模式**：`<Input ref="state_field" disabled={...} on-change={...} />`
   - 惰性创建 `Entity<InputState>`，由 `__rml_populate_refs` 注入到 `ElementRef<InputState>` 字段
   - 适合需要事件监听、命令式访问（focus、set_value 等）的场景
   - placeholder 等需在 InputState builder 上设置（`state_ctor` 已硬编码，需通过 `ElementRef.with_mut` 在首次渲染后修改）

2. **lowercase model 模式**：`<input model={field} placeholder="..." />`
   - 小写 `input` 是内置 HTML 标签，触发 `gen_model_input` 路径
   - 通过 `__rml_get_or_init_input_state(field, placeholder, ...)` 创建 InputState
   - 支持 placeholder 静态属性（直接传入 InputState builder）
   - 自动双向同步：UI↔VM 字段
   - 自动错误提示（field_errors + Tooltip）
   - 适合表单字段、受控输入

## 核心原则

1. **拒绝凑数**：每个 `demo-section` 必须教一个独立 RML 概念。官方文档有但 RML 无对应属性映射的特性不加（如 Input 的 prefix/suffix/cleanable/masked 等）。
2. **代码真实性**：所有案例用 `include_str!("xxx_case.rml")` / `include_str!("xxx_case.rml.rs")`，代码区展示磁盘文件全文。
3. **纯文本描述**：移除所有 `<code>` 标签，描述用纯文本（必要时用引号包裹标识符）。
4. **先验证后实现**：每个待加 section 先确认 RML 框架真实支持该属性/事件/slot，不支持则跳过。
5. **RML 优先**：突出 RML 声明式优势（model 双向绑定、ref 指令、computed 桥接、command 事件、if/each 指令），不照搬官方命令式示例。

## Demo 页面统一标准

1. 使用 `<CaseDocPage>` 模板，demo 内容放 `<template slot="demo">`，API 放 `<template slot="api">`
2. 结构体含 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段
3. 代码样例用 `include_str!` 引用真实文件
4. 无 `code_tab` 字段、无 `on_code_tab_change` 命令（CaseDocPage 内部处理 Tab 切换）
5. 无 `<code>` 标签、无 `<Card>`/`<TabBar>`/`<Tabs>` 脚手架
6. API 表格用 `build_api_table`，内容准确反映 RML 真实属性
7. 每个 `<div class="demo-section">` 教一个独立价值点

## 实施顺序（按工作量降序）

### Phase 1：高工作量组件（2 个）

| 序号 | 组件 | 工作内容 |
|---|---|---|
| 1 | **Input** | 迁移到 CaseDocPage + 大幅补全（基础/ref、placeholder 时机、disabled、size、model 双向绑定、on-change 事件、多输入表单） |
| 2 | **Tree** | 迁移到 CaseDocPage + 大幅补全（基础+TreeState 初始化、expanded 展开、嵌套树、on-activate vs on-select） |

### Phase 2：中工作量组件（4 个）

| 序号 | 组件 | 工作内容 |
|---|---|---|
| 3 | **Accordion** | 纯迁移到 CaseDocPage + include_str! + 去 `<code>` |
| 4 | **Alert** | 纯迁移 + 去 `<code>` |
| 5 | **Avatar** | 迁移 + 小补全（拆分 src/name/placeholder 各独立 section） |
| 6 | **Popover** | 迁移 + 补 slot="trigger" 机制说明 |

### Phase 3：小工作量组件（6 个）

| 序号 | 组件 | 工作内容 |
|---|---|---|
| 7 | **Table** | 删除 1 处 `<code>` 标签（line 52） |
| 8 | **Button** | 核对 API 表格准确性 |
| 9 | **Badge** | 核对 |
| 10 | **Checkbox** | 核对 |
| 11 | **Icon** | 核对 |
| 12 | **Tooltip** | 核对 |

## 逐组件规格

### 1. Input 案例（重点）

**当前状态**：1 节，Legacy，有 `<code>` 标签，rml_sample/rust_sample 是手写缩略字符串。

**目标 section 清单**（每个教一个独立 RML 概念）：

1. **基础用法 + ref 指令**：`<Input ref="input_state" />` + ElementRef<InputState> 字段，演示 Stateful 组件惰性创建机制
2. **placeholder 设置时机**：演示 ElementRef.with_mut 在首次渲染后设置 placeholder 的正确模式（解决当前注释中的悬而未决问题）
3. **disabled 禁用**：`<Input disabled={is_disabled} />` 组件属性切换
4. **尺寸 size**：`<Input size="small" />` 等（Sizable trait 通用属性）
5. **model 双向绑定**（核心 RML 概念）：`<input model={user_name} placeholder="..." />` 小写 builtin tag + 双向同步 + 错误提示
6. **事件监听 on-change**：`<Input ref="..." on-change={on_input_change} />` + cx.subscribe 机制 + 回调签名 `fn(&mut self, &InputState, &mut Context<Self>)`
7. **多输入表单组合**：多个 ref 字段 + 多个 model 字段混合使用，演示真实表单场景

**API 表格内容**（与 RML 真实支持对齐）：
- `ref` - 字符串 - 元素引用名（绑定到 ElementRef<InputState> 字段）
- `disabled` - 布尔/绑定 - 禁用状态（Input 组件属性）
- `size` - xsmall/small/medium/large - 尺寸（Sizable trait）
- `selected` - 布尔/绑定 - 选中态（Selectable trait）
- `on_change` - 事件 - 内容变化回调（参数：&InputState）
- `on_enter` - 事件 - 回车按下回调
- `on_focus` - 事件 - 获得焦点回调
- `on_blur` - 事件 - 失去焦点回调
- `model` - 指令 - 双向绑定（仅小写 `<input>` 标签支持）
- `placeholder` - 字符串 - 占位文本（仅小写 `<input model={...}>` 支持，PascalCase Input 需通过 InputState 设置）

**跳过的官方特性**（RML 不支持）：
- prefix/suffix（codegen 未映射）
- cleanable/mask_toggle/appearance/bordered/focus_bordered（codegen 未映射）
- masked/default_value（InputState builder 专属，RML 不暴露）
- validate/pattern/mask_pattern（InputState builder 专属）

### 2. Tree 案例（重点）

**当前状态**：1 节，Legacy，无 `<code>` 标签，但代码示例是手写缩略字符串。

**目标 section 清单**：

1. **基础用法 + TreeState 初始化**：on_loaded 中 cx.new 创建 TreeState + TreeItem::new 链式构造 + on-activate 事件
2. **展开节点 expanded**：TreeItem.expanded(true) 控制初始展开
3. **多级嵌套树**：child(TreeItem) 链式构造深层结构
4. **on-activate vs on-select 事件差异**：激活（叶子节点点击）vs 选择（节点焦点变化）

**API 表格内容**：
- `ref` - 字符串 - 元素引用名（绑定到 Option<Entity<TreeState>> 字段）
- `on_activate` - 事件 - 激活节点回调（叶子节点点击）
- `on_select` - 事件 - 选择节点回调

**跳过的官方特性**：虚拟滚动、异步加载（RML 无对应 API）

### 3. Accordion 案例

**当前状态**：6 节，Legacy，有 `<code>` 标签。

**目标**：纯迁移到 CaseDocPage + include_str! + 去 `<code>`。保留所有 6 个 section（basic/multiple/sizes/icon/disabled/nested）。

### 4. Alert 案例

**当前状态**：7 节，Legacy，有 `<code>` 标签。

**目标**：纯迁移 + 去 `<code>`。保留所有 7 个 section（variant 关联函数/属性、title+banner、message 优先级、icon、on_close+if 条件渲染、size）。

### 5. Avatar 案例

**当前状态**：4 节，Legacy，有 `<code>` 标签。

**目标**：迁移 + 将"内容模式"拆为 src/name/placeholder 各独立 section。保留"动态绑定"section（model + computed + if 三联，高价值）。

### 6. Popover 案例

**当前状态**：3 节，Legacy，有 `<code>` 标签。

**目标**：迁移 + 补 slot="trigger" 机制说明（具名 slot 路由到 .trigger()）。

### 7. Table 案例

**当前状态**：6 节，Canonical，1 处 `<code>` 标签（line 52）。

**目标**：仅删除 1 处 `<code>` 标签，改用纯文本描述。

### 8-12. Button / Badge / Checkbox / Icon / Tooltip

**目标**：核对 API 表格准确性，验证现有 section 完整性，必要时微调。无 `<code>` 标签需删除。

## 单组件工作流（6 步）

每个组件按以下顺序执行：

1. **验证 RML 支持**（仅 Input/Tree 需要）：读 `crates/ui/src/components/<comp>.rs` 或 `~/.cargo/git/checkouts/gpui-component-*/crates/ui/src/<comp>/` 中源码，确认计划中每个 section 的属性/事件/slot 真实存在，删除不支持的
2. **迁移 .rml.rs**（仅 Legacy 组件）：加 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段、删 `code_tab` 字段、改 `include_str!`、删 `on_code_tab_change` 命令
3. **迁移 .rml**（仅 Legacy 组件）：套 `<CaseDocPage>`、demo 移入 `<template slot="demo">`、删 `<Card>`/`<TabBar>` 脚手架、去 `<code>` 标签
4. **补全 feature section**：逐个加 `<div class="demo-section">`，对应 command/computed 在 .rml.rs 实现
5. **校准 API 表格**：`build_api_table` 三元组与 RML 真实属性对齐
6. **编译 + 运行验证**：`cargo build -p rust-rml-demo` 通过，逐 section 交互验证

## 关键文件路径

### 共享模板
- `demo/src/cases/common/case_doc_page.rml` - CaseDocPage 模板
- `demo/src/cases/common/case_doc_page.rml.rs` - CaseDocPage 结构体
- `demo/src/cases/common/mod.rs` - `build_api_table` 工具函数

### 12 个目标案例
- `demo/src/cases/input_case.rml` / `.rml.rs`
- `demo/src/cases/tree_case.rml` / `.rml.rs`
- `demo/src/cases/accordion_case.rml` / `.rml.rs`
- `demo/src/cases/alert_case.rml` / `.rml.rs`
- `demo/src/cases/avatar_case.rml` / `.rml.rs`
- `demo/src/cases/popover_case.rml` / `.rml.rs`
- `demo/src/cases/table_case.rml` / `.rml.rs`
- `demo/src/cases/button_case.rml` / `.rml.rs`
- `demo/src/cases/badge_case.rml` / `.rml.rs`
- `demo/src/cases/checkbox_case.rml` / `.rml.rs`
- `demo/src/cases/icon_case.rml` / `.rml.rs`
- `demo/src/cases/tooltip_case.rml` / `.rml.rs`

### 规范模式参考
- `demo/src/cases/badge_case.rml.rs` - canonical 范本（include_str! + case_doc_page 字段）
- `demo/src/cases/badge_case.rml` - canonical 范本（CaseDocPage + template slot）

### RML 框架验证源
- `crates/engine/src/tags.rs` - 组件注册表
- `crates/engine/src/compiler/component.rs` - 通用 setter 映射
- `crates/engine/src/compiler/input/event.rs` - Input 事件订阅
- `crates/engine/src/compiler/codegen/binding.rs` - model 双向绑定代码生成
- `crates/engine/src/props_registry.rs` - 组件属性注册表
- `~/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/<latest>/crates/ui/src/<comp>/` - gpui-component 源码

## 验证方法

1. **编译验证**：`cargo build -p rust-rml-demo` 零错误零警告（特别关注未使用字段/导入警告）
2. **Grep 全局校验**：
   - 12 个案例都有 `include_str!`
   - 12 个案例都有 `case_doc_page` 字段
   - 12 个案例都无 `on_code_tab_change` 命令
   - 12 个案例都无 `code_tab` 字段
   - 12 个 `.rml` 文件都无 `<code>` 标签
3. **运行验证**：`cargo run -p rust-rml-demo`，逐个案例逐 section 交互，对照代码区显示的源码确认一致
4. **教学价值校验**：每个 section 标题+描述能独立回答"这个 section 教什么 RML 概念"

## 假设与决策

1. **`placeholder` 属性在 `<Input>` 上的处理**：codegen 当前会生成 `.placeholder(...)` 但 Input 无此方法（编译错误）。Input 案例中**不使用** `<Input placeholder="..." />` 这种写法，改为：
   - PascalCase 模式：通过 `ElementRef.with_mut` 在首次渲染后调用 `set_placeholder`
   - lowercase model 模式：`<input model={field} placeholder="..." />`（已支持）
2. **代码区展示全文**：`include_str!` 会显示完整 .rml/.rml.rs（含 CaseDocPage 样板），保证真实性优先于精简度
3. **canonical 案例的微调**：仅核对 API 表格准确性，不重写已工作的 section
4. **不新增 RML 框架特性**：如发现官方文档有但 RML 未映射的特性（如 Input.prefix/suffix/cleanable），在案例中跳过，不在此任务中扩展 codegen

## 风险

1. **Input placeholder 时机**：codegen 当前生成 `.placeholder(...)` 会编译失败。需在迁移时删除 Input 上的 placeholder 属性，改用 ElementRef.with_mut 或 lowercase model 模式。
2. **include_str! 自引用**：`include_str!("input_case.rml")` 会包含 `include_str!` 自身这行，形成"自引用"。这是预期行为（展示完整文件），但代码区会显示递归字符串。
3. **canonical 案例的微调可能引入回归**：Button/Badge 等已工作，修改 API 表格时需谨慎，仅更新内容不重构结构。
