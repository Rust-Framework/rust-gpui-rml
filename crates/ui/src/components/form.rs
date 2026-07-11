//! Form / Field —— 声明式表单容器与字段组件
//!
//! RML `<Form>` 编译为 `gpui_component::form::Form`，`<Field>` 编译为 `Field`。
//!
//! ## 构造模式
//!
//! ```ignore
//! // Form：默认垂直布局
//! Form::vertical().label_width(px(120.)).child(Field::new().label("Name").child(Input::new(...)))
//!
//! // Form：水平布局
//! Form::horizontal().label_width(px(120.)).child(Field::new().label("Email").child(Input::new(...)))
//!
//! // Field：带描述、必填
//! Field::new().label("Password").description("至少 8 位").required(true).child(Input::new(...))
//! ```
//!
//! ## 注意
//!
//! - Form 不实现 `ParentElement`，其 `.child()` 方法接受 `impl Into<Field>`，非 Field 子节点会编译失败。
//! - Field 实现 `ParentElement`，可包含任意 `AnyElement` 子节点（Input、Switch、Button 等）。

pub use gpui_component::form::{Field, FieldBuilder, Form};
