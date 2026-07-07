//! LSP Position 字符单位转换工具
//!
//! gpui-component 的 `RopeExt::offset_to_position` 返回 char count（Unicode 标量值），
//! 但 LSP 规范要求 `Position.character` 为 UTF-16 码元。
//! 本模块提供 byte offset → UTF-16 码元的正确转换，供 hover/definition/completion provider 使用。

use gpui_component::RopeExt;
use lsp_types::Position;
use ropey::Rope;

/// byte offset → LSP Position（UTF-16 码元）
///
/// 与 `RopeExt::offset_to_position` 的区别：本函数 `character` 字段为 UTF-16 码元计数，
/// 符合 LSP 规范。对 BMP 字符（如中文）两者一致；对辅助平面字符（如 emoji）本函数返回值更大。
pub fn offset_to_position_utf16(text: &Rope, offset: usize) -> Position {
    let point = text.offset_to_point(offset);
    let line = text.slice_line(point.row);

    let mut utf16_count = 0u32;
    let mut byte_count = 0usize;
    for c in line.chars() {
        if byte_count >= point.column {
            break;
        }
        byte_count += c.len_utf8();
        utf16_count += c.len_utf16() as u32;
    }

    Position::new(point.row as u32, utf16_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_text() {
        let rope = Rope::from("hello world");
        // offset 7 → line 0, character 7 (all ASCII, 1 char = 1 UTF-16 码元)
        let pos = offset_to_position_utf16(&rope, 7);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 7);
    }

    #[test]
    fn chinese_text() {
        // 中文：每个字符 3 bytes, 1 UTF-16 码元, 1 char
        let rope = Rope::from("你好世界");
        // offset 6 → 第 2 个字符（"世"）→ character 2
        let pos = offset_to_position_utf16(&rope, 6);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn emoji_text() {
        // emoji 😀：4 bytes, 2 UTF-16 码元, 1 char
        let rope = Rope::from("😀ab");
        // offset 4 → "a" 的位置
        // 期望 character = 2（😀 占 2 个 UTF-16 码元）
        let pos = offset_to_position_utf16(&rope, 4);
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 2);
    }

    #[test]
    fn emoji_before_attribute() {
        // 模拟 .rml 场景：emoji 后跟属性
        // `<div 😀 class="x">` 中光标在 class 上
        // "😀" = 4 bytes, 2 UTF-16; " " = 1 byte, 1 UTF-16
        // 假设光标在 "class" 的 'c' 上，byte offset = 7（"<div " = 5 bytes + "😀" = 4 bytes - 1 = ... ）
        // 实际："<div " = 5 bytes, "😀" = 4 bytes, " " = 1 byte → "class" 起始 = 5+4+1 = 10
        let src = "<div 😀 class=\"x\">";
        let rope = Rope::from(src);
        let class_offset = src.find("class").unwrap();
        let pos = offset_to_position_utf16(&rope, class_offset);
        // "😀 " 前 = 5 bytes ("<div "), 然后 "😀" = 4 bytes, " " = 1 byte
        // UTF-16: "<div " = 5 码元, "😀" = 2 码元, " " = 1 码元 → character = 8
        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 8);
    }

    #[test]
    fn multiline_text() {
        let rope = Rope::from("hello\n😀world");
        // 第二行 "😀world"，offset 6 = 行首（'\n' 后第一个字节）
        let pos = offset_to_position_utf16(&rope, 6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);

        // offset 10 = "w" 的位置（6 + 4 bytes for 😀）
        let pos = offset_to_position_utf16(&rope, 10);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 2); // 😀 占 2 UTF-16 码元
    }

    #[test]
    fn offset_at_line_start() {
        let rope = Rope::from("hello\nworld");
        let pos = offset_to_position_utf16(&rope, 6);
        assert_eq!(pos.line, 1);
        assert_eq!(pos.character, 0);
    }
}
