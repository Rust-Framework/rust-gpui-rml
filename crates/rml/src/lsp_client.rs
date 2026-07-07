//! LspClient: spawn LSP server 子进程，管理 LSP 协议通信。
//!
//! 设计：
//! - I/O 线程：reader 从子进程 stdout 读 Message，writer 向子进程 stdin 写 Message
//! - 请求/响应关联：`AtomicU64` 生成 ID，`HashMap<u64, Sender>` 存储待响应通道
//! - 后台接收线程：从 reader channel 读消息，Response 按 id 匹配 pending 通道
//!
//! `LspClient` 是 `LanguageClient` 的内部 IPC 层，外部应优先使用 `LanguageClient`。

use std::collections::HashMap;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::{anyhow, Result};
use crossbeam_channel::{Receiver, Sender, unbounded};
use lsp_server::{Message, Notification, Request, RequestId};
use lsp_types::{Position, Uri};
use serde_json::Value;

use crate::language_profile::LanguageProfile;

/// LSP server 加载状态（由 server 端 `rml/serverStatus` 通知驱动）
#[derive(Clone, Debug, PartialEq)]
pub enum ServerStatus {
    /// 正在加载 workspace
    Loading,
    /// 加载完成，可提供完整语义服务
    Ready,
    /// 加载失败
    Error(String),
}

/// 将文件路径转换为 `lsp_types::Uri`（lsp-types 0.97 用 Uri 替代 Url）。
pub fn file_path_to_uri(path: &Path) -> Result<Uri> {
    let url = url::Url::from_file_path(path).map_err(|_| anyhow!("invalid file path: {}", path.display()))?;
    let uri_str = url.as_str();
    uri_str.parse::<Uri>().map_err(|e| anyhow!("invalid URI {uri_str}: {e}"))
}

pub struct LspClient {
    writer_tx: Sender<Message>,
    next_id: AtomicU64,
    pending: Arc<Mutex<HashMap<u64, Sender<Result<Value>>>>>,
    doc_version: AtomicU32,
    semantic_tokens_legend: Mutex<Option<lsp_types::SemanticTokensLegend>>,
    status_rx: Receiver<ServerStatus>,
    _child: Child,
}

