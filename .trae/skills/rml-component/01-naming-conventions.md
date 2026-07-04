# 01 命名规范

## 双层命名模型

RML 采用**声明式 kebab-case / 内部 snake_case** 双层命名模型：

| 层级 | 位置 | 风格 | 示例 |
|------|------|------|------|
| 声明式 | `.rml` 文件 tag/attr | kebab-case | `<tab-bar on-click={...} selected-index={idx}>` |
| 内部 | Rust builder 方法 | snake_case | `.on_click(...)` / `.selected_index(...)` |
| 内部 | Rust 字段名 | snake_case | `self.input_state` / `self.tree_state` |
| 内部 | Rust 模块名 | snake_case | `mod tab_bar` / `mod description_list` |

## 桥接机制

### normalize_attr_name

将声明式 kebab-case 属性名转换为内部 snake_case：

```
on-click    → on_click
selected-index → selected_index
label-width → label_width
```

实现：`-` → `_`

### normalize_component_tag

将 kebab-case tag 名转换为 PascalCase：

```
tab-bar      → TabBar
tab-item     → TabItem
status-bar   → StatusBar
native-status-bar → NativeStatusBar
menu-bar     → MenuBar
accordion-item → AccordionItem
```

实现：按 `-` 分割，每段首字母大写。

### canonical_tag

在 `normalize_component_tag` 基础上，处理小写无连字符别名：

```
accordion    → Accordion
item         → AccordionItem
tab          → Tab
table        → Table
column       → Column
descriptions → DescriptionList
description  → DescriptionItem
separator    → DescriptionSeparator
menu         → menu（保留小写，唯一例外）
```

**使用规范**：代码中比对 tag 名时，**必须**使用 `canonical_tag(tag)` 而非裸字符串比对，避免 `<tab-bar>` / `<TabBar>` / `<tab_bar>`[已废弃] 多形式漏洞。

## 事件命名双层模型

| 声明式 (kebab-case) | 内部 (snake_case) | 说明 |
|---------------------|-------------------|------|
| `on-click` | `on_click` | 通用点击 |
| `on-change` | `on_change` | Input/TextInput 值变化 |
| `on-toggle-click` | `on_toggle_click` | Accordion 折叠展开 |
| `on-activate` | `on_activate` | Tree 节点激活 |
| `on-select` | `on_select` | Tree 节点选择 |
| `on-tab-click` | `on_tab_click` | TabWindow 标签点击 |

**tokenizer 强制**：
- `read_attr_name` 拒绝 `_`（强制 kebab-case）
- `read_tag_name` 拒绝 `_`（强制 kebab-case）

## tag 名等价形式

| 规范形式 (kebab-case) | PascalCase | 小写别名 | 已废弃 (snake_case) |
|------------------------|------------|----------|---------------------|
| `tab-bar` | `TabBar` | — | `tab_bar` |
| `tab-item` | `TabItem` | — | `tab_item` |
| `status-bar` | `StatusBar` | — | `status_bar` |
| `native-status-bar` | `NativeStatusBar` | — | `native_status_bar` |
| `menu-bar` | `MenuBar` | `menu` | `menu_bar` |
| `modern-window` | — | — | `modern_window` |
| `tab-window` | — | — | `tab_window` |
| `accordion` | `Accordion` | `accordion` | — |
| `accordion-item` | `AccordionItem` | `item` | — |
| `table` | `Table` | `table` | — |
| `column` | `Column` | `column` | — |
| `descriptions` | `DescriptionList` | `descriptions` | — |
| `description` | `DescriptionItem` | `description` | — |
| `separator` | `DescriptionSeparator` | `separator` | — |

**根标签**（仅 kebab-case）：
- `<window>` / `<modern-window>` / `<tab-window>` / `<dialog>` / `<component>`

## 反模式列表

以下写法**已废弃/禁止**：

```xml
<!-- ❌ snake_case tag -->
<tab_window>        <!-- 应为 <tab-window> -->
<modern_window>     <!-- 应为 <modern-window> -->
<status_bar>        <!-- 应为 <status-bar> -->
<tab_bar>           <!-- 应为 <tab-bar> -->

<!-- ❌ snake_case 属性 -->
<TabBar on_click={...}>    <!-- 应为 on-click -->
<Input on_change={...}>    <!-- 应为 on-change -->

<!-- ❌ 事件单形式 -->
onclick={...}              <!-- 应为 on-click -->
onclick={...}              <!-- 应为 on-click -->

<!-- ❌ size 用 middle -->
<Button size="middle">     <!-- 应为 size="medium" -->

<!-- ❌ 提供 horizontal -->
<DescriptionList horizontal={true}>  <!-- 不提供 horizontal，默认横向 -->
```

## StatusBar 命名冲突解决

`StatusBar` 在 RML 中有两个不同组件：

| tag | 组件 | 用途 |
|-----|------|------|
| `<status-bar>` / `<StatusBar>` | `rml_ui::StatusBar` | RML MVVM 包装，支持 `items={...}` 绑定 |
| `<native-status-bar>` / `<NativeStatusBar>` | `rml_ui::NativeStatusBar` | gpui-component 原生状态栏，手动 `.left()` / `.right()` 组装 |

`canonical_tag("status-bar")` → `"StatusBar"`
`canonical_tag("native-status-bar")` → `"NativeStatusBar"`

代码中比对时**必须**用 `canonical_tag`，避免 `StatusBar` 与 `NativeStatusBar` 混淆。
