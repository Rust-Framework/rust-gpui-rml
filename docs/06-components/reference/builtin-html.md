# 内置 HTML 标签

## 概述

小写 HTML 标签走 **基础轨**（`BuiltinTag`），映射到 `gpui::div()` 等基础构造。与 PascalCase 扩展组件（`component_lookup`）是双轨策略：简单布局用 HTML 标签，交互控件用 `<Button>` / `<Input>` 等。

完整映射见 `crates/engine/src/tags.rs` 与 [2.2 标签映射](../../02-syntax/tags-mapping.md)。

## 标签列表

| 标签 | GPUI 构造 | 自闭合 | 说明 |
|------|-----------|--------|------|
| `div` | `gpui::div()` | 否 | 通用容器 |
| `span` | `gpui::div()` | 否 | 行内容器 |
| `p` | `gpui::div()` | 否 | 段落 |
| `h1`–`h6` | `gpui::div().text_size(px(...))` | 否 | 标题（32/28/24/20/18/16 px） |
| `button` | `gpui::div()` | 否 | **非** gpui-component Button |
| `input` | 见下文 | 是 | 支持 `value={field}` 自动双向绑定 |
| `textarea` | 见下文 | 否 | 支持 `value={field}` 自动双向绑定 |
| `ul` / `ol` | `gpui::div().flex().flex_col()` | 否 | 列表容器 |
| `li` | `gpui::div()` | 否 | 列表项 |
| `img` | `gpui::div()` | 是 | 占位（扩展轨未单独注册 Image 组件） |
| `a` | `gpui::div()` | 否 | 链接占位 |
| `label` | `gpui::div()` | 否 | 标签占位 |
| `br` | `gpui::div().hidden()` | 是 | 换行占位 |

## input / textarea 自动双向绑定

小写 `<input>` / `<textarea>` 使用 `value={field}` 自动触发双向绑定，codegen 生成 `rml_ui::Input::new(&self.__rml_get_or_init_input_state(...))` 并实现双向同步。

```html
<input value={name} placeholder={t("demo.name_placeholder")} />
<textarea value={description} placeholder="描述" />
```

要求：

- ViewModel 字段为 `String`（或 codegen 支持的类型）
- 双向绑定通过 `value={field}` 自动推断，无需额外指令

## 通用属性与事件

### 属性

| 属性 | 说明 |
|------|------|
| `class` | CSS 类名 |
| `id` | 元素 ID |
| `style` | 内联样式 |
| `ref` | 元素引用名 |

### 事件（div 等原生元素）

| 事件 | 说明 |
|------|------|
| `on-click` | 点击 |
| `onmouseenter` / `onmouseleave` | 悬停 |
| 等 | 见 [事件绑定](../../05-events/event-binding.md) |

小写 `button` 支持原生事件，但**无** gpui-component 按钮样式。

## 完整示例

`demo/src/cases/two_way_case.rml`：

```html
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.two_way.title")}</h2>
        <div class="form">
            <input value={name} placeholder={t("demo.name_placeholder")} />
            <input value={age} placeholder={t("demo.age_placeholder")} />
            <p class="profile">{profile_summary}</p>
        </div>
    </div>
</component>
```

## 常见错误

1. **用 `<button>` 期望按钮组件** — 应使用 `<Button>`。
2. **用 `<input type="checkbox">` 期望 Checkbox** — 应使用 `<Checkbox>`。

## 相关组件

- [input.md](./input.md) — PascalCase Input 组件
- [button.md](./button.md) — PascalCase Button 组件
