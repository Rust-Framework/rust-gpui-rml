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
| `on-click` | 命令 | `{handler}` | 点击回调（强类型直接调用 `#[command]` 方法） |
| `command` | `Arc<RelayCommand>` 字段 | `{field}` | 声明式命令绑定，点击经 `ICommand::execute` 调度（见下） |

### `command={field}` 声明式命令绑定

`on-click={method}` 是强类型直接调用——codegen 生成 `this.method(&ev, cx)`。`command={field}` 则是声明式命令绑定：ViewModel 持有 `Arc<RelayCommand>` 字段，点击时经 `ICommand::can_execute` / `execute` 动态调度，适用于命令可复用、可快捷键、可命令面板等场景（对齐 WPF `ICommand`）。

```html
<menu-item label="Save" command={save_command} />
```

```rust
#[derive(Default)]
#[component]
pub struct MyView {
    pub save_command: Arc<RelayCommand>,  // 框架提供 Default（no-op 空对象）
}

impl ILifecycle for MyView {
    fn on_loaded(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        self.save_command = Arc::new(RelayCommand::new(cx, |this, cx| {
            this.save(cx);
        }));
    }
}
```

`command` 与 `on-click` 同时声明时，`command` 优先。详见 [4.4 命令系统 · 声明式命令绑定](../../04-code-behind/command-system.md)。

## 子节点语义

| 子节点 | 行为 |
|--------|------|
| 嵌套 `menu-item` | 子菜单 |
| 其他 RML 节点 | 自定义行（`menu_element`） |

## 示例

```html
<menu-item label="Copy" icon="Copy" on-click={on_copy} />
<menu-separator />
<menu-item header="" label="Edit" />
<menu-item label="Wrap" checked={word_wrap} on-click={toggle_wrap} />
<menu-item label="Docs" href="https://example.com" icon="Help" />
<menu-item label="New">
    <menu-item label="File" on-click={on_new_file} />
</menu-item>
<menu-item label="Dark Mode" on-click={on_toggle} />
```

## Codegen 对照

```html
<menu-item label="Copy" on-click={on_copy} />
```

→ `PopupMenuItem::new("Copy").on_click({ weak entity → this.on_copy })`

## 相关

- [context-menu.md](./context-menu.md)
- [dropdown-menu.md](./dropdown-menu.md)
