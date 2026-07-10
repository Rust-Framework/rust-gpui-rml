//! OtpInput codegen 模块入口（OTP 输入）。
//!
//! - `setters.rs`：OtpInput 专用属性映射（groups）
//!
//! length / masked / default_value 由 OtpInputTranslator 注入 state_ctor 闭包，
//! 不生成 setter。on_change / on_focus / on_blur 通过 cx.subscribe 事件订阅，
//! 复用 input::event 的 InputEvent 模式。

pub mod setters;
