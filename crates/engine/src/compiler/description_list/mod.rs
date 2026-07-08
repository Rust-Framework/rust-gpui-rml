//! DescriptionList 系列组件 codegen 模块。
//!
//! 由 `ItemsComponentTranslator` 按 canonical_tag == "DescriptionList" 调用。
//!
//! ## 模块结构
//!
//! - `gen`：容器 codegen（构造 + 属性 + 子节点 `.child()`/`.separator()` 注入）
//! - `item`：`<description>` 子节点 codegen（label 构造器 + value/span setter + 子节点作为 value）
//! - `setters`：DescriptionList/DescriptionItem 专用属性 → builder 方法映射

pub mod gen;
pub mod item;
pub mod setters;

pub use gen::gen_description_list;
