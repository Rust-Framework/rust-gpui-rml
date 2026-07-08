# Slot 作用域架构修复 + Bottom 面板 Flat TabBar 改造

## Context

`demo/src/shell/main_window.rml.rs` 的 `render_bottom_panel` 当前是手写状态条（L422-483），仅展示 `panel` 元信息，无法通过按钮交互操控 resizable。

**根因（架构级）**：`SlotRenderer` 闭包首参为 `&dyn ISlotScope`（slot.rs:109-111），是渲染期引用，无法被 `'static` 闭包（按钮 `on_click`）捕获。docs 与 memory 此前规划 `to_op_handle()` API 是补丁式叠加——在 ISlotScope 之上再引入 `SlotScopeOp` trait，属异味设计，拒绝。

**架构级修复**：把 `SlotRenderer` 首参从 `&dyn ISlotScope` 改为 `Arc<dyn ISlotScope>`，scope 天然 `'static`，按钮闭包直接 `panel.clone()` 捕获。无需新 trait，ISlotScope trait 本体方法签名不变。同时重写 bottom 面板为 flat TabBar + suffix 按钮形式，落地 scope 延迟调用。

用户已确认（tabs 内容 / close 语义 / 接受架构修复 / 文档同步）。

## 改动

### 1. `crates/core/src/slot.rs` — SlotRenderer 签名升级 + is\_maximized 查询方法

* `SlotRenderer` 类型（L109-111）：`Fn(&dyn ISlotScope, &mut Window, &mut App)` → `Fn(Arc<dyn ISlotScope>, &mut Window, &mut App)`

* ISlotScope trait 增加 `is_maximized` 默认方法（查询方法，与 current\_size/container\_size/has\_resizable 同域，非补丁）：

  ```rust
  fn is_maximized(&self, _cx: &App) -> bool { false }
  ```

* 模块注释（L8/L16/L21/L105/L107）：`&dyn ISlotScope` → `Arc<dyn ISlotScope>`

* 顶部 `use gpui::{App, Pixels, Window};` 增加 `use std::sync::Arc;`（如缺失）

### 2. `crates/engine/src/compiler/codegen/shell.rs` — wrap\_shell\_slot codegen

* `wrap_shell_slot`（L40-65）生成的闭包签名与 scope\_var 注入改为 `std::sync::Arc<dyn rml_core::slot::ISlotScope>`（codegen 用全限定路径）：

  * L44：`let {name}: &dyn ... = scope;` → `let {name}: std::sync::Arc<dyn rml_core::slot::ISlotScope> = scope;`

  * L52：`move |scope: &dyn ...` → `move |scope: std::sync::Arc<dyn rml_core::slot::ISlotScope>, ...`

* 文档注释（L22/L37）：`&dyn ISlotScope` → `Arc<dyn ISlotScope>`

### 3. `crates/engine/src/compiler/user_component.rs` — 自定义组件 slot 闭包 codegen

* L117、L136 两处闭包首参：`_scope: &dyn rml_core::slot::ISlotScope` → `_scope: std::sync::Arc<dyn rml_core::slot::ISlotScope>`

* 注释（L86/L97）：`&dyn ISlotScope` → `Arc<dyn ISlotScope>`

### 4. `crates/engine/src/compiler/codegen/node.rs` — 自定义组件 slot 调用

* L269：`f(&rml_core::slot::NullSlotScope::new(...), _window, cx)` → `f(std::sync::Arc::new(rml_core::slot::NullSlotScope::new(...)), _window, cx)`

* 注释（L249/L251/L253）：`&dyn ISlotScope` → `Arc<dyn ISlotScope>`

### 5. `crates/ui/src/window/tab_window.rs` — 构造/调用/setter/TabWindowSlotScope

* **scope 构造**（L558-574）：`NullSlotScope::new("x")` → `Arc::new(NullSlotScope::new("x"))`；`TabWindowSlotScope::new(...)` → `Arc::new(TabWindowSlotScope::new(...))`。统一类型 `Arc<dyn ISlotScope>`（或 `Arc<dyn rml_core::slot::ISlotScope>`）。

* **闭包调用**（L581/L585/L589/L593/L597/L601）：`f(&scope, window, cx)` → `f(scope.clone(), window, cx)`（Arc clone 廉价；或 scope 单次使用直接 move）

* **setter 签名**（L326/L336/L424/L434/L444/L454）：`Box<dyn Fn(&dyn ISlotScope, &mut Window, &mut App) -> AnyElement + Send + Sync>` → `Box<dyn Fn(Arc<dyn ISlotScope>, &mut Window, &mut App) -> AnyElement + Send + Sync>`（注意：6 处 setter，其中 L424/L434/L444/L454 是 slot\_left/right/bottom/status 的内联 Box 类型；如能统一用 `SlotRenderer` 别名更佳，但保持现状不引入额外重构）

* **TabWindowSlotScope 实现 is\_maximized**（L89 impl 块）：

  ```rust
  fn is_maximized(&self, cx: &App) -> bool {
      self.prev_size.as_ref()
          .map(|e| e.read(cx).lock().unwrap().is_some())
          .unwrap_or(false)
  }
  ```

* import（L32）：增加 `use std::sync::Arc;`（如缺失）

### 6. `demo/src/shell/main_window.rml.rs` — render\_bottom\_panel 重写（L412-483）

