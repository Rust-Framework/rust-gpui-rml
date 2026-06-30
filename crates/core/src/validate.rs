//! 校验接口（Phase B-3.3）
//!
//! 用户可通过 `#[validate(MyValidator)]` 引用实现 `IValidate` 的类型，
//! 自定义校验逻辑（含跨字段校验）。规则式（range/length/required/regex/custom）
//! 与接口式（IValidate）互斥。

use gpui::SharedString;

/// 校验结果
///
/// - `Pass`：通过
/// - `Fail(msg)`：失败，附带默认错误消息
#[derive(Debug, Clone)]
pub enum ValidResult {
    Pass,
    Fail(SharedString),
}

/// 校验接口
///
/// 实现此 trait 的类型可作为 `#[validate(MyValidator)]` 引用。
/// 类型必须实现 `Default`（codegen 通过 `MyValidator::default()` 构造实例）。
///
/// # 方法
///
/// - `valid(&self, value: &str) -> ValidResult`：简单校验（仅根据 value 判断）
/// - `valid_with_view(&self, value: &str, view: &dyn Any) -> ValidResult`：带视图上下文校验
///   （默认委托给 `valid`，重写后可访问 view 的其他字段进行跨字段校验）
/// - `message(&self, result: &ValidResult) -> Option<SharedString>`：结果→消息转换
///   （默认从 `Fail(msg)` 提取，可重写以实现 i18n 或自定义映射）
///
/// # view 参数说明
///
/// `valid_with_view` 的 `view: &dyn Any` 是视图结构体引用（即 `&self`）。
/// 实现者需 `view.downcast_ref::<MyView>()` 取回具体类型。此设计让 validator
/// 无需自行获取外部状态，所有依赖由 codegen 注入。
///
/// # 示例
///
/// ```rust,ignore
/// use rml_core::validate::{IValidate, ValidResult};
///
/// #[derive(Default)]
/// struct EmailValidator;
///
/// impl IValidate for EmailValidator {
///     fn valid(&self, value: &str) -> ValidResult {
///         if value.contains('@') {
///             ValidResult::Pass
///         } else {
///             ValidResult::Fail("邮箱格式错误".into())
///         }
///     }
/// }
/// ```
pub trait IValidate: Default + Send + Sync {
    /// 简单校验：仅根据 value 判断
    fn valid(&self, value: &str) -> ValidResult {
        let _ = value;
        ValidResult::Pass
    }

    /// 带视图上下文的校验：可访问 view 的其他字段
    ///
    /// 默认实现委托给 `valid`。重写后可通过 `view.downcast_ref::<MyView>()` 访问跨字段。
    fn valid_with_view(&self, value: &str, view: &dyn std::any::Any) -> ValidResult {
        let _ = view;
        self.valid(value)
    }

    /// 将校验结果转换为错误消息
    ///
    /// - 返回 `None`：校验通过（不显示错误）
    /// - 返回 `Some(msg)`：校验失败，UI 显示红色边框 + tooltip(msg)
    fn message(&self, result: &ValidResult) -> Option<SharedString> {
        match result {
            ValidResult::Pass => None,
            ValidResult::Fail(msg) => Some(msg.clone()),
        }
    }
}
