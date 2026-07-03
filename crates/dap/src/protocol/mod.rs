//! DAP 协议层
//!
//! 对标 lsp crate 中 `lsp-server`/`lsp-types` 的职责，但 DAP 协议类型手写最小子集
//! （不引入外部 dap crate，避免版本锁定与不成熟依赖）。
//!
//! - `types`：DAP 消息信封（Request/Response/Event/Message + 序列号管理）
//! - `codec`：`Content-Length` 帧编解码，stdio 读写适配

pub mod codec;
pub mod types;

pub use codec::{decode_message, encode_message};
pub use types::{Event, Message, Request, Response, Seq};
