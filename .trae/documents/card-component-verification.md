# Card 组件封装验证计划

## 摘要

参考 Ant Design Card 标准，将 `demo/src/components` 下基于 RML 模板 + slot 机制的 Card 升级为 `ui` crate 中 Code-based Rust struct 组件，让用户通过 `<Card>` 标签开箱即用。

**当前状态**：实现阶段（Task 1-5）已完成，本计划仅覆盖 Task 6 验证。

## 当前状态分析

通过 Phase 1 探索已确认以下文件全部就位：

### 新建文件
| 文件 | 职责 |
|---|---|
| `crates/ui/src/components/card.rs` | Card 组件核心实现（CardVariant + Card struct + ParentElement/Styled/InteractiveElement/StatefulInteractiveElement/Sizable/RenderOnce impl） |
| `crates/engine/src/compiler/card/setters.rs` | Card 专用属性 → builder 方法映射（含 tag 守卫 + 17 个单元测试） |
| `crates/engine/src/compiler/card/mod.rs` | 模块入口，仅 mod 声明 + re-export |

### 修改文件
| 文件 | 改动 |
|---|---|
| `crates/engine/src/compiler/mod.rs` | 添加 `pub mod card;` |
| `crates/engine/src/compiler/component.rs` | L215 静态 setter 委托 / L343 绑定 setter 委托 |
| `crates/engine/src/tags.rs` | L402 注册 `Card → Stateless` kind |
| `crates/engine/src/compiler/props_registry.rs` | L85 登记 Card 专用属性 |
| `crates/ui/src/components/mod.rs` | 添加 `pub mod card;` + `pub use card::{Card, CardVariant};` |
| `crates/ui/src/lib.rs` | 在 components re-export 追加 `Card, CardVariant` |
| `demo/src/cases/slot_case.rml` | 改用新 Card API |
| `demo/src/cases/slot_case.rml.rs` | 移除 Entity<Card> 字段，SlotCase 为空 struct |
| `demo/src/components/mod.rs` | 移除 card 模块 |

### 删除文件
- `demo/src/components/card.rml`
- `demo/src/components/card.rml.rs`

## 验证步骤（Task 6）

按以下顺序执行，每步通过后进入下一步：

### 1. Card setter 单元测试
```bash
cargo test -p rust-rml-engine --lib card
```
**验证**：17 个 setter 测试全部通过（static_setter 7 + bind_setter 10）。

### 2. ui crate 构建
```bash
cargo build -p rust-rml-ui
```
**验证**：card.rs 实现 + re-export 正确，无编译错误。

### 3. engine crate 构建
```bash
cargo build -p rust-rml-engine
```
**验证**：card/mod.rs + card/setters.rs + component.rs 委托逻辑正确。

### 4. 全量回归测试
```bash
cargo test -p rust-rml-engine
```
**验证**：无现有测试回归（基线：lib 294 + integration 42 = 336 测试，加 card 17 个 setter 测试应为 353）。

### 5. Clippy 零警告检查
```bash
cargo clippy -p rust-rml-ui
cargo clippy -p rust-rml-engine
```
**验证**：card 相关代码零 clippy 警告（pre-existing 警告除外）。

## 决策记录

- **封装形式**：Code-based Rust struct（非 RML 模板）—— 用户通过 `<Card>` 标签开箱即用
- **API 范围**：标准核心 —— title + extra + cover + bordered + hoverable + size + body 内容（不含 actions 数组和、tabs）
- **ComponentKind**：Stateless（需 id）—— 因 hoverable 需 `.hover()` 闭包作用于 stateful div，codegen 自动注入 `("rml_el", N)` id
- **tag 守卫**：card setter 从通用 dispatcher 调用（非专用 gen_xxx 路径），必须检查 `tag == "Card"` 防止误匹配其他组件同名属性（如 `<Button title="...">`）

## 实施说明

本计划仅剩验证步骤。所有文件改动已就位，无需新增编辑。验证通过后即视为 Card 组件封装任务完成。
