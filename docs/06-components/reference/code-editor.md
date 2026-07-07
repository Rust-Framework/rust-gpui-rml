# CodeEditor

## 概述

`CodeEditor` 是基于 gpui-component `Input` 的多行代码编辑器封装，**Stateful** 组件。codegen 生成 `rml_ui::Input::new(...).code_editor(language).multi_line(true)`，并自动应用等宽字体、贴边 padding、关闭聚焦边框等代码编辑器语义默认值。

> 与 `<Input>` 的区别：`CodeEditor` 内置 `multi_line(true)` + `code_editor(language)` 语法高亮，字段类型为 `Option<Entity<InputState>>`（声明式 `value` 时无需手动管理 state）。

## 基本用法

```html
<!-- 声明式 value + language（推荐）：内联创建 InputState，无需 code-behind -->
<CodeEditor value={rml_sample} language="rml" />

<!-- ref 模式：手动管理 editor_state，配合 on-change 事件 -->
<CodeEditor ref="rml_editor" language="rust" on-change={on_editor_change} />
```

## 属性

| 属性 | 类型 | 绑定 | 默认值 | 说明 |
|------|------|------|--------|------|
| `value` | 字符串 | `{expr}` 或静态字符串 | — | 初始代码内容；声明式内联创建 InputState |
| `language` | 字符串 | — | `"rml"` | 语法高亮语言（`rml` / `rust` / `json` 等） |
| `bordered` | 布尔 | — | `true`（Input 默认） | 外边框开关；`bordered="false"` 关闭 |
| `focus_bordered` | 布尔 | — | `false` | 聚焦边框开关；`focus_bordered="true"` 启用 |
| `context-menu` | 方法名 | — | — | 自定义右键菜单构建方法 |

> 布尔属性语义：空值或 `"true"` 为 true，其他为 false（如 `bordered` / `bordered="true"` 均为 true）。

## 样式定制

CodeEditor 的语义默认值均可通过**通用样式属性**（见 [builtin-html.md](./builtin-html.md) 样式属性章节）声明式覆盖。未设置的项使用默认值，设置的项用用户值。

| 通用样式属性 | 覆盖的默认值 | 示例 |
|--------------|--------------|------|
| `font-family` | `cx.theme().mono_font_family` | `font-family="Fira Code"` |
| `font-size` | `cx.theme().mono_font_size` | `font-size="14px"` |
| `padding` | `p_0()`（贴边） | `padding="8px"` |
| `width` | `w_full()`（铺满） | `width="600px"` |
| `height` | `360px`（固定高度） | `height="500px"` 或 `height="full"` |

示例：

```html
<!-- 自定义字体 + 字号 + padding + 高度 -->
<CodeEditor
    value={code}
    language="rust"
    font-family="Fira Code"
    font-size="13px"
    padding="12px"
    height="full"
    bordered="false"
/>
```

## 主题

默认值引用以下主题项（见 `cx.theme()`）：

- `mono_font_family` — 等宽字体族（代码编辑器默认字体）
- `mono_font_size` — 等宽字号（代码编辑器默认字号）

如需全局调整代码编辑器字体，修改主题即可；如需单个编辑器使用不同字体，用 `font-family` / `font-size` 属性覆盖。

## 事件

CodeEditor 同 Input，事件通过 `InputState: EventEmitter<InputEvent>` + `cx.subscribe` 订阅模式处理（非 setter 链路）：

| 事件 | 回调签名 | 说明 |
|------|----------|------|
| `on-change` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 内容变化 |
| `on-enter` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 回车键 |
| `on-focus` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 获得焦点 |
| `on-blur` | `fn(&mut self, state: &InputState, cx: &mut Context<Self>)` | 失去焦点 |

## 数据绑定

### 声明式 `value`（推荐）

```html
<CodeEditor value={rml_sample} language="rml" />
```

codegen 自动生成 `InputState::new(...).code_editor(language).multi_line(true).default_value(&code)`，无需在 ViewModel 中声明 `editor_state` 字段或 `on_loaded` 初始化。

### `ref` 模式 + 事件

```html
<CodeEditor ref="rml_editor" language="rust" on-change={on_editor_change} />
```

ViewModel 需声明字段（类型为 `Option<Entity<InputState>>`），在 `on_loaded` 中按需初始化。事件回调通过 `cx.subscribe` 订阅 `InputEvent::Change` 等触发。

## 子节点 / 插槽

不支持容器子节点。

## 使用场景举例

### 1. Tab 内嵌代码展示（accordion_case.rml）

```html
<TabBar bordered selected-index={code_tab} on-click={on_code_tab_change}>
    <Tab label=".rml">
        <CodeEditor ref="rml_editor" value={rml_sample} language="rml" bordered="false" />
    </Tab>
    <Tab label=".rml.rs">
        <CodeEditor ref="rust_editor" value={rust_sample} language="rust" bordered="false" />
    </Tab>
</TabBar>
```

### 2. LSP 工作区（高度铺满）

```html
<CodeEditor ref="workspace_editor" language="rust" height="full" on-change={on_code_change} />
```

### 3. 文档展示（自定义样式）

```html
<CodeEditor
    value={snippet}
    language="json"
    font-size="12px"
    padding="16px"
    height="auto"
    bordered="false"
    focus_bordered="false"
/>
```

## 默认值清单

| 项 | 默认值 | 可覆盖属性 |
|----|--------|------------|
| 字体族 | `cx.theme().mono_font_family` | `font-family` |
| 字号 | `cx.theme().mono_font_size` | `font-size` |
| padding | `0`（贴边） | `padding` |
| 宽度 | `100%`（铺满） | `width` |
| 高度 | `360px` | `height`（如 `height="full"` 铺满 / `height="500px"` 固定） |
| 外边框 | `true`（Input 默认） | `bordered` |
| 聚焦边框 | `false` | `focus_bordered` |
| language | `"rml"` | `language` |
| multi_line | `true`（内置） | — 不可覆盖 |

> 原则：未设置的样式项使用上表默认值；用户通过 RML 属性设置的项优先于默认值，且不会重复生成默认调用。

## RML 未覆盖的 gpui-component API

以下 Input builder 方法需在 Rust code-behind 手写（RML 暂不映射）：

- `.prefix()` / `.suffix()` — 前后缀元素
- `.mask()` / `.mask_toggle()` — 密码遮罩
- `.cleanable()` — 清除按钮
- `.placeholder()` — 占位文字（CodeEditor 多行模式下不显示）
- `.appearance(bool)` — 外观开关

## 相关组件

- [input.md](./input.md) — CodeEditor 的基础组件
- [text-input.md](./text-input.md) — 单行文本输入
- [builtin-html.md](./builtin-html.md) — 通用样式属性参考
