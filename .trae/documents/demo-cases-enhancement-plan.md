# RML Demo 案例增强计划

## Context

RML 框架的 demo 案例是用户学习框架的主要途径。当前 12 个已注册组件的案例存在三类问题：

1. **遗留模式**：6 个案例（Input/Tree/Accordion/Alert/Avatar/Popover）使用独立布局而非 `CaseDocPage` 共享模板，代码样例是缩略硬编码字符串（与真实 demo 不一致），且描述中大量使用 `<code>` 标签导致显示异常。
2. **功能覆盖不足**：多数案例仅 4-7 个功能小节，官方文档通常有 8-15 个。Input 和 Tree 各仅 1 节。
3. **代码样例失真**：遗留案例的 `rml_sample`/`rust_sample` 是手写缩略字符串，与实际 demo 脱节。

**目标**：每个案例页面都能让开发者学会对应组件的 RML 用法——每个功能小节、每条示例代码、每行 API 说明都必须有实际教学价值，拒绝"为了做而做"。

## 核心原则

- **拒绝凑数**：每个 demo-section 必须教一个独立 RML 概念（ref 指令 / model 绑定 / computed / command / slot / if 条件渲染等）。官方文档有但 RML 无对应属性映射的特性不加。
- **代码真实性**：所有案例改用 `include_str!("xxx_case.rml")` / `include_str!("xxx_case.rml.rs")`，代码区展示磁盘文件全文，杜绝样例与 demo 漂移。
- **纯文本描述**：移除所有 `<code>` 标签，描述用纯文本。
- **先验证后实现**：每个待加 section 先确认 RML 框架真实支持该属性/事件/slot，不支持则跳过。

## Demo 页面标准（所有案例必须满足）

1. 使用 `<CaseDocPage>` 模板，demo 内容放 `<template slot="demo">`，API 放 `<template slot="api">`
2. 结构体含 `case_doc_page: Option<gpui::Entity<CaseDocPage>>` 字段
3. 代码样例用 `include_str!` 引用真实文件
4. 无 `code_tab` 字段、无 `on_code_tab_change` 命令（CaseDocPage 内部处理 Tab 切换）
5. 无 `<code>` 标签、无 `<Card>`/`<TabBar>`/`<Tabs>` 脚手架
6. API 表格用 `build_api_table`，内容准确反映 RML 属性
7. 每个 `<div class="demo-section">` 教一个独立价值点

## 工作顺序

### Phase 1：基础组件（9 个）

按"缺口大小"排序，缺口最大的先做：

| 序号 | 组件 | 当前状态 | 工作内容 |
|---|---|---|---|
| 1 | **Input** | 1 节，遗留，有 `<code>` | 迁移 + 大幅补全（最重点，教学价值最高） |
| 2 | **Accordion** | 6 节，遗留，有 `<code>` | 纯迁移 + 去 code 标签 |
| 3 | **Alert** | 7 节，遗留，有 `<code>` | 纯迁移 + 去 code 标签 |
| 4 | **Avatar** | 4 节，遗留，有 `<code>` | 迁移 + 微调 |
| 5 | **Button** | 9 节，已规范 | 补 icons / custom children（需验证） |
| 6 | **Badge** | 5 节，已规范 | 验证 offset 是否支持，补则加 |
| 7 | **Checkbox** | 5 节，已规范 | 验证 indeterminate 是否支持 |
| 8 | **Icon** | 6 节，已规范 | 核对准确性 |
| 9 | **Tooltip** | 4 节，已规范 | 已完整，仅核对 |

### Phase 2：布局与高级组件（3 个）

| 序号 | 组件 | 当前状态 | 工作内容 |
|---|---|---|---|
| 10 | **Tree** | 1 节，遗留，有 `<code>` | 迁移 + 大幅补全 |
| 11 | **Popover** | 3 节，遗留，有 `<code>` | 迁移 + 补 slot/受控概念 |
| 12 | **Table** | 6 节，已规范 | 删 1 处 `<code>` 标签 |

## 逐组件规格

### Input（重点）
- 基础用法 + ref 指令（Stateful 组件惰性创建 Entity 机制）
- placeholder / default_value 设置时机（演示 on_loaded 后通过 ElementRef.with_mut 设置的正确模式——解决当前注释中的悬而未决问题）
- disabled 禁用（组件属性 vs State 属性区分）— 需验证
- 尺寸 size（Sizable trait 通用属性）— 需验证
- prefix / suffix 插槽（具名 slot 模式）— 需验证 RML Input 是否支持
- model 双向绑定（核心 RML 概念）— 需验证 Input 是否在 model 白名单
- 事件监听 on-change（command 事件绑定 + 回调签名）— 需验证事件名
- 多输入表单组合（多 ref 字段 + 表单组合）

