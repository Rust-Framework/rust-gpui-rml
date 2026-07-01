# menu（索引）

## 概述

RML 菜单 codegen（`crates/engine/src/compiler/menu/`）直译 gpui-component PopupMenu。子标签仅 **`menu-item`** + **`menu-separator`**。

## 标签（kebab-case 推荐）

| 容器 | 子项 |
|------|------|
| `context-menu` | `menu-item`, `menu-separator` |
| `dropdown-menu` | 同上 |
| `menu-bar` / `menu` | 同上 |
| `app-menu-bar` | — |

PascalCase（`ContextMenu` 等）仍兼容。规范见 [tags-mapping.md](../../02-syntax/tags-mapping.md) §2.2.9。

## 文档

- [menu-items.md](./menu-items.md)
- [context-menu.md](./context-menu.md)
- [dropdown-menu.md](./dropdown-menu.md)
- [menu-bar.md](./menu-bar.md)
- [app-menu-bar.md](./app-menu-bar.md)

## 快速示例

```html
<context-menu>
    <div>Right-click</div>
    <menu-item label="Copy" onclick={on_copy} />
</context-menu>

<dropdown-menu anchor="TopRight">
    <Button label="Options" ghost="" />
    <menu-item label="Help" href="https://example.com" />
</dropdown-menu>

<menu items={menu_items} />
```
