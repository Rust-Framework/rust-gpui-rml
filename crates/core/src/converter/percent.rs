//! 百分比格式化转换器：`0.85` → `"85%"`

use super::trait_def::IConverter;

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

#[cfg(test)]
mod tests {
    use super::*;

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
