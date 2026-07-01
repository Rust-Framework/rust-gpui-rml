# menu-item / menu-separator

## 概述

菜单容器内仅两种子标签（WPF 风格）：

| RML 标签（推荐） | 别名 | 说明 |
|------------------|------|------|
| `menu-item` | `MenuItem` | 操作项、分组标题、链接、子菜单、自定义行 |
| `menu-separator` | `MenuSeparator`、`separator` | 分隔线 |

`<menu-bar>` 由 `rml_ui::MenuBar` 渲染，默认样式（无需 CSS）：

| 默认项 | 值 | 定制 |
|--------|-----|------|
| 按钮间距 | 4px | `MenuBar::gap(6.)` |
| 按钮高度 | 22px | `menu_bar_button` 常量 |
| 按钮内边距 | 6×2 px | `MenuBar::button_pad_x()` / `button_pad_y()` |
| 按钮外边距 | 上下 2px | `MenuBar::button_margin(4.)` |

## menu-item 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 显示文字 |
| `header` | 布尔标志 | — | 分组标题（不可点击） |
| `icon` | IconName | — | 如 `icon="Copy"` |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `checked` | 布尔 | `{expr}` | 勾选态 |
| `href` | 字符串 | `{expr}` | 链接项 |
| `onclick` | 命令 | `{handler}` | 点击回调 |

## 子节点语义

| 子节点 | 行为 |
|--------|------|
| 嵌套 `menu-item` | 子菜单 |
| 其他 RML 节点 | 自定义行（`menu_element`） |

## 示例

```html
<menu-item label="Copy" icon="Copy" onclick={on_copy} />
<menu-separator />
<menu-item header="" label="Edit" />
<menu-item label="Wrap" checked={word_wrap} onclick={toggle_wrap} />
<menu-item label="Docs" href="https://example.com" icon="Help" />
<menu-item label="New">
    <menu-item label="File" onclick={on_new_file} />
</menu-item>
<menu-item label="Dark Mode" onclick={on_toggle} />
```

## Codegen 对照

```html
<menu-item label="Copy" onclick={on_copy} />
```

→ `PopupMenuItem::new("Copy").on_click({ weak entity → this.on_copy })`

## 相关

- [context-menu.md](./context-menu.md)
- [dropdown-menu.md](./dropdown-menu.md)
