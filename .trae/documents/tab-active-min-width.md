# Tab 激活项最小宽度保护

## Summary

当 TabWindow 打开过多 tab 触发宽度压缩时，所有 tab（含激活项）均使用 `min_w_0()`，导致激活项标题可被挤压到完全不可见。本次为激活项引入固定最小宽度常量，确保其标题在压缩模式下始终可读，非激活项保持可完全压缩。

## Current State Analysis

### 压缩逻辑数据流

1. **溢出检测**（[tabs.rs:499-549](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tabs.rs#L499-L549)）：当 `Tabs::menu=true` 时，通过独立测量层（absolute + opacity:0 + flex_shrink_0）测得内容自然宽度 `content_width`，与视口宽度 `viewport_width` 比较。`content_width > viewport_width + 0.5px` 时 `is_overflow=true`。

2. **行容器压缩**（[tabs.rs:657](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tabs.rs#L657)）：`tabs-inner` 内的 tab 行在 `is_overflow` 时从 `flex_shrink_0()` 切到 `flex_1().min_w_0()`。

3. **逐 tab 压缩**（[tabs.rs:676](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tabs.rs#L676)）：每个 tab 调用 `.compress(is_overflow)`。

4. **Tab 外层 base**（[tab.rs:909](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs#L909)）：
   ```rust
   .when_else(self.compress, |this| this.flex_1().min_w_0(), |this| this.flex_shrink_0())
   ```
   压缩模式下所有 tab 一律 `flex_1().min_w_0()` —— **无任何最小宽度下限**。

5. **Tab 内层 content**（[tab.rs:848](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs#L848)）：`min_w_0()` + 标签 `div().min_w_0().truncate()` 实现省略号截断。

### 问题

`tab.rs:909` 的 `min_w_0()` 对所有 tab 一视同仁，激活项 (`self.selected=true`) 与非激活项共享同一压缩策略。当 tab 数量很多时，每个 tab 被均分到极小宽度，激活项标题也被截断成 `...` 甚至更少，无法辨认当前选中了哪个 tab。

### 现有测试覆盖

`crates/ui/src/components/tab/tab.rs` 末尾仅有 `is_double_click` 的单元测试（tab.rs:1062-1093），无压缩相关测试。

## Proposed Changes

### 单点修改：[tab.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs)

**1. 新增常量**（放在文件顶部 `DOUBLE_CLICK_WINDOW` 附近，约 tab.rs:30 处）：

```rust
/// 压缩模式下激活 tab 的最小宽度下限。
///
/// 确保激活 tab 标题在多 tab 溢出压缩时仍可读（约 6-8 个中文字符或图标+短文本+关闭按钮）。
/// 非激活 tab 不设下限，可完全压缩以优先保障激活项可见性。
const COMPRESS_ACTIVE_MIN_W: Pixels = px(120.);
```

**2. 修改外层 base 压缩分支**（[tab.rs:909](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs#L909)）：

原代码：
```rust
.when_else(self.compress, |this| this.flex_1().min_w_0(), |this| this.flex_shrink_0())
```

改为：
```rust
.when_else(self.compress, |this| {
    this.flex_1()
        .when(self.selected, |this| this.min_w(COMPRESS_ACTIVE_MIN_W))
        .when(!self.selected, |this| this.min_w_0())
}, |this| this.flex_shrink_0())
```

**为什么只改外层 base，不改内层 content（tab.rs:848）？**

内层 content 的 `min_w_0()` 是为了让标签在 tab 内部能正确触发 `truncate()` 省略号。外层 base 设了 `min_w(120)` 后，tab 本身不会小于 120px，内层 content 在 120px 范围内自由收缩 + 截断，这正是期望行为。若内层也设 min_w，反而会阻止标签截断。

### Flex 布局行为说明

修改后，压缩模式下 flex 算法分两步分配宽度：
1. 先满足各 tab 的 `min_w`：激活项得 120px，非激活项得 0px
2. 剩余空间按 `flex_1`（flex_grow=1）均分给所有 tab

结果：激活项 = 120px + (剩余空间 / N)，非激活项 = (剩余空间 / N)。激活项始终略宽于非激活项且不低于 120px，标题可读；非激活项可被压缩到接近 0（仅显示省略号或图标），优先保障激活项可见性。

## Assumptions & Decisions

1. **仅激活项保护**：非激活项保持 `min_w_0()` 可完全压缩。当 tab 极多时，非激活项可能压缩到仅显示 `...`，这是可接受的——用户的核心诉求是"激活项一定可见"，非激活项可通过点击或菜单按钮切换后变为激活项再显示完整标题。

2. **固定常量 120px**：基于默认 Size（text_sm ~13px + 12px*2 内边距 + ~20px 关闭按钮）测算，120px 可容纳约 5-6 个中文字符或 10 个英文字符。不区分 Size（XSmall/Small/Large）——用户明确选择固定常量，避免过度设计。

3. **不新增公开 API**：遵循项目约束（"prefer adding variants to existing enums over exposing new interfaces"），不新增 `Tab::active_min_w()` 构建器方法。常量 `COMPRESS_ACTIVE_MIN_W` 为模块私有。

4. **不影响非压缩模式**：`compress=false` 时仍走 `flex_shrink_0()` 分支，自然宽度 + 水平滚动，行为不变。

5. **不影响测量层**：测量层 tab（[tabs.rs:560-584](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tabs.rs#L560-L584)）调用 `.measurement()` 但不调用 `.compress()`，`compress=false`，不受影响。

## Verification

1. **编译验证**：`cargo build -p rust-rml-ui` 通过，无警告。

2. **现有测试不回归**：`cargo test -p rust-rml-ui` 全量通过（当前 880+ 测试）。

3. **手动验证**（在 demo 中）：
   - 打开 TabWindow demo，新增大量 tab 直至触发溢出压缩
   - 确认激活 tab 标题始终可读（不少于 ~5 字符 + 关闭按钮）
   - 确认非激活 tab 可被压缩（省略号截断）
   - 切换激活项后，新激活项立即获得最小宽度保护，旧激活项恢复可压缩
   - tab 数量减少到不溢出时，恢复正常自然宽度（无 min_w 残留效果）

4. **边界场景**：
   - 激活项有 close 按钮：120px 内仍能显示部分标题 + 关闭按钮 ✓
   - 激活项是 icon-only（无 label）：icon 居中显示，120px 足够 ✓
   - Underline variant（padding_x=0）：120px 内容区更大，标题显示更多字符 ✓
