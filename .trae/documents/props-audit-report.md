# 组件属性注册完整度审计报告

**生成时间**：2026-07-08
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

属性清单依据：`code_editor/gen.rs:220-226` 的 `is_handled_inline` 列表 + input/event.rs 处理的 4 个事件。

### 验证
- `cargo test -p rust-rml-engine --lib props_registry::tests::code_editor_props_registered` ✅ 通过
- `cargo test -p rust-rml-engine --lib props_registry::tests` 全部 23 个测试通过

---

## 2. 双向校验机制

### 机制概述
建立了 props_registry ↔ gen.rs ↔ setter 的双向往返校验，防止属性漂移：

| 方向 | 校验内容 | 测试函数 |
|------|---------|---------|
| 反向（gen.rs → registry） | gen.rs HANDLED_PROPS 中的属性必须已登记 | `code_editor_inline_props_are_registered` |
| 正向（registry → setter/inline） | COMPONENT_PROPS 中的专用属性必须有 setter 映射或在 HANDLED_PROPS 中 | `registered_props_have_setter_or_inline_handling` |
| 覆盖（component_lookup → registry） | 枚举已路由但未登记的组件 | `components_without_props_entry_audit` |

### HANDLED_PROPS 契约
`code_editor/gen.rs` 新增 `pub const HANDLED_PROPS: &[&str]`，声明本模块内联处理的全部属性。此常量作为 gen.rs 与 props_registry 之间的双向校验契约。其他有专属 gen.rs 的组件可后续逐步采纳此模式。

### 正向校验的跳过列表
以下组件有专属 gen.rs 但尚未声明 HANDLED_PROPS，正向校验暂时跳过（待后续采纳）：
```
Label, Separator, Icon, Kbd, Tag, Alert, RadioGroup,
Tabs, TabBar, Table, DescriptionList, Popover, Accordion,
Tree, AccordionItem, Tab, Column, DescriptionItem
```

### 正向校验的多值探测
正向校验对每个专用属性尝试多个代表值（`""`、`"true"`、`"test"`、`"0"`、`"normal"`、`"small"`），任一返回 Some 即说明 setter 存在。这避免了枚举型属性（如 GroupBox 的 `variant`）因空值返回 None 而误报。

---

## 3. 未登记组件清单

以下 12 个组件在 `component_lookup` 中注册，但未在 `COMPONENT_PROPS` 中登记。这不一定是 bug（组件可能仅依赖 COMMON_* 通用属性），但需要人工审查确认是否有专用属性需要登记。

| 组件 | 可能的专用属性 | 初步判断 | 优先级 |
|------|-------------|---------|--------|
| **Slider** | min / max / step | 很可能需要登记（Slider 通常有范围和步长配置） | 高 |
| **Progress** | percent / show_info | 很可能需要登记（进度条核心属性） | 高 |
| **ProgressCircle** | percent / show_info | 同 Progress，环形进度条 | 高 |
| **Switch** | （checked 在 COMMON_BIND_PROPS） | 可能不需要专用条目 | 低 |
| **Checkbox** | （checked 在 COMMON_BIND_PROPS，label 在 COMMON_STATIC_PROPS） | 可能不需要专用条目 | 低 |
| **Button** | （label/primary 等均在 COMMON_STATIC_PROPS） | 不需要专用条目 | 无 |
| **ButtonGroup** | （容器组件，无专用属性） | 不需要专用条目 | 无 |
| **Label** | （label 在 COMMON_STATIC_PROPS） | 不需要专用条目 | 无 |
| **TitleBar** | （StatelessNoId 容器，无专用属性） | 不需要专用条目 | 无 |
| **StatusBar** | （StatelessNoId 容器，无专用属性） | 不需要专用条目 | 无 |
| **ActivityBar** | （EntityRef，无专用属性） | 不需要专用条目 | 无 |
| **MenuBar** | （items 走 menu 模块 bind_setter） | 需确认 items 是否需要登记 | 中 |

### 建议后续行动
1. **高优先级**：检查 Slider/Progress/ProgressCircle 的 gpui-component 实现，确认 min/max/step/percent 等属性是否可设置，若可则补充 COMPONENT_PROPS 条目
2. **中优先级**：确认 MenuBar 的 `items` 属性是否已在 menu 模块的 bind_setter 中处理，若是则补充登记
3. **低优先级**：Switch/Checkbox 的 checked 属性已在 COMMON_BIND_PROPS，无需额外登记
4. **无需行动**：Button/ButtonGroup/Label/TitleBar/StatusBar/ActivityBar 仅依赖通用属性，无需登记

---

## 4. 测试输出

```
$ cargo test -p rust-rml-engine --lib props_registry::tests -- --nocapture

running 23 tests
...
test compiler::props_registry::tests::code_editor_props_registered ... ok
test compiler::props_registry::tests::code_editor_inline_props_are_registered ... ok
test compiler::props_registry::tests::registered_props_have_setter_or_inline_handling ... ok
test compiler::props_registry::tests::components_without_props_entry_audit ... ok
...
test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 903 filtered out

[rml audit] Components in component_lookup but not in COMPONENT_PROPS:
  ["Button", "ButtonGroup", "Checkbox", "Label", "Progress", "ProgressCircle",
   "Slider", "Switch", "TitleBar", "StatusBar", "ActivityBar", "MenuBar"]
```

---

## 5. 修改文件清单

| 文件 | 改动 |
|------|------|
| `crates/engine/src/compiler/props_registry.rs` | 添加 CodeEditor 条目 + 4 个测试函数 |
| `crates/engine/src/compiler/code_editor/gen.rs` | 添加 HANDLED_PROPS 常量 |
