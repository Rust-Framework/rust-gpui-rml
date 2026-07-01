# rust-rml-ui

> RML 扩展组件库 —— 封装 [`gpui-component`](https://github.com/longbridge/gpui-component)，提供 Button / Input / Dialog / List / Form 等高级组件，以及内置窗口类型。

## 职责

本 crate 是 RML 框架 **双轨制组件策略** 中的「扩展轨」（Layer 5）：

| 轨道 | 来源 | 用途 |
|------|------|------|
| 原生轨 | GPUI 内置（`div`/`img`/`text` 等） | HTML 基础标签映射 |
| **扩展轨（本 crate）** | `gpui-component` | 复杂交互组件（Dialog/Form/List 等） |

在 `.rml` 模板中，PascalCase 标签（`<Button>`/`<Input>`/`<Dialog>`）由 codegen 路由到本 crate 的构造器。

## 设计规范

1. **零成本抽象**：直接 re-export `gpui-component` 类型，避免不必要的 wrapper struct。codegen 生成的代码与用户手写 `gpui_component::Button::new(...)` 等价。
2. **声明式 vs MVVM**：菜单/状态栏等**声明式**标签由 engine `compiler/menu/` 直译 gpui-component API；本 crate 仅保留 **MVVM 数据绑定**适配（`IMenuItem`/`Menu`/`RmlStatusBar`/`TreeView`），不重复包装 `PopupMenu`。
3. **单一入口**：[`init`] 是 gpui-component 的唯一初始化函数，封装了 theme/global_state/root/focus_trap/dialog/sheet/list 等模块的子初始化。
4. **Feature 开关**：通过 `ui-components` feature（默认开启）控制。关闭后本 crate 退化为空实现，整个 RML 框架仅使用原生 GPUI 元素。
5. **铁律遵守**：`#![forbid(unsafe_code)]` 全 crate 启用；所有 trait 仍以 `I` 开头（本 crate 直接复用 gpui-component 的 trait，不引入新 trait）。

## 关键 API

### 初始化

```rust
// 通常由 RmlApplication::run 自动调用
rml_ui::init(cx);
```

### 组件使用

```rust
use rml_ui::prelude::*;

// 在 #[component] 的 render 中
gpui::div().child(
    Button::new("my-btn")
        .label("Click me")
        .primary()
        .on_click(|_, _, _| println!("clicked"))
)
```

### 窗口顶层 Root

`gpui_component::Root` 是窗口的顶层 view，管理 Dialog/Sheet/Notification 的层级。由 `#[window]` 宏在 `open()` 内部自动创建并包裹业务 view，用户无需手动管理。

### 内置窗口类型

本 crate 提供开箱即用的 `IWindow` 实现：

- **`Window`** — 基础窗口，无装饰，适用于占位窗口、启动画面等简单场景
- **`ModernWindow`** — 现代窗口，使用 `ModernWindowShell` 提供 TitleBar/StatusBar 外观

```rust
use rml_app::RmlApplication;

fn main() {
    RmlApplication::new()
        .main_window::<rml_ui::ModernWindow>()
        .run();
}
```

用户创建带 RML 模板的窗口应使用 `#[window]` 宏。

### ModernWindowShell

`ModernWindowShell` 是内置封装组件，组合 `TitleBar` + `Menu` + `StatusBar`。在 `.rml` 中作为根标签使用：

```html
<ModernWindowShell title="My App" menu={menu_items} status_bar={status_items}>
    <!-- 业务内容 -->
</ModernWindowShell>
```

## 文档参考

- 开发规划：`.trae/documents/rml-bottom-up-architecture-plan.md` §2.5 Layer 5
- 设计文档：`docs/03-components.md`
- 上游 API：`gpui-component` [官方文档](https://docs.rs/gpui-component)
