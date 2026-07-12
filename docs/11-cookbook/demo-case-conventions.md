# 11.6 Demo 案例页规范

> **本节目标**：统一 `demo/src/cases` 中 CaseDocPage 案例的 RML 结构与组合写法，避免布局黏连、语法偏差与文档不同步。

## 标准结构

每个案例（除 `welcome_case` 外）使用 **CaseDocPage** 四段式布局：

```html
<component>
    <CaseDocPage
        title={t("case.xxx.title")}
        description="用户可见说明（中文硬编码 OK）"
        code-rml={rml_sample}
        code-rust={rust_sample}>
        <template slot="demo">
            <!-- 演示区：仅用 demo-section 分场景 -->
        </template>
        <template slot="api">
            <Table columns={api_columns} rows={api_rows} bordered="" stripe="" />
        </template>
    </CaseDocPage>
</component>
```

## 演示区：demo-section

**必须**用 `demo-section` 划分场景，**禁止**在 `slot="demo"` 内堆叠多个无间距 `<Card>`：

```html
<!-- ✅ 正确 -->
<template slot="demo">
    <div class="demo-section">
        <h3>基础用法</h3>
        <p>说明</p>
        <Input value={name} />
    </div>
    <div class="demo-section">
        <h3>禁用状态</h3>
        <Input value={name} disabled="" />
    </div>
</template>

<!-- ❌ 错误：Card 黏连、无统一间距 -->
<template slot="demo">
    <Card title="场景一">...</Card>
    <Card title="场景二">...</Card>
</template>
```

参考范例：`input_case.rml`、`scroll_case.rml`、`tooltip_case.rml`、`key_binding_case.rml`。

### 间距

- 场景内控件组：父级 `display="flex"` + `gap="8px"` 或 `class="button-row"`
- 避免 `style="margin-top: 12px"` 等内联 margin（技术债）

## 组合写法

| 场景 | 推荐写法 | 文档 |
|------|----------|------|
| 输入框快捷键 | `<Input><KeyBinding/></Input>` | [key-binding.md](../06-components/reference/key-binding.md) |
| 作用域快捷键 | `<ShortcutScope><Shortcut/>…</ShortcutScope>` | [key-binding.md](../06-components/reference/key-binding.md) |
| 对话框 | `<Button slot="trigger"/>` + 内容 | [composition-patterns.md](../06-components/composition-patterns.md) |
| 下拉菜单 | 首子节点为触发器 | [dropdown-menu.md](../06-components/reference/dropdown-menu.md) |
| 垂直滚动 | `overflow-y-auto=""` 或 `overflow-y="auto"` | [layout.md](../07-styling/layout.md) |

## i18n

- 案例树名称：`t_static("case.*.title")`（`.rml.rs` 的 `IContribution::name`）
- CaseDocPage `title={t("case.*.title")}`
- `description`、`h3`、API 表说明：中文硬编码（当前策略）
- 切换语言演示：见 `i18n_case`（`cx.current_locale()` / `current_locale_static()`）

## 待迁移案例（布局）

以下案例仍使用 Card 堆叠或缺少 `demo-section`，应逐步迁移：

`card_case`（演示 Card 组件本身，保留 Card）、`edna_case`（自定义布局）、`settings_case`（含带 class 的场景 Card，需手工迁移）。

其余 CaseDocPage 案例已统一为 `demo-section` 分场景（含 `skeleton_case`，2026-07 批量迁移）。

## 语法禁忌（案例 description / API 表）

- 属性名用 **kebab-case**：`on-change`、`font-bold`，不用 `on_change` / `font_bold`
- 指令用 **`if` / `each` / `value={}`**，不用 `r:if` / `r:model`
- 所有用户可见面（description / API 表 / 演示区 `<p>` / `.rml.rs` 注释——`include_str!` 会把注释一起展示）一律不写 codegen 内部符号：`__rml_populate_refs`、codegen 生成串（`rml_key`、`rml_core::element_id::from_key`）、gpui-component 底层分支实现（`h_8 + px_4 + text_base`）、`ElementRef<T>`、`InputStateBridge`、`StateBridge`。注：`__rml_bump_version` 是公开 API（见 [双向绑定](../03-binding/two-way-binding.md)），可在 command 代码与注释中出现

## API 表格编写规范

`slot="api"` 中的表格通过 `build_api_table` 构建，**面向 RML 开发者实际使用**：回答「我用这个组件最常写哪些 props/events/slots？默认值是什么？」不做齐全性枚举——完整的 props 列表归 [组件参考文档](../06-components/reference/)，案例 API 表只列开发者为使用该组件必须知道的条目。

