# ActivityBar 面板色差 + slot_left 收起自适应 + resize handle 隐藏

## Context

当前 ActivityBar 行为已正常，但存在三个相互关联的视觉/交互问题：

1. **背景色差缺失**：`crates/ui/src/components/activity_bar.rs:240` 图标栏 `bar` 与 `panel_body`（L258/L261/L264）都使用 `cx.theme().sidebar`，dark 主题下两者都是 `#0a0a0a`，零色差。gpui-component `ThemeColor` 没有 `panel` 字段；`title_bar=#171717` 是最贴近 VSCode 风格的次亮背景。

2. **slot_left 收起时宽度不收回**：`crates/ui/src/window/tab_window.rs:386-393` 把 slot_left 包在 `resizable_panel().size(self.left_width=260px)` 中固定宽度。当 ActivityBar 内部 `set_active_id(None)` 时 `panel_body` 变 `w_0`，但外层 slot_left 仍占 260px，留下一片空白。

3. **resize handle 应随收起消失**：用户设计目标是「插槽宽度/高度低于阈值时 resizeable 隐藏」。gpui-component `resizable_panel.rs:333-346` 的 resize_handle 在 `panel_ix > 0` 时无条件添加，无阈值判断；要隐藏 handle 必须不让该 panel 进入 `h_resizable` 组。

**MVVM 说明**：当前 ActivityBar 是单 Entity + `Render`，不是 RML 的 `IViewModel`。这是 memory 明确记录的有意取舍（规避 render 上下文创建 Entity 的时序问题），本次不改变该架构。

## 方案概述

- **修复 1**：`panel_body` 改用 `cx.theme().title_bar` 背景 + 条件左边框。
- **修复 2+3**：Host 通过 `cx.observe(&activity_bar)` 监听 `active_id` 变化，动态维护 `slot_left_size` 字段（展开 260 / 收起 48）；`TabWindowShell` render 中当 slot 尺寸 ≤ 阈值时将该 slot 移出 `h_resizable`，改用普通 `div` 渲染（无 resize handle，且不污染 `ResizableState` 的 panel_ix 映射）。

## 具体改动

### 文件 1：`crates/ui/src/components/activity_bar.rs`

**修改 `Render::render` 的 panel_body 部分（L244-265）**：

- `panel_body` 容器 `bg` 由 `cx.theme().sidebar` 改为 `cx.theme().title_bar`
- 增加 `border_l_1().border_color(cx.theme().sidebar_border)`，强化图标栏与面板的视觉分割
- `None` 分支保持 `div().w_0().h_full()` 不变（收起时无内容，无边框渲染）

`bar` 部分保持 `bg(cx.theme().sidebar)` 不变。

### 文件 2：`crates/ui/src/window/tab_window.rs`

**新增常量与 builder 方法**：

```rust
const SLOT_COLLAPSED_THRESHOLD: gpui::Pixels = px(60.);

pub fn left_size(mut self, size: gpui::Pixels) -> Self {
    self.left_width = size;
    self
}
pub fn right_size(mut self, size: gpui::Pixels) -> Self {
    self.right_width = size;
    self
}
pub fn bottom_size(mut self, size: gpui::Pixels) -> Self {
    self.bottom_height = size;
    self
}
```

**重写 `main_row` 构造逻辑（L384-403）**：

核心策略——折叠的 slot 用普通 `div` 渲染并放在 `h_resizable` 之外；展开的 slot 进 `h_resizable`。这样：
- 折叠 slot 无 resize handle（用户目标 3）
- `h_resizable` 内 panel_ix 连续无错位（避免 `ResizableState` 状态污染）
- 展开态行为与现状一致

```rust
let left_collapsed = self.left_width <= SLOT_COLLAPSED_THRESHOLD;
let right_collapsed = self.right_width <= SLOT_COLLAPSED_THRESHOLD;

let mut row = h_flex().w_full().h_full();

// 折叠的 left 放 h_resizable 之前
if let Some(left) = self.slot_left {
    if left_collapsed {
        row = row.child(
            div().w(self.left_width).flex_none().h_full().child(left)
        );
    }
}

// h_resizable 内：展开的 left + center + 展开的 right
let mut main_h = h_resizable("tab-window-main-row");
if let Some(left) = self.slot_left {
    if !left_collapsed {
        main_h = main_h.child(
            resizable_panel()
                .size(self.left_width)
                .flex_none()
                .size_range(px(48.)..px(600.))
                .child(left)
        );
    }
}
main_h = main_h.child(center_col);
if let Some(right) = self.slot_right {
    if !right_collapsed {
        main_h = main_h.child(
            resizable_panel()
                .size(self.right_width)
                .flex_none()
                .size_range(px(160.)..px(800.))
                .child(right)
        );
    }
}
row = row.child(main_h.flex_1().min_w_0());

// 折叠的 right 放 h_resizable 之后
if let Some(right) = self.slot_right {
    if right_collapsed {
        row = row.child(
            div().w(self.right_width).flex_none().h_full().child(right)
        );
    }
}

row
```

**bottom 同理（L357-381）**：

`center_col` 中的 bottom panel 也按 `self.bottom_height <= SLOT_COLLAPSED_THRESHOLD` 判断：
- 折叠：用 `div().h(self.bottom_height).flex_none().child(bottom)` 放在 `v_resizable` 之外
- 展开：保持现有 `resizable_panel().size(self.bottom_height)...` 放在 `v_resizable` 内

### 文件 3：`crates/engine/src/compiler/codegen/shell.rs`

在 `gen_tab_window_wrapper` 的 `match name.as_str()`（L208-215 附近）新增三个分支：

