# Plan: 完成 Tab closable + 垂直滚动条收尾（修复编译错误）

## Summary

上一轮 `/plan`（`tab-closable-and-vertical-scroll.md`）已批准并完成绝大多数实现：Tab `closable` / `on_close` 字段、TabBar `on_close`、TabWindowShell `on_tab_close`、props_registry / setters / shell codegen、demo `on_tab_close` 命令、`main_window.rml` 启用 `closable` + `on-tab-close` 均已落地。唯一阻塞点是 Part B `active_view` 的 `overflow_y_scroll` 编译失败。本计划聚焦于完成这一收尾工作，并采用更贴合用户原意（"垂直滚动条设置显示"）的方案。

---

## Current State Analysis

### 已完成（无需再动）

| 文件 | 已落地改动 |
|---|---|
| `crates/ui/src/components/tab/tab.rs` | `closable` / `on_close` 字段 + builder + `group`/`group_hover` 渲染关闭按钮（hover 显示 + `stop_propagation`） |
| `crates/ui/src/components/tab/tab_item.rs` | `closable` 字段 + `into_header_tab` 透传 |
| `crates/ui/src/components/tab/tab_bar.rs` | `on_close` 字段 + builder + render 装配 |
| `crates/ui/src/window/tab_window.rs` | `on_tab_close` 字段 + builder + render 转发 |
| `crates/engine/src/compiler/props_registry.rs` | `Tab` / `TabItem` 加 `closable`；`tab-window` 加 `on_tab_close`；parity 测试断言已加 |
| `crates/engine/src/compiler/tab_bar/setters.rs` | `static_setter` / `bind_setter` 加 `closable` arm |
| `crates/engine/src/compiler/codegen/shell.rs` | 加 `on_tab_close` event codegen arm |
| `demo/src/shell/main_window.rml` | line 9 加 `on-tab-close="on_tab_close"`；line 15 `<Tab ... closable />` |
| `demo/src/shell/main_window.rml.rs` | `on_tab_close` `#[command]` 已实现（调用 `IWorkbenchManager::close` + bump `activated`） |

### 阻塞点：`active_view` 的 `overflow_y_scroll` 编译错误

当前 `demo/src/shell/main_window.rml.rs:268-284`：
```rust
pub fn active_view(&self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
    let activated = self.activated.read().unwrap().clone();
    if let Some(wb) = activated {
        let iv: &dyn IContribution = wb.as_ref();
        if let Some(visual) = iv.as_visual() {
            return gpui::div()
                .flex()
                .flex_col()
                .size_full()
                .min_h_0()
                .overflow_y_scroll()   // ← E0599: no method named `overflow_y_scroll` found for struct `gpui::Div`
                .child(visual.render(window, cx))
                .into_any_element();
        }
    }
    gpui::div().into_any_element()
}
```

**根因**（经 gpui 源码核实）：
- `overflow_y_scroll` 定义于 `gpui::StatefulInteractiveElement` trait（`crates/gpui/src/elements/div.rs:1324`，位于 `StatefulInteractiveElement` trait block 1185-… 内，而非 `InteractiveElement` trait 683-1181）。
- `Div` impl `InteractiveElement`（line 1588），但 `overflow_y_scroll` 不在该 trait 上；`Div` 直接调用会 E0599。
- 现有 `tab_bar.rs:536` 的 `overflow_x_scroll` 之所以能用，是因为它在 `.id("tabs-inner")` 之后调用 —— `.id()` 把 `Div` 转成 `Stateful<Div>`，而 `Stateful<Div>` impl `StatefulInteractiveElement`。
- 当前 imports `use gpui::{InteractiveElement, IntoElement, ParentElement, Styled, WeakEntity, Window};` 缺 `StatefulInteractiveElement`，且链中未 `.id()`，故即便补 trait import 也不够 —— 必须 `.id()` 把元素 stateful 化。

### 用户原意：要"显示"滚动条

用户原话："垂直滚动条设置显示，否则案例页面过长导致看不全"。

