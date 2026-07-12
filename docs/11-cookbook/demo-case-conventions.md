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
- description 不写 codegen 内部符号（如 `__rml_state`、`ElementRef<T>`、`InputStateBridge`）

## API 表格编写规范

`slot="api"` 中的表格通过 `build_api_table` 构建，**面向 RML 开发者**：回答「在 `.rml` 里能写哪些 props/events/slots？」

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

- ✅ 写什么 + 示例：`占位文本，如 placeholder="用户名"`
- ❌ 编译器映射：`通过 cx.subscribe 订阅 InputEvent::Change`
- ❌ VM 内部类型：`绑定到 ElementRef<SliderState> 字段`

### 齐全性

对照 `docs/06-components/reference/` 与 `props_registry.rs`，列出该组件**所有** RML 面向开发者的 props/events/slots。通用属性（`disabled`、`size`、`label`）按组件实际支持情况列入。

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

**❌ 避免**

```rust
let (cols, rows) = build_api_table(&[
    ("ref", "字符串", "SliderState 元素引用名（配合 ElementRef<SliderState> 字段）"),
    ("min", "f32（Static）", "最小值"),
    ("on_change", "事件", "通过 cx.subscribe 订阅 SliderEvent::Change"),
    ("value", "绑定属性", "StateBridge → set_selected_value"),
]);
```

### description 同步

`CaseDocPage` 的 `description` 与 API 表同一标准：用户可见的中文说明，不含 `on_loaded`、`PascalCase` 编译提示、`Selectable trait` 等内部术语。演示区 `<p>` 说明同理。

参考范例：`key_binding_case`、`input_case`（修订后）、`counter_case`（MVVM 绑定专题，可保留 `#[command]` 等框架概念）。

---

← 返回 [Cookbook 目录](./INDEX.md) · 组合模式 [composition-patterns.md](../06-components/composition-patterns.md)
