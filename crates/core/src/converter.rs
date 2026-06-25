//! `IConverter` trait —— 值转换器契约
//!
//! 用于单向/双向绑定时在 ViewModel 字段类型与 UI 显示类型之间转换。
//! 详见文档 §3.5 值转换器。

/// 值转换器 trait。
///
/// 实现此 trait 的类型可在 `.rml` 中通过 `converter` 属性使用：
/// ```html
/// <p>{active_count, ActiveCountConverter}</p>
/// ```
pub trait IConverter<S, T>: Send + Sync {
    /// 正向转换：ViewModel 值 → UI 显示值
    fn convert_to(&self, source: &S) -> T;

    /// 反向转换：UI 输入值 → ViewModel 值（双向绑定时使用）
    fn convert_from(&self, target: &T) -> S;
}
