# KeyBinding 与 ShortcutScope

## 概述

RML 提供**两种互斥**的声明式快捷键模式，分别对应不同场景：

| 模式 | 组件 | 场景 |
|------|------|------|
| **焦点宿主快捷键** | `<KeyBinding>` 作为 Input/CodeEditor 等子节点 | 输入框、编辑器等获得焦点时生效 |
| **作用域快捷键** | `<ShortcutScope>` + `<Shortcut>` 子节点 | 面板/页面级快捷键，无需焦点宿主 |

二者共用 `normalize_key_source` 将 `Ctrl+S` 等形式归一化为 GPUI `Keystroke::parse` 语法（`ctrl-s`）。

---

## 焦点宿主：KeyBinding

快捷键**必须**声明在获得焦点的宿主**内部**：

```html
<Input ref="demo_input" placeholder="按 Ctrl+S 保存">
    <KeyBinding key="Ctrl+S" on-press={on_save} />
    <KeyBinding key="Ctrl+O" on-press={on_open} />
    <KeyBinding key="Escape" on-press={on_clear} />
</Input>
```

编译器（`key_binding/attach.rs`）将 `KeyBinding` 子节点视为**声明式元数据**，不渲染为 Input 的视觉子节点，而是自动用 KeyBinding 链包裹 Input。

### 支持的焦点宿主

| 标签 | 说明 |
|------|------|
| `Input` / `TextInput` | 文本输入 |
| `NumberInput` | 数字输入 |
| `CodeEditor` | 代码编辑器 |
| `Textarea` / `textarea` | 多行文本 |

宿主**仅允许** `<KeyBinding>` 作为子节点；混入其他子元素将编译报错。不支持 `<KeyBinding>…</KeyBinding>` 外层包裹写法。

### KeyBinding 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `key` | 字符串 | — | 快捷键，如 `Ctrl+S`、`Escape` |
| `when` | 布尔 | `{expr}` | 是否启用（默认 true） |
| `on-press` | 事件 | `{handler}` | 命中后回调 |

---

## 作用域级：ShortcutScope

用于 Save/Open 等**非焦点宿主**快捷键。唯一写法：

```html
<ShortcutScope>
    <Shortcut key="Ctrl+G" on-press={on_global_save} />
    <Shortcut key="Ctrl+H" on-press={on_global_help} />
    <div>...</div>
</ShortcutScope>
```

- `<Shortcut>` 为**声明式元数据**，不渲染；编译器转为 `ShortcutScope::shortcut(key, when, handler)` 调用
- `<ShortcutScope>` 至少包含一个 `<Shortcut/>` 与一个内容子节点
- 单独使用 `<Shortcut>` 将编译报错
- 作用域容器通过 `on_key_down` 监听子树键盘事件冒泡

### Shortcut 属性

与 KeyBinding 相同：`key`（静态）、`when`（绑定，默认 true）、`on-press`（事件）。

### ShortcutScope

无专用属性；可像普通容器一样使用 `class` / `style` 等。

---

## 事件

| 事件 | 回调签名 | 说明 |
|------|----------|------|
| `on-press` | `fn(&mut self, cx: &mut Context<Self>)` | 通过 entity 捕获桥接到视图方法 |

## 完整示例

```html
<div class="demo-section">
    <h3>Input 内快捷键</h3>
    <Input ref="field">
        <KeyBinding key="Ctrl+S" on-press={on_save} />
    </Input>
</div>

<div class="demo-section">
    <h3>作用域快捷键</h3>
    <ShortcutScope>
        <Shortcut key="Ctrl+G" on-press={on_global_action} />
        <div class="button-row">
            <Button label="点击此区域" />
        </div>
    </ShortcutScope>
</div>
```

## 常见错误

1. **在 Button 上挂 KeyBinding 子节点** — 仅焦点宿主支持；Button 级快捷键请用 `ShortcutScope`。
2. **单独写 `<Shortcut>`** — 必须放在 `<ShortcutScope>` 内。
3. **用 `<KeyBinding>` 包裹 div 做全局快捷键** — 编译期拒绝；请改用 `ShortcutScope`。
4. **KeyBinding / Shortcut 子节点内再嵌套元素** — 必须自闭合。

## 相关文档

- [input.md](./input.md) — Input 与 KeyBinding 子节点
- [composition-patterns.md](../composition-patterns.md) — 声明式元数据子节点模式总览
