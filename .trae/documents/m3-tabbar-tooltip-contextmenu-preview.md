# M3' TabBar 能力扩展：Tooltip + Context Menu + Preview + Promote

## Context

当前 TabBar/Tab/TabItem 组件缺少四个能力：关闭按钮无 tooltip、无右键菜单、无 preview 模式、无 promote 触发。本计划基于已确认的 4 项决策与 1 项纠正：

1. **Tooltip 来源**：仅框架默认 key `rml.tab.close`，不暴露属性
2. **Context Menu**：框架内置 Close/Close All/Close Others 三个标准项；业务通过 `on_context_menu` 回调追加扩展项
3. **Preview 状态**：`TabItem.preview: bool` 字段
4. **双击检测**：框架内自实现（250ms 时间窗口）

数据回路必须完备：内置菜单项的动作（Close/Close All/Close Others）必须能触发业务回调，业务通过修改 tabs 数据源驱动 UI 重渲。

## 架构原则

- **框架内置业务无关的标准能力**：标准菜单项文本走 i18n，行为通过 `on_close`/`on_close_all`/`on_close_others` 回调路由到业务
- **业务扩展通过回调注入**：`on_context_menu` 接收 `&mut PopupMenu`，业务往里追加任意项（Pin、Copy Path、Reveal in Finder 等）
- **状态归数据层**：preview 是 `TabItem.preview` 字段，框架不维护 preview slot 索引
- **零兼容性包袱**：直接修改现有 API，不加 deprecation

## 实施路线图

### M3'.1 Close button tooltip + i18n 默认 key

**目标**：关闭按钮 hover 显示 tooltip，文案随语言切换。

**改动文件**：
- `demo/assets/i18n/en-US.json`、`zh-CN.json`：新增 3 个 key
  - `rml.tab.close` → "Close" / "关闭"
  - `rml.tab.close_all` → "Close All" / "关闭全部"
  - `rml.tab.close_others` → "Close Others" / "关闭其他"
  - `rml.tab.promote` → "Promote" / "固定标签页"（M3'.5 用）
- `crates/ui/src/components/tab/tab.rs` L908-933 关闭按钮 div 链：
  - 仅 `!m`（非测量）分支加 `.tooltip(move |_, cx| Tooltip::new(cx.t("rml.tab.close")).build(window, cx))`
  - 不在 Tab 内 `observe_global::<I18nState>`：依赖 TabBar 父组件 observe（demo 已有先例 `i18n_case.rml.rs:44`）

**关键依赖**：`crates/ui/src/lib.rs` 已 re-export `Tooltip`；`ElementExt::tooltip` 在 `Stateful<Div>` 上可用。

**验证**：`cargo test -p rust-rml-ui`；启动 demo，hover tab 关闭按钮显示 tooltip，切换 en/zh 文案变化。

---

### M3'.2 on_close codegen 补全

**目标**：补全 RML 声明式 `<tab-bar on-close={handler}>` 的 codegen 缺口（底层 API 已存在）。

**改动文件**：
- `crates/engine/src/compiler/tab_bar/setters.rs:event_setter` (L120-134)：
  - 增加 `name == "on_close" && tag == "TabBar"` 分支
  - 生成模板与 `on_click` 一致：`.on_close(cx.listener(move |this, idx: &usize, _window, cx| { this.<method>(*idx, cx); }))`
- 增加单测 `event_setter_tab_bar_on_close`

**依赖**：与 M3'.1 并行。

**验证**：`cargo test -p rust-rml-engine event_setter_tab_bar_on_close`；demo `<tab-bar on-close={on_tab_close}>` 编译通过。

---

### M3'.3 Context menu（框架内置标准项 + 业务扩展）

**目标**：右键 tab 弹出菜单，包含 Close/Close All/Close Others 三个标准项 + 业务扩展项。

**核心设计**：
- 框架在 TabBar 内部生成 PopupMenu，加入三个标准项（i18n 文本 + 框架路由回调）
- 业务通过 `on_context_menu` 回调接收 `&mut PopupMenu`，追加任意扩展项
- 标准项与扩展项之间自动插入 separator

**新增 API**：
```rust
// TabBar
pub fn on_close_all<F: Fn(&mut Window, &mut App) + 'static>(self, f: F) -> Self
pub fn on_close_others<F: Fn(&usize, &mut Window, &mut App) + 'static>(self, f: F) -> Self
pub fn on_context_menu<F: Fn(&mut PopupMenu, &usize, &mut Window, &mut App) + 'static>(self, f: F) -> Self
```

