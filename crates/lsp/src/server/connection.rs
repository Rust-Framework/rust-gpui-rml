//! lsp-server 连接循环 + 主循环
//!
//! 单线程读消息 → 分发 → 写响应（rust-analyzer 模式，无 tokio）。
//! MVP 阶段所有处理在主线程同步执行（.rml 文件小，解析毫秒级）。
//! RA workspace 加载在后台线程执行（首次加载 30s+，不阻塞主循环）。

use std::path::PathBuf;
#[cfg(feature = "rust-backend")]
use std::sync::Arc;

use anyhow::Result;
use lsp_server::{Connection, Message};
use lsp_types::{InitializeParams, ServerCapabilities};

use crate::rust::RustSemanticQuery;
#[cfg(not(feature = "rust-backend"))]
use crate::rust::NoopQuery;
#[cfg(feature = "rust-backend")]
use crate::rust::{RaAdapter, RaHost};
use crate::server::dispatch;
use crate::workspace::Workspace;

/// 服务端共享状态（主线程持有，同步处理）
pub struct ServerState {
    pub workspace: Workspace,
    pub rust_query: Box<dyn RustSemanticQuery>,
    /// RA 后端句柄（feature gated）：后台线程加载完成后查询自动生效
    #[cfg(feature = "rust-backend")]
    pub ra_host: Arc<RaHost>,
    /// 从 initialize 参数提取的工作区根路径
    pub root_path: Option<PathBuf>,
    pub shutdown_requested: bool,
}

impl ServerState {
    pub fn new() -> Self {
        #[cfg(feature = "rust-backend")]
        let host = Arc::new(RaHost::new());
        Self {
            workspace: Workspace::new(),
            rust_query: {
                #[cfg(feature = "rust-backend")]
                {
                    Box::new(RaAdapter::new(Arc::clone(&host)))
                }
                #[cfg(not(feature = "rust-backend"))]
                {
                    Box::new(NoopQuery)
                }
            },
            #[cfg(feature = "rust-backend")]
            ra_host: host,
            root_path: None,
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
    let init_params_value = connection.initialize(serde_json::to_value(&capabilities)?)?;
    let init_params: InitializeParams = serde_json::from_value(init_params_value)?;
    let root_path = extract_root_path(&init_params);
    log::info!(
        "initialize handshake complete, root_path: {:?}",
        root_path
    );

    let mut state = ServerState::new();
    state.root_path = root_path;

    main_loop(&connection, &mut state)?;

    io_threads.join()?;
    log::info!("rml-lsp stopped");
    Ok(())
}

/// 从 InitializeParams 提取工作区根路径
#[allow(deprecated)]
fn extract_root_path(params: &InitializeParams) -> Option<PathBuf> {
    if let Some(uri) = &params.root_uri {
        if let Ok(path) = uri.to_file_path() {
            return Some(path);
        }
    }
    if let Some(folders) = &params.workspace_folders {
        if let Some(first) = folders.first() {
            if let Ok(path) = first.uri.to_file_path() {
                return Some(path);
            }
        }
    }
    None
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
pub fn build_capabilities() -> ServerCapabilities {
    use lsp_types::{
        CompletionOptions, HoverProviderCapability, SignatureHelpOptions,
        TextDocumentSyncCapability, TextDocumentSyncKind,
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
        definition_provider: Some(lsp_types::OneOf::Left(true)),
        references_provider: Some(lsp_types::OneOf::Left(true)),
        document_symbol_provider: Some(lsp_types::OneOf::Left(true)),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        signature_help_provider: Some(SignatureHelpOptions {
            trigger_characters: Some(vec![",".to_string(), "(".to_string()]),
            retrigger_characters: None,
            work_done_progress_options: Default::default(),
        }),
        rename_provider: Some(lsp_types::OneOf::Left(true)),
        folding_range_provider: Some(lsp_types::FoldingRangeProviderCapability::Simple(true)),
        semantic_tokens_provider: Some(
            lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                lsp_types::SemanticTokensOptions {
                    legend: lsp_types::SemanticTokensLegend {
                        token_types: crate::semantics::tokens::RML_TOKEN_TYPES.to_vec(),
                        token_modifiers: crate::semantics::tokens::RML_TOKEN_MODIFIERS.to_vec(),
                    },
                    range: Some(true),
                    full: Some(lsp_types::SemanticTokensFullOptions::Bool(true)),
                    ..Default::default()
                },
            ),
        ),
        ..Default::default()
    }
}
