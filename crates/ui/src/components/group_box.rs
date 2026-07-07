//! GroupBox 组件封装 —— 基于 gpui-component 的 GroupBox
//!
//! 分组框容器组件，无 ElementId 构造（RenderOnce），支持 ParentElement。
//!
//! ## 声明式语法
//!
//! ```rml
//! <GroupBox title="基本设置">
//!     <Switch />
//!     <Checkbox />
//! </GroupBox>
//! ```

pub use gpui_component::group_box::{GroupBox, GroupBoxVariants};
