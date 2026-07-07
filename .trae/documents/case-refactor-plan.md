# 案例全面改造计划

> 目标：解决 `accordion_case.rml#L85` 等所有案例中 `<code>` 节点导致换行的问题，并将所有案例统一改造为基于 `CaseDocPage` 模板的工业级文档页。

## 摘要

用户反馈：所有案例的 `<code>` 标签在 RML 中映射为 `gpui::div()`（块级元素），导致行内代码强制换行，破坏阅读体验。同时，案例代码展示使用硬编码 `r#"..."#` 字符串，与实际 `.rml` / `.rml.rs` 文件脱节，看到的交互效果与代码不一致。

改造目标：
1. **修复 `<code>` 换行**：在 demo 层避免使用 `<code>` 标签（不修改框架 `tags.rs`），用纯文本表达行内代码。
2. **统一布局**：所有 57 个案例（除 welcome/overflow 简化版外）改造为 `CaseDocPage` 模板，四段式布局（标题区 + 演示区 + 代码区 + API 区）。
3. **代码即真理**：`rml_sample` / `rust_sample` 改用 `include_str!` 读取实际文件，确保展示代码与运行代码一致。
4. **内容升级**：参考 `https://longbridge.github.io/gpui-component/zh-CN/docs/components/` 官方文档风格，全面覆盖属性/事件/主题/样式/数据绑定/插槽扩展等定制能力。

## 当前状态分析

### 已完成

| 项目 | 文件 | 状态 |
|------|------|------|
| CaseDocPage 模板 | `demo/src/cases/common/case_doc_page.rml` + `.rml.rs` | 已存在 |
| CaseDocPage 样式 | `demo/assets/styles.css`（.case-layout / .case-demo-panel / .case-code-panel / .case-api-panel / .code-block） | 已存在 |
| button_case 改造 | `demo/src/cases/button_case.rml` + `.rml.rs` | 已完成（参考样板） |
| table_case 改造 | `demo/src/cases/table_case.rml` + `.rml.rs` | 已完成（参考样板） |
| event.rs apply_event slot 支持 | `crates/engine/src/compiler/event.rs` 第 107-192 行 | 已完成 |
| event.rs in_slot_context 辅助函数 | `crates/engine/src/compiler/event.rs` 第 19-26 行 | 已完成 |

### 未完成

| 项目 | 当前状态 |
|------|---------|
| `apply_action_event` slot 支持 | event.rs 第 205-231 行，仍用 `cx.listener`，未处理 slot 上下文 |
| `apply_hover_event` slot 支持 | event.rs 第 240-273 行，仍用 `cx.listener`，未处理 slot 上下文 |
| button_case 编译验证 | 改造后未通过 cargo check 验证 |
| 其余 55 个案例改造 | 全部使用旧模式（Card + Tabs + CodeEditor + 硬编码字符串 + `<code>` 标签） |

### 旧模式问题清单（以 `accordion_case.rml` 为例）

1. **`<code>` 强制换行**：第 4 行 `<code>Accordion</code>` 因 `BuiltinTag::Code => "gpui::div()"` 映射为块级元素。
2. **Card 嵌套**：用 `<Card title="演示效果">` 包裹演示，与 CaseDocPage 的 `<template slot="demo">` 重复。
3. **硬编码代码字符串**：`rml_sample()` / `rust_sample()` 用 `r#"..."#` 字符串硬编码，与实际文件内容不一致。
4. **代码展示与运行代码脱节**：硬编码字符串是简化版示例，丢失了案例中的实际演示细节。
5. **TabBar + CodeEditor 组合**：与 CaseDocPage 内置的 TabBar 切换重复。
6. **API Card 包裹**：用 `<Card title="API">` 包裹 API 表格，与 CaseDocPage 的 `<template slot="api">` 重复。

## 改造方案

### Phase A：完成框架 event.rs 修改（前置阻塞）

**目标**：让 slot 闭包内的 `on-action` / `on-hover` / `on-mouse-enter` / `on-mouse-leave` 事件也能正确生成 entity 捕获模式代码。

**修改文件**：`crates/engine/src/compiler/event.rs`

