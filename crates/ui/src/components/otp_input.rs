//! OtpInput 组件封装 —— 基于 gpui-component 的 OtpInput
//!
//! 一次性密码输入框，Stateful 构造器接受 &Entity<OtpState>。
//! OtpState 构造需 length 参数，由 OtpInputTranslator 从 length 属性注入 state_ctor。
//!
//! ## 声明式语法
//!
//! ```rml
//! <OtpInput ref="otp_state" length="6" groups="2" on-change={on_otp_change} />
//! ```

pub use gpui_component::input::{OtpInput, OtpState};
