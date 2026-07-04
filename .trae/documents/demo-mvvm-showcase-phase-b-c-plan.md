# Demo MVVM Showcase 重构计划 — Phase B + C

## 摘要

继续 Phase A（Tab/TabItem 类型修复，已完成）之后的重构工作，目标是消除 demo 中的冗余逻辑，
突出 RML 框架四大优势：**简洁易用、声明式界面设计、MVVM 数据驱动、自动化 UI 刷新**。

- **Phase B**：菜单改为贡献驱动，消除 7 个 RelayCommand 字段 + `init_commands` + `build_menu_tree` 样板
- **Phase C**：i18n 响应式重投影（`observe_global::<I18nState>` → 自动重建 menus/status），简化 `apply_switch_en`

---

## 当前状态分析

### 已完成（Phase A）
- `crates/engine/src/compiler/codegen/render.rs` — tab_window tabs slot 追加 `.into()`
- `crates/ui/src/window/tab_window.rs` — `tab_children` 签名恢复 `Vec<TabItem>`
- `demo/src/shell/main_window.rml.rs` — 通道桥接修复（`cx.spawn` + `AsyncApp::clone`）
- 模板 `<template slot="tabs" each={w in workbenches}>` 声明式 TabBar 迭代正常工作

### 待解决问题（冗余代码清单）

| # | 冗余区域 | 位置 | 行数 |
|---|---------|------|------|
| 1 | 7 个 RelayCommand 字段 | `main_window.rml.rs:51-57` | 7 行 |
| 2 | `init_commands` 初始化 7 个命令 | `main_window.rml.rs:174-198` | 24 行 |
| 3 | `build_menu_tree` 手工构建菜单树 | `main_window.rml.rs:289-333` | 45 行 |
| 4 | `apply_switch_en` 手动重投影 menus/status | `main_window.rml.rs:483-492` | 10 行 |
| 5 | MenuViewModel 旧结构（label/command/children） | `menu_view_model.rs` 全文 | 88 行 |
| 6 | `menu_commands.rs` 已创建但未集成 | 未加入 `mod.rs`，未编译 | 342 行 |

### 关键约束（Phase 1 探索确认）

1. **`#[computed]` 只能在 render 线程调用** — `ComputedCache::get_or_compute` 有 `debug_assert!(is_render_thread())`（`computed_cache.rs:131-136`）。因此 `menus()`/`status()` 不能作为 `#[computed]` 方法（`build_workbench` 等命令处理器需读 `cases`，不在 render 线程）。

2. **scanner 的 `uses_i18n` 检测局限** — 仅检测方法名为 `"t"` 的调用（`scanner.rs:468-473`），`t_static` 宏和 `c.name()` 不会触发。因此 `#[computed]` 方法无法自动依赖 `i18n_version`，不适配 i18n 响应式场景。

3. **`observe_global::<I18nState>` 是成熟模式** — `i18n_case.rml.rs:28` 已有示例，`I18nState` 是 pub struct（`i18n.rs:35`），`set_i18n` 调用 `update_global::<I18nState>` 触发观察者。

4. **`as_command()` 返回 `Option<&dyn ICommand>`** — 借用自底层 `Arc<dyn IContribution>`，闭包中需 clone Arc 后在闭包内重新调用 `as_command()`。

5. **`ensure_status_ready_registered()`** — 当前在 `init_commands` 末尾调用（`main_window.rml.rs:197`），删除 `init_commands` 后需迁移到 `on_loaded`。

---

## Phase B：菜单贡献驱动

### B1：集成 menu_commands.rs + 验证编译

**文件**：`demo/src/shell/mod.rs`

**改动**：在 `pub mod menu_view_model;` 前添加 `pub mod menu_commands;`

**验证**：`cargo check -p rust-rml-demo` — 预期 menu_commands.rs 内 13 个 `#[contribute]` 结构体编译通过（`IContribution`/`ICommand` 手写 impl 完整，`with_main_window` 辅助函数签名正确）。

若出现编译错误，常见修复：
- 导入路径：`use rml_app::IAppContextExt;` 已存在，确认 `IAppContextExt` trait 可见
- `CallContext` 字段访问：确认 `ctx.app` 字段为 `&mut App`
- `MainWindowRef` 可见性：`pub struct MainWindowRef(pub WeakEntity<MainWindow>)` 已 pub

