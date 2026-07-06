# RML Showcase Demo 案例页 UI/内容迭代计划

## 1. Summary（概述）

RML Showcase 的演示案例页目前普遍存在以下问题：

- 视觉风格不统一：大量圆角白卡片堆砌，信息密度低，不符合“大工厂”式工业后台审美。
- 内容错误/错位：示例代码与主题不搭、文案复制粘贴错误（如 Button 页出现 `tab-bar`）、`order` 冲突、贡献点重复注册、案例未注册等。
- 交互示例不完整：部分页面没有实时渲染效果（如 TabBar 页仅显示文字），部分案例只有单段代码、缺少 `.rml` / `.rml.rs` 双 Tab 对照。
- 维护成本高：每个案例都独立手写“标题 + 说明 + 演示 + 代码 + API”的重复布局。

本计划目标：

1. 引入一个共享的 `CaseDocPage` 页面模板（Rust builder 形式），统一所有案例页结构。
2. 修正所有已发现的注册、排序、文案、示例错误。
3. 将案例页视觉改向扁平、密集、低圆角的工业后台风格。
4. 补齐每个案例的实时预览与双 Tab 代码示例，确保示例与主题一致。

## 2. Current State Analysis（现状分析）

基于对 `demo/src/cases/`、`demo/src/shell/`、`demo/assets/styles.css`、`demo/assets/i18n/` 的阅读和探索代理的审计结果：

### 2.1 结构与注册问题

| 文件 | 问题 |
|------|------|
| `demo/src/cases/button_case.rml.rs` | `order = 11`，与 `tab_bar_case.rml.rs` 冲突。 |
| `demo/src/cases/tab_bar_case.rml.rs` | `order = 11`，与 `button_case.rml.rs` 冲突。 |
| `demo/src/cases/avatar_case.rml.rs` | `order = 12`，与 `status_bar_case`、`slot_case` 冲突。 |
| `demo/src/cases/status_bar_case.rml.rs` | `order = 12`；同一文件内 `StatusReady` 被 `#[contribute(id = "status.ready")]` 注册了两次（line 81 与 line 119）。 |
| `demo/src/cases/slot_case.rml.rs` | `order = 12`，与上面冲突；更关键的是该案例实际演示的是 `Card` 组件，但贡献点 id 为 `components.slot`，与已存在的 `card_case` 重复。 |
| `demo/src/cases/overflow_test_case.rml.rs` | 只有 `#[component]`，没有 `#[contribute]`，未注册到左侧案例树。 |

### 2.2 内容与文案问题

| 文件 | 问题 |
|------|------|
| `demo/src/cases/button_case.rml` line 4 | 描述里混入了 `tab-bar`（从 TabBar 页复制粘贴未改）。 |
| `demo/src/cases/code_editor_case.rml` line 9 | `<CodeEditor ref="editor_state" />` 没有 `value`，实时预览区域为空。 |
| `demo/src/cases/title_bar_case.rml` line 41 | `<h3>TitleBar</h3>` 放在 API Card 外部，破坏 `doc-pane > *` 统一宽度。 |
| `demo/src/cases/native_status_bar_case.rml` | 存在类似的 `<h3>` 外置问题。 |
| `demo/src/cases/counter_case.rml` | 仅 `<CodeEditor value={code_sample} />`，无语言、无双 Tab。 |
| `demo/src/cases/two_way_case.rml` | 同上。 |
| `demo/src/cases/i18n_case.rml` | 同上（需验证）。 |
| `demo/src/cases/slot_case.rml` | 同上。 |
| `demo/src/cases/theme_case.rml` 等 Phase 4 / M1' 指令案例 | 多数没有代码示例区域或示例不完整。 |

### 2.3 视觉与布局问题

| 文件 | 问题 |
|------|------|
| `demo/assets/styles.css` | `.case-pane` 内边距 24px、居中对齐，导致内容区窄；`.doc-pane > *` 最大宽度 960px，一屏信息少；卡片圆角大、阴影重。 |
| 各 `*_case.rml` | 大量 `<Card>` 套 `<Card>`，章节之间缺少统一间距与分隔，代码/演示未并置。 |

### 2.4 引擎能力约束（影响方案选择）

