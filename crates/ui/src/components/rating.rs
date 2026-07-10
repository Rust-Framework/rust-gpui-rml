//! Rating 组件封装 —— 基于 gpui-component 的 Rating
//!
//! 星级评分组件，Stateless 构造器接受 ElementId。内部状态由 `window.use_keyed_state` 管理，
//! 无需 Entity<RatingState>。支持 value、max、disabled 等属性，on-click 事件签名为 Fn(&usize, ...)。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Rating value="3" max="5" on-click={on_rating_change} />
//! ```

pub use gpui_component::rating::Rating;