#### A.1 修改 `apply_action_event`（第 205-231 行）

在 `parts.push` 前添加 `let slot = in_slot_context();` 检测，slot 上下文内生成：

```rust
.on_action::<{type_name}>({{
    let __rml_evt_entity = __rml_self_entity.clone();
    move |_action: &{type_name}, _window: &mut gpui::Window, cx: &mut gpui::App| {{
        __rml_evt_entity.update(cx, |this, cx| {{
            this.{method}(_action, _window, cx);
        }});
    }}
}})
```

非 slot 上下文保持原 `cx.listener` 模式。

#### A.2 修改 `apply_hover_event`（第 240-273 行）

在生成 body 前添加 `let slot = in_slot_context();` 检测，slot 上下文内生成：

```rust
.on_hover({{
    let __rml_evt_entity = __rml_self_entity.clone();
    move |is_hovering: &bool, _window: &mut gpui::Window, cx: &mut gpui::App| {{
        __rml_evt_entity.update(cx, |this, cx| {{
            {body}
        }});
    }}
}})
```

其中 `body` 内的 `is_hovering` 引用保持不变（GPUI on_hover 闭包参数名）。

**验证**：`cargo check -p rust-rml-engine` 通过编译。

### Phase B：验证 button_case 编译（解锁阻塞）

**目标**：确认 Phase A 的框架修改后，button_case 的 slot 闭包内事件能正确编译。

**操作**：
1. `cargo check -p rust-rml-demo`
2. 检查 `target/debug/build/rust-rml-demo-*/out/rml_generated/button_case.rs` 中 slot 闭包事件代码是否生成 entity 捕获模式（应包含 `__rml_evt_entity.update(cx, ...)` 而非 `cx.listener`）。
3. 若编译失败，定位错误并修复。

**验证**：`cargo check -p rust-rml-demo` 通过，无编译错误。

### Phase C：批量改造案例（核心工作）

**改造模板**（参考 `button_case.rml` + `.rml.rs` 样板）：

#### C.1 `.rml` 文件改造模式

**改前**：
```rml
<component>
    <div display="flex" flex-direction="column" class="case-pane doc-pane">
        <h2>{t("case.X.title")}</h2>
        <p>... <code>X</code> 是 ... <code>Y</code> 属性 ...</p>
        <Card title="演示效果">
            <div class="demo-section">...</div>
        </Card>
        <Card title="示例代码">
            <p>点击下方 Tab 切换 <code>.rml</code> ...</p>
            <Tabs ...>
                <Tab label=".rml"><CodeEditor value={rml_sample} .../></Tab>
                <Tab label=".rml.rs"><CodeEditor value={rust_sample} .../></Tab>
            </Tabs>
        </Card>
        <Card title="API">
            <Table columns={api_columns} rows={api_rows} .../>
        </Card>
    </div>
</component>
```

**改后**：
```rml
<component>
    <CaseDocPage
        title={t("case.X.title")}
        description="X 是 ... 的组件，支持 ... 能力。描述中不使用 <code> 标签，用纯文本表达行内代码。"
        code-rml={rml_sample}
        code-rust={rust_sample}>
        <template slot="demo">
            <div class="demo-section">
                <h3>特性名</h3>
                <p>特性说明，用纯文本描述属性/事件，不用 <code> 标签。</p>
                <div class="button-row">
                    <!-- 实际演示组件 -->
                </div>
            </div>
            <!-- 多个 demo-section 覆盖不同场景 -->
        </template>
        <template slot="api">
            <h3>X</h3>
            <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
            <p>补充说明（如通用 trait 来源）。</p>
        </template>
    </CaseDocPage>
</component>
```

**简化版模板**（用于 welcome_case / overflow_test_case）：
```rml
<component>
    <CaseDocPage
        title={...}
        description="...">
        <template slot="demo">
            <!-- 仅演示内容，无 code/api 槽位 -->
        </template>
    </CaseDocPage>
</component>
```

#### C.2 `.rml.rs` 文件改造模式