### Accordion
- 6 个现有 section 全部保留（basic/multiple/sizes/icon/disabled/nested 各有教学价值）
- 纯迁移到 CaseDocPage + include_str! + 去 `<code>`

### Alert
- 7 个现有 section 保留（variant 关联函数/属性、title+banner、message 优先级、icon、on_close+if 条件渲染、size）
- 纯迁移 + 去 `<code>`

### Avatar
- 迁移 + 将"内容模式"拆为 src/name/placeholder 各独立 section
- 保留"动态绑定"section（model + computed + if 三联，高价值）

### Button
- 补 icons 图标按钮（Button + Icon 组合）— 需验证
- 补 custom children 自定义子节点 — 需验证
- 跳过：dropdown caret / toggle group / custom variant（属于独立组件或 RML 无法声明式表达）

### Badge / Checkbox / Icon / Tooltip
- 验证少量属性是否支持，补则加，不补则维持现状

### Tree（重点）
- 基础用法 + TreeState 初始化（on_loaded 中 cx.new 创建）
- 展开节点 expanded — 需验证
- 多级嵌套树（child 链式构造）
- on-activate vs on-select 事件签名差异 — 需验证
- 跳过：虚拟滚动、异步加载（RML 无对应 API）

### Popover
- 迁移 + 补 slot="trigger" 机制（具名 slot 路由到 .trigger()）
- 补受控展开（若支持 on-open/on-close 事件）— 需验证

### Table
- 删除 table_case.rml 中 1 处 `<code>` 标签

## 单组件工作流（6 步）

1. **验证 RML 支持**：读 rml_ui 中该组件的属性白名单/compiler 模块，确认计划中每个 section 的属性/事件/slot 真实存在，删除不支持的
2. **迁移 .rml.rs**：加 case_doc_page 字段、删 code_tab、改 include_str!、删 on_code_tab_change
3. **迁移 .rml**：套 CaseDocPage、demo 移入 slot="demo"、删脚手架、去 `<code>`
4. **补全 feature section**：逐个加 demo-section，对应 command/computed 在 .rml.rs 实现
5. **校准 API 表格**：build_api_table 三元组与真实 RML 属性对齐
6. **编译 + 运行验证**：cargo build 通过，逐 section 交互，代码区与 demo 行为一致

## 关键文件

- `demo/src/cases/common/case_doc_page.rml` — CaseDocPage 模板（所有迁移的目标结构）
- `demo/src/cases/common/case_doc_page.rml.rs` — CaseDocPage 结构体定义
- `demo/src/cases/common/mod.rs` — build_api_table 工具函数
- `demo/src/cases/badge_case.rml.rs` — 规范模式标准范本（include_str! + case_doc_page 字段）
- `demo/src/cases/input_case.rml.rs` — 遗留模式范本（迁移工作量最大）
- `crates/engine/src/tags.rs` — 组件注册表（验证属性支持）
- `crates/engine/src/compiler/` — 各组件 codegen 模块（验证属性支持）

## 验证方法

1. `cargo build -p rust-rml-demo` 零错误零警告（特别关注未使用字段/导入警告）
2. Grep 全局确认：12 个案例都有 `include_str!`、无 `on_code_tab_change`、无 `code_tab` 字段、.rml 文件无 `<code>` 标签
3. 运行 demo，逐个案例逐 section 交互，对照代码区显示的源码确认一致
4. 每个组件 section 清单与官方文档特性逐项对照，未覆盖项标注"RML 不支持故跳过"

## 风险

1. **Input placeholder 时机**是最大不确定性。当前注释坦承 on_loaded 阶段 ref 未填充。实施前必须确认 RML 的 ref 延迟注入机制，案例应演示正确解法。
2. **"需验证"项较多**：RML 是 gpui-component 的声明式子集，不能假设底层特性都有 RML 映射。Step 1 验证会筛掉一部分，符合"拒绝为了做而做"。
3. **include_str! 展示全文**：代码区会显示完整 .rml/.rml.rs（含 CaseDocPage 样板），保证真实性优先于精简度。
