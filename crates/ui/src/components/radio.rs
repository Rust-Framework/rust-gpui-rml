//! Radio + RadioGroup 组件封装 —— 基于 gpui-component 的 radio 模块
//!
//! 单选按钮与单选按钮组。Radio 是带 ElementId 的叶子组件（ParentElement + Sizable），
//! RadioGroup 是容器组件（StatelessWithItems），子节点为 Radio。
//!
//! ## 声明式语法
//!
//! ```rml
//! <RadioGroup selected-index={idx} on-click={on_radio_change}>
//!     <Radio label="选项 A" />
//!     <Radio label="选项 B" checked="" />
//! </RadioGroup>
//! ```

pub use gpui_component::radio::{Radio, RadioGroup};
