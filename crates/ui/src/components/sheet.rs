//! Sheet re-export
//!
//! 侧边抽屉组件，从窗口边缘滑入的浮层面板。
//! 构造器需要 `&mut Window, &mut App` 参数（与标准组件不同），
//! codegen 生成 `Sheet::new(_window, cx)` 直接使用 render 上下文变量。

pub use gpui_component::sheet::Sheet;
