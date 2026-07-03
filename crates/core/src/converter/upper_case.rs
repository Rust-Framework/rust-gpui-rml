//! 转大写转换器：`"hello"` → `"HELLO"`

use super::trait_def::IConverter;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upper_case_convert() {
        assert_eq!(UpperCase.convert(&"hello".into()), "HELLO");
        assert_eq!(UpperCase.convert(&"World".into()), "WORLD");
    }
}
