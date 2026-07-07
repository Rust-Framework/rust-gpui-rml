# 案例全面改造计划 v2

> 目标：解决 `accordion_case.rml#L85` 等所有案例中 `<code>` 节点导致换行的问题，并将所有 57 个案例统一改造为基于 `CaseDocPage` 模板的工业级文档页，代码展示使用 `include_str!` 读取实际文件，组件描述参考 gpui-component 官方文档风格。

## 摘要

用户反馈（来自 `accordion_case.rml#L85` 的 `<code>` 换行问题）：
- 所有案例的 `<code>` 标签在 RML 中映射为 `gpui::div()`（块级元素），导致行内代码强制换行
- 案例代码展示使用硬编码 `r#"..."#` 字符串，与实际 `.rml` / `.rml.rs` 文件脱节
- 案例内容缺乏对组件能力的全面覆盖

改造目标：
1. **修复 `<code>` 换行**：在 demo 层避免使用 `<code>` 标签（不修改框架 `tags.rs`），用纯文本表达行内代码
2. **统一布局**：所有 57 个案例改造为 `CaseDocPage` 模板，四段式布局（标题区 + 演示区 + 代码区 + API 区）
3. **代码即真理**：`rml_sample` / `rust_sample` 改用 `include_str!` 读取实际文件
4. **内容升级**：参考 gpui-component 官方文档风格，全面覆盖属性/事件/主题/样式/数据绑定/插槽扩展等定制能力

## 当前状态分析

### 已完成

| 项目 | 文件 | 状态 |
|------|------|------|
| CaseDocPage 模板 | `demo/src/cases/common/case_doc_page.rml` + `.rml.rs` | 已存在 |
| CaseDocPage 样式 | `demo/assets/styles.css` | 已存在 |
| button_case 改造 | `demo/src/cases/button_case.rml` + `.rml.rs` | 已完成（参考样板，使用 CaseDocPage + include_str!） |
| table_case 改造 | `demo/src/cases/table_case.rml` + `.rml.rs` | 部分完成（使用 CaseDocPage，但仍是硬编码字符串） |
| slot 闭包签名升级 | `crates/engine/src/compiler/user_component.rs` | 已完成（3 参数签名 `_scope: &dyn ISlotScope`） |
| `<slot>` 占位符传 NullSlotScope | `crates/engine/src/compiler/codegen/node.rs` | 已完成 |
| TabWindowShell slot 闭包包装 | `crates/engine/src/compiler/codegen/shell.rs` | 已完成（`wrap_shell_slot` 辅助函数） |
| `in_slot_context()` 辅助函数 | `crates/engine/src/compiler/event.rs` 第 24-26 行 | 已定义 |

### 未完成（关键阻塞）

| 项目 | 当前状态 | 影响 |
|------|---------|------|
| `apply_event` slot 上下文支持 | `crates/engine/src/compiler/event.rs` 第 81-128 行，**始终生成 `cx.listener`**，未检测 `in_slot_context()` | slot 闭包内 `cx` 类型为 `&mut App`，`cx.listener` 不可用，导致编译失败 |
| `apply_action_event` slot 上下文支持 | `crates/engine/src/compiler/event.rs` 第 141-167 行，同上 | 同上 |
| `apply_hover_event` slot 上下文支持 | `crates/engine/src/compiler/event.rs` 第 176-... 行，同上 | 同上 |
| button_case 编译验证 | 因上述框架问题，button_case.rml 中 slot 闭包内的事件（如 `on-click={on_basic_click}`）编译失败 | 阻塞所有含事件的案例改造 |
| 其余 55 个案例改造 | 全部使用旧模式（Card + Tabs + CodeEditor + 硬编码字符串 + `<code>` 标签） | 需批量改造 |

### 关键发现

通过检查 `target/debug/build/rust-rml-demo-*/out/rml_generated/button_case.rs`，发现：
- slot 闭包签名已是 3 参数（`_scope: &dyn ISlotScope, _window, cx: &mut App`）— user_component.rs 修改生效
- 但闭包内事件仍生成 `cx.listener(move |this, ev, ...)` — `apply_event` 未检测 slot 上下文
- `in_slot_context()` 辅助函数已定义但**从未被调用**

## 改造方案

### Phase A：修复框架 event.rs（前置阻塞）

**目标**：让 `apply_event`、`apply_action_event`、`apply_hover_event` 在 slot 上下文内生成 entity 捕获模式代码，而非 `cx.listener` 模式。

**修改文件**：`crates/engine/src/compiler/event.rs`

#### A.1 修改 `apply_event`（第 81-128 行）

在函数开头添加 `let slot = in_slot_context();`，对 `Ident` / `MethodName` / `WithArgs` 三个分支都添加 slot 分支：