**改前**：
```rust
use crate::cases::common::build_api_table;

pub struct XCase {
    pub code_tab: usize,
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    // ... 业务字段
}

impl ILifecycle for XCase {
    fn on_loaded(&mut self, ...) {
        // ... 初始化业务字段
        let (cols, rows) = build_api_table(&[...]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl XCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        r#"<component>...</component>"#.to_string()  // 硬编码
    }
    #[computed]
    pub fn rust_sample(&self) -> String {
        r#"use rml::prelude::*;..."#.to_string()  // 硬编码
    }
    #[command]
    pub fn on_code_tab_change(&mut self, idx: usize, _cx: &mut Context<Self>) {
        self.code_tab = idx;
    }
}
```

**改后**：
```rust
use crate::cases::common::{build_api_table, CaseDocPage};

pub struct XCase {
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,  // 新增
    pub api_columns: Vec<TableColumn>,
    pub api_rows: Vec<TableRow>,
    // ... 业务字段（删除 code_tab）
}

impl ILifecycle for XCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));  // 新增
        // ... 初始化业务字段
        let (cols, rows) = build_api_table(&[...]);
        self.api_columns = cols;
        self.api_rows = rows;
    }
}

impl XCase {
    #[computed]
    pub fn rml_sample(&self) -> String {
        include_str!("x_case.rml").to_string()  // 读取实际文件
    }
    #[computed]
    pub fn rust_sample(&self) -> String {
        include_str!("x_case.rml.rs").to_string()  // 读取实际文件
    }
    // 删除 on_code_tab_change（CaseDocPage 内部处理）
}
```

#### C.3 内容升级要求

每个案例的 demo-section 应全面覆盖：
- **基础用法**：最简调用
- **属性变体**：所有 variant（如 Button 的 9 种 variant、Badge 的 Number/Dot/Icon）
- **尺寸**：xsmall/small/medium/large
- **状态**：disabled/selected/loading 等
- **数据绑定**：`{field}` 形式动态绑定
- **事件回调**：`on-click={method}` 等
- **定制能力**：theme/style/slot 扩展点
- **特殊场景**：嵌套/组合/动态切换等

API 表格应包含：所有可用属性、事件、CSS 变量、插槽。

### Phase D：分批执行清单

按组件类别分批，每批 3-5 个案例并行改造，单批完成后 `cargo check` 验证。

#### 批次 1：基础组件（9 个）

- [x] button_case（已完成样板）
- [ ] button_group_case
- [ ] badge_case
- [ ] tag_case
- [ ] label_case
- [ ] separator_case
- [ ] link_case
- [ ] spinner_case
- [ ] icon_case

#### 批次 2：表单组件（8 个）

- [ ] input_case
- [ ] checkbox_case
- [ ] switch_case
- [ ] radio_case
- [ ] slider_case
- [ ] code_editor_case
- [ ] kbd_case
- [ ] tooltip_case

#### 批次 3：容器与展示（9 个）

- [ ] accordion_case
- [ ] card_case
- [ ] collapsible_case
- [ ] group_box_case
- [ ] avatar_case
- [ ] avatar_group_case
- [ ] progress_case
- [ ] progress_circle_case
- [ ] description_list_case

#### 批次 4：表格与树（3 个）

- [ ] table_case（已使用 CaseDocPage，需检查是否升级内容）
- [ ] tree_case
- [ ] pagination_case

#### 批次 5：状态与导航（6 个）

- [ ] status_bar_case
- [ ] native_status_bar_case
- [ ] title_bar_case
- [ ] tab_bar_case
- [ ] tab_preview_case
- [ ] popover_case

#### 批次 6：菜单系列（5 个）

- [ ] menu_context_case
- [ ] menu_dropdown_case
- [ ] menu_editor_case
- [ ] menu_features_case
- [ ] menu_custom_case

#### 批次 7：框架概念演示（15 个）

- [ ] counter_case
- [ ] two_way_case
- [ ] expression_case
- [ ] conditional_case
- [ ] list_case
- [ ] template_slot_case
- [ ] validation_case
- [ ] theme_case
- [ ] else_case
- [ ] once_case
- [ ] html_case
- [ ] key_case
- [ ] show_case
- [ ] ref_case
- [ ] i18n_case
- [ ] alert_case

#### 批次 8：特殊页面（2 个，简化版）

