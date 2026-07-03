//! DAP 消息帧编解码
//!
//! DAP 使用与 LSP 相同的 `Content-Length` 帧格式，在 stdio 上传输：
//!
//! ```text
//! Content-Length: 123\r\n
//! \r\n
//! <123 字节 JSON>
//! ```
//!
//! 本模块提供纯函数编解码（无 I/O 依赖），stdio 读写循环由 `lldb/host.rs` 驱动。

use anyhow::{Context, Result};

use super::types::Message;

/// 序列化消息为 DAP 传输帧（header + JSON body）
pub fn encode_message(message: &Message) -> Result<Vec<u8>> {
    let body = serde_json::to_vec(message).context("serialize DAP message")?;
    let mut out = Vec::with_capacity(body.len() + 32);
    out.extend_from_slice(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes());
    out.extend_from_slice(&body);
    Ok(out)
}

/// 从完整 JSON body 解析消息
pub fn decode_message(body: &[u8]) -> Result<Message> {
    serde_json::from_slice(body).context("deserialize DAP message")
}

/// 从 header 块解析 `Content-Length` 值
///
/// `headers` 为 `\r\n` 分隔的 header 文本（不含尾随 body）。
/// 返回 None 表示未找到 Content-Length。
pub fn parse_content_length(headers: &str) -> Option<usize> {
    for line in headers.split("\r\n") {
        if let Some(rest) = line.strip_prefix("Content-Length:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::types::Event;
    use serde_json::json;

    #[test]
    fn encode_decode_event_roundtrip() {
        let msg = Message::Event(Event {
            seq: 1,
            event: "stopped".to_string(),
            body: Some(json!({"reason": "breakpoint", "threadId": 2})),
        });
        let bytes = encode_message(&msg).unwrap();
        assert!(bytes.starts_with(b"Content-Length: "));
        let body_start = bytes.iter().position(|&b| b == b'\n').unwrap() + 3;
        let decoded = decode_message(&bytes[body_start..]).unwrap();
        match decoded {
            Message::Event(e) => {
                assert_eq!(e.event, "stopped");
                assert_eq!(e.seq, 1);
            }
            _ => panic!("expected Event"),
        }
    }

    #[test]
    fn parse_content_length_found() {
        let headers = "Content-Length: 42\r\n\r\n";
        assert_eq!(parse_content_length(headers), Some(42));
    }

    #[test]
    fn parse_content_length_missing() {
        let headers = "Some-Other-Header: value\r\n\r\n";
        assert_eq!(parse_content_length(headers), None);
    }

    #[test]
    fn parse_content_length_with_multiple_headers() {
        let headers = "Content-Type: application/json\r\nContent-Length: 7\r\n\r\n";
        assert_eq!(parse_content_length(headers), Some(7));
    }
}
