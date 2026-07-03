//! 货币格式化转换器：`1500.0` → `"¥1500.00"`
//!
//! 注：Rust 标准格式化不支持千位分隔符，如需 `¥1,500.00` 格式请用 `num_format` crate。

use super::trait_def::IConverter;

/// 货币格式化转换器：`1500.0` → `"¥1500.00"`
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
