# Plan: Tab `closable` + 关闭按钮 hover 显示 + Tab 关闭事件 & 案例视图垂直滚动

## Summary

为 RML 的 `<Tab>` 组件新增 `closable` 布尔属性（支持静态设置与数据绑定），当开启时自动在标签尾部渲染关闭按钮（X 图标），仅在鼠标悬停该 Tab 时显示，并停止点击事件冒泡以避免误触 tab 切换。同时在 `<tab-window>` Shell 层新增 `on-tab-close` 事件，对称于已有 `on-tab-click`，便于宿主在数据集中移除对应项并同步 UI。最后为 `MainWindow::active_view` 返回的案例视图增加垂直滚动容器，解决案例页过长看不全的问题。

---

## Current State Analysis

### 1. Tab / TabBar 现状
- **`crates/ui/src/components/tab/tab.rs`**（`Tab` 运行时结构）：当前只有 `label / icon / prefix / suffix / disabled / selected / on_click` 等字段；无 `closable`、无 `on_close`。
- **`crates/ui/src/components/tab/tab_bar.rs`**（`TabBar` 运行时结构）：只有一个事件 `on_click: Option<TabBarClickHandler>`（签名 `Fn(&usize, &mut Window, &mut App)`），在 `render()` 中通过 `tab.on_click(move |_, window, cx| on_click(&ix, window, cx))` 逐 tab 装配。
- **`crates/ui/src/components/tab/tab_item.rs`**（`TabItem`，WPF 模式 title+body）：仅 `title / title_icon / disabled / on_click`，无 `closable`。
- **`crates/ui/src/window/tab_window.rs`**（`TabWindowShell`）：内部构造 `TabBar` 并通过 `on_tab_click` builder → 转发到 `TabBar::on_click`（render 中 line 499-501）；无 close 相关字段。

### 2. RML 代码生成 / Props 注册现状
- **`crates/engine/src/compiler/props_registry.rs`** 是属性白名单（line 89-101 Tab / TabBar / TabItem；line 177-182 tab-window SHELL_PROPS）。
- **`crates/engine/src/compiler/tab_bar/setters.rs`** 三类 setter：`static_setter`（含 `menu` 等 bool）、`bind_setter`（含 `disabled` 等）、`event_setter`（含 `on_click` TabBar 事件）。
- **`crates/engine/src/compiler/codegen/shell.rs`** line 336-350 已有 `on_tab_click` 事件 codegen arm（生成 `.on_tab_click({ let weak = cx.weak_entity(); move |index, _window, app| { ... this.<method>(index, cx); } })`），可作为 `on_tab_close` 的镜像模板。
- **`crates/engine/src/parser/mod.rs`** line 345-352 `normalize_attr_name`：将 `kebab-case` 转 `snake_case`，所以 `closable`、`on-close`、`on-tab-close` 自动映射到 Rust 字段名。
- **`crates/engine/src/compiler/validator.rs`** line 156-170：Shell 根标签上未注册的 `Attribute::Event` 会被拒绝，需把 `on_tab_close` 加入 `SHELL_PROPS` 才能通过校验。

### 3. `<component>` 透明包装 & active_view
- **`crates/engine/src/compiler/codegen/node.rs`** line 93-143：`<component content={...}>` 是透明包装，codegen 直接吐出 `content` 表达式，**不包任何 div、不带 overflow**。
- **`demo/src/shell/main_window.rml.rs`** line 265-274 `active_view()`：直接返回 `visual.render(window, cx)`，无外层滚动容器。
- **`gpui` API**：`.overflow_y_scroll()` 是标准方法；本项目目前仅在 `tab_bar.rs` line 521 用过 `overflow_x_scroll`，未使用 y 方向滚动。