### B2：重构 MenuViewModel 为贡献驱动

**文件**：`demo/src/shell/menu_view_model.rs`（全文重写）

**目标**：镜像 `StatusViewModel` 模式 — 从 `(IContribution, ContributionOptions)` 解包为类型化结构，按 `parent_id` 组织树。

**新结构**：
```rust
#[derive(Clone)]
pub struct MenuViewModel {
    pub id: SharedString,
    pub parent_id: Option<SharedString>,
    pub order: i32,
    contribution: Arc<dyn IContribution>,
    pub children: Vec<MenuViewModel>,
}

impl MenuViewModel {
    /// 动态标签 — 委托 contribution.name()，反映当前 locale
    pub fn label(&self) -> SharedString {
        self.contribution.name()
    }

    pub fn has_children(&self) -> bool {
        !self.children.is_empty()
    }

    /// 从贡献条目构造；非 menu 槽位返回 None
    pub fn from_contribution(c: Arc<dyn IContribution>, opts: ContributionOptions) -> Option<Self> {
        if opts.effective_slot() != Some("menu") {
            return None;
        }
        Some(Self {
            id: c.id().into(),
            parent_id: opts.parent_id,
            order: opts.order,
            contribution: c,
            children: Vec::new(),
        })
    }

    /// 从贡献条目列表构建菜单树（按 parent_id 组织，按 order 排序）
    pub fn build_menu_view_models(entries: &[ContribEntry]) -> Vec<MenuViewModel> {
        // 1. 过滤 menu 贡献 → 平铺 MenuViewModel 列表
        // 2. 按 id 建 HashMap<SharedString, MenuViewModel>
        // 3. 遍历：有 parent_id 则挂到父节点 children，否则为根
        // 4. 每层 children 按 order 排序
    }
}
```

**`build_popup_menu` 改动**：
- `item.label` → `item.label()`（方法调用）
- `item.command.clone()` → `item.contribution.clone()` + 闭包内 `contrib.as_command()`：
```rust
let contrib = item.contribution.clone();
if contrib.as_command().is_some() {
    pmi = pmi.on_click(move |_, window, app| {
        if let Some(cmd) = contrib.as_command() {
            let mut ctx = CallContext::new(window, app);
            if cmd.can_execute(&mut ctx) {
                cmd.execute(&mut ctx);
            }
        }
    });
}
```

**删除**：`root()`、`leaf()`、`child()` 构造方法（贡献驱动，不再手工构建）

**导入调整**：
- 新增：`use rml_core::contribution::{ContributionOptions, IContribution, CommandAbilityExt};`
- 新增：`use crate::shell::status_view_model::ContribEntry;`
- 移除：`use rml_core::command::{CallContext, ICommand};` 中的 `ICommand`（不再直接持有 `Arc<dyn ICommand>`）

### B3：MainWindow 清理

**文件**：`demo/src/shell/main_window.rml.rs`

**删除清单**：
1. **7 个 RelayCommand 字段**（L51-57）：
   ```rust
   open_welcome_command: Arc<dyn ICommand>,
   open_button_case_command: Arc<dyn ICommand>,
   // ... 5 more
   exit_command: Arc<dyn ICommand>,
   ```

2. **`init_commands` 方法**（L174-198）— 整个方法删除

3. **`build_menu_tree` 方法**（L289-333）— 整个方法删除

4. **`Default` impl 中的命令字段初始化**（L115-121）— 删除 7 行 `open_*_command: default_cmd.clone()`

5. **`on_loaded` 中的 `self.init_commands(cx);` 调用**（L152）

**修改清单**：

6. **`project_entries` 方法**（L265-285）— menus 改用 `build_menu_view_models`：
   ```rust
   fn project_entries(&mut self) {
       let entries = self.entries.read().unwrap();
       self.cases = entries.iter()
           .filter_map(|(c, o)| CaseViewModel::from_contribution(c.clone(), o.clone()))
           .collect();
       self.status = build_status_view_models(&entries);
       self.menus = MenuViewModel::build_menu_view_models(&entries);  // ← 替换 build_menu_tree()
       // ... activities 不变
   }
   ```

