//! RML 语言服务器（LSP）
//!
//! 参考 Roslyn「编译器即服务」理念，复用 `rust-rml-engine` 的 parser / validator /
//! props_registry / tags / scanner 作为单一信源，LSP 仅作为薄客户端 + 语义叠加层。
//!
//! ## 架构分层
//!
//! | 层 | 职责 | Roslyn 对应 |
//! |----|------|-------------|
//! | `server` | LSP 协议传输 + 请求分发 | Protocol |
//! | `handlers` | LSP 方法处理（initialize/sync/completion/hover） | - |
//! | `workspace` | 文档表 + 项目索引 | Workspace/Document |
//! | `syntax` | 不可变语法树快照 | SyntaxTree |
//! | `semantics` | 绑定路径/命令名解析 | SemanticModel |
//! | `features` | 补全/悬停提供器（组合 syntax + semantics + engine registry） | - |

pub mod crosslang;
pub mod features;
pub mod handlers;
pub mod rust;
pub mod semantics;
pub mod server;
pub mod syntax;
pub mod workspace;

pub use server::connection::run_server;
