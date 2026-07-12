# CodeEditor

## 概述

`CodeEditor` 是基于 gpui-component `Input` 的多行代码编辑器封装，**Stateful** 组件。codegen 生成 `rml_ui::Input::new(...).code_editor(language).multi_line(true)`，并设置代码编辑器**行为**默认值（`focus_bordered(false)`、`bordered(false)`）。

布局与视觉（宽高、字体、padding、背景等）**不在 codegen 中硬编码**，由使用方通过 RML 样式属性、`class` + CSS 变量、或主题覆盖层定制。

> 与 `<Input>` 的区别：`CodeEditor` 内置 `multi_line(true)` + `code_editor(language)` 语法高亮，字段类型为 `Option<Entity<InputState>>`（声明式 `value` 时无需手动管理 state）。

## 基本用法

```html
<!-- 声明式 value + language（推荐）：内联创建 InputState，无需 code-behind -->
<CodeEditor class="rml-code-editor" value={rml_sample} language="rml" height="320px" />

<!-- 填满父容器（父级需建立 flex 高度链） -->
<CodeEditor class="rml-code-editor" height="full" />

<!-- ref 模式：手动管理 editor_state，配合 on-change 事件 -->
<CodeEditor class="rml-code-editor" ref="rml_editor" language="rust" on-change={on_editor_change} />
```

## 属性

| 属性 | 类型 | 绑定 | 默认值 | 说明 |
|------|------|------|--------|------|
| `value` | 字符串 | `{expr}` 或静态字符串 | — | 初始代码内容；声明式内联创建 InputState |
| `language` | 字符串 | — | `"rml"` | 语法高亮语言（`rml` / `rust` / `json` 等） |
| `bordered` | 布尔 | — | `false` | 外边框开关 |
| `focus_bordered` | 布尔 | — | `false` | 聚焦边框开关 |
| `context-menu` | 方法名 | — | — | 自定义右键菜单构建方法 |

> 布尔属性语义：空值或 `"true"` 为 true，其他为 false。

## 样式定制

CodeEditor 支持全部**通用样式属性**（见 [builtin-html.md](./builtin-html.md)）。推荐通过 `class="rml-code-editor"` 应用 demo 内置的现代默认样式，并在 CSS / 主题包中覆盖：

| CSS 变量 / 属性 | 默认（`.rml-code-editor`） | 说明 |
|-----------------|---------------------------|------|
| `background` | `var(--editor-surface)` | 背景色（主题 token，可在 themes/*.css 覆盖） |
| `font-family` / `font-size` | 系统等宽 / `13px` | 可在 CSS 或 RML `font-family` / `font-size` 覆盖 |
| `min-height` | `12rem` | 可在 CSS 覆盖 |
| `width` / `height` | `100%` | 配合 RML `height="full"` 与父级 flex 链 |

示例：

```html
<CodeEditor
    class="rml-code-editor my-editor"
    value={code}
    language="rust"
    height="full"
    font-family="Fira Code"
    padding="12px"
/>
```

```css
/* themes/ocean.css 或业务 styles.css */
.rml-code-editor {
    font-size: 14px;
    background: var(--code-bg);
    min-height: 16rem;
}
```

## 事件

CodeEditor 同 Input，事件通过 `InputState: EventEmitter<InputEvent>` + `cx.subscribe` 订阅模式处理：

| 事件 | 回调签名 | 说明 |
|------|----------|------|
| `on-change` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 内容变化 |
| `on-enter` | 同上 | 回车键 |
| `on-focus` | 同上 | 获得焦点 |
| `on-blur` | 同上 | 失去焦点 |

## 数据绑定

### 声明式 `value`（推荐）

```html
<CodeEditor class="rml-code-editor" value={code_sample} language="rml" height="400px" />
```

### ref + on_loaded

在 `.rml.rs` 的 `on_loaded` 中预创建 `InputState`：

```rust
self.__rml_state.get_or_init_ref("editor", window, cx, |w, c| {
    InputState::new(w, c)
        .code_editor("rust")
        .multi_line(true)
        .default_value("fn main() {}")
});
```

RML：

```html
<CodeEditor class="rml-code-editor" ref="editor" height="full" on-change={on_editor_change} />
```

## 布局建议

| 场景 | 推荐写法 |
|------|----------|
| 固定高度展示 | `height="320px"` + `class="rml-code-editor"` |
| 填满工作区 | 父容器 `flex:1; min-height:0` + `height="full"` |
| 案例页 Tabs | Tabs `style="flex:1; min-height:0"` + CodeEditor `height="full"` |

## 与 Input 对比

| | Input / TextInput | CodeEditor |
|--|-------------------|------------|
| 多行 | 需 `multi_line` | 内置 |
| 语法高亮 | 无 | `code_editor(language)` |
| 默认 bordered | Input 默认 | `false`（避免与容器边框重叠） |
| 样式默认值 | 无 codegen 硬编码 | 无 codegen 硬编码；推荐 `rml-code-editor` class |
