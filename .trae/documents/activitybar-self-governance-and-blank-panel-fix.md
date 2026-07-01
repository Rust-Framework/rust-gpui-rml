# ActivityBar 自治修复 + 案例库面板空白修复计划

## 问题概述

1. **Issue #1 — ActivityBar 组件封装没有自治**：点击活动栏图标按钮不会自行处理激活状态切换和展开面板显隐。
2. **Issue #2 — demo 中案例库活动栏展开面板永远空白**：点击「案例库」图标后，展开面板区域无任何内容渲染。

---

## 根因分析（结合调用链路 + GPUI 原理）

### Issue #1 根因：受控模式激活导致内部状态被旁路

**调用链路**：
```
main_window.rml: <ActivityBar ref="activity-bar" panels={activity_panels} active_panel_id={active_panel_id} on_panel_change="on_panel_change" />
  ↓ engine codegen (component.rs:173-177)
  仅当 panels + active_panel_id 同时绑定时，注入 .panel_body(resolve_active_panel_body(...))
  ↓ ActivityBar::new(...).panels(...).active_panel_id(Some(...)).panel_body(...)
  ↓ activity_bar.rs:235  controlled = self.active_panel_id.is_some()  → true
  ↓ activity_bar.rs:269  if !controlled { ... }  → 跳过内部 BarState 更新
  ↓ 点击回调仅触发 on_panel_change，状态往返依赖 Host
```

**根因**：RML 绑定 `active_panel_id={active_panel_id}` 使 `ActivityBar.active_panel_id = Some(...)`（[activity_bar.rs:235](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L235)），激活受控模式。受控模式下 [activity_bar.rs:269](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L269) 的 `if !controlled` 守卫阻止内部 `BarState` 在点击时更新，激活态切换完全依赖 Host 的 `on_panel_change` 回调往返。这违背了模块文档注释「自治理：激活态 + 面板内容」的设计目标。

**修复方向**：移除 RML 中的受控绑定，使 ActivityBar 回到非受控模式，由内部 `BarState` 自治管理激活态。codegen 在 `active_panel_id` 未绑定时不会注入 `.panel_body(...)`，ActivityBar 回退到 `panel.panel(window, cx)` 路径（[activity_bar.rs:317-321](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L317-L321)）。

### Issue #2 根因：Entity 创建上下文丢失父级链接

**调用链路**：
```
MainWindow::render (首次)
  ↓ __rml_loaded guard → MainWindow::on_loaded
  ↓ refresh_bindings → 填充 activity_panels
  ↓ ActivityBar 渲染 → panel.panel(window, cx)
  ↓ resolve_active_panel_body (activity_panel.rs:76-88)
  ↓ cx.borrow_mut() → &mut App
  ↓ render_contribution_visual (render.rs:9-25)
  ↓ cx.update_global::<ContributionRegistryGlobal, _>(|global, cx: &mut App| {...})
  ↓ visual(&mut ctx, cache) → cache.render_view("samples", CaseActivityPanel::default(), ctx)
  ↓ contribution_cache.rs:36/44  ctx.cx.new(|_| view)  ← App::new，创建根 Entity（无父级链接）
  ↓ 返回 div().child(entity)
  ↓ GPUI 渲染 CaseActivityPanel entity
  ↓ __rml_loaded guard → CaseActivityPanel::on_loaded
  ↓ refresh_tree → map_case_tree_items(MainWindow::ID, cx)  ← 读取贡献条目
  ↓ subscribe_host_changes / cx.observe_global  ← 订阅注册
```

