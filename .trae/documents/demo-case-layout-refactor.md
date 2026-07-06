# Demo 案例布局重构计划

## Context

demo 项目下 51 个案例存在系统性问题：Card 滥用（236 个 Card，平均每文件 4.5 个）、示例代码严重缺失（25 个案例无 code_sample、9 个有死代码）、章节顺序混乱（两套不兼容模式并存）、布局间距不足（h2/h3/p 无默认 font-weight 和 margin）、CSS mapper 不支持 `border` 简写导致 `.demo-section` 分隔线静默失效。

目标：建立统一的案例模板，修复框架层默认样式，使案例成为展示 RML 框架优势的高质量文档。

用户决策：
- 示例代码粒度：**每案例一个 code_sample**（展示主要用法）
- Card 策略：**全部移除 Card**（用 h3 + 间距 + 分隔线组织）
- 实施范围：**框架修复 + 2 个黄金样板先行**，验证后批量推进

---

## 统一案例模板

所有案例采用以下结构（无 Card）：

```
<div v-flex="" class="case-pane doc-pane">
    <h2>{t("case.xxx.title")}</h2>
    <p>组件说明（1-2 句）</p>

    <div class="demo-section">
        <h3>基础用法</h3>
        <p>说明</p>
        [实际演示组件]
    </div>
    ...更多 demo-section（基础→高级）...

    <div class="demo-section">
        <h3>示例代码</h3>
        <CodeEditor value={code_sample} />
    </div>

    <div class="demo-section">
        <h3>API</h3>
        <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
    </div>
</div>
```

章节顺序统一：**标题 → 说明 → 演示段（多个）→ 示例代码 → API**。

---

## Phase 0：框架修复（基础）

### 0.1 h2/h3 标题默认加粗

**文件**：[tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L47-L52)

`BuiltinTag::codegen_ctor` 中 h1-h6 当前只设 `text_size`，不设 `font_weight`。修改：
- H1：`"gpui::div().text_size(gpui::px(32.)).font_weight(gpui::FontWeight::BOLD)"`
- H2-H6：追加 `.font_weight(gpui::FontWeight::SEMIBOLD)`

使用全路径 `gpui::FontWeight` 避免导入问题。`font_weight` 方法来自 `Styled` trait（已在生成代码中 import）。

### 0.2 CSS mapper 支持 border 简写

**文件**：[mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L38-L142)

在 `match prop` 块的 `"border-radius"` 后（第 58 行后）添加：
- `"border"` → `shorthand_border(&value, vars)` — 解析 `1px solid <color>` / `1px solid` / `1px`
- `"border-color"` → `color_method("border_color", &value, vars)`
- `"border-top"` / `"border-bottom"` / `"border-left"` / `"border-right"` → 解析简写，映射为 `border_t_1()` / `border_b_1()` / `border_l_1()` / `border_r_1()` + `border_color()`

新增 `shorthand_border` 函数（参照 `shorthand_padding` 模式）：
- 输入 `Value::List`，拆分为 width/style/color 三部分
- width：`Length(n, Px)` → `border_1()` / `border_2()`（n=1/2/3/4）
- style：`Keyword("solid"/"dashed"/"dotted")` → 忽略（GPUI 不支持 border-style）
- color：`Color` / `Var` / `Keyword` → `border_color(<resolved>)`
- 输出：`"border_1().border_color(gpui::black())"` 形式

GPUI 限制：`border_color` 应用于所有边，无法 per-side 着色。per-side border（`border_b_1`）设宽度，color 仍全局。在代码注释中标注此限制。

### 0.3 修复 styles.css 主题变量

**文件**：[styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css)

4 处 `var(--border)` → `var(--border-color)`（第 84、105、194、199 行）。`--border-color` 在 light.css/dark.css 中已定义，`--border` 未定义。

### 0.4 验证

```bash
cargo test -p rust-rml-engine  # mapper 新测试 + 现有测试
cargo check -p rust-rml-demo   # 编译通过
```

---

## Phase 1：黄金样板（2 个案例）

### 1.1 accordion_case（Pattern A 代表，已有 code_sample）

**文件**：
- [accordion_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml)
- [accordion_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs)

改动：
1. 移除全部 4 个 `<Card>` 包装
2. 组件说明 `<p>` 直接放在 `<h2>` 后
3. 5 个 demo-section 直接作为 doc-pane 子元素（原本嵌套在"演示效果"Card 内）
4. 示例代码段：`<div class="demo-section"><h3>示例代码</h3><CodeEditor value={code_sample} /></div>`
5. API 段：`<div class="demo-section"><h3>API</h3><Table .../></div>`
6. 修复 code_sample 内容：`open=""` → `open-ixs={basic_open}`，与 demo 一致；确保 `on-click`（非 `onclick`）

### 1.2 button_case（Pattern B 代表，无 code_sample）

**文件**：
- [button_case.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml)
- [button_case.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml.rs)

改动：
1. 移除全部 7 个 `<Card>` 包装
2. 6 个演示段转为 `<div class="demo-section">` + `<h3>` 标题
3. 在 `.rml.rs` 新增 `#[computed] fn code_sample()` 方法，展示 Button 基础用法 RML 代码（label/primary/ghost/danger/on-click）
4. 新增示例代码段 + API 段
5. 确保段落间有间距：demo-section 的 `gap: 8px` + doc-pane 的 `gap: 16px`

### 1.3 验证

```bash
cargo check -p rust-rml-demo
cargo run -p rust-rml-demo  # 手动验证：accordion 和 button 页面
```

验证点：
- h2/h3 加粗显示，视觉层级清晰
- .demo-section 底部虚线分隔线可见
- 无 Card 包装，段落间间距合理
- CodeEditor 显示正确的 RML 代码
- 交互功能正常（点击按钮、展开折叠面板等）

---

## 后续阶段（验证通过后执行）

- **Phase 2**：Pattern A 批量（16 个案例）— 移除 Card + 修复 onclick → on-click（7 个案例）+ table_case 硬编码标题改 t()
- **Phase 3**：Pattern B 批量（34 个案例）— 移除 Card + 为 25 个无 code_sample 的案例新增方法 + 为 9 个死代码案例接入 CodeEditor
- **Phase 4**：清理 — 删除 overflow_test_case（无 #[contribute] 的死代码）、统一 h3 i18n、移除孤儿代码

---

## 关键文件清单

| 文件 | Phase | 改动类型 |
|------|-------|---------|
| `crates/engine/src/tags.rs` | 0 | h1-h6 加 font_weight |
| `crates/engine/src/css/mapper.rs` | 0 | 新增 border 简写映射 + 测试 |
| `demo/assets/styles.css` | 0 | `--border` → `--border-color` |
| `demo/src/cases/accordion_case.rml` | 1 | 移除 Card，重组结构 |
| `demo/src/cases/accordion_case.rml.rs` | 1 | 修复 code_sample 内容 |
| `demo/src/cases/button_case.rml` | 1 | 移除 Card，重组结构 |
| `demo/src/cases/button_case.rml.rs` | 1 | 新增 code_sample 方法 |