7. **`render_menu_bar` 方法**（L351-391）— 适配新 MenuViewModel：
   - `m.label` → `m.label()`
   - `m.command.clone()` → `m.contribution.clone()` + `as_command()` 模式（同 B2 的 `build_popup_menu`）

8. **`apply_switch_en` 方法**（L483-492）— 暂时保留手动重投影（Phase C 简化）：
   ```rust
   pub(crate) fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
       cx.set_i18n("en-US");
       self.rebuild_i18n_dependent();  // ← 提取为方法，Phase C 替换
       cx.notify();
   }
   ```

9. **`ensure_status_ready_registered()` 迁移** — 从 `init_commands` 迁到 `init_contribution_host` 末尾：
   ```rust
   fn init_contribution_host(&mut self, cx: &mut Context<Self>) {
       // ... existing ...
       rml_app::contribution::bootstrap_host_contributions(cx, Self::ID);
       crate::cases::status_bar_case::ensure_status_ready_registered();  // ← 迁移到此
   }
   ```

10. **导入清理**：
    - 移除：`use rml_core::command::{ICommand, RelayCommand};`（不再使用 RelayCommand；ICommand 改从 `rml_core::command` 按需导入）
    - 移除：`use rml_core::command::RelayCommand;`
    - 调整：`CommandAbilityExt` 导入（`render_menu_bar` 需 `as_command()`）

11. **文档注释更新**：
    - L40 注释 "菜单改用 RelayCommand 字段（WPF MVVM 模式）" → "菜单经贡献系统注册，menu_commands.rs 声明式定义"
    - L264 注释 "menus 不经贡献系统" → 删除

### B4：验证编译

```bash
cargo check -p rust-rml-demo
```

预期零错误。常见问题：
- `ICommand` 未导入（`render_menu_bar` 闭包中 `cmd.can_execute`/`cmd.execute` 需要 `ICommand` trait in scope）
- `CommandAbilityExt` 未导入（`as_command()` 方法需要 trait in scope）
- `ContribEntry` 类型别名需从 `status_view_model` 导入

---

## Phase C：i18n 响应式 + 清理

### C1：observe_global::<I18nState> 自动重投影

**文件**：`demo/src/shell/main_window.rml.rs`

**新增方法**：
```rust
/// 重建 i18n 依赖的 ViewModel 集合（menus + status）。
/// 由 observe_global::<I18nState> 在 locale 变化时自动调用。
fn rebuild_i18n_dependent(&mut self) {
    let entries = self.entries.read().unwrap();
    self.menus = MenuViewModel::build_menu_view_models(&entries);
    self.status = build_status_view_models(&entries);
}
```

**`on_loaded` 新增**（在 `init_panel_observers` 之后）：
```rust
/// observe i18n 状态变化 → 自动重建 menus/status + cx.notify
cx.observe_global::<rml_core::i18n::I18nState>(|this, cx| {
    this.rebuild_i18n_dependent();
    cx.notify();
})
.detach();
```

**导入新增**：
- `use rml_core::i18n::I18nState;`（或使用全路径 `rml_core::i18n::I18nState`）

### C2：简化 apply_switch_en

**改动**：
```rust
pub(crate) fn apply_switch_en(&mut self, cx: &mut Context<Self>) {
    cx.set_i18n("en-US");
    // observe_global::<I18nState> 自动触发 rebuild_i18n_dependent + cx.notify
}
```

从 10 行（手动重建 menus + status + notify）缩减为 1 行（`set_i18n`）。这是 "observe locale 自动重投影" 的核心展示。

### C3：on_loaded 清理 + 注释更新

**`on_loaded` 最终结构**（8 步，从 9 步缩减）：
```rust
fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    // 1. 通道桥接：ObservableVec → cx.notify
    let (tx, rx) = flume::unbounded();
    self.workbenches = ObservableVec::with_notify(tx);
    cx.spawn(|this: WeakEntity<MainWindow>, cx: &mut gpui::AsyncApp| {
        let mut cx = cx.clone();
        async move {
            while rx.recv_async().await.is_ok() {
                let _ = this.update(&mut cx, |_, cx| cx.notify());
            }
        }
    }).detach();

    self.init_contribution_host(cx);   // 2. 注册 host + bootstrap 贡献 + StatusReady
    self.project_entries();            // 3. 投影 cases/status/activities/menus
    self.init_services(cx);            // 4. MainWindowRef 单例
    self.init_lsp();                   // 5. LSP 子进程
    self.init_workbench(cx);           // 6. workbench 能力 + welcome tab
    self.init_activity_bar(cx);        // 7. ActivityBar + observe
    self.init_panel_observers(cx);     // 8. observe ActivityPanel/LspExplorerPanel
    self.init_i18n_observer(cx);       // 9. observe I18nState → 自动重投影

    cx.notify();
}
```

