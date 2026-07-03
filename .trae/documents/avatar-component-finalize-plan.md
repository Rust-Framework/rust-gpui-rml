# Avatar 组件实现收尾计划

## 概述

RML 框架中 Avatar / AvatarGroup 组件支持已在前一会话中完成 99% 的实现。当前唯一遗留问题是 `component.rs` 中 `label_set_by_attr` 标志位未追踪 Avatar 的 `name` 属性，导致一个测试失败。本计划仅覆盖该 bug 修复与最终验证。

## 当前状态分析

经过 Phase 1 探索，确认以下文件均已正确实现：

### UI 层（已完成）
- `crates/ui/src/components/avatar.rs` — 文档化 re-export，遵循 `alert_dialog.rs` 模式
- `crates/ui/src/components/mod.rs` — `pub mod avatar;` + `pub use avatar::{Avatar, AvatarGroup};`
- `crates/ui/src/lib.rs` — Avatar, AvatarGroup 加入 components re-export
- `crates/ui/src/prelude.rs` — Avatar, AvatarGroup 加入 prelude

### Engine 层（已完成，1 处 bug 待修）
- `crates/engine/src/tags.rs` — Avatar / AvatarGroup 注册为 `StatelessNoId`（L391-400）
- `crates/engine/src/compiler/avatar/mod.rs` — 模块入口，仅 re-export，无业务代码（符合规范）
- `crates/engine/src/compiler/avatar/setters.rs` — static_setter + bind_setter + 18 个单元测试
- `crates/engine/src/compiler/mod.rs` — `pub mod avatar;`
- `crates/engine/src/compiler/props_registry.rs` — `("Avatar", &["src", "name"])` + `("AvatarGroup", &["limit", "ellipsis"])`
- `crates/engine/src/compiler/component.rs` — 委托已配置（L208 static_setter、L332 bind_setter、L164 is_container 排除 Avatar、L184 text_method 映射）
  - **BUG**：L132 `if name == "label"` 未覆盖 Avatar 的 `name` 属性

### Demo 层（已完成）
- `demo/src/cases/avatar_case.rml` — 4 个示例：图片头像、首字母回退、占位图标、AvatarGroup
- `demo/src/cases/avatar_case.rml.rs` — `AvatarCase {}` 空命名结构体（避免 unit struct 宏注入问题）
- `demo/src/cases/mod.rs` — `#[path = "avatar_case.rml.rs"] pub mod avatar_case;`
- `demo/src/cases/catalog.rs` — `"components.avatar" => "case.avatar.title"`
- `demo/assets/i18n/zh-CN.json` — 5 条中文翻译
- `demo/assets/i18n/en-US.json` — 5 条英文翻译

### 文档层（已完成）
- `docs/06-components/reference/avatar.md` — 完整文档（概述、用法、属性表、事件、绑定、子节点规则、示例、常见错误、相关组件）
- `docs/06-components/reference/INDEX.md` — avatar.md 索引条目

## 遗留 Bug 分析

### 失败测试
```
---- compiler::component::tests::gen_component_avatar_name_attr_overrides_text_child stdout ----
assertion failed: !code.contains("Ignored")
```

### 根因定位

`crates/engine/src/compiler/component.rs` L125-135：

```rust
let mut label_set_by_attr = false;
for attr in &elem.attributes {
    match attr {
        Attribute::Static { name, value } => {
            if let Some(setter) = component_static_setter(name, value, &resolved) {
                code.push_str(&setter);
                if name == "label" {           // ← BUG: 仅追踪 "label"
                    label_set_by_attr = true;
                }
            }
        }
        ...
```

当处理 `<Avatar name="Explicit">Ignored</Avatar>` 时：
1. L132：`name == "label"` 为 `false`（实际是 `"name"`），`label_set_by_attr` 保持 `false`
2. L181：`!label_set_by_attr` 为 `true`，进入文本子节点分支
3. L184：`text_method = "name"`（Avatar 特殊映射）
4. 结果：同时生成 `.name("Explicit")` 和 `.name("Ignored")`

### 附带问题（同一根因）

`Attribute::Bind` 分支（L137-142）完全未设置 `label_set_by_attr`，因此 `<Avatar name={user.name}>text</Avatar>` 也会产生重复 `.name(...)` 调用。这是所有组件的通病（`label` bind 也存在），但本次仅修复 Avatar 的 `name` 以保证一致性。

## 提议变更

### 变更 1：修复 `label_set_by_attr` 追踪逻辑（核心修复）

**文件**：`crates/engine/src/compiler/component.rs`

**位置**：L132（`Attribute::Static` 分支内）

**当前代码**：
```rust
if name == "label" {
    label_set_by_attr = true;
}
```

**修改为**：
```rust
if name == "label" || (resolved == "Avatar" && name == "name") {
    label_set_by_attr = true;
}
```

**理由**：Avatar 的 `name` 属性是其主文本标识（无 `.label()` 方法），与 Button 的 `label` 属性语义等价，应在设置后抑制文本子节点映射。

### 变更 2：修复 `Attribute::Bind` 分支的同类问题（一致性修复）

**文件**：`crates/engine/src/compiler/component.rs`

**位置**：L137-142（`Attribute::Bind` 分支）

**当前代码**：
```rust
Attribute::Bind { name, expr } => {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, &resolved) {
        code.push_str(&setter);
    }
}
```

**修改为**：
```rust
Attribute::Bind { name, expr } => {
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, &resolved) {
        code.push_str(&setter);
        if name == "label" || (resolved == "Avatar" && name == "name") {
            label_set_by_attr = true;
        }
    }
}
```

**理由**：`<Avatar name={user.name}>text</Avatar>` 应与静态属性行为一致，避免生成重复 `.name(...)` 调用。同时修复所有组件 `label` bind 的同类问题。

## 假设与决策

1. **不扩展到其他组件的 `name` 属性**：当前仅 Avatar 使用 `name` 作为主文本属性。若未来有其他组件采用相同模式，再扩展条件即可。
2. **不重构 `label_set_by_attr` 命名**：虽然该标志现在也追踪 `name`，但重命名（如 `text_set_by_attr`）会扩大 diff 范围，违背"surgical changes"原则。保留原名，通过注释说明语义。
3. **不新增 bind case 的测试**：现有测试已覆盖 static case（`gen_component_avatar_name_attr_overrides_text_child`）。bind case 行为对称，验证步骤中的全量测试会覆盖回归。

## 验证步骤

### 步骤 1：运行 engine 单元测试
```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p rust-rml-engine --lib
```
**期望**：全部通过（含 `gen_component_avatar_name_attr_overrides_text_child`），无失败。

### 步骤 2：运行 demo 编译验证
```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" build -p rust-rml-demo
```
**期望**：编译成功，无错误。

### 步骤 3：运行 clippy 检查
```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" clippy -p rust-rml-ui -p rust-rml-engine -p rust-rml-demo -- -D warnings
```
**期望**：零警告。

### 步骤 4（可选）：运行集成测试
```powershell
& "$env:USERPROFILE\.cargo\bin\cargo.exe" test -p rust-rml-engine
```
**期望**：lib + integration 测试全部通过。

## 影响范围

| 文件 | 变更类型 | 行数 |
|------|---------|------|
| `crates/engine/src/compiler/component.rs` | 修改 L132 + L137-142 | ~6 行 |

仅触及 1 个文件，变更总量约 6 行，完全聚焦于 bug 修复。