- 当前 RML 用户自定义组件（`#[component]`）不支持通过属性传参：codegen 只处理 `<template slot="...">` 子节点，忽略组件标签上的属性。
- 使用自定义 RML 组件需要父视图持有 `Option<Entity<T>>` 字段并在 `on_loaded` 中初始化，每个案例都加字段会增加重复代码。
- 因此共享模板采用 **Rust builder 组件**（`CaseDocPage` struct + `render()` 方法），在 RML 中通过 `<component content={self.render_doc_page(_window, cx)} />` 嵌入，既满足“共享模板”又避免每个案例维护 Entity 字段。

## 3. Goals & Scope（目标与范围）

**在范围内：**

- 新增/修改 `demo/src/cases/common/` 中的共享页面模板与辅助工具。
- 修改 `demo/assets/styles.css` 统一案例页视觉。
- 修改 `demo/src/cases/*_case.rml` 与 `*_case.rml.rs`，统一使用新模板并补齐双 Tab 代码示例。
- 修正注册/排序/文案错误。
- 更新 `demo/assets/i18n/zh-CN.json` 与 `en-US.json` 中新增的必要文案键。

**不在范围内：**

- 不修改 RML 引擎/编译器。
- 不新增宏（符合 Phase C 约束）。
- 不改动 `MainWindow` / `TabWindow` / `ActivityPanel` 等 shell 核心逻辑（除非为修复注册问题所需的最小调整）。
- 不改动组件库 `rml_ui` 本身。

## 4. Design Decisions（设计决策）

1. **共享模板形式：Rust builder `CaseDocPage`**
   - 位置：`demo/src/cases/common/case_doc_page.rs`，由 `common/mod.rs` 导出。
   - 提供链式 API：`CaseDocPage::new().title(...).description(...).demo(...).code_rml(...).code_rust(...).api(...).render(window, cx)`。
   - 每个案例保留一个 `render_doc_page` 方法，返回 `gpui::AnyElement`，在 `.rml` 中用 `<component content={self.render_doc_page(_window, cx)} />` 嵌入。

2. **页面结构统一为 4 段式**
   - 标题区：组件中文名 + 英文名 + 一句话概述。
   - 演示区：左侧/上方实时渲染效果。
   - 代码区：`.rml` / `.rml.rs` 双 Tab `CodeEditor`。
   - API 区：统一 `Table` 表格（保留 `bordered`/`stripe`）。

3. **视觉风格：工业后台/大工厂**
   - 降低圆角（2–4px），减少白色卡片与阴影。
   - 章节改用扁平带底边框的 `.case-section`，信息密度提高。
   - 演示区与代码区在足够宽度下左右并置；窄窗口自动上下堆叠（通过 flex wrap）。
   - 语义色（Danger/Success/Warning/Info）只用于有语义的场景，避免 pastel 低对比度。

4. **代码示例策略**
   - 每个案例必须提供 `rml_sample` 与 `rust_sample` 两个 `#[computed]` 方法。
   - 示例代码为**精选片段**，突出该案例主题，不要求与文件内容逐字一致，但必须与标题/演示一致。
   - 统一使用 `language="rml"` / `language="rust"`。

5. **属性命名**
   - RML 中继续使用 kebab-case（如 `selected-index`、`on-click`），不使用下划线，符合 label-width 规范。
   - Rust 方法名仍用 snake_case。

## 5. Implementation Steps（实施步骤）

### Step 1：基础设施 — 共享模板与样式

**目标：** 建立统一的案例页模板和基础 CSS，后续案例直接复用。

1.1 在 `demo/src/cases/common/` 下新建 `case_doc_page.rs`。

- 定义 `pub struct CaseDocPage`。
- 字段：title、description、demo element builder、rml sample string、rust sample string、api element builder、code_tab index。
- 方法：
  - `new()`
  - `title(mut self, impl Into<SharedString>)`
  - `description(mut self, impl Into<SharedString>)`
  - `demo<F: Fn(...) -> AnyElement>(mut self, f: F)`
  - `code_rml(mut self, String)`
  - `code_rust(mut self, String)`
  - `api<F: Fn(...) -> AnyElement>(mut self, f: F)`
  - `render(self, window, cx) -> AnyElement`
- `render` 输出结构：
  - 外层 `div().class("case-pane doc-pane")`。
  - 标题 + 描述。
  - `.case-layout` 容器，左侧 `.case-demo-panel`，右侧 `.case-code-panel`。
  - `.case-api-panel` 放置 API 表格。
  - 代码面板内使用 `TabBar` + `CodeEditor`（`.rml` / `.rml.rs`），Tab 切换状态由 `CaseDocPage` 内部持有。
  - 可选：在代码面板标题行放一个“复制”按钮，调用 `cx.write_to_clipboard` 复制当前代码。

