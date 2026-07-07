# LSP 语法服务修复计划

## 概要

修复 `demo\src\lsp` 案例中两个 LSP 语法服务问题：

1. **问题 1**：`.rs/.rml.rs` 文件无 quickinfo（hover）等语法服务，只有着色
2. **问题 2**：`.rml` 文件 hover 不立即触发，且只在 tag 范围内有效，属性范围/值范围无语法支持

## 当前状态分析

### 问题 1 根因

`crates/lsp/src/rust/adapter.rs:116-124` 中 `open_document` 和 `apply_change` 是空实现：

```rust
fn open_document(&mut self, _uri: &Url, _text: &str) {}
fn apply_change(&mut self, _uri: &Url, _text: &str) {}
```

调用链：`handlers/sync.rs` → `state.rust_query.open_document/apply_change` → `RaAdapter`（空实现）。

虽然 RA 通过 `load_workspace_at` 加载了整个 Cargo workspace（包含 .rml.rs 文件），但存在以下问题：

* 用户编辑 .rml.rs 后变更未同步到 RA 的 Vfs，RA 分析的是磁盘旧内容

* 若文件不在已加载的 source root 中（如新创建文件），`url_to_file_id` 返回 None → hover 返回 None

* `RaHost::load` 加载需 30s+，期间 `analysis()` 返回 None，hover 返回 "Loading..." 但用户可能误以为功能损坏

### 问题 2 根因

**字符单位不一致**：

* Client 端 `RopeExt::offset_to_position`（`rope_ext.rs:346-352`）返回 **char count**（Unicode 标量值）

* Server 端 `position_to_byte_offset`（`conv.rs:62-83`）把 `character` 当作 **UTF-16 码元**

* LSP 规范要求 `Position.character` 为 UTF-16 码元

影响范围：

* **中文字符**：1 char = 1 UTF-16 码元 = 3 bytes → **无影响**

* **emoji 字符**：1 char = 2 UTF-16 码元 = 4 bytes → **有影响**，导致 server 计算的 byte\_offset 偏小，光标命中检测落在错误位置

当 .rml 文件某行在光标前有 emoji 时，server 计算的 byte\_offset < 实际光标位置，可能落在标签名 span 内而非属性 span 内，表现为"只在 tag 范围内有效"。

三个 provider 都有此问题：

* `crates/rml/src/providers/hover.rs:32`：`text.offset_to_position(offset)`

* `crates/rml/src/providers/definition.rs:33`：`text.offset_to_position(offset)`

* `crates/rml/src/providers/completion.rs:33`：`text.offset_to_position(offset)`

**注**：gpui-component 的 `position_to_offset`（`rope_ext.rs:336-344`）也有反向 bug（取 N chars 而非 N UTF-16 码元），但该库只读不可改。此 bug 仅影响 popover 定位精度，不影响 hover 内容正确性。

## 修改方案

### 变更 1：实现 RaAdapter 文档同步（问题 1）

**文件**：`crates/lsp/src/rust/host.rs`

新增两个方法到 `RaHost`：

```rust
/// 可变访问 Vfs（用于注入文件内容）
pub fn with_vfs_mut<R>(&self, f: impl FnOnce(&mut Vfs) -> R) -> Option<R> {
    let mut inner = self.inner.lock().ok()?;
    inner.vfs.as_mut().map(f)
}

/// 将 Vfs 中累积的变更应用到 AnalysisHost（触发重分析）
pub fn apply_vfs_changes(&self) {
    let mut inner = match self.inner.lock() {
        Ok(i) => i,
        Err(_) => return,
    };
    let (host, vfs) = match (inner.host.as_mut(), inner.vfs.as_mut()) {
        (Some(h), Some(v)) => (h, v),
        _ => return,
    };
    let changes = vfs.take_changes();
    if changes.is_empty() {
        return;
    }
    let mut change = ra_ap_hir::ChangeWithProcMacros::default();
    for (_, file) in changes {
        let text = match &file.change {
            ra_ap_vfs::Change::Create(bytes, _) | ra_ap_vfs::Change::Modify(bytes, _) => {
                Some(String::from_utf8_lossy(bytes).into_owned())
            }
            ra_ap_vfs::Change::Delete => None,
        };
        change.change_file(file.file_id, text);
    }
    host.apply_change(change);
}
```

**文件**：`crates/lsp/src/rust/adapter.rs`

实现 `open_document` 和 `apply_change`：

```rust
fn open_document(&mut self, uri: &Url, text: &str) {
    if let Some(path) = uri_to_vfs_path(uri) {
        self.host.with_vfs_mut(|vfs| {
            vfs.set_file_contents(path, Some(text.as_bytes().to_vec()));
        });
        self.host.apply_vfs_changes();
    }
}

fn apply_change(&mut self, uri: &Url, text: &str) {
    // 同 open_document：full sync 模式下直接覆盖
    self.open_document(uri, text);
}
```

新增辅助函数 `uri_to_vfs_path`：

```rust
fn uri_to_vfs_path(uri: &Url) -> Option<ra_ap_vfs::VfsPath> {
    if uri.scheme() != "file" {
        return None;
    }
    let path = uri.to_file_path().ok()?;
    let abs = ra_ap_vfs::AbsPathBuf::assert_utf8(path);
    Some(ra_ap_vfs::VfsPath::from(abs))
}
```

### 变更 2：修复字符单位转换（问题 2）

**新文件**：`crates/rml/src/providers/position_util.rs`

