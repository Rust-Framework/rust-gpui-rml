//! `#[validate]` 字段校验属性处理
//!
//! 作为字段属性（类似 `#[element]`），声明字段的校验规则（C# Attribute 风格）。
//! 宏展开时仅剥离属性，规则参数由 `crates/engine/src/build/scanner.rs` 重新提取，
//! 经 `CodegenCtx.field_validations` 传递给 codegen 生成校验代码。
//!
//! ## 设计理由
//!
//! 与 `#[computed]` 的扫描模式一致：宏在用户 crate 编译期运行，
//! codegen 在 build.rs 中运行，二者不能跨阶段直接通信。
//! 宏仅剥离属性避免编译期未识别属性警告，规则解析由 scanner 完成。
//!
//! ## 用户使用示例
//!
//! ```rust,ignore
//! #[window]
//! #[derive(Default)]
//! pub struct Form {
//!     #[validate(required, length(min = 3, max = 20))]
//!     pub name: String,
//!
//!     #[validate(range(min = 0, max = 150))]
//!     pub age: i32,
//! }
//! ```

use syn::Fields;

/// 从字段中剥离内部属性（`#[element]` 与 `#[validate]`）
///
/// 宏展开时调用，避免这些 RML 内部属性遗留导致编译期未识别属性警告。
/// 校验规则参数由 scanner 重新从源码提取（syn 静态解析）。
pub fn strip_internal_attributes(fields: &mut Fields) {
    let Fields::Named(named) = fields else {
        return;
    };
    for f in named.named.iter_mut() {
        f.attrs
            .retain(|a| !a.path().is_ident("element") && !a.path().is_ident("validate"));
    }
}
