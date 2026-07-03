//! 布尔转是/否转换器：`true` → `"是"`，`false` → `"否"`

use super::trait_def::IConverter;

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