### 4. Hover 显示 / 关闭按钮可参考模式
- `tab.rs` line 752、790-800 已大量使用 `.hover(|this| ...)` 调整样式，但这是元素自身 hover，不能直接驱动子元素显隐。
- `tab_bar.rs` line 485 已调用 `.group("tab-bar")`，说明 `group()` 是项目接受的模式。
- `tab_window.rs` line 43-97 `control_button` 是关闭按钮的视觉模板（`Icon::new(icon).small()` + hover bg + `cx.stop_propagation()`）；line 87 的 `cx.stop_propagation()` 模式必须复用，避免关闭按钮误触发 tab 自身的 `on_click`。
- `IconName::Close` 在 `crates/ui/src/components/activity_bar/icon.rs` line 100 已确认存在。
- `group_hover` 在本仓库内零使用，需在实现阶段通过查看 `gpui-component` 依赖（`~/.cargo/git/checkouts/`）确认 API 形态；若不可用则回退到「关闭按钮始终渲染但 `opacity(0)` + 在父 Tab hover 时通过 style refinement 显式覆盖到 opacity(1)」的等价方案（见下方 Implementation Notes）。

### 5. Demo 数据流与移除模式
- `MainWindow.workbenches: ObservableVec<Arc<dyn IWorkbench>>`，`ObservableVec::remove_where` 会自动 bump 版本（`crates/core/src/observable.rs` line 53-62），从而触发 `main_window.rml.rs:122-130` 的后台任务 `cx.notify()` 重渲染。
- 现成 `IWorkbenchManager::close(&self, uri: &Uri)`（main_window.rml.rs line 470-477）已实现「移除 + 若是当前激活项则切到首个剩余项」逻辑。新 `on_tab_close` 命令只需调用它并 bump `activated` 版本（`close` 内部只写 `activated` 不 bump）。

---

## Proposed Changes

### Part A — Tab `closable` + 关闭按钮 + `on-tab-close` 事件链

#### A1. `crates/ui/src/components/tab/tab.rs` — Tab 运行时
- 在 `Tab` 结构（line 437-458）末尾新增 `pub(super) closable: bool` 字段。
- `new()` 中初始化为 `false`（line ~470 附近）。
- 新增 builder：`pub fn closable(mut self, closable: bool) -> Self { self.closable = closable; self }`。
- `render()`（line 645-834）：当 `self.closable && !self.disabled` 时，构造关闭按钮元素并追加为 `suffix`（保留用户已有 suffix 时合入 h_flex 末尾）。关闭按钮规格：
  - `Icon::new(IconName::Close).xsmall()`（≈14px，适配 32px Tab 高度）。
  - 外层 `div().id(("tab-close", self.ix))`，加 `.cursor_pointer()`、`px_1()`、`opacity_0()` 默认隐藏。
  - 通过 `group(format!("tab-{}", self.ix))` 在 `self.base` + `group_hover` 在按钮外层实现「父 Tab hover 才显示」。
  - `.on_click(move |_ev, window, cx| { cx.stop_propagation(); (on_close)(*ix, window, cx); })`，需要 `Tab` 持有 `on_close: Option<TabClickHandler>`（同 `on_click` 字段类型 `Rc<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>`，但 close 实际只用到 `&mut Window, &mut App`；为最小改动复用 `TabClickHandler` 类型）。
- 在 `Tab` 结构新增 `pub(super) on_close: Option<TabClickHandler>` 字段 + `pub fn on_close<F>(mut self, f: F) -> Self where F: Fn(&ClickEvent, &mut Window, &mut App) + 'static` builder（镜像 line 513-522 的 `on_click`）。
- **Fallback**：若 `group_hover` 在 gpui-component 中不可用，则改为在按钮外层用 `.hover(|style| style.opacity(1.0))` —— 注意此为按钮自身 hover 才显示，略弱于父 Tab hover；并加注释 TODO 待 group_hover 确认后切换。

#### A2. `crates/ui/src/components/tab/tab_item.rs` — TabItem 传递 closable
- 在 `TabItem` 结构（line 26-35）新增 `closable: bool` 字段 + `closable(bool)` builder。
- `into_header_tab`（line 111-129）在构造 `Tab` 时把 `self.closable` 透传：`.closable(self.closable)`。

