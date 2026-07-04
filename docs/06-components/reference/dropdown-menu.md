# dropdown-menu

## 概述

`<dropdown-menu>` 声明式封装下拉菜单。第一个非菜单项子节点为触发器（通常 `<Button>`）。

## 用法

```html
<dropdown-menu anchor="TopRight">
    <Button label="Options" ghost="" />
    <menu-item label="New" on-click={on_new} />
    <menu-separator />
    <menu-item label="Exit" on-click={on_exit} />
</dropdown-menu>
```

## 属性

| 属性 | 说明 |
|------|------|
| `anchor` | `TopLeft` / `TopRight` / … |
| `scrollable` | 可滚动 |
| `min_w` / `max_w` / `max_h` | 尺寸（像素） |
| `check_side` | `Left` / `Right` |
| `external_link_icon` | 链接外链图标 |

## Codegen 对照

→ `trigger.dropdown_menu_with_anchor(anchor, |menu, window, cx| { ... })`

## 相关

- [menu-items.md](./menu-items.md)
- Demo 案例：`demo/src/cases/menu_dropdown_case.rml`（`#[contribute]` 注册 id `components.menu.dropdown`）