```rust
"left_size" => builder.push(format!(".left_size({})", shell_bind_expr(expr, ctx)?)),
"right_size" => builder.push(format!(".right_size({})", shell_bind_expr(expr, ctx)?)),
"bottom_size" => builder.push(format!(".bottom_size({})", shell_bind_expr(expr, ctx)?)),
```

`shell_bind_expr` 对 `slot_left_size` 这类 `Expr::Field` 生成 `self.slot_left_size`（Pixels: Copy，无需 `.clone()`）。

### 文件 4：`demo/src/shell/main_window.rml.rs`

**新增字段**（L33 附近）：

```rust
pub slot_left_size: gpui::Pixels,
```

`#[derive(Default)]` 中 `gpui::Pixels` 的 Default 是 `px(0.)`，需要在 `on_loaded` 中显式初始化为 `px(260.)`，或在 `new()` 中设置。考虑到 `MainWindow` 用 `#[derive(Default)]`，在 `on_loaded` 开头设值最稳妥。

**在 `on_loaded` 中注册 observe**（L120-126 附近，紧跟 `activity_bar` 创建之后）：

```rust
self.slot_left_size = px(260.);

if let Some(bar) = &self.activity_bar {
    cx.observe(bar, |this, bar, cx| {
        let collapsed = bar.read(cx).active_id().is_none();
        this.slot_left_size = if collapsed { px(48.) } else { px(260.) };
        cx.notify();
    }).detach();
}
```

说明：
- observe 注册后不会立即触发回调，需手动初始化 `slot_left_size = px(260.)`（与 activate_first 后的展开态一致）
- 闭包内 `this.slot_left_size = ...` 不被 `#[window]` 宏的 AST 扫描识别，故不会自动 bump version；但 `slot_left_size` 不被任何 `#[computed]` 依赖，仅靠 `cx.notify()` 触发重渲即可读到新值
- 若未来有 `#[computed]` 依赖此字段，需在闭包内补 `this.__rml_bump_version("slot_left_size");`

### 文件 5：`demo/src/shell/main_window.rml`

`<tab_window>` 标签新增 `left_size` 属性：

```xml
<tab_window
    title="RML Showcase"
    width="1100"
    height="720"
    startup="CenterScreen"
    icon={IconName::Frame}
    tabs={tab_bar_items}
    selected_tab={selected_tab}
    on_tab_click="on_tab_click"
    show_chrome={show_chrome}
    on_chrome_toggle="on_chrome_toggle"
    left_size={slot_left_size}>
```

## 关键设计取舍

1. **折叠态下 center/right 不可 resize**：因为折叠的 slot 移出了 `h_resizable`，剩下的 panel 仍可 resize（如 right 展开时，center 与 right 之间仍有 handle）。仅当 left 折叠且 right 也折叠时，`h_resizable` 内只剩 center，无 handle 可拖——这是合理的，因为此时窗口主要内容就是 center。

2. **`h_resizable` state 在 panel 数量变化时 truncate/extend**：`sync_panels_count` 会 `truncate` 或 `extend` panels 数组。展开→折叠再→展开时，left 的拖动状态会丢失（回到 `left_width` 初始值）。可接受，符合「收起即重置」的直觉。

3. **不引入 EventEmitter**：严格遵守 memory 约束「ActivityBar must not use ActivityBarEvent/EventEmitter」。改用 `cx.observe` 监听 `cx.notify()` 触发的状态变化。

4. **阈值 60px**：bar_width=48px，展开最小 260px，阈值 60 居中。right/bottom 若有折叠需求，可传 `<60` 的值触发。

## 验证步骤

### 编译

```bash
cargo build -p rust-rml-engine   # 验证 shell.rs codegen
cargo build -p rust-rml-ui       # 验证 tab_window + activity_bar
cargo build -p rust-rml-demo     # 验证 main_window
```

### 运行时验证

```bash
cargo run -p rust-rml-demo
```

预期行为：
1. 启动后 ActivityBar 首项自动激活，slot_left 宽 260px，左侧有 resize handle
2. **色差检查**：图标栏背景 `sidebar`（dark 主题 `#0a0a0a`），面板背景 `title_bar`（`#171717`），有可见色差；面板左侧有细分隔线
3. 点击已激活的图标按钮 → `active_id=None`：
   - slot_left 宽度收缩到 48px（只剩图标栏）
   - **resize handle 消失**（left 不再在 `h_resizable` 内）
   - center 区域自动扩展填满剩余空间
4. 再次点击该图标 → `active_id=Some`：
   - slot_left 展开回 260px
   - resize handle 重现
5. 展开态下拖动 left 与 center 之间的 handle → 正常 resize，size_range 48..600 生效
6. 切换 light 主题 → 色差关系保持（title_bar 仍比 sidebar 略亮）

### 回归检查

- 确认 `tab_window` 的 `default_sizes` API 仍可用（本次新增 `left_size` 等是补充，不删除 `default_sizes`；若 RML 同时传两者，codegen 顺序决定谁胜——建议 RML 只用 `left_size` 等细分 API，`default_sizes` 留给 Rust 代码）
- 确认 `case_activity_panel.rml.rs` 的 `#[contribute]` 贡献系统无影响（`IActivityPanel::panel()` 接口未变）
- 确认 `shell_chrome.rs` 的 `map_activity_panels` 无影响

## 实施顺序

1. `activity_bar.rs`：背景色差 + 边框（独立改动，可先验证视觉）
2. `tab_window.rs`：新增 builder + render 重写（核心结构改动）
3. `shell.rs`：codegen 新增三属性路由
4. `main_window.rml.rs`：新增字段 + observe 注册
5. `main_window.rml`：RML 标签新增 `left_size` 属性
6. 编译 + 运行验证

按此顺序每步编译可过（先改组件，再改框架路由，最后改消费方）。
