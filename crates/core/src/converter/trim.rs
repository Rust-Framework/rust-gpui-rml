//! 去除首尾空白转换器：`" hello "` → `"hello"`

use super::trait_def::IConverter;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_convert() {
        assert_eq!(Trim.convert(&"  hello  ".into()), "hello");
        assert_eq!(Trim.convert(&"\tworld\n".into()), "world");
    }
}
