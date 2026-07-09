# TranslatorRegistry 调度修复与最终验证

## 背景

承接上一会话的工作：
- Phase 1（compiler 目录重组）已完成
- Phase 2 的 6 项 builtin 修复（P0×3 + P1×2 + P2×1）已全部应用并通过 `cargo check -p rust-rml-engine` 与 `props_registry::tests`（19/19）
- Step 7（全工作区验证）被一个**预存的调度 bug** 阻塞：`cargo build --workspace` 失败

本计划覆盖剩余的两处代码修复 + 完整验证，完成后即关闭整个任务。

## 当前状态分析

### Bug 1：`TranslatorRegistry::resolve` 调度阴影（阻塞项）

**文件**：`crates/engine/src/compiler/translator/registry.rs:76-78`

**现状**：
```rust
pub fn resolve(&self, elem: &Element) -> Option<&dyn IRmlTranslator> {
    self.translators.values().find(|t| t.matches(elem)).map(|b| b.as_ref())
}
```

**问题**：`HashMap::values().find()` 迭代顺序不确定。通用 translator（`*stateless-component`，注册于 `component/mod.rs:97`）的 `matches()` 对所有 `Stateless`/`StatelessNoId` 组件返回 true（见 `stateless.rs:36-41`）。当它在迭代中先于专用 translator（如 `Icon`，注册于 `icon.rs:17`）出现时，会**阴影**专用 translator。

**后果**：Icon 落入 `StatelessComponentTranslator`，其 `gen_stateless_body` 调用 `check_missing_mapping`（`stateless.rs:130/140`），因 Icon 的 `name`/`path` 在 `gen_icon` 内联处理（不经 `component_static_setter`），strict 模式报 error，导致 demo 构建失败。

**影响范围**：所有 `ComponentKind` 为 `Stateless`/`StatelessNoId`/`Stateful` 且有专用 translator 的组件（Icon/Label/Separator/Kbd/Tag/Alert/RadioGroup/ActivityBar/Tree/CodeEditor/Tabs/TabBar/Table/DescriptionList/Popover/Accordion）。

### Bug 2：`setters.rs` 错误消息引用已删除的文件名

**文件**：`crates/engine/src/compiler/setters.rs:332`

**现状**：
```rust
let msg = format!(
    "<{}> {} property `{}` is registered in props_registry but has no mapping in component_{}_setter; \
     property will be silently dropped. Add a match arm in crates/engine/src/compiler/component.rs.",
    tag, kind, name, kind
);
```

**问题**：`component.rs` 已在 Phase 1 重组中删除，setter 逻辑现位于 `setters.rs`。错误消息指向不存在的文件，误导开发者。

## 修改方案

### 修改 1：`registry.rs` — 精确标签优先调度

**文件**：`crates/engine/src/compiler/translator/registry.rs`

**改动**：修改 `resolve()` 方法（行 76-78），先按 canonical tag 精确查询，命中且 `matches()` 通过则返回；否则回退到原有的模式匹配。

**修改后**：
```rust
/// 按元素匹配 translator
///
/// 优先按 canonical tag 精确匹配（专用 translator 优先于通配 translator），
/// 避免通用 `*stateless-component` / `*stateful-component` 阴影 Icon / Label 等专用实现。
/// 精确匹配未命中时，回退到遍历 `matches()` 的模式匹配（覆盖通配与透明 translator）。
pub fn resolve(&self, elem: &Element) -> Option<&dyn IRmlTranslator> {
    let canonical = crate::tags::canonical_tag(&elem.tag);
    if let Some(t) = self.translators.get(canonical.as_str()) {
        if t.matches(elem) {
            return Some(t.as_ref());
        }
    }
    self.translators.values().find(|t| t.matches(elem)).map(|b| b.as_ref())
}
```

**正确性论证**：
- 专用 translator（IconTranslator 等）以 canonical tag 名注册（`tag() == "Icon"`），`canonical_tag(&elem.tag)` 对 PascalCase 标签返回自身，`get("Icon")` 命中
- `matches()` guard 防御性校验，避免返回与元素不兼容的 translator
- 通配 translator（`*stateless-component` / `*component-transparent`）的 `tag()` 含 `*` 前缀，canonical tag 永远不含 `*`，精确查询不会命中它们，自动落入回退分支
- builtin translator（div/span/br 等）以小写标签名注册，`canonical_tag` 对小写无 `-` 标签返回自身，精确命中
- 4 个调用方（`codegen/mod.rs:68`、`codegen/node.rs:146`、`printer.rs:29`、`translator/utils.rs:80`）行为一致改善，无回归

### 修改 2：`setters.rs` — 修正错误消息文件引用

**文件**：`crates/engine/src/compiler/setters.rs:332`

**改动**：将 `crates/engine/src/compiler/component.rs` 改为 `crates/engine/src/compiler/setters.rs`。

**修改后**（行 330-334）：
```rust
let msg = format!(
    "<{}> {} property `{}` is registered in props_registry but has no mapping in component_{}_setter; \
     property will be silently dropped. Add a match arm in crates/engine/src/compiler/setters.rs.",
    tag, kind, name, kind
);
```

## 验证步骤

按顺序执行，每步通过后再进入下一步：

1. **engine 单包检查** → `cargo check -p rust-rml-engine`
   - 验证：编译无错误

2. **props_registry 护栏测试** → `cargo test -p rust-rml-engine --lib props_registry::tests`
   - 验证：19/19 通过（与上一会话一致）

3. **全工作区构建** → `cargo build --workspace`
   - 验证：含 demo 的全工作区构建成功（此前因 Icon 调度 bug 失败，修复后应通过）

4. **全工作区测试** → `cargo test --workspace`
   - 验证：所有测试通过

## 假设与决策

- **假设**：RML 组件标签在模板中按 PascalCase 书写（如 `<Icon>`），与现有 `IconTranslator::matches()` 的 `canonical_tag` 约定一致。修复不引入新的标签大小写处理逻辑。
- **决策**：保留 `matches()` guard 而非直接返回精确命中结果——防御性设计，成本为零（canonical 查询已保证 key 一致，guard 仅作双保险）。
- **决策**：错误消息仅改文件名，不扩展提示「或在组件 gen.rs 内联处理」——因为 `check_missing_mapping` 的调用方都是走 setter 链路的通用 translator，内联处理的组件（Icon 等）有自己的 translator 不会触发此错误。
- **不改动**：`stateless.rs` / `stateful.rs` 的 `check_missing_mapping` 调用——调度修复后，有专用 translator 的组件不再落入通用路径，通用路径的 `check_missing_mapping` 对剩余的无专用 translator 组件仍然有效且必要。
