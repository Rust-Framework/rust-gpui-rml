# Demo MVVM Showcase 重构计划

## Summary

将 demo 重构为真正展示 RML 四大优势的标杆示例：**简洁易用、声明式界面设计、MVVM 数据驱动、自动化 UI 刷新**。核心是消除当前 demo 中的冗余逻辑（7 个 RelayCommand 字段 + init_commands 闭包 + build_menu_tree 手工构建的三处一一对应样板，以及 apply_switch_en 手动重投影），让"打开资源 → TabBar 新建 Tab 并激活"的链路完全自动化、零手动同步代码。

本次重构分三个阶段：
- **Phase A**：修复 Tab/TabItem 类型不匹配（阻塞 Phase 5+6 收尾）
- **Phase B**：菜单改贡献驱动（7 ICommand structs + MenuViewModel 从 entries 投影，消除 RelayCommand 样板）
- **Phase C**：i18n 响应式（menus/status 改 `#[computed]`，依赖 i18n_version 自动失效）+ on_loaded 精简 + 验证

## Current State Analysis

### 已完成（前次会话 Phase 1-6）
- `ObservableVec<T>` 响应式集合（`crates/core/src/observable.rs`，6 单测通过）
- codegen 版本路由（ObservableVec 字段 → 内部 AtomicU64）
- `<template slot="tabs" each>` codegen（`TabsEach` + `gen_tab_window_wrapper`）
- `TabWindowShell::tab_children` builder 支持
- demo `workbenches: ObservableVec<Arc<dyn IWorkbench>>` + channel bridge + `#[computed] selected_tab`
- 模板 `<template slot="tabs" each={w in workbenches}>` 声明式迭代

### 当前阻塞
**Tab/TabItem 类型不匹配**：codegen 生成 `.tab_children(self.workbenches.iter().map(|w| rml_ui::Tab::new().label(...)).collect())`，但 `tab_children(items: Vec<TabItem>)` 期望 `Vec<TabItem>`，迭代器产出 `Tab`。`impl From<Tab> for TabItem` 已存在于 `tab_item.rs:165`。

### 待消除的冗余（探索发现）

| # | 冗余 | 位置 | 违背的优势 |
|---|------|------|-----------|
| 1 | 7 个 RelayCommand 字段 + Default 占位 | `main_window.rml.rs:51-57, 107-134` | 简洁易用 |
| 2 | init_commands 7 个闭包 | `main_window.rml.rs:172-195` | 简洁易用 |
| 3 | build_menu_tree 手工树构建 | `main_window.rml.rs:286-330` | 简洁易用 + MVVM |
| 4 | menus 不走贡献系统（与 cases/status/activities 割裂） | `main_window.rml.rs:261, 280` | MVVM 数据驱动 |
| 5 | apply_switch_en 手动重投影 menus/status | `main_window.rml.rs:480-489` | 自动 UI 刷新 |

### 已确认保留（本次不改）
- `render_menu_bar`/`render_status_bar` 命令式渲染（菜单有分组/子级，状态栏有 align 内部逻辑，难以 each 模式化，下一步规划）
- `project_entries` 三处 filter_map（模式清晰，抽象收益低）
- `init_panel_observers` 硬编码（专项优化，超出本次范围）

## Proposed Changes

### Phase A：Tab/TabItem 类型修复（收尾 Phase 5+6）

**目标**：解除 Phase 5+6 的编译阻塞，让声明式 TabBar 迭代跑通。

**文件**：`crates/ui/src/window/tab_window.rs`

**What/Why/How**：
- 修改 `tab_children` builder 签名（L227）：
  ```rust
  // 旧
  pub fn tab_children(mut self, items: Vec<TabItem>) -> Self {
      self.tab_children = items;
      self
  }
  // 新
  pub fn tab_children(mut self, items: impl IntoIterator<Item = impl Into<TabItem>>) -> Self {
      self.tab_children = items.into_iter().map(Into::into).collect();
      self
  }
  ```
- 理由：`From<Tab> for TabItem` 已存在（`tab_item.rs:165`），一次性解决 each 模式（`iter().map(|w| Tab::new()...).collect()`）与列表模式（`vec![Tab::new()...]`）两种 codegen 路径的类型不匹配。对 codegen 零改动，API 更友好（与 `TabBar::child` 接受 `impl Into<TabItem>` 风格一致）。

**验证**：`cargo check -p rust-rml-demo` 通过，TabBar 声明式迭代编译成功。

---

### Phase B：菜单改贡献驱动（消除 RelayCommand 样板）

