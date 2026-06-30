# ActivityBar / TabWindow 重构收尾计划

## 摘要

上一轮迭代完成了 Part 1-3（删除 rml_title_bar.rs、重构 modern_window.rs/tab_window.rs 改用 `gpui_component::TitleBar`、重写 activity_bar.rs 引入 `IActivityPanel`/`IActivityAct` trait 抽象）。本计划聚焦剩余的 Part 4-5：更新 exports/codegen/demo 以匹配新的类型名称，并完成编译验证。

## 当前状态分析

### 已完成（Part 1-3）
- `crates/ui/src/window/rml_title_bar.rs` 已删除 ✓
- `crates/ui/src/window/mod.rs` 已移除 `rml_title_bar` 模块声明 ✓
- `crates/ui/src/window/modern_window.rs` 已改用 `TitleBar::new()` ✓
- `crates/ui/src/window/tab_window.rs` 已按 spec 重构（chrome_toggle 始终作为 prefix 首项、4 插槽 + resizable）✓
- `crates/ui/src/components/activity_bar.rs` 已重写（306 行，含 trait + 类型别名 + Arc<dyn> 存储）✓

### 待完成（Part 4-5）
1. `crates/ui/src/components/mod.rs`（第 6-8 行）仍 re-export 旧名称 `ActivityActionItem`/`ActivityPanelItem`
2. `crates/ui/src/lib.rs`（第 85 行）仍 re-export 旧名称
3. `crates/ui/src/prelude.rs` 缺少 ActivityBar 相关类型
4. `crates/engine/src/compiler/codegen.rs` 有 3 处 `window_controls` 相关死代码：
   - 第 377 行：`gen_modern_window_wrapper` 生成 `.window_controls(self.window_controls())`
   - 第 519 行：`gen_tab_window_wrapper` 生成 `.window_controls(self.window_controls())`
   - 第 583-592 行：`gen_window_extra_methods` 提取 `show_minimize`/`show_maximize`/`show_close` 并生成 `window_controls()` 覆盖
5. `demo/src/shell/main_window.rml.rs` 使用旧类型名 `ActivityPanelItem`

**关键风险**：第 377/519 行生成的 `.window_controls(self.window_controls())` 会编译失败，因为 `ModernWindowShell`/`TabWindowShell` 在 Part 1-2 中已移除 `window_controls()` builder 方法。

## 提议变更

### 变更 1：更新 `crates/ui/src/components/mod.rs`

**文件**：`crates/ui/src/components/mod.rs`
**原因**：activity_bar.rs 已重写，导出的类型名变更
**操作**：替换 re-export 列表

```rust
// 旧（第 6-8 行）：
pub use activity_bar::{
    ActivityActionItem, ActivityBar, ActivityPanelItem,
};
pub use tree_view::TreeView;

// 新：
pub use activity_bar::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels,
    IActivityAct, IActivityPanel,
};
pub use tree_view::TreeView;
```

### 变更 2：更新 `crates/ui/src/lib.rs`

**文件**：`crates/ui/src/lib.rs`（第 85 行）
**原因**：与 mod.rs 保持一致
**操作**：替换 re-export 列表

```rust
// 旧（第 85 行）：
pub use components::{ActivityActionItem, ActivityBar, ActivityPanelItem, TreeView};

// 新：
pub use components::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels,
    IActivityAct, IActivityPanel, TreeView,
};
```

### 变更 3：更新 `crates/ui/src/prelude.rs`

**文件**：`crates/ui/src/prelude.rs`
**原因**：ActivityBar 相关类型应进入 prelude 供用户便捷使用
**操作**：在 `pub use crate::{...}` 列表中添加类型

```rust
// 旧（第 5-11 行）：
pub use crate::{
    Badge, Button, ButtonGroup, ButtonVariants, Checkbox, Dialog, Disableable, Form, Input,
    InputState, IWindowActions, Kbd, Label, List, MenuItem, ModernWindow, ModernWindowShell,
    Notification, NotificationKind, NotificationList, NotificationType, Popover, Progress,
    ProgressCircle, Radio, Root, Select, Selectable, Separator, Sizable, Slider, StatusBar,
    StatusBarItem, StyledExt, Switch, Tab, TabBar, Table, Tag, TitleBar, Tooltip, Window, WindowExt,
};

// 新（添加 ActivityBar 相关类型，按字母序插入）：
pub use crate::{
    ActivityAct, ActivityBar, ActivityPanel, Badge, Button, ButtonGroup, ButtonVariants,
    Checkbox, Dialog, Disableable, Form, IActivityAct, IActivityPanel, Input, InputState,
    IWindowActions, Kbd, Label, List, MenuItem, ModernWindow, ModernWindowShell, Notification,
    NotificationKind, NotificationList, NotificationType, Popover, Progress, ProgressCircle,
    Radio, Root, Select, Selectable, Separator, Sizable, Slider, StatusBar, StatusBarItem,
    StyledExt, Switch, Tab, TabBar, Table, Tag, TitleBar, Tooltip, Window, WindowExt,
};
```

注：`ActivityActs`/`ActivityPanels` 是类型别名，主要用于 `#[computed]` 返回类型，不放入 prelude（用户应通过完整路径 `rml_ui::ActivityPanels` 引用以保持清晰）。

### 变更 4：清理 `crates/engine/src/compiler/codegen.rs`

