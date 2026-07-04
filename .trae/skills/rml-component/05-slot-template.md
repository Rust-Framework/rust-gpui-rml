# 05 插槽与模板

## 基础插槽

**语法**：`<template slot="name">...</template>`

```xml
<modern-window>
    <template slot="menu">
        <MenuBar items={menus} />
    </template>
    <template slot="footer">
        <StatusBar items={status} />
    </template>
    <div>主内容</div>
</modern-window>
```

**partition_slot_children**：将 shell 根元素子节点拆分为插槽与主内容。

## TabWindow 插槽表

| slot 名 | builder 方法 | 说明 |
|---------|--------------|------|
| `menu` | `.menu_slot(...)` | 顶部菜单栏 |
| `title` | `.title_ext_slot(...)` | 标题扩展 |
| `footer` | `.status_slot(...)` | 底部状态栏 |
| `left` | `.slot_left(...)` | 左侧面板（仅 tab-window） |
| `right` | `.slot_right(...)` | 右侧面板（仅 tab-window） |
| `bottom` | `.slot_bottom(...)` | 底部面板（仅 tab-window） |
| `tabs` | `.tab_children(vec![...])` | 标签页集合（仅 tab-window） |

**tabs slot 特殊**：收集所有子节点（应为 `<Tab>` 元素），生成 `.tab_children(vec![...])`，与 `tabs={Vec<TabItem>}` bind 属性互斥（编译期校验）。

## Table 专用模板

Table 使用 `<template slot="header/cell/footer">` 定义列模板：

```xml
<Table>
    <Column key="name" title="Name" />
    <Column key="age" title="Age" />
    <template slot="header"><span>自定义表头</span></template>
    <template slot="cell" field="name">
        <span>{row_data.name}</span>
    </template>
    <template slot="footer"><span>合计</span></template>
</Table>
```

### 生成代码

| slot | 生成 setter | 闭包签名 |
|------|-------------|----------|
| `header` | `.header_template(Arc::new(\|col_idx, column, cx\| ...))` | 3 参 |
| `cell` | `.cell_template("key", Arc::new(\|row_idx, col_idx, row_data, column, cx\| ...))` | 5 参 |
| `footer` | `.footer_template(Arc::new(\|cx\| ...))` | 1 参 |

### cell slot 必填属性

`<template slot="cell" field="key">` 必须包含 `field` 属性，缺失报错。

## Scoped Slot 参数支持

模板内容可引用闭包参数（B8 实现）：

```xml
<template slot="cell" field="name" slot-params="row_idx,row_data">
    <span>{row_data.name}</span>
    <Badge>{row_idx}</Badge>
</template>
```

**slot-params 属性**：声明模板所用参数名（逗号分隔），codegen 据此：
1. 移除闭包参数 `_` 前缀（`_row_idx` → `row_idx`）
2. 在模板内容中支持 `{row_data.field}` → `row_data.field` 直接引用

**参数可见性**：
| slot | 可用参数 |
|------|----------|
| `header` | `col_idx`, `column`, `cx` |
| `cell` | `row_idx`, `col_idx`, `row_data`, `column`, `cx` |
| `footer` | `cx` |

**限制**：未声明 `slot-params` 时，闭包参数保留 `_` 前缀（不可访问）。需要参数访问时必须显式声明。

## TabItem 模板（WPF TabControl 模式）

TabItem 是 WPF TabControl 风格的标签项，支持 title + body：

```xml
<TabBar>
    <tab-item title="Settings" title-icon="Settings">
        <div>设置内容</div>
    </tab-item>
</TabBar>
```

**生成**：`.child(TabItem::new().title("Settings").title_icon(IconName::Settings).child(...))`

## Slot 名称校验

validator 在编译期校验 slot 名：

| 容器 | 合法 slot 名 |
|------|-------------|
| `<modern-window>` | menu, title, footer |
| `<tab-window>` | menu, title, footer, left, right, bottom, tabs |
| `<Table>` | header, cell, footer |

未知 slot 名触发编译错误。

## SlotRenderer trait

用户自定义组件（`@component`）的插槽通过 `SlotRenderer` trait 实现：

```rust
trait SlotRenderer {
    fn render_slot(&self, slot: &str, window: &mut Window, cx: &mut App) -> AnyElement;
}
```

与 Table 模板闭包不同，SlotRenderer 是运行时动态分发。