**新增 `init_i18n_observer` 方法**（封装 observe_global 逻辑，保持 on_loaded 简洁）：
```rust
/// observe i18n 状态变化 → 自动重建 menus/status（响应式重投影）。
fn init_i18n_observer(&mut self, cx: &mut Context<Self>) {
    cx.observe_global::<rml_core::i18n::I18nState>(|this, cx| {
        this.rebuild_i18n_dependent();
        cx.notify();
    })
    .detach();
}
```

**结构体文档注释更新**（L26-40）：
- 移除 "菜单改用 RelayCommand 字段" 注释
- 新增 "菜单/状态栏经贡献系统注册，observe I18nState 自动重投影" 说明

### C4：最终验证

```bash
cargo check -p rust-rml-demo
cargo run -p rust-rml-demo
```

运行时验证清单：
1. 窗口启动 → welcome tab 自动打开并激活
2. 点击案例树 → TabBar 新增 Tab 并激活
3. File > New → 打开 welcome tab（已打开则激活已有）
4. File > Open Button Case → 打开 button case tab
5. View > Toggle Theme → 主题切换
6. View > Switch to English → 菜单/状态栏文案切换为英文（observe 自动重投影）
7. Help > Help Center > Nested → 打开 dropdown menu case
8. Help > Features Group > Features → 打开 features case
9. 关闭 tab → 激活前一个 tab

---

## 假设与决策

### 已确认决策

1. **渲染范围**：菜单和状态栏保持命令式渲染（`render_menu_bar`/`render_status_bar` 方法），不改为 `each` 模式。原因：菜单有分组/子级，状态栏有 align 位置，内部逻辑复杂。each 模式留待后续规划。

2. **i18n 响应式**：采用 `observe_global::<I18nState>` + 手动重建 stored fields，不用 `#[computed]`。原因：`#[computed]` 有 render 线程限制 + scanner `uses_i18n` 检测局限（仅检测方法名 "t"），无法自动依赖 `i18n_version`。

3. **menus/status 保持 stored field**：不改为 `#[computed]` 方法。原因：`#[computed]` 只能在 render 线程调用，而 `build_workbench` 等命令处理器需读 `cases`（不在 render 线程）。menus/status 虽只在 render 时读，但 observe-based 重建已足够简洁。

4. **Phase B 不引入新宏**：`#[contribute]` 宏已有 `command`/`visual` flag + `parent_id`/`order`/`group` + 任意 `key="string"` 属性，无需扩展。

### 关键假设

1. `menu_commands.rs` 的 13 个结构体编译通过（`#[contribute]` 参数合法，`IContribution`/`ICommand` impl 完整）
2. `ContributionOptions::parent_id` 字段存在且类型为 `Option<SharedString>`（已确认 `contribution.rs:25`）
3. `CommandAbilityExt::as_command()` 在 `dyn IContribution` 上可用（已确认 `command.rs:89-94`）
4. `observe_global::<I18nState>` 回调签名 `Fn(&mut T, &mut Context<T>)`（已确认 `i18n_case.rml.rs:28` 模式可行）
5. `build_menu_view_models` 的树构建算法：平铺 → 按 id 建表 → 按 parent_id 挂载 → 排序

---

## 影响范围

| 文件 | 改动类型 | 预估行数变化 |
|------|---------|-------------|
| `demo/src/shell/mod.rs` | 新增 1 行 | +1 |
| `demo/src/shell/menu_commands.rs` | 无改动（已创建） | 0 |
| `demo/src/shell/menu_view_model.rs` | 全文重写 | ~88 → ~110 |
| `demo/src/shell/main_window.rml.rs` | 删除 + 修改 | -80, +30 |

**净效果**：MainWindow 从 ~569 行缩减至 ~519 行，消除全部 RelayCommand 样板，菜单/状态栏完全贡献驱动 + i18n 响应式。