**目标**：将 menus 从手工构建改为贡献系统驱动，与 cases/status/activities 统一模式，消除 7 个 RelayCommand 字段 + init_commands + build_menu_tree 三处一一对应样板。

#### B1. 创建 menu_commands.rs（7 个 ICommand structs）

**新文件**：`demo/src/shell/menu_commands.rs`

7 个命令 struct，每个用 `#[contribute(command, ...)]` 注册到 `demo.shell` host 的 `menu` slot：

| struct | id | parent_id | order | label key | 行为 |
|--------|----|-----------|-------|-----------|------|
| `OpenWelcomeCommand` | `menu.file.new` | `menu.file` | 1 | `menu.file_new` | open_case("welcome") |
| `OpenButtonCaseCommand` | `menu.file.open` | `menu.file` | 2 | `menu.file_open` | open_case("components.button") |
| `ExitCommand` | `menu.file.exit` | `menu.file` | 3 | `menu.file_exit` | cx.quit() |
| `ToggleThemeCommand` | `menu.view.theme` | `menu.view` | 1 | `menu.theme_toggle` | apply_toggle_theme |
| `SwitchEnCommand` | `menu.view.lang` | `menu.view` | 2 | `menu.lang_en` | apply_switch_en |
| `OpenMenuDropdownCaseCommand` | `menu.help.nested` | `menu.help.center` | 1 | `case.menu.nested` | open_case("components.menu.dropdown") |
| `OpenFeaturesCaseCommand` | `menu.help.features` | `menu.help.features.group` | 1 | `case.menu.features.title` | open_case("components.menu.features") |

**额外注册 2 个 submenu root**（非命令，纯分组节点）：用 `#[contribute(command, id="menu.file", label="menu.file", order=1)]` 注册为虚拟命令节点，或用 `group` 属性标识为 root。

> **设计决策**：submenu root（"文件"/"视图"/"帮助"/"帮助中心"/"功能组"）不执行命令，仅作分组。用 `#[contribute(command, id="menu.file", label="menu.file")]` 注册，execute() 为 no-op，is_leaf=false（有 children）。MenuViewModel 构建时按 parent_id 组织树。

每个命令 struct 模式：
```rust
#[contribute(
    command,
    host_id = "demo.shell",
    id = "menu.file.new",
    parent_id = "menu.file",
    order = 1,
    label = "menu.file_new"
)]
pub struct OpenWelcomeCommand;

impl ICommand for OpenWelcomeCommand {
    fn execute(&self, ctx: &mut CallContext) {
        // 经 MainWindowRef 查询 MainWindow entity，调用 open_case
        if let Some(ref_arc) = ctx.app.try_global::<MainWindowRef>() {
            let _ = ref_arc.0.update(&mut ctx.app, |this, cx| {
                this.open_case("welcome".to_string(), cx);
            });
        }
    }
}
```

> **注意**：需确认 `IAppContext::try_global` / `try_service` API 获取 `MainWindowRef`。若不存在，用 `Context::has_global` / `App::global`。参考 `main_window.rml.rs:198-201` 的 `cx.set_service(Arc::new(MainWindowRef(...)))` 与 `init_services`。

#### B2. 重构 MenuViewModel（从 entries 投影）

**文件**：`demo/src/shell/menu_view_model.rs`

**What**：
- 增加字段：`id: String`、`parent_id: Option<String>`、`order: i32`
- 实现 `from_contribution(c: Arc<dyn IContribution>, opts: ContributionOptions) -> Option<Self>`：
  - 过滤 `opts.effective_slot() == Some("menu")`
  - label = `t_static(opts.properties["label"])`（i18n key → 本地化文本）
  - id = `c.id()`
  - parent_id = `opts.parent_id.clone()`
  - order = `opts.order`
  - command = `c.as_command()`（需确认 IContribution 是否有 as_command() 方法，类似 as_visual()）
- 实现 `build_menu_view_models(entries: &[ContribEntry]) -> Vec<MenuViewModel>`：
  - 从 entries 构建 id → MenuViewModel 映射
  - 按 parent_id 组织树：parent_id=None 为顶层，按 order 排序
  - 递归挂载 children

> **关键依赖**：需确认 `IContribution::as_command() -> Option<Arc<dyn ICommand>>` 是否存在。若不存在，参考 `VisualAbilityExt::as_visual()` 模式实现 `CommandAbilityExt::as_command()`。命令能力注册由 `#[contribute(command)]` 宏自动生成（参考 `contribute.rs` 的 command flag 处理）。

#### B3. 重构 MainWindow（删除 RelayCommand 样板）

**文件**：`demo/src/shell/main_window.rml.rs`

