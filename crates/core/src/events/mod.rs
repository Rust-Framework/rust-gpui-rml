//! RML 事件对象类型
//!
//! RML 定义自己的事件对象，作为 GPUI 原生事件的抽象层。
//! 代码生成器负责将 GPUI 事件转换为 RML 事件后传递给命令方法。
//! 所有事件实现 `IEvent` trait，支持 `prevent_default` / `stop_propagation`。
//! 详见文档 §5.2 事件对象。

mod change_event;
mod click_event;
mod flags;
mod focus_event;
mod hover_event;
mod input_event;
mod key_down_event;
mod key_up_event;
mod load_event;
mod mouse_button;
mod mouse_event;
mod resize_event;
mod scroll_event;
mod submit_event;
mod wheel_event;

pub use change_event::ChangeEvent;
pub use click_event::ClickEvent;
pub use focus_event::FocusEvent;
pub use hover_event::HoverEvent;
pub use input_event::InputEvent;
pub use key_down_event::KeyDownEvent;
pub use key_up_event::KeyUpEvent;
pub use load_event::LoadEvent;
pub use mouse_button::MouseButton;
pub use mouse_event::MouseEvent;
pub use resize_event::ResizeEvent;
pub use scroll_event::ScrollEvent;
pub use submit_event::SubmitEvent;
pub use wheel_event::WheelEvent;