1.2 修改 `demo/src/cases/common/mod.rs`。

- `pub mod case_doc_page;`
- 重新导出：`pub use case_doc_page::CaseDocPage;`
- 保留 `build_api_table`，并新增 convenience：`pub fn build_api_table_with_default(props: &[(&str, &str, &str, &str)])`（可选，若默认列成本不高则直接扩展原函数签名）。

1.3 修改 `demo/assets/styles.css`。

- `.case-pane`：padding 改为 `16px 24px`，移除 `align-items: center` / `text-align: center`。
- `.doc-pane > *`：max-width 改为 `1200px`。
- 新增 `.case-layout`：`display: flex; flex-wrap: wrap; gap: 24px; align-items: flex-start;`。
- 新增 `.case-demo-panel`：`flex: 1 1 420px; min-width: 320px;`。
- 新增 `.case-code-panel`：`flex: 1 1 420px; min-width: 320px;`。
- 新增 `.case-section`：`padding: 16px; border: 1px solid var(--border-color); border-radius: 2px; background: var(--surface); margin-bottom: 8px;`。
- 新增 `.case-desc`：`color: var(--text-muted); font-size: 14px; max-width: 800px; line-height: 1.6;`。
- 新增 `.demo-frame`：`border: 1px dashed var(--border-color); padding: 12px; border-radius: 2px;`（用于 TitleBar/StatusBar 等需要边界才能看清的组件）。
- 调整 `.button-row` gap 为 `8px`，移除 `justify-content: center` 默认左对齐。
- 调整 `.demo-section`：padding 改为 `12px 0`，保留底边框但改为实线。

### Step 2：修正注册、排序与重复贡献点

2.1 重新分配 `components` 组的 `order`，避免冲突。

建议新顺序（保持现有相对顺序，仅去重）：

- `button` 11
- `tab_bar` 12
- `avatar` 13
- `slot` 14
- `status_bar` 15
- `accordion` 16
- `table` 17
- `description_list` 18
- `badge` 19
- `label` 20
- `separator` 21
- `tag` 22
- `progress` 23
- `progress_circle` 24
- `button_group` 25
- `avatar_group` 26
- `card` 27
- `title_bar` 28
- `native_status_bar` 29
- `checkbox` 30
- `switch` 31
- `input` 32
- `tree` 33
- `slider` 34
- `code_editor` 35
- `icon` 36
- `kbd` 37
- `tooltip` 38
- `popover` 39

2.2 修复 `demo/src/cases/status_bar_case.rml.rs`。

- 删除文件末尾重复的 `#[contribute(id = "status.ready", ...)]` 块（line 119–139）。
- 保留 `ensure_status_ready_registered` 与 `register_visual_ability::<StatusReady>` 调用。

2.3 处理 `demo/src/cases/overflow_test_case.rml.rs`。

- 选项 A（推荐）：给它补一个 `#[contribute(host_id = "demo.shell", id = "framework.overflow", kind = "case", group = "framework", order = ...)]`，并在 RML 中展示 overflow/滚动相关 demo。
- 选项 B：若该案例已废弃，从 `demo/src/cases/mod.rs` 中移除其 `pub mod overflow_test_case;`。
- **计划采用选项 A**，因为它能增加一个框架能力示例。

### Step 3：文案与示例修正（先改一批典型页面）

3.1 `demo/src/cases/button_case.rml`

- 修正 line 4 的描述，删除 `tab-bar` 字样。
- 描述改为：`Button 是 RML 的按钮组件，标签为 <code>Button</code>。支持 9 种 variant、4 种尺寸、disabled/selected/loading 状态、compact 模式与 tooltip。`
- 每个变体按钮下方增加文字标签，说明对应 variant。
- `rml_sample` / `rust_sample` 已存在，保留并精简。

3.2 `demo/src/cases/progress_case.rml` / `.rml.rs`

- 文案已较好，主要迁移到新模板。
- 在“尺寸”演示区为每种尺寸增加文字标签。

3.3 `demo/src/cases/tab_bar_case.rml` / `.rml.rs`

- 将顶部大段说明拆分为：标题下方一句概述 + API 表格。
- 确保“演示效果”区域真正渲染出所有 variant/size/icon/menu/header/body 示例。
- `rml_sample` 中把 `underline` variant 补上（当前示例里漏了）。

