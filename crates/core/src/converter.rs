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
//! <input model={price | PriceConverter} />
//! <p>{value | Trim | UpperCase}</p>
//! ```
//!
//! codegen 生成 `PriceConverter::convert(&self.price)`。

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

// ──────────────────────────────────────────────────────────────────────────
//  内置转换器
// ──────────────────────────────────────────────────────────────────────────

/// 转大写转换器：`"hello"` → `"HELLO"`
pub struct UpperCase;

impl IConverter for UpperCase {
    type Source = String;
    type Target = String;

    fn convert(&self, value: &String) -> String {
        value.to_uppercase()
    }

    fn convert_back(&self, value: &String) -> Option<String> {
        Some(value.clone())
    }
}

/// 转小写转换器：`"HELLO"` → `"hello"`
pub struct LowerCase;

impl IConverter for LowerCase {
    type Source = String;
    type Target = String;

    fn convert(&self, value: &String) -> String {
        value.to_lowercase()
    }

    fn convert_back(&self, value: &String) -> Option<String> {
        Some(value.clone())
    }
}

/// 去除首尾空白转换器：`" hello "` → `"hello"`
pub struct Trim;

impl IConverter for Trim {
    type Source = String;
    type Target = String;

    fn convert(&self, value: &String) -> String {
        value.trim().to_string()
    }

    fn convert_back(&self, value: &String) -> Option<String> {
        Some(value.clone())
    }
}

/// 布尔转是/否转换器：`true` → `"是"`，`false` → `"否"`
pub struct BoolToYesNo;

impl IConverter for BoolToYesNo {
    type Source = bool;
    type Target = String;

    fn convert(&self, value: &bool) -> String {
        if *value {
            "是".to_string()
        } else {
            "否".to_string()
        }
    }

    fn convert_back(&self, value: &String) -> Option<bool> {
        match value.as_str() {
            "是" | "true" | "1" => Some(true),
            "否" | "false" | "0" => Some(false),
            _ => None,
        }
    }
}

/// 货币格式化转换器：`1500.0` → `"¥1500.00"`
///
/// 注：Rust 标准格式化不支持千位分隔符，如需 `¥1,500.00` 格式请用 `num_format` crate。
pub struct Currency;

impl IConverter for Currency {
    type Source = f64;
    type Target = String;

    fn convert(&self, value: &f64) -> String {
        format!("¥{:.2}", value)
    }

    fn convert_back(&self, value: &String) -> Option<f64> {
        let cleaned = value
            .trim_start_matches('¥')
            .replace(',', "")
            .trim()
            .to_string();
        cleaned.parse().ok()
    }
}

/// 百分比格式化转换器：`0.85` → `"85%"`
pub struct Percent;

impl IConverter for Percent {
    type Source = f64;
    type Target = String;

    fn convert(&self, value: &f64) -> String {
        format!("{:.0}%", value * 100.0)
    }

    fn convert_back(&self, value: &String) -> Option<f64> {
        value.trim_end_matches('%').parse::<f64>().ok().map(|v| v / 100.0)
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_case_convert() {
        assert_eq!(UpperCase.convert(&"hello".into()), "HELLO");
        assert_eq!(UpperCase.convert(&"World".into()), "WORLD");
    }

    #[test]
    fn lower_case_convert() {
        assert_eq!(LowerCase.convert(&"HELLO".into()), "hello");
        assert_eq!(LowerCase.convert(&"World".into()), "world");
    }

    #[test]
    fn trim_convert() {
        assert_eq!(Trim.convert(&"  hello  ".into()), "hello");
        assert_eq!(Trim.convert(&"\tworld\n".into()), "world");
    }

    #[test]
    fn bool_to_yes_no_convert() {
        assert_eq!(BoolToYesNo.convert(&true), "是");
        assert_eq!(BoolToYesNo.convert(&false), "否");
    }

    #[test]
    fn bool_to_yes_no_convert_back() {
        assert_eq!(BoolToYesNo.convert_back(&"是".into()), Some(true));
        assert_eq!(BoolToYesNo.convert_back(&"否".into()), Some(false));
        assert_eq!(BoolToYesNo.convert_back(&"unknown".into()), None);
    }

    #[test]
    fn currency_convert() {
        assert_eq!(Currency.convert(&1500.0), "¥1500.00");
        assert_eq!(Currency.convert(&0.5), "¥0.50");
    }

    #[test]
    fn currency_convert_back() {
        assert_eq!(Currency.convert_back(&"¥1500.00".into()), Some(1500.0));
        assert_eq!(Currency.convert_back(&"invalid".into()), None);
    }

    #[test]
    fn percent_convert() {
        assert_eq!(Percent.convert(&0.85), "85%");
        assert_eq!(Percent.convert(&1.0), "100%");
    }

    #[test]
    fn percent_convert_back() {
        assert_eq!(Percent.convert_back(&"85%".into()), Some(0.85));
        assert_eq!(Percent.convert_back(&"invalid".into()), None);
    }
}
