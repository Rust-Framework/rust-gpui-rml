//! Collapsible 组件封装 —— 基于 gpui-component 的 Collapsible
//!
//! 折叠面板组件，无 ElementId 构造（RenderOnce），支持 ParentElement。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Collapsible open={is_open}>
//!     <div>折叠内容</div>
//! </Collapsible>
//! ```

pub use gpui_component::collapsible::Collapsible;
