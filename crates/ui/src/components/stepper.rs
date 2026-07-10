//! Stepper 组件封装 —— 基于 gpui-component 的 Stepper
//!
//! 步骤指示器，构造器接受 ElementId，子节点为 `<StepperItem>` / `<step-item>`。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Stepper selected-index="1" on-click={on_step_click}>
//!     <step-item icon="Check">步骤一</step-item>
//!     <step-item>步骤二</step-item>
//!     <step-item>步骤三</step-item>
//! </Stepper>
//! ```

pub use gpui_component::stepper::{Stepper, StepperItem};
