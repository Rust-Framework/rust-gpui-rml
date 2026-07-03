//! lsp-server 连接循环 + 主循环
//!
//! 单线程读消息 → 分发 → 写响应（rust-analyzer 模式，无 tokio）。
//! MVP 阶段所有处理在主线程同步执行（.rml 文件小，解析毫秒级）。

use anyhow::Result;
use lsp_server::{Connection, Message};
use lsp_types::ServerCapabilities;

use crate::server::dispatch;
use crate::workspace::Workspace;

/// 服务端共享状态（主线程持有，同步处理）
pub struct ServerState {
    pub workspace: Workspace,
    pub shutdown_requested: bool,
}

impl ServerState {
    pub fn new() -> Self {
        Self {
            workspace: Workspace::new(),
            shutdown_requested: false,
        }
    }
}

impl Default for ServerState {
    fn default() -> Self {
        Self::new()
    }
}

/// 启动 LSP 服务器（stdio 传输）
pub fn run_server() -> Result<()> {
    log::info!("rml-lsp starting on stdio");
    let (connection, io_threads) = Connection::stdio();

    let capabilities = build_capabilities();
    let _params = connection.initialize(serde_json::to_value(&capabilities)?)?;
    log::info!("initialize handshake complete");

    let mut state = ServerState::new();

    main_loop(&connection, &mut state)?;

    io_threads.join()?;
    log::info!("rml-lsp stopped");
    Ok(())
}

/// 主消息循环
fn main_loop(connection: &Connection, state: &mut ServerState) -> Result<()> {
    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                // shutdown 是特殊请求：handle_shutdown 返回 true 表示收到 shutdown
                if connection.handle_shutdown(&req)? {
                    state.shutdown_requested = true;
                    break;
                }
                dispatch::handle_request(req, state, connection)?;
            }
            Message::Notification(not) => {
                dispatch::handle_notification(not, state, connection)?;
            }
            Message::Response(_) => {}
        }
    }
    Ok(())
}

/// 构建 ServerCapabilities
fn build_capabilities() -> ServerCapabilities {
    use lsp_types::{
        CompletionOptions, HoverProviderCapability, TextDocumentSyncCapability,
        TextDocumentSyncKind,
    };

    ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["<".to_string(), " ".to_string(), "{".to_string()]),
            resolve_provider: Some(false),
            work_done_progress_options: Default::default(),
            all_commit_characters: None,
            completion_item: None,
        }),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: None,
        references_provider: None,
        document_formatting_provider: None,
        ..Default::default()
    }
}
