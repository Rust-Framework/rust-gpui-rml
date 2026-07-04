# context-menu

## 概述

`<context-menu>` 声明式封装 gpui-component 右键菜单。非 `menu-item`/`menu-separator` 子节点为触发区域。

## 用法

```html
<context-menu>
    <div class="file-row">Right-click me</div>
    <menu-item label="Open" icon="FolderOpen" on-click={on_open} />
    <menu-separator />
    <menu-item label="New">
        <menu-item label="File" icon="File" on-click={on_new_file} />
    </menu-item>
</context-menu>
```

## 容器属性

`scrollable`、`min_w`、`max_w`、`max_h`、`check_side`、`external_link_icon`（同 [dropdown-menu.md](./dropdown-menu.md)）。

## Codegen 对照

→ `trigger.context_menu(|menu, window, cx| { ... })`

## 相关

- [menu-items.md](./menu-items.md)
- Demo 案例：`demo/src/cases/menu_context_case.rml`（`#[contribute]` 注册 id `components.menu.context`）