### 列结构（固定三列）

| 列 | 含义 |
|----|------|
| 属性 | RML 属性名（kebab-case） |
| 类型 | 开发者友好类型 |
| 说明 | 用途 + 示例值 |

### 类型词汇表

使用：`string`、`number`、`bool`、`event`、`slot`、`binding`、`string / binding`、`bool / binding`

**禁止**：`f32（Static）`、`ElementRef<T>`、`static: String`、`bind: bool`、`InputStateBridge`、`StateBridge`、`cx.subscribe`

### 属性名

- 事件一律 kebab-case：`on-change`、`on-press`，不用 `on_change`
- 复合属性：`menu-width`、`default-value`、`col-span`
- `ref` 行：仅当组件使用 ref 模式时出现，简述 `ref="name"` 绑定到 ViewModel 同名字段，不写 Rust 泛型

### 说明写法

- ✅ 写什么 + 示例 + 默认值：`占位文本，如 placeholder="用户名"`
- ✅ 布尔/枚举属性必须标注默认值：`禁用，默认 false`、`尺寸：xsmall | small | medium | large，默认 medium`
- ❌ 编译器映射：`通过 cx.subscribe 订阅 InputEvent::Change`
- ❌ VM 内部类型：`绑定到 ElementRef<SliderState> 字段`

### 行排序

按使用频率从高到低排列，最常用的 3-5 个 props 排在最前（通常是 `label` / `value` / `on-click` 这类）。开发者扫一眼表格顶部就能覆盖 80% 场景。

### 选择性（反补全）

案例 API 表**不追求齐全**，只列开发者为使用该组件必须知道的条目。原则：

- **只列该组件专属 API**。通用样式 trait 方法（`font-bold` / `font-semibold` / `h-flex` / `v-flex` 等 `StyledExt`、`Sizable` 提供的无组件特定语义的成员）**不列入**——它们对所有组件都适用，归 [样式文档](../07-styling/)，在每个组件表里重复等于噪声。
- **保留有组件特定语义的 trait 方法**。`size`（影响 Button 高度）、`disabled`（影响 Button 交互）虽源自 trait，但对该组件有具体行为，应列出并说明默认值。
- **不枚举相关但本案例未演示的指令**。例如 `conditional_case` 讲 `if`，不要把 `each` 一并塞进表里凑数；`each` 归 `list_case` / `key_case`。
- 对照 [组件参考文档](../06-components/reference/) 做减法：参考文档是权威全集，案例表是精选子集。

### 正反例

**✅ 推荐**

```rust
let (cols, rows) = build_api_table(&[
    ("key", "string", "快捷键，如 key=\"Ctrl+S\""),
    ("when", "bool / binding", "是否启用（默认 true）"),
    ("on-press", "event", "命中快捷键时回调"),
    ("on-change", "event", "内容变化时回调"),
    ("value", "binding", "双向绑定到 ViewModel 字段，如 value={username}"),
    ("ref", "string", "元素引用名，如 ref=\"my_input\""),
]);
```

**❌ 避免（类型词汇表违规）**

```rust
let (cols, rows) = build_api_table(&[
    ("ref", "字符串", "SliderState 元素引用名（配合 ElementRef<SliderState> 字段）"),
    ("min", "f32（Static）", "最小值"),
    ("on_change", "事件", "通过 cx.subscribe 订阅 SliderEvent::Change"),
    ("value", "绑定属性", "StateBridge → set_selected_value"),
]);
```

**❌ 避免（补全式填充——把通用 trait 与未演示指令塞进表里凑数）**

```rust
// Button 案例里塞入 StyledExt 通用字体权重（每个组件都有，不是 Button 专属）
("font-bold / font-semibold / font-medium ...", "布尔标志", "字体权重"),
// conditional_case 讲 if，却把 each 一起塞进来
("each={x in items}", "指令", "遍历可迭代对象"),
```

### description 同步

`CaseDocPage` 的 `description` 与 API 表同一标准：用户可见的中文说明，不含 `on_loaded`、`PascalCase` 编译提示、`Selectable trait` 等内部术语。演示区 `<p>` 说明同理。

参考范例：`button_case`（API 区修订后，开发者视角样板）、`key_binding_case`、`input_case`（修订后）、`counter_case`（MVVM 绑定专题，可保留 `#[command]` 等框架概念）。

---

← 返回 [Cookbook 目录](./INDEX.md) · 组合模式 [composition-patterns.md](../06-components/composition-patterns.md)
