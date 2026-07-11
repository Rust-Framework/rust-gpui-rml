//! NumberInput 组件封装 —— 基于 gpui-component 的 NumberInput
//!
//! 数字输入框，Stateful 构造器接受 &Entity<InputState>（复用 InputState）。
//! 支持 value 双向绑定、placeholder、appearance 等属性。
//! on_change/on_focus/on_blur/on_enter 走 InputEvent 事件订阅（同 Input/TextInput）。
//!
//! 步进按钮（增减）默认由 InputState 内部处理，直接更新值并触发 InputEvent::Change。
//! 如需外部接管步进逻辑，在 on_loaded 中调用 `state.set_step(None, window, cx)`，
//! 此时步进将发射 NumberInputEvent::Step 事件供订阅。
//!
//! ## 声明式语法
//!
//! ```rml
//! <NumberInput ref="num_state" placeholder="请输入数字" on-change={on_num_change} />
//! ```

pub use gpui_component::input::{NumberInput, NumberInputEvent};
