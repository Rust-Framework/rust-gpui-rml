# 组件属性注册完整度审计报告

**生成时间**：2026-07-08（最近更新）
**审计范围**：crates/engine/src/compiler/props_registry.rs 与 tags.rs component_lookup 的一致性
**触发原因**：demo 编译失败，CodeEditor 的 `bordered` 属性被 validator 拒绝（"unknown property 'bordered' on <CodeEditor>: not in component property registry"）

---

## 1. CodeEditor 修复摘要

### 问题
- `tags::component_lookup` 注册了 `CodeEditor`（tags.rs:432），但 `COMPONENT_PROPS` 完全缺失该条目
- `code_editor/gen.rs` 内联处理 5 个 static 属性 + 1 个 bind 属性 + 4 个 event 属性，全部绕过通用 setter 链路
- validator 对 Bind/Event 属性强制检查 `is_prop_registered`，导致 demo 中使用 CodeEditor 的 `bordered`/`on_change` 等属性时编译失败

### 修复
在 `COMPONENT_PROPS` 中添加 CodeEditor 条目（props_registry.rs:97-100）：

```rust
("CodeEditor", &["language", "bordered", "focus_bordered", "context_menu",
                 "on_change", "on_enter", "on_focus", "on_blur"]),
```

属性清单依据：`code_editor/gen.rs` 的 `HANDLED_PROPS` 常量 + input/event.rs 处理的 4 个事件。

### bordered/focus_bordered 支持 bind 表达式

**问题**：`bordered` / `focus_bordered` 原本只匹配 `Attribute::Static`（`bordered="false"`），当用户写 `bordered={false}`（`Attribute::Bind`）时属性被静默丢弃。

**修复**：`code_editor/gen.rs` 的 `bordered` / `focus_bordered` 提取增加 `Attribute::Bind` 分支，通过 `gen_expr_code` 处理表达式（支持 `false` 字面量、字段引用、方法调用）。类型从 `Option<bool>` 改为 `Option<String>`，统一 static/bind 两条路径。`is_handled_inline` 的 `Attribute::Bind` 分支同步增加 `bordered` / `focus_bordered` 跳过。

### 验证
- `cargo test -p rust-rml-engine --lib code_editor::gen` 20 个测试通过（含 3 个新增 bind 测试）
- `cargo test -p rust-rml-engine --lib props_registry::tests` 全部通过

---

## 2. 双向校验机制

### 机制概述
建立了 props_registry ↔ gen.rs ↔ setter 的三向往返校验，防止属性漂移：

| 方向 | 校验内容 | 测试函数 |
|------|---------|---------|
| 反向（gen.rs → registry） | gen.rs HANDLED_PROPS 中的属性必须已登记 | `inline_handled_props_are_registered` |
| 正向（registry → setter/inline） | COMPONENT_PROPS 中的专用属性必须有 setter 映射或在 HANDLED_PROPS 中 | `registered_props_have_setter_or_inline_handling` |
| 覆盖（component_lookup → registry） | 枚举已路由但未登记的组件 | `components_without_props_entry_audit` |

### HANDLED_PROPS 契约

5 个有专属 gen.rs 的组件已声明 `pub const HANDLED_PROPS: &[&str]`，作为 gen.rs 与 props_registry 之间的双向校验契约：

| 组件 | 文件 | HANDLED_PROPS |
|------|------|---------------|
| CodeEditor | `code_editor/gen.rs` | `value`, `language`, `bordered`, `focus_bordered`, `context_menu`, `on_change`, `on_enter`, `on_focus`, `on_blur` |
| Separator | `separator.rs` | `vertical`, `dashed` |
| Icon | `icon.rs` | `name`, `path` |
| Kbd | `kbd.rs` | `key`, `outline`, `appearance` |
| Tag | `tag.rs` | `primary`, `secondary`, `danger`, `success`, `warning`, `info`, `outline` |

