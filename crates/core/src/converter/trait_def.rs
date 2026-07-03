//! `IConverter` trait 定义

/// 值转换器 trait。
///
/// - `Source`：ViewModel 侧的类型
/// - `Target`：UI 侧的类型
///
/// 实现此 trait 的类型可在 `.rml` 中通过 `|` 管道符使用：
/// ```html
/// <p>{price | PriceConverter}</p>
/// ```
///
/// 双向绑定时 `convert` 用于 ViewModel → UI，`convert_back` 用于 UI → ViewModel。
pub trait IConverter: Send + Sync {
    /// ViewModel 侧的类型
    type Source;
    /// UI 侧的类型
    type Target;

    /// 正向转换：ViewModel 值 → UI 显示值
    fn convert(&self, value: &Self::Source) -> Self::Target;

    /// 反向转换：UI 输入值 → ViewModel 值（双向绑定时使用）
    ///
    /// 返回 `Option` 表示反向转换可能失败。失败时 RML 保持 ViewModel 字段不变。
    fn convert_back(&self, value: &Self::Target) -> Option<Self::Source>;
}
