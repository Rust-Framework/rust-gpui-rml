# rust-rml-app

> RML 应用启动器 —— WPF 风格 Application + main_window 内置 API。

## 职责

`rust-rml-app` 封装 GPUI 的 `Application`，提供 WPF 风格的 `RmlApplication::new().main_window::<W>().run::<L>()` 声明式入口 API。`main_window` 是内置功能，非扩展 trait。

**核心约束**：
- `#![forbid(unsafe_code)]` 全 crate 启用
- 仅依赖 `rust-rml-core`（trait 契约，含 `IWindow`、`assets`、`i18n`、`theme`）+ `gpui` / `gpui_platform`（运行时）+ `gpui-component-assets`（图标资源）
- 不依赖 engine / macros / ui，保持启动器轻量
- **不持有任何资源状态**：`assets` / `i18n` / `theme` 由 `rml_core` 中的全局 `OnceLock` 管理，
  build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动注册，
  `RmlApplication` 不提供 `.assets(...)` 方法

## 模块结构

| 模块 | 核心类型 | 职责 |
|------|---------|------|
| `lib.rs` | — | crate 入口，`extern crate` 别名 + 模块声明 |
| `application.rs` | `RmlApplication<W>` | 应用启动器：类型状态模式，`main_window::<W>()` + `run::<L>()` |
| `lifecycle.rs` | `IAppLifecycle` | 应用级生命周期契约（`on_launch`/`on_exit`/`on_activate`/`on_deactivate`） |
| `resources.rs` | — | i18n / theme 资源访问的便捷封装，统一走 `rml_core::assets::load` / `load_str` |

## 用法

### 声明式 API（推荐，配合 `#[rml::main]`）

```rust
extern crate rust_rml_engine as rml;      // 提供 #[rml::main]
extern crate rust_rml_app as rml_app;

mod app;

// `#[rml::main]` 自动注入 `rml::embed_assets!()`（include build.rs 生成的 rml_assets.rs）。
// 生成文件内的 `#[ctor::ctor]` 函数在 main 之前自动调用 `rml_core::assets::init(...)`,
// 因此此处无需手写资源初始化代码。模式（嵌入/文件系统）由 build.rs 的 `.assets(path, embed)` 决定。
#[rml::main]
fn main() {
    rml_app::RmlApplication::new()
        .main_window::<MyWindow>()
        .run::<app::Startup>();
}
```

`main_window::<W>()` 要求 `W: IWindow + Default + 'static`，`run::<L>()` 自动调用 `L::on_launch(cx)` 并打开主窗口。

### 命令式 API

```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;

use rml_app::IAppLifecycle;

struct MyApp;

impl IAppLifecycle for MyApp {
    fn on_launch(&mut self, cx: &mut gpui::App) {
        // 手动打开窗口（不使用 main_window API）
    }
}

#[rml::main]
fn main() {
    rml_app::RmlApplication::new().run::<MyApp>();
}
```

## 设计规范

1. **类型状态模式**：`RmlApplication<W = NoWindow>` 使用泛型参数追踪是否已设置主窗口，`main_window::<W>()` 实现类型转换
2. **内置 main_window**：`main_window` 是 `RmlApplication` 的内置方法，非扩展 trait（无 `RmlApplicationExt`）
3. **GPUI 适配**：使用 `gpui_platform::application()` 替代 `Application::new()`，适配 Zed gpui 版本，并通过 `.with_assets(gpui_component_assets::Assets)` 注册图标资源
4. **依赖方向**：`app` crate 不依赖 `ui` crate；`IWindow` trait 在 `core` crate 中定义
5. **资源零感知**：`RmlApplication` 不感知资源加载方式（嵌入 / 文件系统），所有资源初始化由 build.rs + `#[rml::main]` + `#[ctor::ctor]` 在 `main` 之前完成
6. **生命周期类型参数**：`run::<L>()` 而非 `run()` —— 通过类型参数 `L: IAppLifecycle` 注入生命周期钩子，避免与无主窗口的命令式入口 `run::<A>()` 冲突
