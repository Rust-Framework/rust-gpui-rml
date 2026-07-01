//! `IWindowExt` —— rml_ui 层的窗口扩展 trait
//!
//! 为 `IWindow` 提供 `open_rooted` 默认实现，封装 `init(cx)` + `Root::new()` 包裹逻辑。
//!
//! ## 为什么在 rml_ui 而非 rml_core
//!
//! `open_rooted` 内部需要调用 `rml_ui::init(cx)`（初始化 gpui-component 全局状态）
//! 和 `rml_ui::Root::new()`（包裹视图提供通知路由）。rml_core 仅依赖 GPUI 基础类型，
//! 不能依赖 rml_ui，因此 `IWindow::open` 在 rml_core 中保持必需方法，
//! 由 rml_ui 通过本 trait 提供默认实现。
//!
//! ## 使用方式
//!
//! 通常无需手动调用——codegen 生成的 `impl IWindow::open` 和内置 `Window`/`ModernWindow`
//! 都会调用 `IWindowExt::open_rooted`。手动实现 `IWindow` 时也可调用以复用逻辑：
//!
//! ```rust,ignore
//! impl IWindow for MyWindow {
//!     fn open(&mut self, cx: &mut App) {
//!         rml_ui::IWindowExt::open_rooted(self, cx);
//!     }
//!     // ...
//! }
//! ```

use gpui::{App, AppContext};

use rml_core::window::IWindow;

/// rml_ui 层的窗口扩展 trait
///
/// 为所有 `IWindow` 实现提供 `open_rooted` 默认实现，消除 open() 三份重复
/// （codegen 模板 + builtin Window + builtin ModernWindow）。
pub trait IWindowExt: IWindow {
    /// 含 init + Root 包裹的完整 open 实现
    ///
    /// 内部步骤：
    /// 1. `init(cx)` 初始化 gpui-component 全局状态
    /// 2. `window_options()` 从 IWindow 配置构建 WindowOptions
    /// 3. `cx.open_window()` 创建 OS 窗口，闭包内 `Self::default()` 创建视图实例
    ///    并用 `Root::new()` 包裹以启用通知路由
    /// 4. `set_handle()` 存储窗口句柄
    fn open_rooted(&mut self, cx: &mut App) {
        // gpui-component 由 RmlApplication::bootstrap_runtime 初始化；此处仅确保 i18n catalog 可用
        rml_core::i18n::ensure_i18n(cx);
        let options = self.window_options_for(cx);
        let handle = cx
            .open_window(options, |window, cx| {
                let view = cx.new(|_| Self::default());
                cx.new(|cx| crate::Root::new(view, window, cx))
            })
            .expect("failed to open window");
        self.set_handle(handle.into());
    }
}

// Blanket impl：所有 IWindow 实现自动获得 open_rooted
impl<W: IWindow> IWindowExt for W {}