签名变更：`panel: &dyn rml_core::slot::ISlotScope` → `panel: std::sync::Arc<dyn rml_core::slot::ISlotScope>`

重写为 flat TabBar + suffix 按钮结构：

```rust
pub fn render_bottom_panel(
    &self, panel: Arc<dyn rml_core::slot::ISlotScope>,
    _window: &mut Window, _cx: &mut gpui::Context<Self>,
) -> gpui::AnyElement {
    use gpui::{IntoElement, ParentElement, Styled, div, px};
    use rml_ui::{ActiveTheme, TabBar};
    use crate::components::tab::TabItem;
    use gpui_component::{IconName, button::{Button, ButtonVariants as _}, Sizable as _};

    let is_maximized = panel.is_maximized(_cx);

    let bar = TabBar::new(("bottom-panel-tabs", 0usize))
        .flat()
        .child(TabItem::new().title("TERMINAL"))
        .child(TabItem::new().title("OUTPUT"))
        .child(TabItem::new().title("PROBLEMS"))
        .last_empty_space(div().flex_1())  // 撑开 → suffix 右对齐
        .suffix(self.render_bottom_suffix(panel.clone(), is_maximized));

    div()
        .flex().flex_col().size_full()
        .p_0().m_0()  // 面板外边距为零
        .bg(_cx.theme().background)
        .child(bar)
        .child(
            div().flex_1().px(px(12.)).py(px(8.))
                .text_size(px(12.))
                .text_color(_cx.theme().muted_foreground)
                .child("$ demo terminal — scope variable accessible from slot content"),
        )
        .into_any_element()
}
```

新增辅助方法 `render_bottom_suffix`（同 MainWindow impl 块，接 `Arc<dyn ISlotScope>`）：

* 最大化/还原按钮：`Button::new("bottom-toggle").xsmall().ghost().icon(...)`，根据 `is_maximized` 切换 `IconName::Maximize` / `IconName::WindowRestore`，`on_click` 中 `if h.is_maximized(cx) { h.restore(w, cx) } else { h.maximize(w, cx) }`

* 关闭按钮：`Button::new("bottom-close").xsmall().ghost().icon(IconName::Close)`，`on_click` 调用 `h.close(window, cx)`

* 闭包捕获 `panel.clone()`（Arc clone，满足 'static）

* 删除 L421 关于 `to_op_handle()` 的过时注释

### 7. 文档与案例同步

* `docs/06-components/slots.md`：

  * L116：修正过时签名（补充 `Arc<dyn ISlotScope>` 首参，原文缺 ISlotScope 参数）

  * L217/L295/L308：移除 `to_op_handle()` 规划描述，改为说明 `Arc<dyn ISlotScope>` 天然支持 'static 闭包延迟调用

  * L229/L236/L270/L287/L304：`&dyn ISlotScope` → `Arc<dyn ISlotScope>`

* `demo/src/cases/slot_scope_case.rml`：

  * L29：移除"to\_op\_handle 规划中"描述，补充 Arc 用法说明

  * "限制"卡片（L32-37）：移除延迟调用限制条目

* `demo/src/cases/slot_scope_case.rml.rs` L34：字符串 "\&dyn ISlotScope" → "Arc<dyn ISlotScope>"

## 关键文件

* [slot.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/slot.rs) — SlotRenderer 类型 + is\_maximized

* [shell.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs) — wrap\_shell\_slot codegen

* [user\_component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/user_component.rs) — 自定义组件 slot codegen

* [node.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) — 自定义组件 slot 调用

* [tab\_window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs) — 构造/调用/setter/is\_maximized 实现

* [main\_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) — render\_bottom\_panel 重写

* [slots.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/slots.md) — 文档同步

## 复用资源

* `TabBar::flat()` / `.suffix()` / `.last_empty_space()` / `.child(TabItem)` — [tab\_bar.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab_bar.rs)

* `TabItem::new().title(...)` — [tab\_item.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab_item.rs)

* `Button::new().xsmall().ghost().icon(...)` — [tabs.rs:803-806](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tabs.rs#L803-L806)

* `IconName::Maximize` / `WindowRestore` / `Close` — 已确认可用

* `TabWindowSlotScope` 现有 maximize/restore/close 逻辑 — [tab\_window.rs:98-145](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L98-L145)

* `state.rs` 的 `SlotRenderer` 类型别名自动适配，无需改 — [state.rs:67](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/state.rs#L67)

## 验证

1. 编译：`cargo build -p rust-rml-core -p rust-rml-ui -p rust-rml-engine -p rust-rml-demo`
2. 测试：`cargo test -p rust-rml-core`（slot 模块）+ `cargo test -p rust-rml-engine`（codegen）
3. 运行 demo 手动验证：

   * bottom 面板显示 flat TabBar，3 个 tab（TERMINAL/OUTPUT/PROBLEMS）

   * suffix 右对齐显示最大化 + 关闭按钮

   * 点击最大化：面板高度→容器高度，图标切换为还原

   * 点击还原：恢复原高度

   * 点击关闭：面板高度→0（折叠）

   * 面板外层无 padding/margin（TabBar 紧贴容器边界）
4. 验证 slot 闭包签名变更未破坏其他 slot（menu/title/footer/left/right）渲染

