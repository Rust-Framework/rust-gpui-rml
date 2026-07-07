//! Alert 组件封装 —— 基于 gpui-component 的 Alert
//!
//! Alert 用于显示一条消息给用户，支持 info/success/warning/error 四种 variant。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Alert info="" title="提示" message="操作成功" />
//! <Alert warning="" on-close={handle_close}>这是一条警告消息</Alert>
//! ```
//!
//! variant 通过两种形式声明：
//! - 布尔属性：`info=""` / `success=""` / `warning=""` / `error=""` → 关联函数构造
//! - `variant="info"` → builder 方法 `.with_variant(AlertVariant::Info)`
//!
//! `message` 属性优先于文本子节点（构造器第二参数）。

pub use gpui_component::alert::{Alert, AlertVariant};