- gpui 的 `overflow_y_scroll`（StatefulInteractiveElement）只设 `overflow.y = Scroll`，**不渲染可见 scrollbar UI**（仅滚轮/触控板可滚动）。
- gpui-component 的 `overflow_y_scrollbar`（`ScrollableElement` trait，`crates/ui/src/scroll/scrollable.rs:53`）返回 `Scrollable<Div>`，其 `RenderOnce` 内部构造 `div().id().size_full()` 外壳 + `div().overflow_y_scroll().track_scroll()` 内层 + 自定义 `Scrollbar` overlay，**会渲染可见滚动条**。
- `ScrollableElement` impl for `Div`（line 160），trait bound 为 `InteractiveElement + Styled + ParentElement + Element`，`Div` 全部满足 —— **`overflow_y_scrollbar` 可直接在 `gpui::div()` 上调用，无需 `.id()`**。
- `Scrollable<Div>` impl `Styled` / `ParentElement` / `IntoElement`，所以后续 `.child(...)` / `.into_any_element()` 链式可继续。

**结论**：用 `overflow_y_scrollbar` 既贴合"显示滚动条"原意，又避开 `.id()` + `StatefulInteractiveElement` 的额外 ceremony，且 `Scrollable::render` 内部已 `size_full()` 自动填满父容器。

---

## Proposed Changes

### 1. `demo/src/shell/main_window.rml.rs` — 修正 imports 与 `active_view` 实现

**imports（line 3）**：移除不再需要的 `InteractiveElement`，加入 `gpui_component::scroll::ScrollableElement as _`。

```rust
use gpui::{IntoElement, ParentElement, Styled, WeakEntity, Window};
use gpui_component::scroll::ScrollableElement as _;
```

> 说明：`InteractiveElement` 在本文件中无其它消费者（原本仅为 `overflow_y_scroll` 引入）；移除可消除 warning。`Styled` 仍需保留以支持 `.flex()` / `.size_full()` 等。`ParentElement` 保留以支持 `.child(...)`。

**`active_view`（line 268-284）**：把 `gpui::div().flex().flex_col().size_full().min_h_0().overflow_y_scroll()` 改为 `gpui::div().size_full().min_h_0().overflow_y_scrollbar()`。

```rust
pub fn active_view(&self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
    let activated = self.activated.read().unwrap().clone();
    if let Some(wb) = activated {
        let iv: &dyn IContribution = wb.as_ref();
        if let Some(visual) = iv.as_visual() {
            return gpui::div()
                .size_full()
                .min_h_0()
                .overflow_y_scrollbar()
                .child(visual.render(window, cx))
                .into_any_element();
        }
    }
    gpui::div().into_any_element()
}
```

**变更点逐项解释**：
- 删 `.flex().flex_col()`：`Scrollable::render` 内部已用 `div().flex().flex_col()` 包裹内容区（`scrollable.rs:135-139`），外层重复设 flex 无意义。
- 保留 `.size_full()`：会通过 `style.size` 被 `Scrollable::render` 继承到外壳（`scrollable.rs:122-125`），让外壳填满 tab-window 主体区。
- 保留 `.min_h_0()`：作为 flex 子项允许收缩（tab-window 主体区是 flex 布局）。`min_h_0` 不会被 `Scrollable::render` 继承到外壳（只继承 `size`），但作用于内部 `self.element` 上配合 `flex_1()`（`scrollable.rs:146-147`）仍有助于正确收缩。
- `.overflow_y_scrollbar()`：替换原 `.overflow_y_scroll()`，返回 `Scrollable<Div>` 并渲染可见垂直滚动条。

### 2. 文档（仅本计划文件，无代码文档改动）

无其它文档需同步。原 `tab-closable-and-vertical-scroll.md` 的 Verification Steps 6.4 描述（"主体区域出现垂直滚动条"）与新方案一致。

---

## Assumptions & Decisions

1. **采用 `overflow_y_scrollbar` 而非 `overflow_y_scroll` + `.id()`**：
   - 用户原话"滚动条设置显示"明确要求可见滚动条 → `overflow_y_scrollbar` 直接满足。
   - `overflow_y_scrollbar` 无需 `.id()` ceremony（`ScrollableElement` impl for `Div` 已存在），代码更简洁。
   - `Scrollable` 内部自带 `size_full()` + `flex()` + `track_scroll()`，少写 4 行样板。
   - 已确认 `rust-rml-demo` 依赖 `gpui-component`（`demo/Cargo.toml:14`），trait 在 `gpui_component::scroll::ScrollableElement` 路径可达。