#### A3. `crates/ui/src/components/tab/tab_bar.rs` — TabBar 暴露 `on_close`
- 在 `TabBar` 结构（line 41-55）新增 `on_close: Option<TabBarClickHandler>` 字段（同 `on_click` 类型 `Rc<dyn Fn(&usize, &mut Window, &mut App)>`）。
- `new()` 初始化 `None`。
- 新增 builder `pub fn on_close<F>(mut self, f: F) -> Self where F: Fn(&usize, &mut Window, &mut App) + 'static { self.on_close = Some(Rc::new(f)); self }`（镜像 line 165-171 `on_click`）。
- 在 `render()`（line 540-577 装配每个 Tab 处）：若 `self.on_close.is_some()`，对每个 tab 调用 `tab.on_close(move |_, window, cx| on_close(&ix, window, cx))`，与现有 `tab.on_click(...)` 装配方式完全对称（line 560-562）。
- 注意：`Tab` 的 `closable` 由 `TabItem` 透传，`on_close` 由 `TabBar` 透传，两者解耦 —— 即使某个 TabItem 未设 closable，TabBar 设了 on_close 也不会出现关闭按钮（因为 Tab.closable=false 时不渲染按钮）。

#### A4. `crates/ui/src/window/tab_window.rs` — TabWindowShell 暴露 `on_tab_close`
- 在 `TabWindowShell` 结构（line 142-166）新增 `on_tab_close: Option<TabClickHandler>` 字段（与 `on_tab_click` 同类型，line 156 旁）。
- `new()` 初始化 `None`（line 179 附近）。
- 新增 builder `pub fn on_tab_close(mut self, f: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self`（镜像 line 245-251 `on_tab_click`）。
- `render()`（line 499-501 附近）在转发 `on_tab_click` 之后追加：
  ```rust
  if let Some(on_close) = self.on_tab_close {
      tab_bar = tab_bar.on_close(move |ix, window, cx| on_close(*ix, window, cx));
  }
  ```

#### A5. `crates/engine/src/compiler/props_registry.rs` — 注册新属性
- `Tab` 行（line 96-99）：在 `"on_click"` 之后追加 `"closable"`。
- `TabItem` 行（line 101）：在 `"on_click"` 之后追加 `"closable"`。
- `tab-window` SHELL_PROPS 行（line 177-182）：在 `"on_tab_click"` 之后追加 `"on_tab_close"`。
- 若 props_registry.rs 末尾有 parity 测试（line 286-340 附近），同步加入测试断言，避免回归。

#### A6. `crates/engine/src/compiler/tab_bar/setters.rs` — Tab / TabItem 的 closable setter
- `static_setter`（line 21-43）：新增 arm
  ```rust
  "closable" => {
      let bool_val = if value.is_empty() || value.eq_ignore_ascii_case("true") { "true" } else { "false" };
      Some(format!(".closable({})", bool_val))
  }
  ```
  （镜像 `menu` 的 bool 处理，line 30-37）。
- `bind_setter`（line 51-85）：新增 arm `"closable" => Some(format!(".closable({})", rust_expr))`（镜像 `menu` bind arm）。
- 注意：`closable` 对 `Tab` 与 `TabItem` 都生效，两个 tag 共用同一 setter arm（无需 if tag ==）。

#### A7. `crates/engine/src/compiler/codegen/shell.rs` — tab-window 的 on_tab_close 事件 codegen
- 在 `on_tab_click` arm（line 336-350）之后追加 `on_tab_close` arm，模板完全一致（仅方法名 `.on_tab_click` → `.on_tab_close`）：
  ```rust
  Attribute::Event { name, handler } if name == "on_tab_close" => {
      let method = match handler {
          EventHandler::Ident(m) | EventHandler::MethodName(m) => m.as_str(),
          EventHandler::WithArgs(m, _) => m.as_str(),
      };
      code.push_str(&format!(
          ".on_tab_close({{\n                    \
           let weak = cx.weak_entity();\n                    \
           move |index: usize, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
           if let Some(entity) = weak.upgrade() {{\n                            \
           entity.update(app, |this, cx| {{ this.{}(index, cx); }});\n                        \
           }}\n                    }}\n                }})",
          method
      ));
  }
  ```

#### A8. `demo/src/shell/main_window.rml` — 启用 closable + 绑定 close 事件
- Line 8：在 `on-tab-click="on_tab_click"` 之后追加 `on-tab-close="on_tab_close"`。
- Line 14：`<Tab label={w.name()} />` 改为 `<Tab label={w.name()} closable />`（所有 workbench tab 默认允许关闭；后续如需差异化可用 `closable={w.closable()}` 绑定）。

