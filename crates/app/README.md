# rust-rml-app

> RML 应用启动器 —— WPF 风格 Application + main_window 内置 API。

## 职责

`rust-rml-app` 封装 GPUI 的 `Application`，提供 WPF 风格的 `RmlApplication::new().main_window::<W>().run()` 声明式入口 API。`main_window` 是内置功能，非扩展 trait。

**核心约束**：
- `#![forbid(unsafe_code)]` 全 crate 启用
- 仅依赖 `rust-rml-core`（trait 契约，含 `IWindow`）+ `gpui` / `gpui_platform`（运行时）
- 不依赖 engine / macros / ui，保持启动器轻量

## 模块结构

| 模块 | 核心类型 | 职责 |
|------|---------|------|
| `lib.rs` | — | crate 入口，`extern crate` 别名 + 模块声明 |
| `application.rs` | `RmlApplication<W>` | 应用启动器：类型状态模式，`main_window::<W>()` + `run()` |
| `lifecycle.rs` | `IAppLifecycle` | 应用级生命周期契约（`on_launch`/`on_exit`/`on_activate`/`on_deactivate`） |
| `resources.rs` | — | 资源加载（Phase B：assets/ 目录的图标/字体/i18n 缓存） |

## 用法

### 声明式 API（推荐）

```rust
use rml_app::RmlApplication;

fn main() {
    RmlApplication::new()
        .main_window::<MyWindow>()
        .run();
}
```

`main_window::<W>()` 要求 `W: IWindow + Default + 'static`，`run()` 自动调用 `W::default().open(cx)` 打开主窗口。

### 命令式 API

```rust
use rml_app::{IAppLifecycle, RmlApplication};

struct MyApp;

impl IAppLifecycle for MyApp {
    fn on_launch(&mut self, cx: &mut gpui::App) {
        // 手动打开窗口
    }
}

fn main() {
    RmlApplication::new().run::<MyApp>();
}
```

## 设计规范

1. **类型状态模式**：`RmlApplication<W = NoWindow>` 使用泛型参数追踪是否已设置主窗口，`main_window::<W>()` 实现类型转换
2. **内置 main_window**：`main_window` 是 `RmlApplication` 的内置方法，非扩展 trait（无 `RmlApplicationExt`）
3. **GPUI 适配**：使用 `gpui_platform::application()` 替代 `Application::new()`，适配 Zed gpui 版本
4. **依赖方向**：`app` crate 不依赖 `ui` crate；`IWindow` trait 在 `core` crate 中定义