impl LspClient {
    /// spawn LSP server 子进程并完成 initialize 握手。
    ///
    /// `profile` 描述 server 二进制名、参数与搜索路径；`workspace_root` 用于
    /// `initialize` 的 rootUri 与二进制相对路径解析。
    pub fn spawn(profile: &LanguageProfile, workspace_root: &Path) -> Result<Self> {
        let bin_path = resolve_binary(profile, workspace_root)?;

        let mut cmd = Command::new(&bin_path);
        for arg in &profile.server_args {
            cmd.arg(arg);
        }
        let mut child = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("failed to spawn {} at {}: {}", profile.server_binary, bin_path.display(), e))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("failed to take child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow!("failed to take child stdout"))?;

        let (writer_tx, writer_rx) = unbounded::<Message>();
        let pending: Arc<Mutex<HashMap<u64, Sender<Result<Value>>>>> =
            Arc::new(Mutex::new(HashMap::new()));
        let pending_for_reader = pending.clone();
        let (status_tx, status_rx) = unbounded::<ServerStatus>();

        // Writer 线程：从 channel 接收 Message → 写入子进程 stdin
        thread::Builder::new()
            .name("LspClientWriter".to_owned())
            .spawn(move || {
                let mut stdin = stdin;
                for msg in writer_rx {
                    if let Err(e) = msg.write(&mut stdin) {
                        log::error!("LSP write error: {e}");
                        break;
                    }
                }
            })?;

        // Reader 线程：从子进程 stdout 读 Message → 分发
        thread::Builder::new()
            .name("LspClientReader".to_owned())
            .spawn(move || {
                let mut reader = BufReader::new(stdout);
                while let Ok(Some(msg)) = Message::read(&mut reader) {
                    match msg {
                        Message::Response(resp) => {
                            if let Some(id) = request_id_to_u64(&resp.id) {
                                let mut map = pending_for_reader.lock().unwrap();
                                if let Some(tx) = map.remove(&id) {
                                    let result = if let Some(err) = &resp.error {
                                        Err(anyhow!(
                                            "LSP error {}: {}",
                                            err.code,
                                            err.message
                                        ))
                                    } else {
                                        Ok(resp.result.unwrap_or(Value::Null))
                                    };
                                    let _ = tx.send(result);
                                }
                            }
                        }
                        Message::Notification(not) => {
                            if not.method == "rml/serverStatus" {
                                if let Some(status) = parse_server_status(&not.params) {
                                    let _ = status_tx.send(status);
                                }
                            } else {
                                log::debug!("LSP notification: {}", not.method);
                            }
                        }
                        Message::Request(req) => {
                            log::debug!("LSP server request: {}", req.method);
                        }
                    }
                }
                // 子进程关闭：通知所有 pending 请求失败
                let mut map = pending_for_reader.lock().unwrap();
                for (_, tx) in map.drain() {
                    let _ = tx.send(Err(anyhow!("LSP server closed")));
                }
            })?;

        let client = Self {
            writer_tx,
            next_id: AtomicU64::new(1),
            pending,
            doc_version: AtomicU32::new(0),
            semantic_tokens_legend: Mutex::new(None),
            status_rx,
            _child: child,
        };

        client.initialize(workspace_root)?;

        Ok(client)
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::SeqCst)
    }

    fn next_version(&self) -> u32 {
        self.doc_version.fetch_add(1, Ordering::SeqCst) + 1
    }

    fn send_request(&self, method: &str, params: Value) -> Receiver<Result<Value>> {
        let id = self.next_id();
        let (tx, rx) = unbounded();
        self.pending.lock().unwrap().insert(id, tx);

        let request = Request {
            id: RequestId::from(id as i32),
            method: method.to_owned(),
            params,
        };
        let _ = self.writer_tx.send(request.into());
        rx
    }

    fn send_notification(&self, method: &str, params: Value) {
        let not = Notification {
            method: method.to_owned(),
            params,
        };
        let _ = self.writer_tx.send(not.into());
    }

    fn initialize(&self, workspace_root: &Path) -> Result<()> {
        let root_uri = file_path_to_uri(workspace_root)?;

        let params = serde_json::json!({
            "processId": null,
            "rootUri": root_uri.as_str(),
            "capabilities": {},
        });

        let rx = self.send_request("initialize", params);
        let result = rx
            .recv()
            .map_err(|e| anyhow!("initialize recv error: {e}"))??;
        log::info!("LSP initialize success: {:?}", result);

        // 缓存 semantic tokens legend（供 CodeEditorTab 安装 provider 时读取）
        let legend = result
            .get("capabilities")
            .and_then(|c| c.get("semanticTokensProvider"))
            .and_then(|p| p.get("legend"))
            .and_then(|l| serde_json::from_value::<lsp_types::SemanticTokensLegend>(l.clone()).ok());
        if let Some(lg) = legend {
            *self.semantic_tokens_legend.lock().unwrap() = Some(lg);
        }

        self.send_notification("initialized", serde_json::json!({}));

        Ok(())
    }

    pub fn open_document(&self, uri: &Uri, text: &str, language_id: &str) {
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.as_str(),
                "languageId": language_id,
                "version": 0,
                "text": text,
            }
        });
        self.send_notification("textDocument/didOpen", params);
    }

    pub fn change_document(&self, uri: &Uri, text: &str) {
        let version = self.next_version();
        let params = serde_json::json!({
            "textDocument": {
                "uri": uri.as_str(),
                "version": version,
            },
            "contentChanges": [{
                "text": text,
            }],
        });
        self.send_notification("textDocument/didChange", params);
    }

    pub fn completion(&self, uri: &Uri, position: Position) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        });
        self.send_request("textDocument/completion", params)
    }

    pub fn hover(&self, uri: &Uri, position: Position) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        });
        self.send_request("textDocument/hover", params)
    }

    pub fn definition(&self, uri: &Uri, position: Position) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        });
        self.send_request("textDocument/definition", params)
    }

    pub fn references(
        &self,
        uri: &Uri,
        position: Position,
        include_declaration: bool,
    ) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
            "context": { "includeDeclaration": include_declaration },
        });
        self.send_request("textDocument/references", params)
    }

    pub fn document_symbol(&self, uri: &Uri) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
        });
        self.send_request("textDocument/documentSymbol", params)
    }

    pub fn folding_range(&self, uri: &Uri) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
        });
        self.send_request("textDocument/foldingRange", params)
    }

    pub fn formatting(&self, uri: &Uri) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "options": { "tabSize": 2, "insertSpaces": true },
        });
        self.send_request("textDocument/formatting", params)
    }

    pub fn signature_help(&self, uri: &Uri, position: Position) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
        });
        self.send_request("textDocument/signatureHelp", params)
    }

    pub fn rename(
        &self,
        uri: &Uri,
        position: Position,
        new_name: &str,
    ) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
            "position": position,
            "newName": new_name,
        });
        self.send_request("textDocument/rename", params)
    }

    /// 返回 LSP server 在 initialize 阶段声明的 semantic tokens legend
    ///
    /// 供 `RmlSemanticTokensProvider::new` 读取，避免 provider 实例化时硬编码 legend。
    pub fn semantic_tokens_legend(&self) -> Option<lsp_types::SemanticTokensLegend> {
        self.semantic_tokens_legend.lock().unwrap().clone()
    }

    /// 返回 server 状态接收器（`rml/serverStatus` 通知驱动）
    ///
    /// 调用方应在后台 task 中 `recv()` 此 receiver，状态变化时更新 UI。
    pub fn status_receiver(&self) -> Receiver<ServerStatus> {
        self.status_rx.clone()
    }

    /// textDocument/semanticTokens/full
    ///
    /// 拉取整个文档的 semantic tokens（delta 编码的 `SemanticTokens`）。
    /// gpui-component 的 `Lsp::update_semantic_tokens` 内部已做 viewport 过滤，
    /// provider 调本方法取全量后由 gpui-component 端裁剪。
    pub fn semantic_tokens_full(&self, uri: &Uri) -> Receiver<Result<Value>> {
        let params = serde_json::json!({
            "textDocument": { "uri": uri.as_str() },
        });
        self.send_request("textDocument/semanticTokens/full", params)
    }
}