**文件**：`crates/engine/src/compiler/codegen.rs`
**原因**：ModernWindowShell/TabWindowShell 已移除 `window_controls()` builder，生成的 `.window_controls(...)` 调用会导致编译失败；`show_minimize`/`show_maximize`/`show_close` 属性因 TitleBar 内置按钮无法过滤而已无意义
**操作**：删除 3 处代码

**4a. 删除第 377 行**（`gen_modern_window_wrapper` 末尾）：
```rust
// 删除：
code.push_str(".window_controls(self.window_controls())");
```

**4b. 删除第 519 行**（`gen_tab_window_wrapper` 末尾）：
```rust
// 删除：
code.push_str(".window_controls(self.window_controls())");
```

**4c. 删除第 583-592 行**（`gen_window_extra_methods` 中的 show_minimize/show_maximize/show_close 逻辑）：
```rust
// 删除：
    let minimize = extract_static_bool_attr(elem, "show_minimize").unwrap_or(true);
    let maximize = extract_static_bool_attr(elem, "show_maximize").unwrap_or(true);
    let close = extract_static_bool_attr(elem, "show_close").unwrap_or(true);
    if !minimize || !maximize || !close {
        out.push_str(&format!(
            "\n    fn window_controls(&self) -> rml_core::window::WindowControlButtons {{\n        \
             rml_core::window::WindowControlButtons {{ minimize: {minimize}, maximize: {maximize}, close: {close} }}\n    }}\n"
        ));
    }
```

**4d. 更新 `gen_window_extra_methods` 文档注释**（第 544 行）：
```rust
// 旧：
/// 生成 IWindow 可选配置方法（left/top/startup/min_size/window_controls）

// 新：
/// 生成 IWindow 可选配置方法（left/top/startup/min_size）
```

注：`WindowControlButtons` 类型与 `IWindow::window_controls()` trait 方法保留在 `crates/core/src/window.rs`（属于公共 API，Native chrome 模式仍可能使用），仅删除 codegen 中对它的消费。

### 变更 5：更新 `demo/src/shell/main_window.rml.rs`

**文件**：`demo/src/shell/main_window.rml.rs`
**原因**：使用新的 `ActivityPanel` struct 和 `ActivityPanels` 类型别名
**操作**：更新 import 和 `activity_icons()` 方法

```rust
// 旧（第 4-6 行）：
use rml_ui::{
    ActivityPanelItem, IconName, MenuItem, StatusBarItem, TabItem, TreeState,
};

// 新：
use rml_ui::{
    ActivityPanel, ActivityPanels, IconName, MenuItem, StatusBarItem, TabItem, TreeState,
};
```

```rust
// 旧（第 45-52 行）：
#[computed]
pub fn activity_icons(&self) -> Vec<ActivityPanelItem> {
    let _ = self.i18n_version;
    vec![
        ActivityPanelItem::new("samples", IconName::BookOpen, t_static("shell.samples"))
            .active(true),
    ]
}

// 新：
#[computed]
pub fn activity_icons(&self) -> ActivityPanels {
    let _ = self.i18n_version;
    vec![
        ActivityPanel::new("samples", IconName::BookOpen, t_static("shell.samples"))
            .active(true)
            .into_arc(),
    ]
}
```

**关键点**：
- 返回类型改为 `ActivityPanels`（类型别名，单 token），避免 `#[computed]` 宏的 `return_type_str()` 将 `Vec<Arc<dyn IActivityPanel>>` 错误合并为 `Vec<Arc<dynIActivityPanel>>`
- 构造调用 `.into_arc()` 将 `ActivityPanel` 转为 `Arc<dyn IActivityPanel>` 以匹配 `ActivityPanels` 类型

## 假设与决策

1. **`WindowControlButtons` 类型保留**：虽 codegen 不再消费，但 `IWindow::window_controls()` trait 方法仍属公共 API，保留以支持未来 Native chrome 模式或自定义标题栏场景
2. **`ActivityActs`/`ActivityPanels` 不入 prelude**：类型别名主要用于 `#[computed]` 返回类型声明，用户应通过完整路径引用以保持类型清晰
3. **demo RML 模板无需修改**：`demo/src/shell/main_window.rml` 未使用 `show_minimize`/`show_maximize`/`show_close` 属性，ActivityBar 的 `panels={activity_icons}` 绑定表达式不变
4. **codegen 的 `component.rs` 无需修改**：`panels`/`actions` bind setter 已正确生成 `.panels({}.clone())`/`.actions({}.clone())`，`ActivityPanels`/`ActivityActs` 实现 `Clone`

## 验证步骤

1. **编译验证**：
   ```powershell
   cargo build
   ```
   预期：workspace 全量编译通过，无错误

2. **测试验证**：
   ```powershell
   cargo test
   ```
   预期：所有测试通过（含 codegen 集成测试）

3. **关键功能点检查**：
   - `ModernWindowShell` 不再调用 `.window_controls()`
   - `TabWindowShell` 不再调用 `.window_controls()`
   - `activity_icons()` 返回 `ActivityPanels` 类型
   - `ActivityPanel::new(...).into_arc()` 正确构造 `Arc<dyn IActivityPanel>`
   - `cargo build` 无 "method not found" 或 "type mismatch" 错误

## 执行顺序

1. 变更 1：`crates/ui/src/components/mod.rs`
2. 变更 2：`crates/ui/src/lib.rs`
3. 变更 3：`crates/ui/src/prelude.rs`
4. 变更 4：`crates/engine/src/compiler/codegen.rs`（4a → 4b → 4c → 4d）
5. 变更 5：`demo/src/shell/main_window.rml.rs`
6. `cargo build` 验证
7. `cargo test` 验证
