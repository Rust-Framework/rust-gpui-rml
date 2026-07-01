# 单 Host 贡献中心化 Demo 重构 — 续接计划

## Summary

续接上一会话中断的重构：将 demo 从 3-host 架构（`demo.shell.activity-bar` / `demo.shell.status` / `demo.shell.case-tree`）收敛为单一 `demo.shell` host，按 `properties["kind"]` 分类（menu/activity/status/case）。MainWindow 作为 host 管理者，通过 `on_changed` 回调驱动 UI 绑定刷新；CaseActivityPanel 封装案例树并自注册为 activity；案例组件通过 `#[contribute]` 自注册为 case。

上一会话已完成：宏扩展（`crates/macros/src/contribute.rs`）、`contributions.rs` 映射层、`CaseActivityPanel` 组件、4 个案例组件的 `#[contribute]` 标注、`catalog.rs` 精简。本计划仅处理剩余的 5 个文件修改 + 3 个文件/目录删除 + 编译验证。

## Current State Analysis

### 已完成（验证通过，无需改动）
- `crates/macros/src/contribute.rs` — 支持 `kind`/`parent_id` 参数 + 多 item 解析
- `demo/src/shell/contributions.rs` — `SHELL_HOST` 常量 + 4 个 `build_*` 函数 + 3 个 `register_*` 辅助
- `demo/src/shell/case_activity_panel.rml.rs` + `.rml` — 自注册 activity，`observe_global` 自刷新树
- `demo/src/cases/{counter,two_way,button,i18n}_case.rml.rs` — 已加 `#[contribute(... kind="case")]`
- `demo/src/cases/catalog.rs` — 已精简为 `OpenTab` + `case_title_key()`

### 待修改（本计划范围）
| 文件 | 当前状态 | 问题 |
|------|---------|------|
| `demo/src/shell/main_window.rml.rs` | 旧 3-host 架构 | 引用已删除的 `navigation`/`bindings`/`hosts`/`cases::init_tree_state`/`cases::refresh_tree_state` |
| `demo/src/shell/main_window.rml` | 内嵌 `<Tree>` | 应改为 `<CaseActivityPanel />` |
| `demo/src/shell/mod.rs` | 声明 `hosts`/`bindings` 模块 | 需改为 `contributions`/`case_activity_panel` |
| `demo/src/cases/mod.rs` | re-export `init_tree_state`/`refresh_tree_state` | catalog.rs 已无这两个函数，编译断裂 |
| `demo/src/app.rs` | 调用 `features::ensure_hosts`/`features::register_all` | 需改为单 host ensure + 各 `__rml_register_*` 调用 |

### 待删除
- `demo/src/shell/hosts.rs` — 3 个 host_id 常量，已被 `contributions::SHELL_HOST` 取代
- `demo/src/shell/bindings.rs` — 旧映射函数，已被 `contributions.rs` 取代
- `demo/src/features/` 整个目录 — `case_tree.rs`/`samples_panel.rs`/`status_text.rs`/`navigation.rs`/`mod.rs`，功能已由 `contributions.rs` + `CaseActivityPanel` + `main_window::activate_case` 取代

### 不触碰（用户明确指示）
- `demo/src/cases/menu_context_case.rml.rs` / `menu_dropdown_case.rml.rs` / `menu_editor_case.rml.rs` / `menu_features_case.rml.rs` / `menu_custom_case.rml.rs` — 由另一任务处理
- 这些案例的 `Entity` 字段 + `<div if={...}>` 条件渲染块保留在 main_window 中

## Proposed Changes

### 1. `demo/src/shell/main_window.rml.rs`（重写）

**目标**：移除旧 3-host 依赖，改用单 host + `contributions` 映射层 + 静态 weak 桥接。

**关键改动**：

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use gpui::{BorrowAppContext, Entity, SharedString, Window};
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_ui::{ActivityPanels, MenuItems, StatusBarItems, TabItem};

