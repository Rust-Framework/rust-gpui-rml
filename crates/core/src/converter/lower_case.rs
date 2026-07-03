//! 转小写转换器：`"HELLO"` → `"hello"`

use super::trait_def::IConverter;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_case_convert() {
        assert_eq!(LowerCase.convert(&"HELLO".into()), "hello");
        assert_eq!(LowerCase.convert(&"World".into()), "world");
    }
}
