# CodeEditor 声明式定制能力改造

## Context

`crates/engine/src/compiler/code_editor/gen.rs` 的 `style_chain` 中存在大量**无条件硬编码**的默认样式调用：

```rust
let style_chain = format!(
    ".font_family(cx.theme().mono_font_family.clone())\n            \
     .text_size(cx.theme().mono_font_size)\n            \
     .p_0(){}\n            \
     .focus_bordered(false){}",
    height_chain, border_chain
);
```

问题：
1. **无法定制**：`focus_bordered` 不在通用样式通道（`style_attr.rs`）中，用户无法通过 RML 覆盖；其余项虽可被 setter 链覆盖，但无文档说明。
2. **违反"禁止硬编码"原则**：默认值无条件生成，而非"先看 rml 有没有设置 - 没有设置则使用默认值"。
3. **文档缺失**：`docs/06-components/reference/` 有 33 个组件文档，但无 `code-editor.md`，开发者无法知道哪些属性/样式可定制。

目标：按用户三步要求，先文档、再确认通用通道、最后去硬编码，让 CodeEditor 的默认行为可被 RML 属性声明式覆盖。

## 现状关键发现

### 通用样式通道（已存在，无需改动）
[style_attr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs) 通过 CSS mapper 统一处理以下属性，所有组件共用，经 [component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 的 `component_static_setter` → `apply_style_attr` 路由：

- `font_family` / `font_size` / `padding` / `width` / `height` / `border` / `border_color` 等均已在通用通道中
- 用户写 `font-family="Inter"` → 归一化为 `font_family` → `apply_style_attr` 生成 `.font_family("Inter")` → 在 setter 链中应用（位于 style_chain 之后，可覆盖默认值）

### CodeEditor 特有属性（不在通用通道，需 gen.rs 专门处理）
- `value` / `language` — 构造器内联处理
- `h_full` — 快捷属性，已处理
- `context_menu` — 已处理
- `bordered` — 已处理，支持 static（`bordered="false"`）与 bind（`bordered={false}` / `bordered={field}`）
- `focus_bordered` — 已处理，支持 static 与 bind 两种形式（同 `bordered`）

### gen.rs 代码流程
1. `ctor_expr` = 构造器 + `style_chain`（硬编码默认值）
2. `for attr in &elem.attributes` 循环应用 setter 链（用户属性，在默认值之后，可覆盖）

## 实施步骤

### 步骤 1：编写 CodeEditor 文档

新建 [code-editor.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/reference/code-editor.md)，参考 [input.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/reference/input.md) 格式，包含以下章节：

1. **概述**：基于 Input 的代码编辑器，Stateful 组件，字段类型 `Option<Entity<InputState>>`
2. **基本用法**：声明式 value + language 示例
3. **属性清单**（表格）：
   | 属性 | 类型 | 绑定 | 默认值 | 说明 |
   |------|------|------|--------|------|
   | `value` | 字符串 | `{expr}` | — | 初始代码内容 |
   | `language` | 字符串 | — | `"rml"` | 语法高亮语言 |
   | `bordered` | 布尔 | — | `true`（Input 默认） | 外边框开关 |
   | `focus_bordered` | 布尔 | — | `false` | 聚焦边框开关 |
   | `h-full` | 布尔标志 | — | `false` | 高度铺满父容器 |
   | `context-menu` | 方法名 | — | — | 自定义右键菜单 |
4. **样式定制**（重点章节）：列出可通过通用样式属性覆盖的默认值
   - `font-family` → 覆盖默认 `cx.theme().mono_font_family`
   - `font-size` → 覆盖默认 `cx.theme().mono_font_size`
   - `padding` → 覆盖默认 `p_0()`
   - `width` → 覆盖默认 `w_full()`
   - `height` → 覆盖默认 `360px`
   - 每项附示例：`<CodeEditor font-family="Inter" font-size="14px" padding="8px" height="500px" />`
5. **主题**：说明默认值引用的主题项（`mono_font_family` / `mono_font_size`）
6. **事件**：`on-change` / `on-enter` / `on-focus` / `on-blur`（与 Input 一致）
7. **数据绑定**：`value={expr}` 单向绑定 + ref 模式
8. **使用场景举例**：Tab 内嵌代码编辑器、LSP 工作区（h-full）、文档展示
9. **默认值清单**：明确列出所有默认行为
10. **RML 未覆盖的 gpui-component API**：`prefix` / `suffix` / `mask` / `cleanable` 等

同步更新 [INDEX.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/reference/INDEX.md)，添加 code-editor 条目。

### 步骤 2：通用通道确认（无需改动代码）

通用样式属性已通过 `component_static_setter` → `apply_style_attr` 统一处理，CodeEditor 用户属性会经 setter 链应用。此架构正确，步骤 3 的改造保证用户属性在默认值之后应用即可覆盖。

### 步骤 3：去硬编码 — 改为"检查用户属性 → 未设置则用默认值"

修改 [code_editor/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs)：

**3.1 收集用户已设置的属性名**

```rust
use std::collections::HashSet;
let user_set: HashSet<&str> = elem.attributes.iter().filter_map(|attr| match attr {
    Attribute::Static { name, .. } | Attribute::Bind { name, .. } => Some(name.as_str()),
    _ => None,
}).collect();
```

**3.2 新增 `focus_bordered` 属性解析**（与 `bordered` 同模式，支持 static + bind）

```rust
let focus_bordered: Option<String> = elem.attributes.iter().find_map(|attr| match attr {
    Attribute::Static { name, value, .. } if name == "focus_bordered" => {
        Some((value.is_empty() || value.eq_ignore_ascii_case("true")).to_string())
    }
    Attribute::Bind { name, expr, .. } if name == "focus_bordered" => {
        Some(super::super::codegen::gen_expr_code(expr, &lv, &computed))
    }
    _ => None,
});
```

**3.3 将 style_chain 改为条件性默认值**

```rust
let mut defaults = String::new();
if !user_set.contains("font_family") {
    defaults.push_str("\n            .font_family(cx.theme().mono_font_family.clone())");
}
if !user_set.contains("font_size") {
    defaults.push_str("\n            .text_size(cx.theme().mono_font_size)");
}
if !user_set.contains("padding") {
    defaults.push_str("\n            .p_0()");
}
if !user_set.contains("width") && !user_set.contains("w") {
    defaults.push_str("\n            .w_full()");
}
// height：h_full 属性优先，其次用户 height/w，最后默认 360px
if h_full {
    defaults.push_str("\n            .h_full()");
} else if !user_set.contains("height") && !user_set.contains("h") {
    defaults.push_str("\n            .h(gpui::px(360.))");
}
// focus_bordered：用户指定时用用户值，否则默认 false
match focus_bordered {
    Some(b) => defaults.push_str(&format!("\n            .focus_bordered({})", b)),
    None => defaults.push_str("\n            .focus_bordered(false)"),
}
// bordered：保持现有 border_chain 逻辑（未指定时不生成，保持 Input 默认 true）
let border_chain = match bordered {
    Some(b) => format!("\n            .bordered({})", b),
    None => String::new(),
};
let style_chain = format!("{}{}", defaults, border_chain);
```

**3.4 将 `focus_bordered` 加入 `is_handled_inline` 列表**

在 setter 链循环中，避免 `bordered` / `focus_bordered` 被重复处理（static 和 bind 均需跳过）：
```rust
let is_handled_inline = match attr {
    Attribute::Static { name, .. } => {
        name == "value" || name == "language" || name == "context_menu" || name == "bordered" || name == "focus_bordered"
    }
    Attribute::Bind { name, .. } => {
        name == "value" || name == "bordered" || name == "focus_bordered"
    }
    _ => false,
};
```

### 步骤 4：更新单测

修改 [gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs) 测试模块：

- `gen_code_editor_minimal`：验证未设置任何样式属性时，生成全部默认值（font_family/text_size/p_0/w_full/h(360.)/focus_bordered(false)）
- 新增 `gen_code_editor_user_font_overrides_default`：验证 `font_family="Inter"` 时不再生成默认 `.font_family(cx.theme()...)`
- 新增 `gen_code_editor_focus_bordered_true`：验证 `focus_bordered="true"` 生成 `.focus_bordered(true)`
- 新增 `gen_code_editor_user_height_overrides_default`：验证 `height="500px"` 时不再生成默认 `.h(gpui::px(360.))`
- 新增 `gen_code_editor_user_padding_overrides_default`：验证 `padding="8px"` 时不再生成默认 `.p_0()`

## 验证方式

1. **单测**：`cargo test -p rust-rml-engine --lib compiler::code_editor` 全部通过
2. **全量回归**：`cargo test -p rust-rml-engine --lib` 831 个测试无回归
3. **编译验证**：`cargo build -p rust-rml-ui` 通过（TabBar 上一轮改动）
4. **Demo 验证**：`cargo build -p rust-rml-demo` 通过，运行 demo 确认 accordion_case.rml 的 CodeEditor 正常渲染
5. **文档检查**：确认 `docs/06-components/reference/code-editor.md` 存在且 INDEX.md 已更新

## 关键文件

| 文件 | 改动 |
|------|------|
| `docs/06-components/reference/code-editor.md` | 新建文档 |
| `docs/06-components/reference/INDEX.md` | 添加 code-editor 条目 |
| `crates/engine/src/compiler/code_editor/gen.rs` | 去硬编码 + focus_bordered 支持 + 单测 |

## 不改动的文件

- `crates/engine/src/compiler/codegen/style_attr.rs` — 通用通道已正确，无需扩展
- `crates/engine/src/compiler/component.rs` — 通用 setter 路由已正确
- `crates/ui/src/components/tab/tab_bar.rs` — 上一轮已加 bordered 支持
