# 组件组合模式

> **本节目标**：统一理解 RML 中「谁包裹谁」「子节点是 UI 还是元数据」三类组合模式，避免与 KeyBinding 类似的语法偏差。

## 模式总览

| 模式 | 典型组件 | 触发/宿主关系 | 子节点语义 |
|------|----------|---------------|------------|
| **声明式元数据子节点（焦点）** | `KeyBinding` → `Input` | 快捷键挂在焦点宿主上 | 子节点不渲染，仅 codegen 元数据 |
| **声明式元数据子节点（作用域）** | `Shortcut` → `ShortcutScope` | 快捷键挂在面板/页面作用域 | `<Shortcut/>` 不渲染；其余为内容 |
| **slot 触发器** | `Dialog`、`Popover`、`Sheet`、`HoverCard`、`Notification` | `slot="trigger"` 标记触发器 | 其余兄弟为内容 |
| **首子节点触发器** | `dropdown-menu`、`context-menu` | 第一个非 `menu-item` 子节点为触发器 | 其余为菜单项 |

三种模式**并存**，由各类组件的 codegen 分别实现；不要混用语法（例如不要用 KeyBinding 包裹 div 代替 ShortcutScope）。

---

## 1. 声明式元数据子节点（KeyBinding — 焦点宿主）

快捷键**必须**写在焦点控件**内部**：

```html
<Input ref="field" placeholder="编辑">
    <KeyBinding key="Ctrl+S" on-press={on_save} />
</Input>
```

**实现**：`crates/engine/src/compiler/components/key_binding/attach.rs` + 各焦点宿主 translator。

**规则**：

- 宿主仅接受 `<KeyBinding>` 子节点
- KeyBinding 作为子节点时必须自闭合（无嵌套内容）
- 不支持外层 `<KeyBinding>…</KeyBinding>` 包裹写法

详见 [key-binding.md](./reference/key-binding.md)。

---

## 1b. 声明式元数据子节点（ShortcutScope — 作用域级）

作用域快捷键**必须**写在 `ShortcutScope` **内部**：

```html
<ShortcutScope>
    <Shortcut key="Ctrl+G" on-press={on_global_save} />
    <div>...</div>
</ShortcutScope>
```

**实现**：`crates/engine/src/compiler/components/shortcut_scope/attach.rs`。

**规则**：

- `ShortcutScope` 至少一个 `<Shortcut/>` + 一个内容子节点
- `<Shortcut>` 必须自闭合；单独使用 `<Shortcut>` 编译报错
- 不支持 KeyBinding 包裹 div 等替代写法

详见 [key-binding.md](./reference/key-binding.md)。

---

## 2. slot 触发器（浮层 / 对话框）

**标准**：显式 `slot="trigger"` 标记触发元素。

```html
<Dialog open={dialog_open} title="确认">
    <Button slot="trigger" label="打开" />
    <p>对话框正文</p>
    <template slot="footer">
        <Button label="确定" on-click={on_ok} />
    </template>
</Dialog>
```

适用：`Dialog`、`AlertDialog`、`Popover`、`Sheet`、`HoverCard`、`Notification` 等。

---

## 3. 首子节点触发器（菜单）

**标准**：第一个非菜单项子元素为触发器，**不使用** `slot="trigger"`。

```html
<dropdown-menu>
    <Button label="操作" />
    <menu-item label="复制" on-click={on_copy} />
    <menu-item label="粘贴" on-click={on_paste} />
</dropdown-menu>
```

实现：`compiler/menu/children.rs` 的 `partition_menu_children`。

---

## Demo 案例布局规范（CaseDocPage）

案例页演示区应使用 **`demo-section`** 分场景，避免多个 `<Card>` 纵向黏连：

```html
<CaseDocPage title={t("case.xxx.title")} ...>
    <template slot="demo">
        <div class="demo-section">
            <h3>场景一</h3>
            <p>说明文字</p>
            <!-- 演示组件 -->
        </div>
        <div class="demo-section">
            <h3>场景二</h3>
            ...
        </div>
    </template>
</CaseDocPage>
```

- 多场景用 `demo-section`，不用多个并列 `<Card title="...">`
- 子元素间距优先用父级 `gap`，避免 `style="margin-top: …"`
- 案例标题走 `t("case.*.title")`；场景说明可用中文硬编码（见 demo 规范）

详见 [demo-case-conventions.md](../11-cookbook/demo-case-conventions.md)。

---

## 语法规范速查

| 能力 | ✅ RML 写法 | ❌ 错误写法 |
|------|------------|------------|
| 条件渲染 | `if={cond}` | `r:if`、`v-if` |
| 列表 | `each={x in xs}` `key={x.id}` | `r:each`、`v-for` |
| 双向绑定 | `value={field}` | `r:model`、`v-model` |
| 事件 | `on-click={fn}` | `onclick`、`on:click` |
| Input 变更 | `on-change={fn}` | `onchange`（无连字符） |

完整命名规范见 [syntax-philosophy.md](../02-syntax/syntax-philosophy.md) 与 Skill [01-naming-conventions.md](../../.trae/skills/rml-component/01-naming-conventions.md)。
