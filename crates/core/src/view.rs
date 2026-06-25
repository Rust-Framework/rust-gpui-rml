//! `IRmlView` trait —— RML 视图标记
//!
//! 标记结构体为 RML 视图的 Code-Behind，声明关联的 `.rml` 模板路径。
//! `#[view]` 宏自动实现此 trait。

use crate::view_model::IViewModel;

/// RML 视图标记 trait。
///
/// 实现此 trait 的结构体：
/// 1. 自身即 GPUI Entity
/// 2. 由编译器在 `OUT_DIR` 生成 `impl Render`
/// 3. 可作为 `RmlApplication::run::<Root>()` 的根视图
pub trait IRmlView: IViewModel {
    /// 关联的 `.rml` 模板路径（相对于 `src` 目录）。
    /// 由 `#[view]` 宏根据命名约定或 `template=` 参数生成。
    fn rml_template() -> &'static str;
}