```rust
//! LSP Position 字符单位转换工具
//!
//! gpui-component 的 `RopeExt::offset_to_position` 返回 char count，
//! 但 LSP 规范要求 `Position.character` 为 UTF-16 码元。
//! 本模块提供 byte offset → UTF-16 码元的正确转换。

use gpui_component::input::{Bias, Point, RopeExt};
use lsp_types::Position;
use ropey::Rope;

/// byte offset → LSP Position（UTF-16 码元）
pub fn offset_to_position_utf16(text: &Rope, offset: usize) -> Position {
    let point = text.offset_to_point(offset);
    let line_start = text.line_start_offset(point.row);
    let byte_len = offset.saturating_sub(line_start);

    let mut utf16_count = 0u32;
    let mut byte_count = 0usize;
    for c in text.chars_at(line_start) {
        if byte_count >= byte_len {
            break;
        }
        byte_count += c.len_utf8();
        utf16_count += c.len_utf16() as u32;
    }

    Position::new(point.row as u32, utf16_count)
}
```

**文件**：`crates/rml/src/providers/mod.rs`

新增模块声明：

```rust
pub mod position_util;
```

**文件**：`crates/rml/src/providers/hover.rs`

替换 `text.offset_to_position(offset)`：

```rust
use super::position_util::offset_to_position_utf16;

// 旧：let position = text.offset_to_position(offset);
let position = offset_to_position_utf16(text, offset);
```

**文件**：`crates/rml/src/providers/definition.rs`

同样替换。

**文件**：`crates/rml/src/providers/completion.rs`

同样替换。

### 变更 3：添加诊断日志（验证用）

**文件**：`crates/lsp/src/rust/adapter.rs`

在 `hover` 方法中添加日志：

```rust
fn hover(&self, uri: &Url, pos: Position) -> Option<HoverInfo> {
    let analysis = match self.host.analysis() {
        Some(a) => a,
        None => {
            log::info!("[rml-lsp] hover: RA not ready, uri={}", uri);
            return Some(HoverInfo {
                content: "Loading rust-analyzer workspace...".to_string(),
                range: None,
            });
        }
    };
    let file_id = match url_to_file_id(&self.host, uri) {
        Some(f) => f,
        None => {
            log::warn!("[rml-lsp] hover: file not in Vfs, uri={}", uri);
            return None;
        }
    };
    // ... 其余逻辑不变
}
```

**文件**：`crates/lsp/src/rust/host.rs`

在 `load` 完成时添加日志：

```rust
pub fn load(&self, workspace_path: PathBuf) -> Result<()> {
    // ... 加载逻辑 ...
    inner.ready = true;
    log::info!("[rml-lsp] RA workspace loaded: {:?}", workspace_path);
    Ok(())
}
```

**文件**：`crates/rml/src/providers/hover.rs`

在发送 hover 请求前添加日志：

```rust
impl HoverProvider for LspHoverProvider {
    fn hover(&self, text: &Rope, offset: usize, _window: &mut Window, cx: &mut App) -> Task<Result<Option<Hover>>> {
        let position = offset_to_position_utf16(text, offset);
        log::debug!("[rml-lsp] client hover: offset={}, pos={:?}", offset, position);
        let rx = self.client.hover(&self.uri, position);
        // ...
    }
}
```

## 假设与决策

1. **`ChangeWithProcMacros`** **导入路径**：假设 `ra_ap_hir::ChangeWithProcMacros` 可直接使用（ide crate 内部就是这样用的）。若导入失败，需查 hir crate 的 pub re-export。
2. **`change_file`** **方法签名**：假设 `ChangeWithProcMacros::change_file(file_id, Option<String>)`（基于 ide crate `from_single_file` 示例）。
3. **Vfs 注入安全性**：`set_file_contents` 对已存在文件是 Modify，对新文件是 Create，均安全。不会影响 RA 已加载的 crate graph。
4. **反向转换 bug 接受**：gpui-component `position_to_offset` 对 emoji 有 bug（取 N chars 而非 N UTF-16 码元），但只读不可改。此 bug 仅影响 popover 定位精度，不影响 hover 内容。本计划不修复此问题。
5. **`chars_at`** **方向**：`RopeExt::chars_at(offset)` 返回从 offset 向前的字符迭代器（已在 `word_range` 中验证）。
6. **不修改 server 端** **`conv.rs`**：server 端 `position_to_byte_offset` 和 `offset_to_position` 的 UTF-16 实现是正确的，符合 LSP 规范。只需修复 client 端。

## 验证步骤

1. **编译验证**：

   * `cargo build -p rust-rml-lsp --features rust-backend`

   * `cargo build -p rust-rml`

   * `cargo build -p rust-rml-demo`

2. **单元测试**：

   * `cargo test -p rust-rml-lsp --features rust-backend`

   * `cargo test -p rust-rml`

   * 验证现有 conv.rs 测试通过（UTF-16 转换正确性）

   * 为 `position_util.rs` 添加 emoji 字符转换测试

3. **集成验证（手动）**：

   * 启动 demo lsp 案例

   * 等待 30s+ 让 RA 加载完成（观察日志 `[rml-lsp] RA workspace loaded`）

   * 在 .rml.rs 文件上 hover → 应显示 RA 的 quickinfo

   * 在 .rml 文件的标签名上 hover → 应显示标签文档

   * 在 .rml 文件的属性名上 hover → 应显示属性文档

   * 在 .rml 文件的属性值上 hover → 应显示值文档

   * 在包含 emoji 的 .rml 行上 hover 属性 → 应正确命中属性（而非回退到标签名）

4. **回归验证**：

   * 确认纯 ASCII .rml 文件 hover 仍正常工作

   * 确认 .rml.rs 着色（semantic tokens）不受影响