#### A9. `demo/src/shell/main_window.rml.rs` — 实现 on_tab_close 命令
- 在 `on_tab_click`（line 375-380）之后新增：
  ```rust
  #[command]
  pub fn on_tab_close(&mut self, index: usize, cx: &mut Context<Self>) {
      let wb = self.workbenches.get(index);
      if let Some(wb) = wb {
          let uri = wb.uri().to_string();
          IWorkbenchManager::close(self, &uri.parse().unwrap());
          self.__rml_bump_version("activated");
          cx.notify();
      }
  }
  ```
- 说明：`IWorkbenchManager::close` 已实现 `remove_where` + 自动重定向 `activated` 到首个剩余项（main_window.rml.rs line 470-477），但只 bump `workbenches` 版本不 bump `activated`，所以这里手动 bump 一次。
- 错误处理：`uri.parse().unwrap()` 在 demo 中可接受（Uri 来自系统）；若需更稳健可改 `uri.parse().expect("valid uri")` 或返回 `Result`，但为保持与现有 `on_tab_click` 简洁度一致，沿用 `unwrap`。

---

### Part B — 案例视图垂直滚动条

#### B1. `demo/src/shell/main_window.rml.rs` — `active_view` 包装滚动容器
- 修改 `active_view`（line 265-274），把 `visual.render(window, cx)` 用 `gpui::v_flex().size_full().min_h_0().overflow_y_scroll()` 包裹：
  ```rust
  pub fn active_view(&self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
      let activated = self.activated.read().unwrap().clone();
      if let Some(wb) = activated {
          let iv: &dyn IContribution = wb.as_ref();
          if let Some(visual) = iv.as_visual() {
              return gpui::v_flex()
                  .size_full()
                  .min_h_0()
                  .overflow_y_scroll()
                  .child(visual.render(window, cx))
                  .into_any_element();
      }
      gpui::div().into_any_element()
  }
  ```
- **不动 `<component>` 框架语义**：`<component content={...}>` 是透明包装（node.rs line 93-143），改它会影响所有使用者。仅在 demo 的 `active_view` 出口包裹是最小外科改动。
- `min_h_0()` 必加：保证 flex 子项能正确收缩从而触发 `overflow_y_scroll` 生效（gpui flex 默认 min-height auto 会撑爆容器）。
- `size_full()` 必加：让滚动容器填满 tab-window 主体区域，否则容器高度随内容增长，不会出现滚动条。

---

## Assumptions & Decisions

1. **`on-tab-close` 走 Shell 顶层属性路径**（而非 `<TabBar on-close>` 或 `<Tab on-close>`）：因为用户的 RML 直接写 `<tab-window>`，TabBar 由 shell 内部构造，slot 模板无法注入事件。与 `on-tab-click` 完全对称，最符合现有架构。
2. **`closable` 放在 `<Tab>` 与 `<TabItem>` 层**：每个 tab 独立控制是否可关闭（用户原话「允许设置或绑定是否允许关闭」），与 `disabled` 字段语义一致。TabBar 的 `on_close` 与之解耦：只有 `closable=true` 的 Tab 才渲染按钮，按钮触发后调用 TabBar 装配的 `on_close(&ix, ...)`。
3. **关闭按钮显隐用 `group` + `group_hover`**：父 Tab 加 `group("tab-{ix}")`，按钮外层加 `group_hover(|s| s.opacity(1.0)).opacity(0.0)`。`group()` 在本项目 `tab_bar.rs:485` 已是接受模式。若实现时确认 `group_hover` 在 gpui-component 依赖中不可用，回退方案见 A1 Fallback。
4. **`cx.stop_propagation()` 必须复用**：避免关闭按钮 click 同时触发 Tab 自身 `on_click`（即关闭时不应顺便切到该 tab）。模式来自 `tab_window.rs:87`。
5. **`active_view` 的滚动包装放在 demo 侧**：不改 `<component>` 框架语义。后续若多个 Shell 使用 `<component>` 都需要滚动，再考虑给 `<component>` 加可选 `scroll-y` 属性 —— 当前需求只此一处，避免过度设计（CLAUDE.md §2 简洁性优先）。
6. **错误处理沿用 demo 现有风格**：`unwrap` / `expect` 与 main_window.rml.rs 现有 `panic!`-less 但 `unwrap` 风格一致（`activated.read().unwrap()` 等）。不引入 `Result` 改造。
7. **`IconName::Close` 而非 `IconName::WindowClose`**：`Close` 是通用关闭图标（已用于 ActivityBar），`WindowClose` 是窗口关闭专用（红色三态），语义不符。

