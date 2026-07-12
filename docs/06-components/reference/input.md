# Input

## 概述

`Input` 是 gpui-component 文本输入框的 RML 封装，**Stateful** 组件。支持 `value={field}` 自动双向绑定（InputStateBridge 机制），与小写 `<input>` 行为一致。

## 基本用法

```html
<!-- PascalCase Input 自动双向绑定 -->
<Input value={username} placeholder="用户名" />

<!-- 小写 input 同样自动双向绑定 -->
<input value={username} placeholder="用户名" />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `placeholder` | 字符串 | — | 占位文字 |
| `label` | 字符串 | `{expr}` | 标签文字 |
| `tooltip` | 字符串 | — | 悬停提示 |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `value` | 任意 | `{field}` | **自动双向绑定**（InputStateBridge 机制） |
| `small` / `large` 等 | 布尔标志 | — | 与其他组件共享的尺寸/变体属性 |

## 事件

| 事件 | 回调签名 | 说明 |
|------|----------|------|
| `on-change` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 输入内容变化；**仅 Input/TextInput 支持** |
| `on-click` | `fn(&mut self, ev: &ClickEvent, cx: &mut Context<Self>)` | 点击 |

## 数据绑定

### `value={field}` 自动双向绑定

`<Input value={field}>` 与小写 `<input value={field}>` 行为一致，均通过 InputStateBridge 自动双向同步：

- **正向同步**（VM→UI）：render 时对比字段版本号，变化则 `InputState::set_value`
- **反向同步**（UI→VM）：订阅 `InputEvent::Change`，回写字段 + `bump_version` + `cx.notify()`

```html
<Input value={username} placeholder="用户名" />
```

支持 `value={field | Converter}` 转换器语法（如 `Currency`、`Percent`）。

### `<Input>` ref 模式

通过 `ref="name"` 引用手动管理的 `Entity<InputState>`，适用于需要自定义 InputState 配置的场景。需在 struct 中声明 `input_state: Option<Entity<InputState>>` 并在 `on_loaded` 中初始化。

## 子节点 / 插槽

### KeyBinding 声明式子节点（推荐）

`Input` 可包含若干自闭合 `<KeyBinding>` 子节点，编译器自动包裹监听快捷键：

```html
<Input ref="demo_input" placeholder="按 Ctrl+S">
    <KeyBinding key="Ctrl+S" on-press={on_save} />
</Input>
```

仅允许 `KeyBinding` 作为子节点；不可混入文本或其他元素。详见 [key-binding.md](./key-binding.md)。

### 文本 label

可选单个文本子节点作为 `label`（与 KeyBinding 子节点互斥使用场景需注意：有 KeyBinding 时勿再加文本子节点）。

## 完整示例

```html
<Input value={username} placeholder="用户名" />
<p>当前用户名：{username}</p>
```

## 常见错误

1. **缺少 `input_state` 字段** — 仅 `ref` 模式需要；`value={field}` 自动双向绑定无需手动声明 `Entity<InputState>`。
2. **在 Button 上写 `on-change`** — 仅 Input/TextInput 映射 `on-change`。

## 相关组件

- [key-binding.md](./key-binding.md) — Input 内声明式快捷键
- [text-input.md](./text-input.md) — 同 `Input` 的别名标签
- [builtin-html.md](./builtin-html.md) — 小写 `<input>` 双向绑定详解

## RML 未覆盖的 gpui-component API

`.prefix()`、`.suffix()`、`.mask()`、`.cleanable()` 等 Input builder 方法需 Rust 手写。