**改动文件**：
- `crates/ui/src/components/tab/tab_bar.rs`：
  - L57 区域加字段 `on_close_all: Option<Rc<dyn Fn(&mut Window, &mut App)>>`、`on_close_others: Option<Rc<dyn Fn(&usize, &mut Window, &mut App)>>`、`on_context_menu: Option<Rc<dyn Fn(&mut PopupMenu, &usize, &mut Window, &mut App)>>`
  - L180 区域加 3 个 builder 方法
  - L630 区域透传：把三个回调包装为 `Fn(&ClickEvent, &mut Window, &mut App) -> Entity<PopupMenu>` 给 Tab，TabBar 负责生成标准菜单 + 调用业务回调追加
- `crates/ui/src/components/tab/tab.rs`：
  - L462 加 `context_menu_provider: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) -> Entity<PopupMenu>>>` 字段
  - L624 加 builder
  - render 链 L946 块内加 `.context_menu(move |menu, w, cx| provider(&ClickEvent::default(), w, cx))`
- `crates/engine/src/compiler/tab_bar/setters.rs:event_setter`：
  - 增加 `on_close_all`、`on_close_others`、`on_context_menu` 三个分支
  - `on_close_all` 签名 `Fn(&mut Window, &mut App)`（无 idx）—— 生成模板需调整
  - `on_context_menu` 签名特殊（接收 `&mut PopupMenu`），生成模板：
    ```
    .on_context_menu(cx.listener(move |this, menu: &mut PopupMenu, idx: &usize, window, cx| {
        this.<method>(menu, *idx, window, cx);
    }))
    ```

**标准菜单项生成逻辑**（在 TabBar render 内部，包装 provider 时）：
1. 创建 `PopupMenu::new(cx)`
2. `.item(PopupMenuItem::new(cx.t("rml.tab.close")).on_click(...))` — 触发 `on_close(&ix)`
3. `.item(PopupMenuItem::new(cx.t("rml.tab.close_others")).disabled(num_tabs <= 1).on_click(...))` — 触发 `on_close_others(&ix)`
4. `.item(PopupMenuItem::new(cx.t("rml.tab.close_all")).on_click(...))` — 触发 `on_close_all()`
5. 如果 `on_context_menu` 注册：`.separator()` + 调用业务回调 `(menu, &ix, w, cx)`
6. 返回 `menu`（Entity）

**风险点**：
- `context_menu` trait bound 需要 `InteractiveElement + ParentElement + Styled`，Tab 根 `self.base.id(self.ix)` 满足
- 现有 L940 `on_mouse_down(MouseButton::Left, ...)` 与右键 `MouseButton::Right` 不冲突
- 业务回调返回的 PopupMenu 必须是同一 Entity，不能在闭包里 new 新的——所以业务签名是 `Fn(&mut PopupMenu, ...)` 而非 `Fn(...) -> PopupMenu`

**依赖**：与 M3'.1/.2 并行。

**验证**：
- 单测 `event_setter_tab_bar_on_close_all`、`event_setter_tab_bar_on_close_others`、`event_setter_tab_bar_on_context_menu`
- demo 右键 tab 弹菜单显示三个标准项；切换 en/zh 文案变化；点击 Close 触发回调

---

### M3'.4 Preview 字段 + italic 视觉

**目标**：preview tab 显示 italic 标题，业务通过 `TabItem.preview` 字段控制。

**改动文件**：
- `crates/ui/src/components/tab/tab_item.rs`：
  - L26-37 加字段 `pub(super) preview: bool`
  - 加 builder `pub fn preview(mut self, preview: bool) -> Self { self.preview = preview; self }`
  - `into_header_tab()` (L119-138) 透传 `.preview(self.preview)`
- `crates/ui/src/components/tab/tab.rs`：
  - L450-472 加字段 `preview: bool`
  - 加 builder `pub fn preview(mut self, preview: bool) -> Self`
  - render label 分支（L796-808 区域）：`when(self.preview, |this| this.italic())`，仅 label 分支，icon/children 不变
- `crates/engine/src/compiler/tab_bar/setters.rs`：
  - `static_setter` 加 `"preview"` → `.preview(<bool>)`（参考 closable L39-46）
  - `bind_setter` 加 `"preview"` → `.preview(<expr>)`（参考 closable L84-89）

**依赖**：与 M3'.3 并行。

**验证**：单测 `bind_setter_tab_item_preview`；demo `<tab-item preview={is_preview}>` 文字斜体显示。

---

### M3'.5 on_promote + 双击检测

**目标**：双击 preview tab 触发 promote 回调。