3.4 `demo/src/cases/slot_case.rml` / `.rml.rs`

- `slot_case` 实际演示的是 `Card` 组件，且与已有的 `card_case` 内容重复。由于 `card_case` 已覆盖更完整的 Card API（borderless、cover、footer 等），本计划决定**移除 `slot_case`**：
  - 删除 `demo/src/cases/slot_case.rml`
  - 删除 `demo/src/cases/slot_case.rml.rs`
  - 从 `demo/src/cases/mod.rs` 中移除 `pub mod slot_case;`
  - 从 i18n 文件中移除 `case.slot.*` 键
- `components.slot` 贡献点 id 随之退役；外部链接若指向该 id，会自然失效（左侧树不再显示该节点）。

3.5 `demo/src/cases/code_editor_case.rml` / `.rml.rs`

- 给基础用法 `CodeEditor` 设置 `value={rml_sample}` 或一段默认 Rust 代码，避免空白。
- 保持 `ref="editor_state"` 演示引用能力。

### Step 4：批量迁移剩余案例到新模板

对每个案例文件执行以下操作：

4.1 `.rml.rs` 侧统一变更

- 保留 `#[contribute]`、`IContribution`、`ILifecycle`、字段、命令。
- 若还没有 `code_tab`，删除它（由 `CaseDocPage` 内部管理）。
- 确保存在 `rml_sample` 与 `rust_sample` 两个 `#[computed]` 方法；若只有 `code_sample`，拆分为两个。
- 新增方法：

```rust
pub fn render_doc_page(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
    CaseDocPage::new()
        .title(t_static("case.xxx.title"))
        .description("一句话概述...")
        .demo(|window, cx| self.render_demo(window, cx))
        .code_rml(self.rml_sample())
        .code_rust(self.rust_sample())
        .api(|window, cx| self.render_api(window, cx))
        .render(window, cx)
}
```

- 提供 `render_demo` 与 `render_api` 辅助方法（直接在 Rust 中构建 element，避免在 RML 中重复布局）。

4.2 `.rml` 侧统一变更

- 文件内容简化为：

```rml
<component>
    <component content={self.render_doc_page(_window, cx)} />
</component>
```

- 删除所有 `<Card>`、`<h2>`、`<p>` 说明、`<TabBar>` 代码切换等重复布局代码。
- 仅保留那些必须直接在 RML 中演示的特性（如 `template slot="header"`、双向绑定 `input model={...}`），这些作为 `render_demo` 的返回值内部再嵌入 RML 模板或 GPUI element。对于复杂 RML 片段，可以继续在该案例自己的 `.rml` 里写 `<div class="case-section">...</div>` 并通过 `render_demo` 返回该片段。

4.3 需要迁移的案例清单

按组分批处理：

- **Binding 组**：`counter_case`, `two_way_case`。
- **i18n 组**：`i18n_case`。
- **Menu 组**：`menu_context_case`, `menu_dropdown_case`, `menu_editor_case`, `menu_features_case`, `menu_custom_case`。
- **Framework 组**：`expression_case`, `conditional_case`, `list_case`, `template_slot_case`, `validation_case`, `theme_case`, `overflow_test_case`。
- **指令组**：`else_case`, `once_case`, `html_case`, `key_case`, `show_case`, `ref_case`。
- **组件组其余**：`accordion_case`, `avatar_case`, `avatar_group_case`, `badge_case`, `button_group_case`, `card_case`, `checkbox_case`, `description_list_case`, `icon_case`, `input_case`, `kbd_case`, `label_case`, `native_status_bar_case`, `popover_case`, `progress_circle_case`, `separator_case`, `slider_case`, `switch_case`, `table_case`, `tag_case`, `title_bar_case`, `tooltip_case`, `tree_case`。
- **删除**：`slot_case`（与 `card_case` 重复，不再迁移）。

### Step 5：欢迎页与 i18n 更新

5.1 `demo/src/cases/welcome_case.rml.rs`

- 保持卡片式总览不变，但调整卡片尺寸与间距以匹配新的密集风格。
- 可选：为每个分组增加小标题样式。

5.2 `demo/assets/i18n/zh-CN.json` / `en-US.json`

- 新增通用键：
  - `case.common.demo` = “演示效果” / "Live Demo"
  - `case.common.code` = “示例代码” / "Code Sample"
  - `case.common.api` = “API” / "API"
  - `case.common.copy` = “复制” / "Copy"
- 检查并补全每个案例可能缺失的标题键。

