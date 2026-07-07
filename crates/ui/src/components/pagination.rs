//! Pagination 组件封装 —— 基于 gpui-component 的 Pagination
//!
//! 分页组件，带 ElementId 构造，支持 Disableable + Sizable。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Pagination current-page={page} total-pages={total} on-click={on_page_change} />
//! ```

pub use gpui_component::pagination::Pagination;
