# 组件属性注册完整度审查与纠正

## Summary

修复 CodeEditor 在 `COMPONENT_PROPS` 中缺失导致的 demo 编译阻塞，并建立 props_registry ↔ gen.rs ↔ setter 的双向往返校验机制，防止未来再次出现"已路由但未注册"或"已处理但未登记"的属性漂移。本次仅修复 CodeEditor 阻塞，其余组件的潜在缺失列入审计报告供后续决策。

## Current State Analysis

### 问题根因
- [tags.rs:432](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L432) 注册了 `CodeEditor`（`component_lookup` 可查）
- [props_registry.rs:92-182](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L92-L182) 的 `COMPONENT_PROPS` **完全没有 CodeEditor 条目**
- [code_editor/gen.rs:220-226](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L220-L226) 内联处理 5 个 static 属性 + 1 个 bind 属性，且通过 `input/event.rs` 处理 4 个事件属性，全部绕过 `component_static_setter` 链路
- [validator.rs:231-243](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs#L231-L243) 对 Bind/Event 属性强制检查 `is_prop_registered` → CodeEditor 的 `bordered`/`on_change` 等被拒绝 → demo 编译失败

### 现有校验机制的缺口
1. **单向校验**：[component.rs:688-711](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L688-L711) 的 `check_missing_mapping` 只检查"已注册但无 setter 映射"，不检查反向"有 setter/inline 处理但未注册"
2. **inline 处理盲区**：CodeEditor 等有专属 gen.rs 的组件，属性在内联路径处理，完全绕过 setter 链路，`check_missing_mapping` 永远不会触发
3. **路由表对齐测试也是单向**：[props_registry.rs:503-515](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L503-L515) 的 `component_props_tags_align_with_routing_table` 只验证 `COMPONENT_PROPS → component_lookup`，不检查反向

### component_lookup 中未在 COMPONENT_PROPS 登记的组件（共 13 个）
| 组件 | 可能需要登记的专用属性 | 状态 |
|------|----------------------|------|
| **CodeEditor** | language/bordered/focus_bordered/context_menu + Input 事件 | **本次修复** |
| Button | （依赖 COMMON_STATIC_PROPS，可能无专用属性） | 待审查 |
| ButtonGroup | （可能无专用属性） | 待审查 |
| Checkbox | （checked 在 COMMON_BIND_PROPS） | 待审查 |
| Label | （label 在 COMMON_STATIC_PROPS） | 待审查 |
| Progress | percent? show_info? | 待审查 |
| ProgressCircle | percent? | 待审查 |
| Slider | min/max/step? | 待审查 |
| Switch | （checked 在 COMMON_BIND_PROPS） | 待审查 |
| TitleBar | （容器组件，可能无专用属性） | 待审查 |
| StatusBar | （容器组件，可能无专用属性） | 待审查 |
| ActivityBar | （EntityRef，可能无专用属性） | 待审查 |
| MenuBar/menu | （items 走 menu 模块 bind_setter） | 待审查 |

## Proposed Changes

### Phase 1: 修复 CodeEditor 注册（立即解除阻塞）

**文件**：[props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)

**改动**：在 `COMPONENT_PROPS` 中 `TextInput` 条目之后添加 CodeEditor 条目：

```rust
// CodeEditor 专用（基于 Input，language/bordered/focus_bordered/context_menu 由 code_editor/gen.rs 内联处理，
// on_change/on_enter/on_focus/on_blur 走 input/event.rs 订阅机制）
("CodeEditor", &["language", "bordered", "focus_bordered", "context_menu",
                 "on_change", "on_enter", "on_focus", "on_blur"]),
```

**依据**：属性清单来自 [code_editor/gen.rs:220-226](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L220-L226) 的 `is_handled_inline` 列表 + input/event.rs 处理的 4 个事件。

**验证**：`cargo test -p rust-rml-engine --lib props_registry::tests` 全部通过（含新增 `code_editor_props_registered` 测试）。

### Phase 2: 声明 CodeEditor 的内联处理属性（建立反向校验契约）

**文件**：[code_editor/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs)

**改动**：在文件顶部（`use` 之后）添加公开常量，声明本模块内联处理的全部属性：

```rust
/// 本模块内联处理的属性清单（作为 gen.rs ↔ props_registry 双向校验的契约）
///
/// - static: value/language/bordered/focus_bordered/context_menu
/// - bind: value
/// - event: on_change/on_enter/on_focus/on_blur（经 input/event.rs 订阅机制处理）
///
/// 这些属性不经过 component_static_setter / component_bind_setter / component_event_setter 链路，
/// 因此 check_missing_mapping 无法检测其注册状态。本常量供 props_registry 的双向校验测试使用。
pub const HANDLED_PROPS: &[&str] = &[
    "value", "language", "bordered", "focus_bordered", "context_menu",
    "on_change", "on_enter", "on_focus", "on_blur",
];
```

**验证**：`cargo build -p rust-rml-engine` 编译通过。

### Phase 3: 建立双向往返校验测试

**文件**：[props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) 的 `#[cfg(test)] mod tests` 模块

**新增测试 1**（反向：gen.rs → registry）：

```rust
/// 校验 CodeEditor gen.rs 内联处理的属性已全部在 COMPONENT_PROPS 登记
///
/// 方向：gen.rs HANDLED_PROPS → props_registry
/// 缺失则说明 gen.rs 处理了未登记属性，validator 会拒绝用户使用该属性
#[test]
fn code_editor_inline_props_are_registered() {
    use crate::compiler::code_editor::gen::HANDLED_PROPS;
    for prop in HANDLED_PROPS {
        assert!(
            is_prop_registered("CodeEditor", prop),
            "CodeEditor gen.rs handles prop '{}' but it's not in COMPONENT_PROPS",
            prop
        );
    }
}
```

**新增测试 2**（正向：registry → setter/inline）：

```rust
/// 校验 COMPONENT_PROPS 中登记的每个专用属性都有对应的 setter 映射或 inline 处理
///
/// 方向：props_registry → setter / HANDLED_PROPS
/// 缺失则说明登记了属性但 codegen 会静默丢弃（check_missing_mapping 仅在 .rml 实际使用时才触发）
///
/// 检查规则（按属性前缀分类）：
/// - on* 前缀 → 检查 component_event_setter 返回 Some，或在 HANDLED_PROPS 中
/// - 其余 → 检查 component_static_setter 或 component_bind_setter 返回 Some，或在 HANDLED_PROPS 中
/// - 例外：COMMON_* 列表中的通用属性跳过（已有通用 setter）
#[test]
fn registered_props_have_setter_or_inline_handling() {
    use crate::compiler::component::{component_static_setter, component_bind_setter};
    // 收集所有有 HANDLED_PROPS 声明的组件（目前仅 CodeEditor）
    let inline_handled: &[(&str, &[&str])] = &[
        ("CodeEditor", crate::compiler::code_editor::gen::HANDLED_PROPS),
    ];

    for (tag, props) in COMPONENT_PROPS {
        for prop in *props {
            // 跳过通用属性（COMMON_* 已有通用 setter，无需逐组件检查）
            if COMMON_STATIC_PROPS.contains(prop)
                || COMMON_BIND_PROPS.contains(prop)
                || COMMON_EVENT_PROPS.contains(prop)
            {
                continue;
            }

            // 检查是否在某个 gen.rs 的 HANDLED_PROPS 中
            if let Some((_, handled)) = inline_handled.iter().find(|(t, _)| *t == *tag) {
                if handled.contains(prop) {
                    continue;
                }
            }

            // 调用 setter 函数验证有映射
            let has_setter = if prop.starts_with("on") {
                // event 属性：跳过（需要构造 EventHandler mock，且 event setter 通常有副作用）
                // 改由 check_missing_mapping 在实际 .rml 编译时覆盖
                true
            } else {
                component_static_setter(prop, "", tag).is_some()
                    || component_bind_setter(prop, "", &[], &[], tag).is_some()
            };
            assert!(
                has_setter,
                "Component '{}' prop '{}' is registered but has no setter mapping (not in component_static_setter/component_bind_setter, not in HANDLED_PROPS)",
                tag, prop
            );
        }
    }
}
```

**新增测试 3**（反向：component_lookup → COMPONENT_PROPS 覆盖检查）：

```rust
/// 枚举 component_lookup 中注册但未在 COMPONENT_PROPS 登记的组件
///
/// 这不一定是 bug（组件可能仅依赖 COMMON_* 通用属性），但需要人工审查确认。
/// 本测试打印未登记组件清单，供审计报告使用。
#[test]
fn components_without_props_entry_audit() {
    use crate::tags;
    // component_lookup 中所有 PascalCase 组件标签
    let all_components = [
        "Button", "Alert", "ButtonGroup", "Badge", "Checkbox", "Label",
        "Separator", "DescriptionList", "Tag", "Progress", "ProgressCircle",
        "Slider", "Switch", "Input", "TextInput", "CodeEditor",
        "TitleBar", "StatusBar", "ActivityBar", "Tree",
        "MenuBar", "Accordion", "Avatar", "AvatarGroup",
        "Breadcrumb", "Card", "Tabs", "TabBar", "Icon", "Kbd",
        "Table", "Popover", "Spinner", "Skeleton", "Link",
        "Collapsible", "GroupBox", "Pagination", "Radio", "RadioGroup",
    ];
    let unregistered: Vec<&str> = all_components.iter()
        .filter(|tag| {
            tags::component_lookup(tag).is_some()
            && !COMPONENT_PROPS.iter().any(|(t, _)| *t == **tag)
        })
        .copied()
        .collect();
    // CodeEditor 在 Phase 1 后应已登记
    assert!(
        !unregistered.contains(&"CodeEditor"),
        "CodeEditor must be registered after Phase 1"
    );
    // 其余未登记组件仅打印，不 fail（需人工审查是否有专用属性）
    if !unregistered.is_empty() {
        eprintln!("[rml audit] Components in component_lookup but not in COMPONENT_PROPS: {:?}", unregistered);
    }
}
```

**验证**：`cargo test -p rust-rml-engine --lib props_registry::tests` 全部通过。

### Phase 4: 生成审计报告

**操作**：
1. 运行 `cargo test -p rust-rml-engine --lib props_registry -- --nocapture` 捕获 `components_without_props_entry_audit` 的输出
2. 汇总测试结果
3. 写入 `.trae/documents/props-audit-report.md`

**报告内容**：
- CodeEditor 修复摘要
- 双向校验机制说明
- 未登记组件清单（来自测试 3 的输出）
- 各组件是否需要补充专用属性的初步判断

**验证**：`.trae/documents/props-audit-report.md` 存在且内容完整。

## Assumptions & Decisions

1. **修复范围**：仅修复 CodeEditor 阻塞（用户决策"仅修 CodeEditor 阻塞"）。其余 12 个未登记组件列入审计报告，不在本次修复。
2. **权威源**：双向往返校验（用户决策"双向往返校验"）。机制覆盖两个方向：
   - 正向：registry → setter/inline（登记的属性必须有映射或内联处理）
   - 反向：gen.rs HANDLED_PROPS → registry（内联处理的属性必须已登记）
3. **HANDLED_PROPS 模式**：本次仅 CodeEditor 采用。其他有专属 gen.rs 的组件（label/separator/icon/kbd/tag/alert/radio_group/tabs/tab_bar/table/description_list/popover/accordion/tree）可后续逐步采纳。
4. **正向校验的 event 属性豁免**：`component_event_setter` 需要 `EventHandler` 参数，构造 mock 成本高且 event setter 通常有副作用。event 属性的正向校验依赖现有 `check_missing_mapping` 在实际 .rml 编译时覆盖。
5. **测试 3 不 fail 未登记组件**：组件可能合法地仅依赖 COMMON_* 通用属性（如 Button），未登记不一定是 bug。测试仅打印清单供人工审查。

## Verification Steps

| 步骤 | 命令 | 预期结果 |
|------|------|---------|
| Phase 1 | `cargo test -p rust-rml-engine --lib props_registry::tests::code_editor_props_registered` | 通过 |
| Phase 2 | `cargo build -p rust-rml-engine` | 编译通过 |
| Phase 3 | `cargo test -p rust-rml-engine --lib props_registry::tests -- --nocapture` | 全部通过，`components_without_props_entry_audit` 打印未登记清单 |
| Phase 4 | 检查 `.trae/documents/props-audit-report.md` | 文件存在，内容完整 |
| Demo 验证 | `cargo build -p rust-rml-demo` | CodeEditor 相关属性不再报 "unknown property" 错误 |
