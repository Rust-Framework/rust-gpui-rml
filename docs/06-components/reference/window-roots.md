# 根节点：window / modern_window / tab_window / dialog / component

## 概述

RML 文件必须有且仅有一个根标签。根节点决定 codegen 输出：窗口 `impl IWindow`、对话框 `open/close` 方法，或可复用 `#[component]` 模板。

| 根标签 | 用途 | 生成代码 |
|--------|------|----------|
| `window` | 基础窗口（透明标题栏） | `impl IWindow` + `Render` |
| `modern_window` | 自绘 TitleBar/Menu/StatusBar 外壳 | `impl IWindow` + `ModernWindowShell` |
| `tab_window` | TabBar 标题栏 + 多插槽高级窗口 | `impl IWindow` + `TabWindowShell` |
| `dialog` | 模态对话框（非独立 OS 窗口） | `open(window, cx)` / `close(cx)` |
| `component` | 可复用组件片段 | 仅 `impl Render` |

## window

### 基本用法

```html
<window title="我的应用" width="800" height="600">
    <div class="content">...</div>
</window>
```

### 属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `title` | 字符串 | 窗口标题 |
| `width` / `height` | 数字 | 尺寸（px） |
| `left` / `top` | 数字 | 初始位置（可选） |
| `startup` | 字符串 | 如 `CenterScreen` |
| `min_width` / `min_height` | 数字 | 最小尺寸 |

`chrome` 固定为 `WindowChrome::Transparent`。

## modern_window

在 `window` 基础上包裹 `ModernWindowShell`，支持菜单/页脚插槽。

### 属性（绑定）

| 属性 | 绑定 | 说明 |
|------|------|------|
| `menu` | `{expr}` | 菜单区元素 |
| `footer` | `{expr}` | 页脚区元素 |
| `icon` | `{IconName::...}` | 窗口图标 |

### 插槽子节点

| 插槽标签 | 说明 |
|----------|------|
| `slot="menu"` | 菜单区 |
| `slot="title"` | 标题扩展区 |
| `slot="footer"` | 页脚区 |

其余子节点为主内容区。

## tab_window

Demo 主窗口使用的根类型，包裹 `TabWindowShell`。

### 布局结构

单行标题栏（`TitleBar` + `TabBar`）：

```text
| 图标切换 | 主窗口菜单 | Title | Tab1 | Tab2 | … | 可扩展区 | 窗口操作 |
```

- **图标切换**：`icon` + `on_chrome_toggle`；`show_chrome=true` 时显示 `ChevronLeft`，折叠后仅保留图标与 `ChevronRight`，菜单与标题隐藏。
- **主窗口菜单**：`slot="menu"` 内容嵌入 TabBar `prefix`（非独立菜单行）。
- **Tab 溢出**：宽度不足时启用 TabBar `.menu()` 下拉，避免横向滚动条（依赖 gpui-component TabBar 行为）。
- **可扩展区**：`slot="title"` → `title_ext_slot` → TabBar `suffix`。

主体区域（`h_resizable` + `v_resizable`）：

```text
| 插槽1 (slot="left")  |  Tab Body + 插槽3 (slot="bottom")  | 插槽2 (slot="right") |
| 空则隐藏，右缘拖拽  |  主内容 + 底栏上缘拖拽高度          | 空则隐藏，左缘拖拽  |
```

底部 **插槽4**：`slot="footer"` → `status_slot`（状态栏，空则隐藏，不可拖拽）。

### 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `title` | 字符串 | — | 窗口标题（同 `IWindow::title()`） |
| `width` / `height` | 数字 | — | 尺寸 |
| `startup` | 字符串 | — | 如 `CenterScreen` |
| `icon` | IconName | `{expr}` | 窗口图标 |
| `tabs` | `Vec<TabItem>` | `{tab_bar_items}` | 标签页数据 |
| `selected_tab` | `usize` | `{selected_tab}` | 当前选中索引 |
| `show_chrome` | `bool` | `{show_chrome}` | 是否显示菜单与标题（图标切换按钮始终可见） |
| `menu` / `footer` | 元素 | `{expr}` | 备用绑定（优先用插槽） |