---

## Verification Steps

### 编译验证
1. `cargo build -p rml_ui` —— 验证 Tab / TabBar / TabItem / TabWindowShell 改动编译通过。
2. `cargo build -p rml_engine` —— 验证 props_registry / setters / shell.rs codegen 改动编译通过。
3. `cargo build -p rml_demo` —— 验证 demo 的 `on_tab_close` 命令与 `active_view` 改动编译通过。

### 单测 / Parity 验证
4. `cargo test -p rml_engine -- component_props_tags_align_with_routing_table` —— 验证新增的 `closable` / `on_tab_close` 与路由表对齐（props_registry.rs line 372 测试）。
5. 若在 props_registry.rs 增加了对应测试断言（A5），运行该测试。

### 运行时手测
6. `cargo run -p rml_demo`，在主窗口：
   - 6.1 鼠标移入某个 Tab：右侧出现 X 关闭按钮；移出消失。
   - 6.2 点击 X：对应 Tab 被移除；若关闭的是当前激活 Tab，自动切到首个剩余 Tab；若关闭最后一个 Tab，主体显示空白。
   - 6.3 点击 X 时不会同时触发该 Tab 的选中（即 `stop_propagation` 生效）。
   - 6.4 切换到任一案例（如 ButtonCase / FormCase），主体区域出现垂直滚动条，可上下滚动查看完整内容；窗口缩放时滚动条自适应。
7. 边界：所有 Tab 关闭后，主体应保持空白不崩溃；再触发新增 workbench（如打开 ActivityBar 中其他 case）应正常恢复。

### 回归验证
8. 切换 `show_chrome`、调整 `left-size`、tab 选中切换、ActivityBar 切 case 等既有路径仍正常（未被 closable / on_close 字段引入的副作用破坏）。
9. 案例视图原有内联滚动（若 case 内部有自己的 ScrollHandle）应仍正常 —— 外层 `overflow_y_scroll` 不应嵌套冲突（min_h_0 + size_full 保证外层先收缩，内层 ScrollHandle 仍可独立工作）。

---

## Files Touched

| File | Change |
|---|---|
| `crates/ui/src/components/tab/tab.rs` | Tab 加 `closable` / `on_close` 字段 + builder + 渲染关闭按钮 |
| `crates/ui/src/components/tab/tab_item.rs` | TabItem 加 `closable` 字段 + 透传到 `into_header_tab` |
| `crates/ui/src/components/tab/tab_bar.rs` | TabBar 加 `on_close` 字段 + builder + render 装配 |
| `crates/ui/src/window/tab_window.rs` | TabWindowShell 加 `on_tab_close` 字段 + builder + render 转发 |
| `crates/engine/src/compiler/props_registry.rs` | Tab / TabItem 加 `closable`；tab-window 加 `on_tab_close` |
| `crates/engine/src/compiler/tab_bar/setters.rs` | `static_setter` / `bind_setter` 加 `closable` arm |
| `crates/engine/src/compiler/codegen/shell.rs` | 加 `on_tab_close` event codegen arm |
| `demo/src/shell/main_window.rml` | line 8 加 `on-tab-close`；line 14 Tab 加 `closable` |
| `demo/src/shell/main_window.rml.rs` | 加 `on_tab_close` 命令；`active_view` 包 `overflow_y_scroll` |

**未触碰**：`<component>` 框架 codegen（node.rs）、validator.rs（自动通过 props_registry 校验）、`ObservableVec` / `IWorkbenchManager` 既有 close 逻辑（已满足需求）。
