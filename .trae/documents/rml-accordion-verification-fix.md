# RML Accordion 支持 — 验证与修复计划

## 概述

前一轮会话已完成 Accordion 支持的全部 10 项实现变更（re-export、ComponentKind::StatelessWithItems、tags 注册、codegen、setters、props_registry、validator、demo 案例、单元测试）。本计划聚焦于**验证阶段发现的 1 个 Accordion 代码缺陷修复**，并明确区分出阻塞 demo 端到端验证的预存贡献系统重构错误（不在本任务范围内）。

## 当前状态分析

### 已确认到位的实现（无需改动）

| 变更 | 文件 | 状态 |
|---|---|---|
| 1. re-export Accordion | `crates/ui/src/lib.rs:44` | ✓ |
| 2. ComponentKind::StatelessWithItems | `crates/engine/src/tags.rs:262` | ✓ |
| 3. component_lookup 注册 Accordion | `crates/engine/src/tags.rs:371-375` | ✓ |
| 4. is_item_builder_tag 函数 | `crates/engine/src/tags.rs:385-387` | ✓ |
| 5. gen_component StatelessWithItems 分支 + gen_item_builder | `crates/engine/src/compiler/component.rs:74-82, 154-181, 219-261` | ✓ |
| 6. setters (multiple/bordered/open/icon/title-bind/on_toggle_click) | `crates/engine/src/compiler/component.rs:466-478, 576-590, 677-689` | ✓（但 title 静态 setter 缺失，见下） |
| 7. props_registry 注册 | `crates/engine/src/compiler/props_registry.rs:76-79` | ✓ |
| 8. validator is_item_builder_tag 扩展 | `crates/engine/src/compiler/validator.rs:148` | ✓ |
| 9. demo 案例 (accordion_case.rml/.rml.rs + mod.rs + catalog + i18n) | `demo/src/cases/accordion_case.*` | ✓ |
| 10. 16 个单元测试 | `crates/engine/src/compiler/component.rs:1407+` | ✓ |

### 编译/测试验证结果

- `cargo check -p rust-rml-engine --lib`：**通过**（无错误）
- `cargo test -p rust-rml-engine --lib`：**237 passed, 3 failed**
  - `compiler::component::tests::gen_component_accordion_with_item` — **Accordion 缺陷**（本计划修复）
  - `build::contribution_generator::tests::parse_host_id_from_multi_field_attr` — 预存贡献系统重构问题
  - `build::contribution_generator::tests::parse_contribution_registrars_extracts_host_id` — 预存贡献系统重构问题
- `cargo check -p rust-rml-demo`：**失败（10 errors）** — 全部为预存贡献系统重构问题（`ContributionEntry`/`IHostEntity`/`extract_visual`/`build_activity_panels` 已删除但 demo/shell 未同步更新）

### Accordion 缺陷根因

`gen_component_accordion_with_item` 测试用例：
```rust
// <AccordionItem title="Section 1">Content</AccordionItem>
let item = make_element(
    "AccordionItem",
    vec![Attribute::Static { name: "title".into(), value: "Section 1".into() }],
    vec![Node::Text("Content".into())],
);
// 期望：code.contains(".title(\"Section 1\")")
```

**问题**：`component_static_setter` 没有 `title` 分支。`title` 仅在 `component_bind_setter`（`title={expr}`）中实现，静态形式 `title="..."` 被静默丢弃。

**警告日志**（测试输出）：
```
[rml warning] <AccordionItem> static property `title` is registered in props_registry 
but has no mapping in component_static_setter; property will be silently dropped.
```

### 预存贡献系统重构错误（不在本任务范围）

demo 编译失败的 10 个错误全部源于 `crates/core/src/contribution.rs` 中 `IHostEntity`/`ContributionEntry` 等类型已删除，但以下 demo 文件仍引用它们：
- `demo/src/shell/shell_chrome.rs:11` — `use rml_core::contribution::ContributionEntry;`
- `demo/src/shell/main_window.rml.rs:42,107,151` — `impl IHostEntity`, `build_activity_panels`, `extract_visual`
- `demo/src/shell/activity_panel.rml.rs:33` — `impl IHostEntity`
- 连锁错误：`ActivityPanel`/`MainWindow` 未实现 `ILifecycle`（原由 `IHostEntity` 隐式提供）