```rust
pub fn apply_event(name: &str, handler: &EventHandler, _ctx: &CodegenCtx) -> String {
    if is_hover_event(name) {
        return apply_hover_event(name, handler);
    }
    if name == "on_action" {
        return apply_action_event(handler);
    }

    let (gpui_type, on_method, convert_expr) = match event_binding(name) {
        Some(binding) => binding,
        None => return String::new(),
    };

    let slot = in_slot_context();

    match handler {
        EventHandler::Ident(method) | EventHandler::MethodName(method) => {
            if slot {
                format!(
                    ".{on_method}({{\n    \
                     let __rml_evt_entity = __rml_self_entity.clone();\n    \
                     move |ev: &{gpui_type}, _window: &mut gpui::Window, cx: &mut gpui::App| {{\n        \
                     __rml_evt_entity.update(cx, |this, cx| {{\n            \
                     let rml_ev = {convert_expr};\n            \
                     this.{method}(&rml_ev, cx);\n            \
                     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n        \
                     }});\n    }}\n}})"
                )
            } else {
                // 原 cx.listener 模式（保持不变）
                format!(
                    ".{}(cx.listener(move |this, ev: &{}, _window, cx| {{\n                    \
                     let rml_ev = {};\n                    this.{}(&rml_ev, cx);\n                    \
                     if rml_ev.is_propagation_stopped() {{ cx.stop_propagation(); }}\n                }}))",
                    on_method, gpui_type, convert_expr, method
                )
            }
        }
        EventHandler::WithArgs(method, args) => {
            // 同上：slot 分支用 entity 捕获模式，非 slot 分支保持原样
        }
    }
}
```

#### A.2 修改 `apply_action_event`（第 141-167 行）

在 `parts.push` 前添加 `let slot = in_slot_context();`，slot 上下文内生成 entity 捕获模式。

#### A.3 修改 `apply_hover_event`（第 176-... 行）

在生成 body 前添加 `let slot = in_slot_context();`，slot 上下文内生成 entity 捕获模式。

**验证**：
```bash
cargo check -p rust-rml-engine
cargo check -p rust-rml-demo
```
检查生成的 `button_case.rs` 中 slot 闭包内事件代码应包含 `__rml_evt_entity.update(cx, ...)` 而非 `cx.listener`。

### Phase B：验证 button_case 编译

**目标**：确认 Phase A 框架修改后，button_case 的 slot 闭包内事件能正确编译。

**操作**：
1. `cargo clean -p rust-rml-demo` 清除旧生成代码
2. `cargo check -p rust-rml-demo`
3. 检查生成的 `button_case.rs` slot 闭包内事件代码是否为 entity 捕获模式

**验证**：`cargo check -p rust-rml-demo` 通过，无编译错误。

### Phase C：爬取 gpui-component 官方文档

**目标**：学习官方文档的组件描述风格，用于后续案例内容升级。

**操作**：
对每个组件，访问 `https://longbridge.github.io/gpui-component/zh-CN/docs/components/<component>` 并提取：
- 组件概述（一句话定位）
- 核心属性列表（含类型、默认值、说明）
- 事件回调签名
- 主题/样式定制点
- 典型用法示例

**组件清单**（按案例类别分组）：
- 基础：Button, Badge, Tag, Label, Separator, Link, Spinner, Icon
- 表单：Input, Checkbox, Switch, Radio, Slider, CodeEditor, Kbd
- 容器：Accordion, Card, Collapsible, GroupBox, Avatar, Progress, DescriptionList
- 表格树：Table, Tree, Pagination
- 状态导航：StatusBar, TabBar, TitleBar, Popover, Tooltip
- 菜单：ContextMenu, DropdownMenu

### Phase D：批量改造案例（核心工作）

**改造模板**（参考 `button_case.rml` + `.rml.rs` 样板）：

#### D.1 `.rml` 文件改造模式

**改前**（旧模式问题）：
```rml
<component>
    <div class="case-pane doc-pane">
        <h2>{t("case.X.title")}</h2>
        <p>... <code>X</code> 是 ... <code>Y</code> 属性 ...</p>
        <Card title="演示效果">...</Card>
        <Card title="示例代码">
            <Tabs>
                <Tab label=".rml"><CodeEditor value={rml_sample} /></Tab>
                <Tab label=".rml.rs"><CodeEditor value={rust_sample} /></Tab>
            </Tabs>
        </Card>
        <Card title="API"><Table ... /></Card>
    </div>
</component>
```

**改后**（新模式）：
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
        </template>
        <template slot="api">
            <h3>X</h3>
            <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
        </template>
    </CaseDocPage>
</component>
```

**简化版模板**（用于 welcome_case / overflow_test_case）：
```rml
<component>
    <CaseDocPage title={...} description="...">
        <template slot="demo">
            <!-- 仅演示内容，无 code/api 槽位 -->
        </template>
    </CaseDocPage>
</component>
```

#### D.2 `.rml.rs` 文件改造模式

**改前**：
```rust
pub struct XCase {
    pub code_tab: usize,  // 删除
    // ...
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
        self.code_tab = idx;  // 删除
    }
}
```

**改后**：
```rust
use crate::cases::common::{build_api_table, CaseDocPage};