### 正向校验的跳过列表
以下组件有专属 gen.rs 但尚未声明 HANDLED_PROPS，正向校验暂时跳过（待后续采纳）：
```
Alert, RadioGroup, Tabs, TabBar, Table, DescriptionList, Popover, Accordion,
AccordionItem, Tab, Column, DescriptionItem
```

### 正向校验的多值探测
正向校验对每个专用属性尝试多个代表值（`""`、`"true"`、`"test"`、`"0"`、`"normal"`、`"small"`），任一返回 Some 即说明 setter 存在。这避免了枚举型属性（如 GroupBox 的 `variant`）因空值返回 None 而误报。

---

## 3. component_lookup ↔ COMPONENT_PROPS 一致性

### 护栏机制
`components_without_props_entry_audit` 测试从 `print` 改为 `assert!(unregistered.is_empty(), ...)`，强制要求 `component_lookup` 中的每个组件必须在 `COMPONENT_PROPS` 中登记（即使无专用属性，也需空条目显式声明）。

### 已添加的空条目
以下 12 个组件无专用属性（仅依赖 COMMON_* 通用属性），但已显式登记空条目以通过一致性校验：

```rust
("Button", &[]),
("ButtonGroup", &[]),
("Checkbox", &[]),
("Label", &[]),
("Switch", &[]),
("TitleBar", &[]),
("NativeStatusBar", &[]),
("ActivityBar", &[]),
("MenuBar", &[]),
// TODO: 以下组件可能有专用属性待审查确认
("Progress", &[]),
("ProgressCircle", &[]),
("Slider", &[]),
```

### 建议后续行动
1. **高优先级**：检查 Slider/Progress/ProgressCircle 的 gpui-component 实现，确认 min/max/step/percent 等属性是否可设置，若可则补充 COMPONENT_PROPS 条目
2. **中优先级**：为 Alert/RadioGroup/Tabs/TabBar/Table/DescriptionList/Popover/Accordion 的 gen.rs 添加 HANDLED_PROPS 声明，从跳过列表移除
3. **低优先级**：Switch/Checkbox 的 checked 属性已在 COMMON_BIND_PROPS，无需额外登记

---

## 4. size 属性规范

### 遵循原生写法
`Medium` 是 `Size` enum 的 `#[default]`，即组件原生默认。RML 遵循 gpui-component 原生写法，对默认值不生成冗余的 `.with_size()` 调用：

| RML 写法 | 生成代码 | 说明 |
|----------|----------|------|
| `size="xsmall"` | `.with_size(rml_ui::Size::XSmall)` | 超小 |
| `size="small"` | `.with_size(rml_ui::Size::Small)` | 小 |
| `size="large"` | `.with_size(rml_ui::Size::Large)` | 大 |
| `size="medium"` / `size="default"` | **无调用** | 原生默认 |
| 不写 `size` | **无调用** | 同 medium/default |

`size` 位于 `COMMON_STATIC_PROPS`，对所有实现 `Sizable` trait 的组件生效。

---

## 5. 修改文件清单

| 文件 | 改动 |
|------|------|
| `crates/engine/src/compiler/props_registry.rs` | 添加 CodeEditor 条目 + 12 个空条目 + 4 个测试函数 + size 注释 + 规范提醒注释 |
| `crates/engine/src/compiler/code_editor/gen.rs` | 添加 HANDLED_PROPS 常量 + bordered/focus_bordered 支持 bind 表达式 + 3 个 bind 测试 |
| `crates/engine/src/compiler/separator.rs` | 添加 HANDLED_PROPS 常量 |
| `crates/engine/src/compiler/icon.rs` | 添加 HANDLED_PROPS 常量 |
| `crates/engine/src/compiler/kbd.rs` | 添加 HANDLED_PROPS 常量 |
| `crates/engine/src/compiler/tag.rs` | 添加 HANDLED_PROPS 常量 |
| `crates/engine/src/compiler/component.rs` | size setter：medium/default 返回 None（遵循原生写法） |
| `crates/engine/src/tags.rs` | component_lookup 前添加规范提醒注释 |
