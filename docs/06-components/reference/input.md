# Input

## 概述

`Input` 是 gpui-component 文本输入框的 RML 封装，**Stateful** 组件。codegen 要求 ViewModel 持有 `input_state: Entity<InputState>` 字段（路由表 `state_field: "input_state"`），生成 `rml_ui::Input::new(&self.input_state)`。

> 双向绑定场景更推荐小写 `<input model={field}>`（见 [builtin-html.md](./builtin-html.md)），无需手动管理 `Entity<InputState>`。

## 基本用法

```html
<!-- 方式 A：model 指令（推荐） -->
<input model={username} placeholder="用户名" />

<!-- 方式 B：PascalCase 组件（需 code-behind 准备 input_state） -->
<Input placeholder="搜索" onchange={on_search_change} />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `placeholder` | 字符串 | — | 占位文字 |
| `label` | 字符串 | `{expr}` | 标签文字 |
| `tooltip` | 字符串 | — | 悬停提示 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `value` | 任意 | `{expr}` | 静态显示值（单向） |
| `small` / `large` 等 | 布尔标志 | — | 与其他组件共享的尺寸/变体属性 |

## 事件

| 事件 | 回调签名 | 说明 |
|------|----------|------|
| `onchange` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 输入内容变化；**仅 Input/TextInput 支持** |
| `on-click` | `fn(&mut self, ev: &ClickEvent, cx: &mut Context<Self>)` | 点击 |

## 数据绑定

### `model` 指令（`<input>` / `<textarea>`）

在 ViewModel 上声明普通字段（如 `username: String`），RML 写：

```html
<input model={username} placeholder={t("login.username")} />
```

codegen 自动生成 `__rml_get_or_init_input_state` 管理 `Entity<InputState>` 与字段双向同步。见 `demo/src/cases/two_way_case.rml`：

```html
<input model={name} placeholder={t("demo.name_placeholder")} />
<input model={age} placeholder={t("demo.age_placeholder")} />
```

### `<Input>` 组件

需手动在 struct 中声明：

```rust
input_state: Entity<InputState>,
```

并在 `on_loaded` 中初始化（若未使用 `model` 指令）。

## 子节点 / 插槽

不支持容器子节点；可选单个文本子节点作为 `label`。

## 完整示例

`demo/src/shell/login_dialog.rml`：

```html
<dialog title="RML Demo" width="420">
    <div class="login-form">
        <input model={username} placeholder={t("login.username")} />
        <Button label={t("login.submit")} primary="" on-click={on_login} />
    </div>
</dialog>
```

## 常见错误

1. **`<Input model={field}>`** — `model` 指令仅适用于小写 `<input>`/`<textarea>`，不能用于 `<Input>`。
2. **缺少 `input_state` 字段** — 使用 `<Input>` 时未声明 `Entity<InputState>` 会导致编译错误。
3. **在 Button 上写 `onchange`** — 仅 Input/TextInput 映射 `onchange`。

## 相关组件

- [text-input.md](./text-input.md) — 同 `Input` 的别名标签
- [builtin-html.md](./builtin-html.md) — `model` 双向绑定详解

## RML 未覆盖的 gpui-component API

`.prefix()`、`.suffix()`、`.mask()`、`.cleanable()` 等 Input builder 方法需 Rust 手写。
