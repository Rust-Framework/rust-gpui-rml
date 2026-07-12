# Button

## 概述

`Button` 是 gpui-component 按钮的 RML 封装，标签为 PascalCase `<Button>`。路由表注册为 **Stateless** 组件，codegen 生成 `rml_ui::Button::new(id)` 链式调用。

> 小写 `<button>` 属于内置 HTML 轨，映射为 `gpui::div()`，**不是**本组件。需要按钮样式与交互时请使用 `<Button>`。

## 基本用法

```html
<Button label="提交" primary="" on-click={on_submit} />
<Button label={t("demo.click_btn")} ghost="" on-click={on_click} />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 按钮文字，`.label(...)` |
| `icon` | 字符串 | — | 图标名称（PascalCase），如 `icon="Play"`，映射到 `IconName::Play` |
| `tooltip` | 字符串 | — | 悬停提示 |
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` / `ghost` / `link` / `text` | 布尔标志 | — | 空值或 `true` 时启用对应变体，如 `primary=""` |
| `small` / `xsmall` / `large` | 布尔标志 | — | 尺寸 |
| `compact` / `loading` | 布尔标志 | — | 紧凑 / 加载态 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `selected` | 布尔 | `{expr}` | 选中态 |
| `font_*` | 布尔标志 | — | `font_bold`、`font_semibold` 等字体权重 |
| `h_flex` / `v_flex` | 布尔标志 | — | 布局快捷方法（较少用于按钮） |

## 事件

| 事件 | 回调签名（code-behind） | 说明 |
|------|-------------------------|------|
| `on-click` | `fn(&mut self, ev: &ClickEvent, cx: &mut Context<Self>)` | 点击；支持 `on-click={method}` 或 `on-click="method"` |

## 数据绑定

- `label={expr}` — 动态文字
- `disabled={expr}` — 条件禁用
- `selected={expr}` — 条件选中

子节点文本可作为 `label` 的简写（与 `label=` 属性互斥）：

```html
<Button primary="" on-click={on_click}>点击我</Button>
```

## 子节点 / 插槽

不支持容器子节点。仅单个文本子节点可替代 `label` 属性。

## 完整示例

来自 `demo/src/cases/button_case.rml`：

```html
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.button.title")}</h2>
        <p class="count">{button_demo_text}</p>
        <div class="button-row">
            <Button label={t("case.button.primary")} primary="" on-click={on_button_demo_click} />
            <Button label={t("case.button.ghost")} ghost="" on-click={on_button_demo_click} />
            <Button label={t("case.button.danger")} danger="" on-click={on_button_demo_click} />
        </div>
    </div>
</component>
```

## 常见错误

1. **使用 `variant="primary"`** — RML 不支持 `variant` 属性，应写 `primary=""`。
2. **混用 `<button>` 与 `<Button>`** — 小写 `button` 不走组件路由，无 gpui-component 样式。
3. **期望 `onchange`** — Button 仅支持 `on-click`，`onchange` 在 codegen 中被忽略。

## 相关组件

- [button-group.md](./button-group.md) — 按钮组
- [builtin-html.md](./builtin-html.md) — 原生 `button` 轨

## RML 未覆盖的 gpui-component API

`.dropdown_menu()`、`.keyboard_shortcut()` 等 builder 方法需在 Rust 中手写 `Button::new(...)` 构造。
