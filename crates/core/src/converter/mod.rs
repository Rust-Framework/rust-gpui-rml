//! `IConverter` trait —— 值转换器契约
//!
//! 用于单向/双向绑定时在 ViewModel 字段类型与 UI 显示类型之间转换。
//! 详见文档 §3.5 值转换器。
//!
//! ## 使用方式
//!
//! 在 `.rml` 中用 `|` 管道符：
//! ```html
//! <p>{price | PriceConverter}</p>
//! <input value={price | PriceConverter} />
//! <p>{value | Trim | UpperCase}</p>
//! ```
//!
//! codegen 生成 `PriceConverter::convert(&self.price)`。

mod bool_to_yes_no;
mod currency;
mod lower_case;
mod percent;
mod trait_def;
mod trim;
mod upper_case;

pub use bool_to_yes_no::BoolToYesNo;
pub use currency::Currency;
pub use lower_case::LowerCase;
pub use percent::Percent;
pub use trait_def::IConverter;
pub use trim::Trim;
pub use upper_case::UpperCase;
