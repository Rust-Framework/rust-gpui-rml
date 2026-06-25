//! `IModel` trait —— RML 响应式数据模型的基础标记
//!
//! `#[derive(IModel)]` 派生此 trait。纯数据 Model 与 ViewModel 均需实现。
//! IModel 本身不引用 GPUI 类型，纯数据 Model 可在无窗口环境使用。

/// RML 响应式模型基础 trait。
///
/// 所有 `#[derive(IModel)]` 的结构体自动实现此 trait。
/// `pub` 字段自动成为可绑定字段，`.rml` 模板中可通过 `{field}` 访问。
pub trait IModel: 'static + Send + Sync {
    /// 返回字段元信息（名称、类型），供绑定引擎在编译期/运行期使用。
    /// MVP 阶段返回空切片，阶段二由派生宏生成实际元信息。
    fn rml_fields(&self) -> &'static [FieldMeta] {
        &[]
    }
}

/// 字段元信息
#[derive(Debug, Clone)]
pub struct FieldMeta {
    pub name: &'static str,
    pub ty: &'static str,
}