use crate::cases::{
    self, ButtonCase, CounterCase, I18nCase, MenuContextCase, MenuDropdownCase,
    MenuEditorCase, MenuFeaturesCase, MenuCustomCase, OpenTab, TwoWayCase, WelcomeCase,
};
use crate::shell::contributions::{self, SHELL_HOST};
```

- **移除 import**：`crate::features::navigation`、`crate::shell::{bindings, hosts}`、`TreeState`
- **新增 static 桥接**：
  ```rust
  static MAIN_WINDOW_WEAK: Mutex<Option<gpui::WeakEntity<MainWindow>>> = Mutex::new(None);

  pub fn activate_case(case_id: String, app: &mut gpui::App) {
      if let Ok(guard) = MAIN_WINDOW_WEAK.lock() {
          if let Some(weak) = guard.as_ref() {
              if let Some(entity) = weak.upgrade() {
                  entity.update(app, |main, cx| { main.open_case(case_id, cx); });
              }
          }
      }
  }
  ```
  替代 `features/navigation.rs` 的作用，供 `CaseActivityPanel::on_case_activate` 调用。

- **struct 字段改动**：
  - 移除：`case_tree_state: Option<Entity<TreeState>>`（树状态由 CaseActivityPanel 自管）
  - 移除：`theme_cmd: Option<Arc<dyn ICommand>>`、`lang_cmd: Option<Arc<dyn ICommand>>`
  - 新增：`menu_commands: HashMap<String, Arc<dyn ICommand>>`（按 menu entry id 查找命令）
  - 保留：`i18n_version`（驱动 `tab_bar_items` computed 重算）、所有 case entity 字段（含 5 个 menu_*_case）

- **`on_loaded` 重写**：
  1. 初始化 welcome tab（保留原逻辑）
  2. 初始化所有 case entity（保留原逻辑，含 menu_*_case）
  3. 存储 weak 引用到 `MAIN_WINDOW_WEAK`
  4. 构建 menu commands 并存入 `menu_commands`：
     ```rust
     self.menu_commands.insert(
         "menu.theme_toggle".to_string(),
         Arc::new(RelayCommand::new(cx, |this, cx| this.apply_toggle_theme(cx))),
     );
     self.menu_commands.insert(
         "menu.lang_en".to_string(),
         Arc::new(RelayCommand::new(cx, |this, cx| this.apply_switch_en(cx))),
     );
     ```
  5. `Self::wire_host_changed(cx);` — 单 host 监听
  6. `self.refresh_bindings(cx);` — 首次构建 activity/status/menu 绑定
  7. login dialog defer（保留原逻辑）

- **`wire_host_changed`**（替代 `wire_contribution_sync`）：
  ```rust
  fn wire_host_changed(cx: &mut Context<Self>) {
      let weak = cx.weak_entity();
      cx.update_global::<rml_app::contribution::ContributionRegistryGlobal, _>(|global, _| {
          global.0.set_host_on_changed(
              SHELL_HOST,
              Box::new(move |app| {
                  if let Some(entity) = weak.upgrade() {
                      entity.update(app, |main, cx| { main.refresh_bindings(cx); });
                  }
              }),
          );
      });
  }
  ```

- **`refresh_bindings`**（替代 `refresh_shell_bindings` + `rebuild_menu_items`）：
  ```rust
  fn refresh_bindings(&mut self, cx: &mut Context<Self>) {
      self.activity_panels = contributions::build_activity_panels(cx, &self.active_panel_id);
      self.status_items = contributions::build_status_items(cx);
      self.menu_items = contributions::build_menu_items(cx, &self.menu_commands);
  }
  ```

- **移除方法**：`rebuild_menu_items`、`refresh_shell_bindings`、`wire_contribution_sync`、`on_case_activate`（已移至 CaseActivityPanel）

- **`apply_switch_en` 改动**：
  - 移除 `if let Some(tree) = self.case_tree_state.as_ref() { cases::refresh_tree_state(tree, cx); }`（CaseActivityPanel 通过 `observe_global::<I18nState>` 自刷新）
  - 移除 `self.rebuild_menu_items(cx);`（由 `refresh_bindings` 统一处理）
  - 改为 `self.refresh_bindings(cx);`
  - 保留 tab 标题更新 + `i18n_version` bump + `cx.notify()`

- **`on_panel_change` 改动**：`self.refresh_shell_bindings(cx)` → `self.refresh_bindings(cx)`

- **`open_case` 保留**：被 `activate_case` 桥接函数调用

### 2. `demo/src/shell/main_window.rml`（局部改动）

```diff
 <slot_left>
     <ActivityBar panels={activity_panels} on_panel_change="on_panel_change">
         <div if={active_panel_id == "samples"} class="nav-tree">
-            <Tree on_activate="on_case_activate" />
+            <CaseActivityPanel />
         </div>
     </ActivityBar>
 </slot_left>
```

其余（menu/status_bar/case-host 条件渲染）保持不变。

### 3. `demo/src/shell/mod.rs`（重写）

```rust
pub mod contributions;
#[path = "case_activity_panel.rml.rs"]
pub mod case_activity_panel;
#[path = "login_dialog.rml.rs"]
pub mod login_dialog;
#[path = "main_window.rml.rs"]
pub mod main_window;

pub use case_activity_panel::CaseActivityPanel;
pub use login_dialog::LoginDialog;
pub use main_window::MainWindow;
```

移除：`pub mod hosts;`、`pub mod bindings;`

### 4. `demo/src/cases/mod.rs`（局部改动）

```diff
-pub use catalog::{case_title_key, init_tree_state, refresh_tree_state, OpenTab};
+pub use catalog::{case_title_key, OpenTab};
```

修复编译断裂（catalog.rs 已无 `init_tree_state`/`refresh_tree_state`）。其余模块声明 + re-export 全部保留（含 5 个 menu_*_case）。

### 5. `demo/src/app.rs`（重写）

```rust
//! 应用启动引导 —— 声明式入口：on_launch 仅做全局初始化，主窗口由框架管理