### 事件

| 事件 | 回调 | 说明 |
|------|------|------|
| `on_tab_click` | `fn(&mut self, index: usize, cx: &mut Context<Self>)` | 标签点击 |
| `on_chrome_toggle` | `fn(&mut self, cx: &mut Context<Self>)` | 标题栏 chrome 切换 |

### 插槽子节点

| 插槽标签 | 说明 |
|----------|------|
| `slot="menu"` | 标题栏内菜单（TabBar prefix） |
| `slot="title"` | 标题栏右侧扩展区（TabBar suffix） |
| `slot="footer"` | 底部状态栏 |
| `slot="left"` | 左侧栏（如 ActivityBar），右缘可拖拽调宽 |
| `slot="right"` | 右侧栏，左缘可拖拽调宽 |
| `slot="bottom"` | 主内容区下方栏（如输出面板），上缘可拖拽调高 |

非插槽子节点为主内容区。

### 完整示例

`demo/src/shell/main_window.rml`：

```html
<tab_window
    title="RML Showcase"
    width="1100"
    height="720"
    startup="CenterScreen"
    icon={IconName::Frame}
    tabs={tab_bar_items}
    selected_tab={selected_tab}
    on_tab_click="on_tab_click"
    show_chrome={show_chrome}
    on_chrome_toggle="on_chrome_toggle">

    <template slot="left">
        <ActivityBar panels={activity_panels} on_panel_change="on_panel_change">
            <div if={active_panel_id == "samples"}>
                <Tree on_activate="on_case_activate" />
            </div>
        </ActivityBar>
    </template>

    <template slot="menu">
        <menu-bar>
            <menu-item label={t("menu.view")}>
                <menu-item label={t("menu.theme_toggle")} on-click="on_menu_theme_toggle" />
            </menu-item>
        </menu-bar>
    </template>

    <template slot="title">
        <Button label="Docs" ghost="" />
    </template>

    <template slot="bottom">
        <div class="shell-output">Output panel</div>
    </template>

    <template slot="footer">
        <status_bar items={status_items} />
    </template>

    <div class="case-host">
        <WelcomeCase />
    </div>
</tab_window>
```

## dialog

模态对话框，依赖父窗口 Root 层渲染。

### 属性

| 属性 | 类型 | 说明 |
|------|------|------|
| `title` | 字符串 | 对话框标题 |
| `width` | 数字 | 宽度（px） |
| `margin_top` | 数字 | 顶部偏移 |

### 用法

ViewModel 需 `#[window]` 宏（注入 `__rml_state`，其 `window_handle` 字段供对话框 `open`/`close` 使用）。在父窗口 `on_loaded` 中调用：

```rust
window.defer(cx, |window, cx| {
    LoginDialog::default().open(window, cx);
});
```

`demo/src/shell/login_dialog.rml`：

```html
<dialog title="RML Demo" width="420" margin_top="120">
    <div class="login">
        <input model={username} placeholder={t("login.username")} />
        <Button label={t("login.submit")} primary="" on-click={on_login} />
    </div>
</dialog>
```

## component

可复用 UI 片段，无窗口语义。配合 `#[component]` 宏在父视图中嵌入：

```html
<!-- cases/button_case.rml -->
<component>
    <div class="case-pane">
        <Button label={t("case.button.primary")} primary="" on-click={on_button_demo_click} />
    </div>
</component>
```

父窗口引用：`<ButtonCase />`（PascalCase 与 struct 名对应）。

## 常见错误

1. **多个根元素** — 每个 `.rml` 只能有一个根标签。
2. **混用 `window` 与 `tab_window` 插槽** — `slot="left"` 等仅 `tab_window` / `modern_window` 支持。
3. **dialog 当独立窗口** — dialog 不能 `IWindow::open` 单独启动，必须从父窗口 `open()`。
4. **未 `#[window]` 却用 dialog** — 缺少 `__rml_state.window_handle` 字段。

## 相关文档

- [activity-bar.md](./activity-bar.md)、[menu.md](./menu.md)、[status-bar.md](./status-bar.md)
- [插槽机制](../slots.md)
- [自定义组件](../custom-components.md)