fn request_id_to_u64(id: &RequestId) -> Option<u64> {
    let value = serde_json::to_value(id).ok()?;
    match value {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn resolve_binary(profile: &LanguageProfile, workspace_root: &Path) -> Result<PathBuf> {
    // 1. 环境变量覆盖（如 RML_LSP_PATH / RA_PATH）
    if let Some(env_var) = profile.server_path_env {
        if let Ok(path) = std::env::var(env_var) {
            let p = PathBuf::from(path);
            if p.exists() {
                return Ok(p);
            }
        }
    }
    // 2. workspace_root 下的搜索路径（target/debug, target/release 等）
    let bins = if cfg!(windows) {
        vec![format!("{}.exe", profile.server_binary), profile.server_binary.clone()]
    } else {
        vec![profile.server_binary.clone()]
    };
    for search_dir in &profile.server_search_paths {
        for build_profile in ["debug", "release"] {
            for bin in &bins {
                let p = workspace_root.join(search_dir).join(build_profile).join(bin);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    // 3. 回退：依赖 PATH 查找（返回二进制名，由 OS 解析）
    Ok(PathBuf::from(&profile.server_binary))
}

/// 解析 `rml/serverStatus` 通知参数为 `ServerStatus`
fn parse_server_status(params: &Value) -> Option<ServerStatus> {
    let status = params.get("status")?.as_str()?;
    let message = params.get("message").and_then(|v| v.as_str()).unwrap_or("");
    match status {
        "loading" => Some(ServerStatus::Loading),
        "ready" => Some(ServerStatus::Ready),
        "error" => Some(ServerStatus::Error(message.to_string())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_loading_status() {
        let params = serde_json::json!({ "status": "loading", "message": "Loading..." });
        assert_eq!(parse_server_status(&params), Some(ServerStatus::Loading));
    }

    #[test]
    fn parse_ready_status() {
        let params = serde_json::json!({ "status": "ready", "message": "ready" });
        assert_eq!(parse_server_status(&params), Some(ServerStatus::Ready));
    }

    #[test]
    fn parse_error_status() {
        let params = serde_json::json!({ "status": "error", "message": "load failed" });
        assert_eq!(
            parse_server_status(&params),
            Some(ServerStatus::Error("load failed".to_string()))
        );
    }

    #[test]
    fn parse_unknown_status_returns_none() {
        let params = serde_json::json!({ "status": "unknown" });
        assert_eq!(parse_server_status(&params), None);
    }

    #[test]
    fn parse_missing_status_field_returns_none() {
        let params = serde_json::json!({ "message": "no status" });
        assert_eq!(parse_server_status(&params), None);
    }
}