**What**：
- 删除 7 个 RelayCommand 字段（L51-57）
- 删除 `init_commands` 方法（L172-195），保留 `ensure_status_ready_registered()` 调用移至 `on_loaded`
- 删除 `build_menu_tree` 方法（L286-330）
- `Default` impl 删除 7 个 `default_cmd.clone()` 初始化
- `project_entries` 增加 menus 投影：`self.menus = build_menu_view_models(&entries);`，删除"menus 不经贡献系统"注释
- `apply_switch_en` 删除 `self.menus = self.build_menu_tree();`（Phase C 处理）
- `init_contribution_host` 保持不变（注册 host + bootstrap）
- `on_loaded` 删除 `self.init_commands(cx)` 行，将 `ensure_status_ready_registered()` 移至 `project_entries` 前

#### B4. mod.rs 导出

**文件**：`demo/src/shell/mod.rs`
- 增加 `pub mod menu_commands;`（或 `mod menu_commands;`）

**验证**：`cargo check -p rust-rml-demo` 通过；菜单仍能渲染（数据源从手工构建改为 entries 投影，渲染逻辑不变）。

---

### Phase C：i18n 响应式 + on_loaded 精简

**目标**：让 menus/status 在 i18n 切换时自动刷新，消除 apply_switch_en 手动重投影；精简 on_loaded 顺序耦合。

#### C1. menus/status 改为 #[computed]

**文件**：`demo/src/shell/main_window.rml.rs`

**What**：
- 删除 `menus: Vec<MenuViewModel>` 字段（L46）
- 删除 `status: Vec<StatusViewModel>` 字段（L47）
- 新增 `#[computed] pub fn menus(&self) -> Vec<MenuViewModel>`：
  ```rust
  #[computed]
  pub fn menus(&self) -> Vec<MenuViewModel> {
      let entries = self.entries.read().unwrap();
      build_menu_view_models(&entries)
  }
  ```
- 新增 `#[computed] pub fn status(&self) -> Vec<StatusViewModel>`：
  ```rust
  #[computed]
  pub fn status(&self) -> Vec<StatusViewModel> {
      let entries = self.entries.read().unwrap();
      build_status_view_models(&entries)
  }
  ```
- `Default` impl 删除 `menus`/`status` 字段初始化
- `project_entries` 删除 `self.menus = ...` 和 `self.status = ...` 赋值（改为 computed 自动构建）

**Why**：`#[contributehost]` 宏注入 `i18n_version` 字段，`scanner.rs:249-252` 自动为 computed 方法添加 `i18n_version` 依赖。i18n 切换 → 框架 bump i18n_version → computed 失效 → 重算 → 模板刷新。零手动同步代码。

#### C2. 简化 apply_switch_en

**文件**：`demo/src/shell/main_window.rml.rs`

**What**：
```rust
// 旧（L480-489）
pub(crate) fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
    cx.set_i18n("en-US");
    self.menus = self.build_menu_tree();
    self.status = { let entries = self.entries.read().unwrap(); build_status_view_models(&entries) };
    cx.notify();
}
// 新
pub(crate) fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
    cx.set_i18n("en-US");
    // menus/status 为 #[computed]，依赖 i18n_version，set_i18n 后自动失效重算
    cx.notify();
}
```

#### C3. on_loaded 精简

**文件**：`demo/src/shell/main_window.rml.rs`

**What**：当前 on_loaded 9 步（L137-158），删除 `init_commands` 后剩 8 步。补充注释说明顺序依赖：
- `init_contribution_host` 必须最先（注册 host + bootstrap 所有 `#[contribute]`）
- `ensure_status_ready_registered` 必须在 `project_entries` 前（使 as_visual() 查询生效）
- `project_entries` 在 host 注册后（entries 已填充）
- `init_workbench` 在 `init_lsp` 后（依赖 lsp_provider）

精简后 on_loaded 约 12 行：
```rust
fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    // 通道桥接：ObservableVec 写 → cx.notify → 自动 UI 刷新
    let (tx, rx) = flume::unbounded();
    self.workbenches = ObservableVec::with_notify(tx);
    cx.spawn(|this, mut cx| async move {
        while rx.recv_async().await.is_ok() {
            let _ = this.update(&mut cx, |_, cx| cx.notify());
        }
    }).detach();

    self.init_contribution_host(cx);
    crate::cases::status_bar_case::ensure_status_ready_registered();
    self.project_entries();
    self.init_services(cx);
    self.init_lsp();
    self.init_workbench(cx);
    self.init_activity_bar(cx);
    self.init_panel_observers(cx);
    cx.notify();
}
```

