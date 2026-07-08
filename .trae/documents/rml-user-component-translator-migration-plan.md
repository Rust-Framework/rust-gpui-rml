# RML 用户组件 codegen 迁移至 translator 计划

## 背景与目标

`crates/engine/src/compiler/user_component.rs` 仍是集中式用户组件代码生成入口，持有 `pub fn gen_user_component` 及大量私有助手。`UserComponentTranslator`（`translator/user_component.rs`）虽已接入注册表，但其 `to_rust` 仍委托给旧入口。

本计划完成 Phase 6 清理：将 `gen_user_component` 逻辑整体迁移到 `UserComponentTranslator` 内部，删除 `compiler/user_component.rs`，使用户组件与其他扩展组件一样走 translator 新路径。

## 现状

* `compiler/user_component.rs` 仅被 `translator/user_component.rs` 调用。

* 核心函数 `gen_user_component` 与助手 `gen_prop_assign`、`gen_slot_content`、`partition_user_component_children`、`gen_static_assign`、`gen_bind_assign` 均为模块私有。

* 包含约 450 行单元测试，覆盖属性注入、slot 闭包捕获父视图数据等场景。

## 实施步骤

### 1. 迁入核心逻辑到 `translator/user_component.rs`

将 `compiler/user_component.rs` 的实现整体移入 `translator/user_component.rs`，结构如下：

```rust
// 公开类型与注册
pub struct UserComponentTranslator { ... }
impl IRmlTranslator for UserComponentTranslator { ... }
pub fn register_user_components(...) { ... }

// 主体生成（模块私有）
fn gen_user_component_body(...)

// 属性注入助手（模块私有）
fn gen_prop_assign(...)
enum PropValue<'a> { ... }
fn gen_static_assign(...)
fn gen_bind_assign(...)

// 子节点 / slot 助手（模块私有）
fn partition_user_component_children(...)
fn gen_slot_content(...)

// 单元测试
#[cfg(test)]
mod tests { ... }
```

`UserComponentTranslator::to_rust` 改为直接调用模块内部 `gen_user_component_body`，不再跨模块委托：

```rust
fn to_rust(...) -> Result<(String, bool), CodegenError> {
    let info = ctx.user_components.get(self.tag)
        .ok_or_else(|| CodegenError::new(format!("user component <{}> not found", self.tag)))?;
    let mut code = gen_user_component_body(info, elem, ctx, id_counter, loop_vars)?;
    // ... CSS 样式追加 ...
    Ok((code, false))
}
```

### 2. 复用已有工具函数

不复制实现，通过 import 复用：

* `component_bind_rust_expr`、`parse_bool` → `compiler/component.rs`

* `extract_state_refs` → `compiler/tabs/tab.rs`

* `gen_node` → `compiler/codegen/mod.rs`

* `apply_css_styles` → `compiler/codegen/attribute.rs`

### 3. 删除旧文件

* 删除 `crates/engine/src/compiler/user_component.rs`。

* 从 `crates/engine/src/compiler/mod.rs` 移除 `pub mod user_component;`。

### 4. 更新相关注释

* `compiler/mod.rs` 中 `CodegenCtx.user_components` 与 `self_alias` 的 doc 注释仍引用 `gen_user_component`，改为引用 `UserComponentTranslator`。

### 5. 迁移单元测试

将原文件 `#[cfg(test)] mod tests` 整体搬入 `translator/user_component.rs`。仅修改测试辅助函数 `gen` 使其调用 `gen_user_component_body`；其余断言保持不动。

## 验证

```bash
cargo check -p rust-rml-engine --lib
cargo test -p rust-rml-engine --lib user_component
cargo test -p rust-rml-engine --lib
cargo clippy -p rust-rml-engine --lib
```

## 关键文件

* `crates/engine/src/compiler/translator/user_component.rs`（迁入逻辑）

* `crates/engine/src/compiler/mod.rs`（移除模块声明、更新注释）

* `crates/engine/src/compiler/user_component.rs`（删除）