- [ ] welcome_case（仅 title+description+demo 槽，无 code/api）
- [ ] overflow_test_case（仅 title+description+demo 槽，无 code/api）

### Phase E：最终验证

1. `cargo check -p rust-rml-demo` 全量编译通过
2. `cargo test -p rust-rml-engine` 引擎测试无回归
3. 启动 demo，逐一点击所有案例确认：
   - 无 `<code>` 换行问题
   - 代码区显示的 `.rml` / `.rml.rs` 与实际文件内容一致
   - 演示效果与代码描述一致
   - API 表格信息完整
4. 抽样对比 3-5 个案例的 `rml_sample()` 返回值与 `include_str!` 文件内容字节级一致

## 假设与决策

### 决策

1. **`<code>` 修复方式 = demo 级修复**：不修改框架 `tags.rs` 的 `BuiltinTag::Code => "gpui::div()"` 映射（避免影响其他依赖该行为的代码），在案例 RML 中直接避免使用 `<code>` 标签，用纯文本表达行内代码。

2. **代码展示实现 = `include_str!`**：编译时读取实际 `.rml` / `.rml.rs` 文件，确保代码展示与运行代码字节级一致。

3. **改造范围 = 全部 57 个案例**：含组件演示、框架概念演示、特殊页面。welcome_case 和 overflow_test_case 使用简化版 CaseDocPage（仅 title+description+demo 槽，无 code/api 槽）。

4. **slot 闭包事件支持 = 框架级修复**：修改 `event.rs` 的 `apply_action_event` 和 `apply_hover_event`，在 slot 上下文内生成 entity 捕获模式代码（与 `apply_event` 保持一致）。

### 假设

1. 现有 `case_doc_page.rml` + `.rml.rs` + `styles.css` 的 CaseDocPage 实现无需修改。
2. `include_str!` 路径使用相对路径（`"x_case.rml"` / `"x_case.rml.rs"`），与 .rml.rs 文件同目录。
3. 案例改造后 `code_tab` 字段及其 `on_code_tab_change` 命令应删除（由 CaseDocPage 内部管理）。
4. `rml_editor` / `rust_editor` 等 `ref` 引用应删除（CaseDocPage 内部管理 CodeEditor）。

## 验证步骤

### Phase A 验证

```bash
cargo check -p rust-rml-engine
```
预期：编译通过，无错误。

### Phase B 验证

```bash
cargo check -p rust-rml-demo
```
预期：编译通过，无错误。检查生成的 `button_case.rs` 应包含 `__rml_evt_entity.update(cx, ...)` 模式。

### Phase C 批次验证

每批完成后：
```bash
cargo check -p rust-rml-demo
```
预期：编译通过。

### Phase E 最终验证

```bash
cargo check -p rust-rml-demo
cargo test -p rust-rml-engine
```
预期：全部通过，无回归。

启动 demo 后人工验证：
- 所有不使用 `<code>` 标签
- 代码区显示与文件内容一致
- 演示效果与代码描述一致
- API 表格信息完整

## 风险与缓解

| 风险 | 缓解 |
|------|------|
| `apply_action_event` / `apply_hover_event` 修改影响其他生成代码 | 仅在 `in_slot_context() == true` 时切换分支，非 slot 上下文保持原 `cx.listener` 模式 |
| 改造案例数量大（55 个），单次易出错 | 分 8 批，每批 3-9 个，每批完成后 cargo check 验证 |
| `include_str!` 路径错误 | 路径与 .rml.rs 文件同目录，使用相对路径 |
| 案例业务字段遗漏 | 改造模板明确保留业务字段和 #[command] 方法，仅替换布局和 code_sample 实现 |
| 特殊页面（welcome/overflow）改造后失去原有功能 | 使用简化版 CaseDocPage，仅替换外层 div，保留内部业务逻辑 |

## 执行顺序

1. Phase A：修改 `event.rs`（约 30 分钟）
2. Phase B：验证 button_case 编译（约 5 分钟）
3. Phase C：按批次 1→8 顺序改造（每批约 30-60 分钟，总计约 6-8 小时）
4. Phase E：最终验证（约 30 分钟）

总预估：7-10 小时。