### Step 6：细节打磨

6.1 代码面板

- 确保 `CodeEditor` 有 `language` 属性。
- 控制代码区最大高度 360px（与项目约束一致），必要时使用 CSS 或组件属性限制。
- “复制”按钮复制当前选中 Tab 的内容。

6.2 API 表格

- 统一使用 `build_api_table`。
- 属性名不使用下划线（RML 属性用 `-`，但表格里列出的是属性名，可用 `on-click` 等 kebab-case）。
- 类型列使用等宽字体或标签样式（可选）。

6.3 演示区

- 组件案例必须展示真实组件实例。
- 对需要在边界中才能看清的组件（TitleBar、StatusBar、NativeStatusBar）使用 `.demo-frame`。
- 状态切换类案例（Button、Progress、Switch 等）提供实时操作控件与状态文本。

## 6. File Changes Overview（文件变更总览）

### 新增

- `demo/src/cases/common/case_doc_page.rs` — 共享页面模板。

### 修改

- `demo/src/cases/common/mod.rs` — 导出 `CaseDocPage`，扩展 `build_api_table`（如需要）。
- `demo/assets/styles.css` — 新案例页视觉样式。
- `demo/src/cases/mod.rs` — 删除 `pub mod slot_case;`；保留 `overflow_test_case` 并补充 `#[contribute]`。
- `demo/src/cases/status_bar_case.rml.rs` — 删除重复 `StatusReady` 注册。
- `demo/src/cases/overflow_test_case.rml.rs` — 补充 `#[contribute]` 与内容。
- `demo/src/cases/*_case.rml` — 全部简化为调用 `render_doc_page`。
- `demo/src/cases/*_case.rml.rs` — 统一 order、补齐 `rml_sample`/`rust_sample`、新增 `render_doc_page`/`render_demo`/`render_api`。
- `demo/src/cases/welcome_case.rml.rs` — 微调样式。
- `demo/assets/i18n/zh-CN.json` / `en-US.json` — 新增通用键、补齐标题、删除 `case.slot.*`。

### 删除

- `demo/src/cases/slot_case.rml`
- `demo/src/cases/slot_case.rml.rs`

## 7. Verification Steps（验证步骤）

1. **编译**
   - `cargo check -p rust-rml-demo`（或项目实际 package 名）无错误。
   - 确认无重复 `#[contribute]` id 导致的运行期警告。

2. **运行时基础检查**
   - 启动 Demo，左侧案例树无重复项、无缺失项。
   - 点击每个案例树节点，Tab 能正常打开，关闭后激活策略符合预期（关闭 index N 激活 N-1）。

3. **页面内容检查**
   - 每个案例页显示：标题、描述、演示区、`.rml` / `.rml.rs` 双 Tab 代码区、API 区。
   - 演示区确实渲染出对应组件/特性，不是空白或纯文字。
   - 代码示例与案例主题一致，无复制粘贴错误。

4. **已知问题回归检查**
   - Button 页描述不再出现 `tab-bar`。
   - CodeEditor 页基础用法有默认代码内容。
   - TabBar 页展示所有 variant。
   - Card 页展示 Card 效果与代码；`slot_case` 已从左侧树移除。
   - StatusBar 页底部状态栏无重复注册。

5. **视觉检查**
   - 页面不再过度使用圆角卡片，章节间距统一。
   - 左右分栏在 1100px 窗口下正常，缩小后自动堆叠。
   - Warning/Info 等语义色对比度可接受。

6. **i18n 检查**
   - 切换英文后，新增键与案例标题正确显示。

## 8. Risks & Notes（风险与备注）

- **工作量较大**：涉及约 40+ 个案例文件。建议按 Step 4 分批提交/验证，不要一次全部重写。
- **RML 自定义组件属性限制**：若后续引擎支持自定义组件属性，可将 `CaseDocPage` 从 Rust builder 迁移为 RML 组件，届时只需改动 `common/case_doc_page.rml` 与引用方式。
- **CodeEditor 高度限制**：需确认 `CodeEditor` 当前是否支持 `max-h` 或必须靠外层容器限制；计划优先通过外层 `.case-code-panel` 的 `max-height` 约束。
- **复制按钮**：若 `cx.write_to_clipboard` 在当前 GPUI 封装中不可用，可降级为仅显示代码，不阻塞主流程。
- **不引入新宏**：所有共享逻辑用普通 Rust builder 实现，符合 Phase C 约束。
