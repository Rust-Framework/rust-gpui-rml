//! HoverCard 专用属性 setter
//!
//! ## 属性映射
//!
//! - `anchor="top-left"` → `.anchor(gpui::Anchor::TopLeft)`
//! - `appearance="false"` → `.appearance(false)`
//! - `open_delay="600"` → `.open_delay(std::time::Duration::from_millis(600))`
//! - `close_delay="300"` → `.close_delay(std::time::Duration::from_millis(300))`

/// HoverCard 专用静态属性 setter
pub fn static_setter(name: &str, value: &str) -> Option<String> {
    match name {
        "anchor" => {
            let anchor = match value {
                "top-left" => "TopLeft",
                "top-center" => "TopCenter",
                "top-right" => "TopRight",
                "bottom-left" => "BottomLeft",
                "bottom-center" => "BottomCenter",
                "bottom-right" => "BottomRight",
                "left-center" => "LeftCenter",
                "right-center" => "RightCenter",
                _ => return None,
            };
            Some(format!(".anchor(gpui::Anchor::{})", anchor))
        }
        "appearance" => {
            if value.eq_ignore_ascii_case("false") {
                Some(".appearance(false)".into())
            } else {
                Some(String::new())
            }
        }
        "open_delay" => {
            let ms: u64 = value.parse().ok()?;
            Some(format!(".open_delay(std::time::Duration::from_millis({}))", ms))
        }
        "close_delay" => {
            let ms: u64 = value.parse().ok()?;
            Some(format!(".close_delay(std::time::Duration::from_millis({}))", ms))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_setter_anchor() {
        assert_eq!(
            static_setter("anchor", "top-left"),
            Some(".anchor(gpui::Anchor::TopLeft)".to_string())
        );
        assert_eq!(
            static_setter("anchor", "bottom-center"),
            Some(".anchor(gpui::Anchor::BottomCenter)".to_string())
        );
    }

    #[test]
    fn static_setter_appearance_false() {
        assert_eq!(static_setter("appearance", "false"), Some(".appearance(false)".into()));
    }

    #[test]
    fn static_setter_appearance_true_no_op() {
        let s = static_setter("appearance", "true").unwrap();
        assert!(s.is_empty());
    }

    #[test]
    fn static_setter_open_delay() {
        assert_eq!(
            static_setter("open_delay", "600"),
            Some(".open_delay(std::time::Duration::from_millis(600))".to_string())
        );
    }

    #[test]
    fn static_setter_close_delay() {
        assert_eq!(
            static_setter("close_delay", "300"),
            Some(".close_delay(std::time::Duration::from_millis(300))".to_string())
        );
    }

    #[test]
    fn static_setter_open_delay_invalid() {
        assert_eq!(static_setter("open_delay", "abc"), None);
    }

    #[test]
    fn static_setter_unknown() {
        assert_eq!(static_setter("unknown", "x"), None);
    }
}
