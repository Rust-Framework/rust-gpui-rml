# rust-rml-app

> RML 应用启动器与窗口管理。

## 职责

`rust-rml-app` 封装 GPUI 的 `Application` + 窗口创建，提供简洁的 `RmlApplication::new().run::<RootView>()` 入口 API。负责应用级初始化、窗口生命周期管理、资源加载。

**核心约束**：
- `#![forbid(unsafe_code)]` 全 crate 启用
- 仅依赖 `rust-rml-core`（trait 契约）+ `gpui` / `gpui_platform`（运行时）
- 不依赖 engine / macros，保持启动器轻量

## 模块结构

| 模块 | 核心类型 | 职责 |
|------|---------|------|
| `lib.rs` | — | crate 入口，`extern crate` 别名 + 模块声明 |
| `application.rs` | `RmlApplication` | 应用启动器：`new()` / `title()` / `size()` / `run::<R>()` |
| `window.rs` | `WindowManager` | 多窗口管理（Phase B：open/close/list/窗口间通信） |
| `resources.rs` | `Resources` | 资源加载（Phase B：assets/ 目录的图标/字体/i18n 缓存） |

## 用法

```rust
use gpui::px;
use rml_app::RmlApplication;

fn main() {
    RmlApplication::new()
        .title("My App")
        .size(px(800.), px(600.))
        .run::<MyView>();
}
```

`run::<R>()` 要求 `R: IRmlView + Render + Default + 'static`，其中 `Render` 由 `#[view]` 宏 + `build.rs` 自动生成。

## 设计规范

1. **单入口**：用户只需调用 `RmlApplication::new().run::<R>()`，内部处理 GPUI Application/Window 全部细节
2. **GPUI 适配**：使用 `gpui_platform::application()` 替代 `Application::new()`，适配 Zed gpui 版本
3. **窗口选项**：默认创建带标题栏的窗口ed，支持 `title()` / `size()` 链式配置
4. **扩展点**：Phase B 将新增 `with_global<G>()` / `with_extensions()` / 多窗口 API
