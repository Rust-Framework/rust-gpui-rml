//! 鼠标按钮枚举

/// 鼠标按钮
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MouseButton {
    #[default]
    Left,
    Right,
    Middle,
    Other(u16),
}
