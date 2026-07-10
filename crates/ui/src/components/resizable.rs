//! Resizable 组件封装 —— 基于 gpui-component 的 Resizable
//!
//! 可调整面板组，用于创建可拖拽调整大小的面板布局。
//! 构造器为函数 `h_resizable(id)` / `v_resizable(id)`，非 `ResizablePanelGroup::new(id)`。
//! 子节点为 `<resizable-panel>`，通过 `resizable_panel()` 构造。
//!
//! ## 声明式语法
//!
//! ```rml
//! <resizable direction="horizontal" size="300px">
//!     <resizable-panel size="200px">
//!         <div>Panel 1</div>
//!     </resizable-panel>
//!     <resizable-panel>
//!         <div>Panel 2</div>
//!     </resizable-panel>
//! </resizable>
//! ```

pub use gpui_component::resizable::{
    ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, h_resizable,
    resizable_panel, v_resizable,
};
