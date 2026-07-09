//! Kbd 专用属性 setter
//!
//! - `outline=""` / `outline="true"` → `.outline()`
//! - `appearance="false"` → `.appearance(false)`（appearance 默认 true，仅在 false 时显式设置）
//! - `appearance="true"` → 无操作（默认值）

/// Kbd 专用静态属性 setter
///
/// - `outline=""` / `outline="true"` → `.outline()`
/// - `appearance="false"` → `.appearance(false)`（appearance 默认 true，仅在 false 时显式设置）
/// - `appearance="true"` → 无操作（默认值）
pub fn kbd_static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "outline" if value.is_empty() || value.eq_ignore_ascii_case("true") => {
            Some(".outline()".into())
        }
        "appearance" if value.eq_ignore_ascii_case("false") => {
            Some(".appearance(false)".into())
        }
        "appearance" if value.is_empty() || value.eq_ignore_ascii_case("true") => {
            // 默认值，无需生成代码
            Some(String::new())
        }
        _ => None,
    }
}