这些错误来自进行中的贡献系统重构（git status 显示 `crates/core/src/contribution.rs` 等多个文件已修改），**与 Accordion 支持无关**，不应在本任务中修复。

## 提议变更

### 变更 1：在 `component_static_setter` 添加 `title` 静态属性映射

**文件**：[crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)

**位置**：`component_static_setter` 函数，约 466-478 行（Accordion 专用 static setter 区域）

**What**：添加 `"title"` match 分支，生成 `.title(<value>)`。

**Why**：AccordionItem 的 `title(impl IntoElement)` 接受任意元素，静态字符串 `title="Section 1"` 应直接生成 `.title("Section 1")`。当前仅 bind 形式 `title={expr}` 有映射，静态形式被静默丢弃，导致 `gen_component_accordion_with_item` 测试失败。

**How**：
```rust
// AccordionItem 专用：title="Section 1" → .title("Section 1")
// title(impl IntoElement) 接受任意元素，静态字符串直接传递
// 注：与 label 静态 setter 模式一致（.label({:?})）
"title" => Some(format!(".title({:?})", value)),
```

插入位置：在现有 `"icon"` 分支（478 行）之后、通用样式属性 `"class" | "id" | ...` 分支（480 行）之前。

**影响范围**：
- `title` 不在 `COMMON_STATIC_PROPS` 中，仅 `AccordionItem` 在 `COMPONENT_PROPS` 注册了 `title`
- 其他扩展组件若未来需要 `title` 静态属性，也会自动受益
- Shell 标签的 `title` 由 `shell.rs` 独立处理，不受影响

## 假设与决策

| 决策点 | 选项 | 决策 | 理由 |
|---|---|---|---|
| `title` 静态 setter 作用域 | A. 仅限 `tag == "AccordionItem"`<br>B. 通用（不限 tag） | **B** | 与 `icon`/`label` 等现有 setter 风格一致；`props_registry` 已约束 `title` 仅注册给 `AccordionItem`，warning 机制会捕获未映射的组件；未来组件复用无需改代码 |
| 是否修复预存贡献系统错误 | A. 一并修复<br>B. 不修复 | **B** | 贡献系统重构是独立进行中的任务（见 git status 大量修改文件）；用户请求仅为 Accordion 支持；修复需理解完整重构上下文，超出本任务范围 |
| demo 端到端验证 | A. 修复贡献系统后验证<br>B. 仅验证 engine 单元测试 | **B** | engine 单元测试已覆盖 codegen 正确性；demo 编译被预存错误阻塞，非 Accordion 问题 |

## 验证步骤

### 1. 单元测试验证（核心）

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p rust-rml-engine --lib compiler::component::tests::gen_component_accordion_with_item 2>&1"
```

**预期**：`gen_component_accordion_with_item` 测试通过，不再出现 warning 日志。

### 2. 全量 Accordion 测试验证

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p rust-rml-engine --lib gen_component_accordion 2>&1"
```

**预期**：8 个 `gen_component_accordion_*` 测试全部通过。

### 3. Engine 全量测试回归

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo test -p rust-rml-engine --lib 2>&1"
```

**预期**：238 passed, 2 failed（仅剩 2 个预存 `contribution_generator` 测试失败，与 Accordion 无关）。

### 4. Demo 编译验证（预期仍失败）

```powershell
cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo check -p rust-rml-demo 2>&1"
```

**预期**：仍因预存贡献系统错误失败（10 errors），但**不应出现任何 Accordion 相关错误**。若出现 Accordion 错误，则需进一步排查。

## 实施顺序

1. **变更 1**：在 `component_static_setter` 添加 `"title"` 静态 setter（1 行代码）
2. **验证 1**：运行 `gen_component_accordion_with_item` 单测确认通过
3. **验证 2**：运行全量 Accordion 单测确认无回归
4. **验证 3**：运行 engine 全量测试确认仅剩 2 个预存失败
5. **验证 4**：运行 demo check 确认无 Accordion 相关错误

## 文件变更清单

| 文件 | 类型 | 行数估计 |
|---|---|---|
| `crates/engine/src/compiler/component.rs` | 修改 | +3 |

总计：~3 行变更。