2. **不引入 `rust-rml-ui` 的 `ScrollableElement` re-export**：当前仅 demo 一处使用滚动条，按 CLAUDE.md §2 简洁性优先，不为单次使用做框架级封装。若后续多 Shell 需滚动，再考虑在 `rust-rml-ui::lib.rs` 加 `pub use gpui_component::scroll::ScrollableElement;`。
3. **保留 `min_h_0()` 即使 `Scrollable` 只继承 `size`**：`min_h_0` 作用于内部 element 的 style，配合 `Scrollable::render` 中 `self.element.size_auto().flex_1()` 让内部 element 在 `flex_col` 容器中正确收缩 —— 这是 flex 布局的标准防御性写法，不会引入回归。
4. **不动 `<component>` 框架 codegen**：原计划已确认 `<component content={...}>` 是透明包装（`node.rs:93-143`），改动影响所有使用者。本次只在 demo `active_view` 出口包裹，最小外科改动。
5. **不重做 Part A 任何内容**：经核查 `tab.rs` / `tab_item.rs` / `tab_bar.rs` / `tab_window.rs` / `props_registry.rs` / `setters.rs` / `shell.rs` / `main_window.rml` / `main_window.rml.rs` 的 `on_tab_close` 命令均已正确落地，无回归风险。

---

## Verification Steps

### 编译验证
1. `cargo build -p rust-rml-ui` —— 应仍通过（本计划不动 ui crate）。
2. `cargo build -p rust-rml-engine` —— 应仍通过（本计划不动 engine crate）。
3. `cargo build -p rust-rml-demo` —— **核心验证**：`active_view` 应编译通过，无 `overflow_y_scroll` / `unused import` 错误。

### Parity 测试
4. `cargo test -p rust-rml-engine -- component_props_tags_align_with_routing_table` —— 验证 props_registry 中 `closable` / `on_tab_close` 与路由表对齐（上轮已加断言，本计划不动，应仍通过）。

### 运行时手测
5. `cargo run -p rml_demo`，在主窗口：
   - 5.1 鼠标移入某个 Tab：右侧出现 X 关闭按钮；移出消失（验证 Part A `group_hover`）。
   - 5.2 点击 X：对应 Tab 被移除；若是当前激活项，自动切到首个剩余项（验证 `on_tab_close` → `IWorkbenchManager::close`）。
   - 5.3 点击 X 时不会同时触发该 Tab 选中（验证 `stop_propagation`）。
   - 5.4 切换到任一长案例（如 ButtonCase / FormCase）：**主体右侧出现可见垂直滚动条**，可上下滚动查看完整内容；窗口缩放时滚动条自适应（验证 Part B `overflow_y_scrollbar`）。
6. 边界：所有 Tab 关闭后主体空白不崩溃；再触发 ActivityBar 打开新 case 应正常恢复。

### 回归验证
7. `show_chrome` 切换、`left-size` 拖拽、tab 选中切换、ActivityBar 切 case 等既有路径仍正常。
8. 案例内部若自带 ScrollHandle（如某些 case 内嵌滚动），外层 `overflow_y_scrollbar` 不应与之冲突（`Scrollable` 外壳 `size_full` + 内部 `flex_1` 让内层 ScrollHandle 仍可独立工作）。

---

## Files Touched

| File | Change |
|---|---|
| `demo/src/shell/main_window.rml.rs` | line 3 imports：移 `InteractiveElement`，加 `gpui_component::scroll::ScrollableElement as _`；`active_view`（line 268-284）：链式改 `.size_full().min_h_0().overflow_y_scrollbar().child(...)` |

**未触碰**：Part A 的全部 9 个文件（已正确落地，无回归需要）；`<component>` 框架 codegen；props_registry / parity 测试。

---

## Rollback

若 `overflow_y_scrollbar` 在 demo 运行时出现滚动条样式异常或与 case 内部滚动冲突，回退方案：
- 改回 `gpui::div().id("active-view-scroll").size_full().min_h_0().overflow_y_scroll()`
- imports：`use gpui::{InteractiveElement, IntoElement, ParentElement, StatefulInteractiveElement as _, Styled, WeakEntity, Window};`
- 该回退方案不渲染可见滚动条 UI（仅滚轮滚动），但编译可通过；可作为兜底。