**根因**：`render_contribution_visual` 在 `cx.update_global` 闭包内创建 Entity（[render.rs:13](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/render.rs#L13)），闭包参数为 `&mut App`。`ComponentEntityCacheImpl::render_view` 的缓存未命中分支调用 `ctx.cx.new(|_| view)`（[contribution_cache.rs:36,44](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution_cache.rs#L36)），这是 `App::new`，创建的是**根 Entity**，与 `MainWindow` 没有父-子观察链接。

旧代码（commit `69fbc42` 之前）在 `MainWindow::on_loaded` 中通过 `Context::<MainWindow>::new()` 创建 `CaseActivityPanel`，建立父-子链接，注释明确写道「可靠路径，避免 visual 缓存时序问题」。改为延迟缓存创建后，根 Entity 丢失父级链接，导致：
- `subscribe_host_changes` / `cx.observe_global` 的订阅通知无法正确冒泡到父窗口触发重绘
- 面板内容渲染时机异常，Tree 数据虽加载但 UI 不刷新

**修复方向**：在 `MainWindow::on_loaded` 中通过 `Context::<MainWindow>::new()` 预创建 `CaseActivityPanel` Entity（有父级链接），并通过 `pre_register` 注入 `ComponentEntityCacheImpl`。后续 `render_view` 命中缓存直接返回预注册 Entity，跳过 `App::new` 路径，恢复父-子观察链接。

---

## 修复有效性论证

### Issue #1 修复有效性
- 移除 `active_panel_id` 绑定后，codegen（[component.rs:173-177](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L173-L177)）的 `if let (Some(panels), Some(active)) = (...)` 条件不满足，不再注入 `.panel_body(...)`
- `ActivityBar.active_panel_id` 保持 `None`，`controlled = false`
- 点击时 `if !controlled` 分支执行，内部 `BarState` 自行切换 `active`，`window.refresh()` 触发重绘
- 激活面板内容通过 `panel.panel(window, cx)` 回退路径获取（[activity_bar.rs:317-321](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar.rs#L317-L321)）

### Issue #2 修复有效性
- `pre_register`（[contribution.rs:114-118](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs#L114-L118) + [contribution_cache.rs:58-67](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution_cache.rs#L58-L67)）将 Entity 插入 `entries` map
- `render_view`（[contribution_cache.rs:32](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution_cache.rs#L32)）首查 `self.entries.get(contribution_id)`，命中预注册 Entity 直接返回，跳过 `App::new`
- 预注册 Entity 由 `Context::<MainWindow>::new()` 创建，具有父-子链接，订阅通知正常冒泡
- `CaseActivityPanel::on_loaded` 中的 `subscribe_host_changes` / `cx.observe_global` 正常工作，Tree 数据变更触发重绘

---

## 当前状态

| 任务 | 状态 | 文件 |
|------|------|------|
| Task 1: 移除 RML 受控绑定 | ✅ 已完成 | [main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml#L14) |
| Task 3: 添加 `pre_register` 方法 | ✅ 已完成 | [contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs#L114-L118) + [contribution_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution_cache.rs#L58-L67) |
| Task 5: 清理 render 帧副作用 | ✅ 已完成 | [case_activity_panel.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_activity_panel.rml.rs) |
| Task 2: 移除 Host 受控状态 | ⏳ 待执行 | main_window.rml.rs |
| Task 4: 预注册 CaseActivityPanel | ⏳ 待执行 | main_window.rml.rs |
| Task 6: 构建验证 | ⏳ 待执行 | - |

---

## 待执行变更

### Task 2: 移除 `main_window.rml.rs` 中的受控状态

**文件**: [main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

1. **移除 `active_panel_id` 字段**（[第 29 行](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L29)）：
   ```rust
   // 删除：
   active_panel_id: Option<gpui::SharedString>,
   ```

2. **移除 `refresh_bindings` 中的播种块**（[第 122-124 行](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L122-L124)）：
   ```rust
   // 删除：
   if self.active_panel_id.is_none() {
       self.active_panel_id = self.activity_panels.first().map(|p| p.id());
   }
   ```

3. **移除 `on_panel_change` 方法**（[第 141-149 行](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L141-L149)）：
   ```rust
   // 删除整个方法：
   #[command]
   pub fn on_panel_change(&mut self, panel_id: &gpui::SharedString, cx: &mut Context<Self>) {
       ...
   }
   ```

### Task 4: 预注册 CaseActivityPanel Entity

**文件**: [main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

1. **添加导入**（文件顶部，已部分完成）：
   ```rust
   use rml_app::contribution::ContributionRegistryGlobal;
   ```
   注：`CaseActivityPanel` 导入已存在（[第 12 行](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L12)）。

2. **在 `on_loaded` 中添加预注册**（[第 107 行 `self.refresh_bindings(cx);` 之前](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L107)）：
   ```rust
   let panel = cx.new(|_| CaseActivityPanel::default());
   cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
       global.0.entity_cache_mut().pre_register("samples", panel);
   });
   ```

   - `cx.new(|_| ...)` 使用 `Context::<MainWindow>::new()` 创建子 Entity（有父级链接）
   - `cx.update_global` 访问 `ContributionRegistryGlobal`，调用 `entity_cache_mut()` 获取 `&mut ComponentEntityCacheImpl`
   - `pre_register("samples", panel)` 将 Entity 注入缓存，contribution id `"samples"` 与 `#[contribute(id = "samples", ...)]` 一致
   - 后续 `render_view("samples", ...)` 命中缓存，直接返回预注册 Entity

### Task 6: 构建验证

1. `cargo build -p rust-rml-demo` — 编译 demo
2. 检查生成代码（`target/.../main_window.rs` 或 build 产物）确认无 `.panel_body(resolve_active_panel_body(...))` 注入
3. `cargo test --workspace` — 全量测试
4. `cargo run -p rust-rml-demo` — 视觉验证：
   - 点击活动栏「案例库」图标，面板展开显示 Tree
   - 再次点击，面板收起
   - 切换其他面板图标，激活态正确切换

---

## 假设与决策

1. **contribution id `"samples"`**：与 `CaseActivityPanel` 的 `#[contribute(id = "samples", ...)]` 一致，硬编码在预注册代码中。若未来 id 变更需同步。
2. **预注册时机**：在 `on_loaded` 中 `refresh_bindings` 之前执行，确保首次 ActivityBar 渲染时缓存已就绪。
3. **不修改 `render_view` 默认路径**：保留 `App::new` 作为未预注册场景的回退，仅对需要父级链接的组件预注册。
4. **`window.refresh()` 已在 Task 5 清理**：`case_activity_panel.rml.rs` 的 `on_loaded` 已移除 `window.refresh()`，避免渲染帧内重绘副作用。