use gpui::{App, BorrowAppContext};
use rml_app::IAppLifecycle;
use rml_app::contribution::ContributionRegistryGlobal;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

use crate::cases::{button_case, counter_case, i18n_case, two_way_case};
use crate::shell::case_activity_panel;
use crate::shell::contributions;

#[derive(Default)]
pub struct Startup;

impl IAppLifecycle for Startup {
    fn on_launch(&mut self, cx: &mut App) {
        cx.set_style("styles.css");
        cx.set_i18n("zh-CN");
        cx.set_theme("light");

        // 单 host 预创建
        cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.ensure_host(contributions::SHELL_HOST);
        });

        // 案例分类根节点（cat.binding / cat.components / cat.i18n）
        contributions::register_case_categories(cx);

        // 案例组件自注册（kind=case）
        counter_case::__rml_register_countercase(cx);
        two_way_case::__rml_register_twowaycase(cx);
        button_case::__rml_register_buttoncase(cx);
        i18n_case::__rml_register_i18ncase(cx);

        // ActivityBar 面板自注册（kind=activity）
        case_activity_panel::__rml_register_caseactivitypanel(cx);

        // 菜单元数据（kind=menu）—— 命令由 MainWindow.menu_commands 按 id 挂接
        contributions::register_menu_entry(cx, "menu.theme_toggle", "menu.theme_toggle", 0);
        contributions::register_menu_entry(cx, "menu.lang_en", "menu.lang_en", 10);

        // 状态栏元数据（kind=status）
        contributions::register_status_entry(cx, "status.ready", "status.ready", 0);
    }
}
```

**说明**：
- 菜单/状态条目使用 i18n key 作为 id + name_key（`menu.theme_toggle`/`menu.lang_en`/`status.ready`），MainWindow 按 id 在 `menu_commands` 查找命令
- `status.ready` i18n key 若不存在，需在 i18n 资源中补充（或复用 `shell.welcome`）
- 注册顺序：host ensure → 分类根 → case 组件 → activity 面板 → menu/status 元数据。MainWindow 创建后 `on_loaded` 中 wire_host_changed + refresh_bindings 读取所有已注册条目

### 6. 删除文件

- `demo/src/shell/hosts.rs`
- `demo/src/shell/bindings.rs`
- `demo/src/features/` 整个目录（`mod.rs`/`case_tree.rs`/`samples_panel.rs`/`status_text.rs`/`navigation.rs`）

### 7. i18n 补充（若需要）

检查 `demo/assets/i18n/zh-CN.json` 与 `en-US.json` 是否存在 `status.ready` key。若缺失则补充：
- zh-CN: `"status.ready": "就绪"`
- en-US: `"status.ready": "Ready"`

若用户偏好复用现有 key，可改为 `register_status_entry(cx, "status.welcome", "shell.welcome", 0)`。

## Assumptions & Decisions

1. **menu_*_case 不触碰**：用户明确指示「那是另一个任务在处理的工作，你别动」。保留它们的 entity 字段 + 条件渲染块 + cases/mod.rs 声明。
2. **`activate_case` 桥接方式**：采用 `static Mutex<Option<WeakEntity<MainWindow>>>` 替代旧 `features/navigation.rs` 的 handler 函数指针。CaseActivityPanel 通过 `crate::shell::main_window::activate_case(id, cx)` 调用。
3. **菜单命令侧表**：菜单贡献是纯元数据（TextContribution），不携带命令。MainWindow 维护 `HashMap<String, Arc<dyn ICommand>>`，`build_menu_items` 按 entry id 查找挂接。这避免了元数据贡献需要持有 GPUI listener 的难题。
4. **i18n 刷新分工**：CaseActivityPanel 自管树刷新（`observe_global::<I18nState>`）；MainWindow 管理标签栏 + 菜单 + 状态栏刷新（`apply_switch_en` → `refresh_bindings`）。`i18n_version` 字段保留以驱动 `tab_bar_items` computed 重算。
5. **注册时机**：所有贡献在 `app.rs::on_launch` 中注册（先于 MainWindow 创建）。MainWindow `on_loaded` 仅做 wire + 首次 `refresh_bindings`。
6. **`#[contribute]` 生成的注册函数命名**：`__rml_register_<lowercase_struct_name>`，即 `CounterCase` → `__rml_register_countercase`，`CaseActivityPanel` → `__rml_register_caseactivitypanel`。

## Verification Steps

1. **编译验证**：`cargo build --workspace` — 期望无错误
2. **测试验证**：`cargo test --workspace` — 期望已有测试全通过
3. **运行时验证**（手动）：启动 demo，确认：
   - ActivityBar 显示 "samples" 图标，点击展开案例树
   - 案例树显示 3 个分类（binding/components/i18n）+ 子案例
   - 点击案例节点打开对应 Tab
   - 菜单栏显示 "主题切换" / "切换英文" 两项，点击生效
   - 状态栏显示就绪文本
   - 切换英文后，树/菜单/状态/标签标题全部刷新