**双击检测设计**：
- Tab 内部用 `window.use_keyed_state::<Option<Instant>>` 存上次点击时间，key 形如 `tab-dbl-{ix}`
- 提取纯函数 `fn is_double_click(prev: Option<Instant>, now: Instant) -> bool` 便于单测
- 现有 L940 `on_mouse_down(Left, stop_propagation)` 必须合并到新 handler（gpui 同事件不支持多 handler）

**新增 API**：
```rust
// TabBar
pub fn on_promote<F: Fn(&usize, &mut Window, &mut App) + 'static>(self, f: F) -> Self
// Tab 内部字段（不暴露 builder）
on_promote: Option<Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>>
```

**改动文件**：
- `crates/ui/src/components/tab/tab.rs`：
  - L450-472 加 `on_promote` 字段
  - render 链 L935-945 现有 `on_mouse_down` 块改造为：
    ```rust
    .on_mouse_down(MouseButton::Left, move |_, window, cx| {
        cx.stop_propagation();
        let now = Instant::now();
        let state = window.use_keyed_state::<Option<Instant>>(...);
        let prev = *state.read(cx);
        if is_double_click(prev, now) {
            if let Some(on_promote) = &on_promote { on_promote(&ClickEvent::default(), window, cx); }
            state.update(cx, |v, _| *v = None);
        } else {
            state.update(cx, |v, _| *v = Some(now));
        }
    })
    ```
- `crates/ui/src/components/tab/tab_bar.rs`：
  - 加 `on_promote` 字段+builder
  - L630 区域透传给 Tab
- `crates/engine/src/compiler/tab_bar/setters.rs:event_setter`：
  - 加 `on_promote` 分支，模板与 `on_close` 一致（`Fn(&usize, ...)`）

**风险点**：
- 三击误触发：双击后清空 `last_click_at`，第三击重新开始
- 与 `on_click` 的触发顺序：`on_mouse_down` 先于 `on_click`，双击检测不影响单击 `on_click` 行为
- 测量态 `m=true` 跳过双击检测（避免测量层副作用）

**依赖**：M3'.4 完成（demo 联动需 preview 视觉），可并行开发。

**验证**：单测 `is_double_click` 纯函数；demo 双击 preview tab 触发 promote 回调，标题取消斜体。

---

### M3'.6 Demo 集成

**目标**：完整演示 VSCode 风格的 tab 体验。

**改动文件**：
- 新建 `demo/src/cases/tab_preview_case.rml`（模块化，不堆积到现有 `tab_bar_case.rml`）
- `demo/src/cases/mod.rs`：注册新 case（order=63，紧随 popover case=62）
- demo ViewModel 加方法：
  - `on_tab_close(&mut self, idx: usize, cx)`
  - `on_tab_close_all(&mut self, cx)`
  - `on_tab_close_others(&mut self, idx: usize, cx)`
  - `on_tab_context_menu(&mut self, menu: &mut PopupMenu, idx: usize, w, cx)` — 追加 "Pin Tab" 项
  - `on_tab_promote(&mut self, idx: usize, cx)`
- demo 状态：
  - `tabs: Vec<TabData>`、`preview_index: Option<usize>`、`selected_index: usize`
  - promote 时 `tabs[idx].preview = false; preview_index = None`

**演示流程**：
1. 点击 sidebar 文件 → 在 preview slot 打开（italic 标题）
2. 双击 preview tab → promote（标题正常）
3. 右键 tab → 弹菜单（Close/Close All/Close Others + Pin Tab）
4. 切换 en/zh → 菜单文案变化

**依赖**：M3'.1–.5 全部完成。

---

## 依赖关系

```
M3'.1 ─┬─ M3'.6
M3'.2 ─┤
M3'.3 ─┤
M3'.4 ─┼─ M3'.5 ─┘
```

M3'.1/.2/.3/.4 完全并行；M3'.5 依赖 M3'.4（仅 demo 联动需）；M3'.6 依赖全部。

## 关键文件清单

| 文件 | 角色 | 涉及里程碑 |
|---|---|---|
| `crates/ui/src/components/tab/tab.rs` | Tab 组件 | M3'.1, M3'.3, M3'.4, M3'.5 |
| `crates/ui/src/components/tab/tab_bar.rs` | TabBar 组件 | M3'.3, M3'.5 |
| `crates/ui/src/components/tab/tab_item.rs` | 数据层 | M3'.4 |
| `crates/engine/src/compiler/tab_bar/setters.rs` | RML codegen | M3'.2, M3'.3, M3'.4, M3'.5 |
| `