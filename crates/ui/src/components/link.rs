//! Link 组件封装 —— 基于 gpui-component 的 Link
//!
//! 超链接组件，带 ElementId 构造，支持 ParentElement。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Link href="https://example.com">访问网站</Link>
//! <Link on-click={on_link_click}>点击跳转</Link>
//! ```

pub use gpui_component::link::Link;