pub struct XCase {
    pub case_doc_page: Option<gpui::Entity<CaseDocPage>>,  // 新增
    // ... 业务字段（删除 code_tab）
}

impl ILifecycle for XCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.case_doc_page = Some(cx.new(|_cx| CaseDocPage::default()));  // 新增
        // ... 初始化业务字段
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

#### D.3 内容升级要求

每个案例的 demo-section 应全面覆盖（参考 gpui-component 官方文档风格）：
- **基础用法**：最简调用
- **属性变体**：所有 variant（如 Button 的 9 种 variant、Badge 的 Number/Dot/Icon）
- **尺寸**：xsmall/small/medium/large
- **状态**：disabled/selected/loading 等
- **数据绑定**：`{field}` 形式动态绑定
- **事件回调**：`on-click={method}` 等
- **定制能力**：theme/style/slot 扩展点
- **特殊场景**：嵌套/组合/动态切换等

API 表格应包含：所有可用属性、事件、CSS 变量、插槽。

### Phase E：分批执行清单

按组件类别分批，每批 3-5 个案例并行改造，单批完成后 `cargo check` 验证。

#### 批次 1：基础组件（8 个）
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
- [ ] table_case（升级内容，移除硬编码字符串）
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

#### 批次 7：框架概念演示（16 个）
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

### Phase F：最终验证

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

1. **`<code>` 修复方式 = demo 级修复**：不修改框架 `tags.rs` 的 `BuiltinTag::Code => "gpui::div()"` 映射，在案例 RML 中直接避免使用 `<code>` 标签，用纯文本表达行内代码。

2. **代码展示实现 = `include_str!`**：编译时读取实际 `.rml` / `.rml.rs` 文件，确保代码展示与运行代码字节级一致。

3. **改造范围 = 全部 57 个案例**：含组件演示、框架概念演示、特殊页面。welcome_case 和 overflow_test_case 使用简化版 CaseDocPage。

4. **slot 闭包事件支持 = 框架级修复**：修改 `event.rs` 的 `apply_event`、`apply_action_event`、`apply_hover_event`，在 slot 上下文内生成 entity 捕获模式代码。

5. **内容质量 = 参考 gpui-component 官方文档**：爬取官方文档学习组件描述风格，确保案例内容全面覆盖属性/事件/主题/样式/数据绑定/插槽扩展。

### 假设

1. 现有 `case_doc_page.rml` + `.rml.rs` + `styles.css` 的 CaseDocPage 实现无需修改。
2. `include_str!` 路径使用相对路径（`"x_case.rml"` / `"x_case.rml.rs"`），与 .rml.rs 文件同目录。
3. 案例改造后 `code_tab` 字段及其 `on_code_tab_change` 命令应删除（由 CaseDocPage 内部管理）。
4. `rml_editor` / `rust_editor` 等 `ref` 引用应删除（CaseDocPage 内部管理 CodeEditor）。
5. build script 会自动检测 event.rs 变更并重新生成 .rml 输出（通过 engine crate 依赖追踪）。

## 验证步骤

### Phase A 验证
```bash
cargo check -p rust-rml-engine
```
预期：编译通过，无错误。

### Phase B 验证
```bash
cargo clean -p rust-rml-demo
cargo check -p rust-rml-demo
```
预期：编译通过。检查生成的 `button_case.rs` 应包含 `__rml_evt_entity.update(cx, ...)` 模式。

### Phase D 批次验证
每批完成后：
```bash
cargo check -p rust-rml-demo
```
预期：编译通过。

### Phase F 最终验证
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
| `apply_event` 修改影响其他生成代码 | 仅在 `in_slot_context() == true` 时切换分支，非 slot 上下文保持原 `cx.listener` 模式 |
| 改造案例数量大（55 个），单次易出错 | 分 8 批，每批 3-9 个，每批完成后 cargo check 验证 |
| `include_str!` 路径错误 | 路径与 .rml.rs 文件同目录，使用相对路径 |
| 案例业务字段遗漏 | 改造模板明确保留业务字段和 #[command] 方法，仅替换布局和 code_sample 实现 |
| 特殊页面改造后失去原有功能 | 使用简化版 CaseDocPage，仅替换外层 div，保留内部业务逻辑 |
| build script 缓存陈旧 | Phase B 使用 `cargo clean -p rust-rml-demo` 强制重新生成 |

## 执行顺序

1. **Phase A**：修改 `event.rs` 三个 apply_* 函数（约 60 分钟）
2. **Phase B**：验证 button_case 编译（约 10 分钟）
3. **Phase C**：爬取 gpui-component 官方文档（约 30 分钟）
4. **Phase D**：按批次 1→8 顺序改造（每批约 30-60 分钟，总计约 6-8 小时）
5. **Phase F**：最终验证（约 30 分钟）

总预估：8-11 小时。
