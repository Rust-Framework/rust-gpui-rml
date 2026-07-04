# menu-bar / menu

## 概述

`<menu-bar>` 与兼容别名 `<menu>` 用于标题栏水平菜单（`<template slot="menu">`）。

## 声明式

```html
<menu-bar>
    <menu-item label={t("menu.file")}>
        <menu-item label={t("menu.file_new")} on-click={on_new} />
        <menu-separator />
        <menu-item label={t("menu.file_exit")} on-click={on_exit} />
    </menu-item>
</menu-bar>
```

## 数据绑定

```html
<menu items={menu_items} />
```

## Codegen

- 声明式 → `gpui_component::h_flex()` + `Button::dropdown_menu` + `compiler/menu/item.rs` 直译 `PopupMenu`
- `items` → `rml_ui::Menu::new(...).items(...)` 或 `rml_ui::render_menu_bar_from_items(...)`（MVVM 运行时渲染，非 codegen 包装）

## 相关

- [menu-items.md](./menu-items.md)