**验证**：
1. `cargo check -p rust-rml-demo` 通过
2. `cargo test -p rust-rml-core` 通过（ObservableVec 6 单测）
3. 手动运行 demo：打开案例 → TabBar 新增 Tab 并激活；切换 i18n → 菜单/状态栏标签自动刷新；点击菜单项 → 命令执行

## Assumptions & Decisions

### 已确认决策
1. **Tab 机制**：模板定制模式（`<template slot="tabs" each>`，已实现）
2. **菜单范围**：重构为贡献驱动（7 ICommand structs + MenuViewModel 从 entries 投影）
3. **渲染下沉**：菜单/状态栏保持命令式（内部有分组/align 逻辑，下一步规划）
4. **i18n 响应式**：observe locale 自动重投影 → 实为 `#[computed]` 依赖 `i18n_version`（框架已支持，无需手动 observe）

### 关键假设（需实现时验证）
1. **`IContribution::as_command()` 存在**：类似 `VisualAbilityExt::as_visual()`。若不存在，需实现 `CommandAbilityExt::as_command()`（参考 `VisualAbilityExt` 模式）。`#[contribute(command)]` 宏应自动注册命令能力。
2. **`MainWindowRef` 获取 API**：`cx.set_service(Arc::new(MainWindowRef(...)))` 已注册，命令 struct 需通过 `App::try_global`/`try_service` 获取。需确认 `IAppContext` 或 `App` 上的查询方法。
3. **`ContributionOptions::parent_id` 字段存在**：summary 确认 `#[contribute]` 支持 `parent_id` 参数 → `ContributionOptions` builder。需读 `ContributionOptions` 结构确认字段名。
4. **submenu root 注册**：用 `#[contribute(command, id="menu.file", label="menu.file")]` 注册虚拟命令节点（execute no-op），或用 `group`/`is_root` 属性标识。实现时确认 `#[contribute]` 是否支持非命令的纯分组节点注册。
5. **`#[computed]` 返回 Vec**：`ComputedCache::get_or_compute` 要求 `T: Clone`，`Vec<MenuViewModel>` 是 Clone（MenuViewModel derive Clone）。OK。

### 不做的事
- 不改 `render_menu_bar`/`render_status_bar` 命令式渲染（下一步规划）
- 不改 `cases`/`activities` 字段为 computed（控制范围；cases 名称刷新依赖 case 实现）
- 不改 `init_panel_observers` 硬编码（专项优化）
- 不改 `project_entries` 三处 filter_map 抽象（模式清晰）
- 不改框架 core（i18n 响应式机制已就绪）

## Verification Steps

1. **Phase A 验证**：
   - `cargo check -p rust-rml-demo` 通过
   - 确认 `tab_children` 签名变更不破坏其他 crate（`cargo check --workspace`）

2. **Phase B 验证**：
   - `cargo check -p rust-rml-demo` 通过
   - 确认 7 个命令 struct 的 `#[contribute]` 注册成功（编译期 assert）
   - 确认 `MenuViewModel::from_contribution` + `build_menu_view_models` 正确构建树
   - 确认 `MainWindow` 无 7 个 RelayCommand 字段

3. **Phase C 验证**：
   - `cargo check -p rust-rml-demo` 通过
   - `cargo test -p rust-rml-core` 通过
   - 确认 `menus`/`status` 是 `#[computed]` 方法（非字段）
   - 确认 `apply_switch_en` 无手动重投影
   - 确认 `on_loaded` 无 `init_commands` 调用

4. **最终验证**：
   - `cargo check --workspace` 全通过
   - `cargo test --workspace` 全通过
   - 手动运行 demo 验证四大优势：
     - **简洁易用**：无 7 个 RelayCommand 字段样板，菜单声明式注册
     - **声明式界面**：TabBar `<template each>` 迭代
     - **MVVM 数据驱动**：menus/cases/status/activities 全从 entries 投影
     - **自动 UI 刷新**：打开资源 → TabBar 自动新增 Tab；i18n 切换 → menus/status 自动刷新

## 实现顺序

1. Phase A（Tab/TabItem 修复）→ cargo check 验证
2. Phase B1（menu_commands.rs）→ 确认 as_command() API
3. Phase B2（MenuViewModel 重构）→ 确认 from_contribution
4. Phase B3（MainWindow 删除 RelayCommand）→ cargo check 验证
5. Phase C1（menus/status 改 computed）→ cargo check 验证
6. Phase C2（apply_switch_en 简化）+ C3（on_loaded 精简）
7. 最终 cargo check + cargo test 全验证
