//! DAP 消息信封类型
//!
//! DAP（Debug Adapter Protocol）使用 JSON-RPC 2.0 变体，消息经 `Content-Length` 帧分隔
//! 在 stdio 上传输。本模块定义消息信封，具体命令/事件体（`arguments`/`body`）用
//! `serde_json::Value` 承载，由 `lldb/adapter.rs` 在边界处转换为中性类型。
//!
//! 序列号约定：`seq` 为本端发出的消息递增编号；`request_seq` 用于响应中回指对应请求。

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 消息序列号
pub type Seq = u64;

// 注：协议级错误统一用 `anyhow::Error`（与 lsp crate 一致），codec 函数返回
// `anyhow::Result`，错误处通过 `.context()` 附加帧解析上下文。

/// DAP 消息（顶层枚举）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Message {
    Request(Request),
    Response(Response),
    Event(Event),
}

/// 请求（客户端 → 适配器）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    /// 序列号（发送端递增）
    pub seq: Seq,
    /// 命令名（如 "launch"/"setBreakpoints"/"stackTrace"）
    pub command: String,
    /// 命令参数（命令体，命令特定的 JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Value>,
}

/// 响应（适配器 → 客户端，对应某个请求）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// 对应请求的 seq
    pub request_seq: Seq,
    /// 是否成功
    pub success: bool,
    /// 命令名（与请求一致）
    pub command: String,
    /// 失败时的错误消息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 响应体（命令特定的 JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}

/// 事件（适配器 → 客户端，单向通知）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    /// 序列号（事件自身递增，调试客户端通常忽略）
    pub seq: Seq,
    /// 事件名（如 "stopped"/"terminated"/"output"）
    pub event: String,
    /// 事件体（事件特定的 JSON）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub body: Option<Value>,
}
