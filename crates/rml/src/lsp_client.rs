//! LspClient: spawn `rml-lsp --stdio` 子进程，管理 LSP 协议通信。
//!
//! 设计：
//! - I/O 线程：reader 从子进程 stdout 读 Message，writer 向子进程 stdin 写 Message
//! - 请求/响应关联：`AtomicU64` 生成 ID，`HashMap<u64, Sender>` 存储待响应通道
//! - 后台接收线程：从 reader channel 读消息，Response 按 id 匹配 pending 通道

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
    _child: Child,
}

impl LspClient {
    /// spawn rml-lsp --stdio 子进程并完成 initialize 握手。
    pub fn spawn(workspace_root: &Path) -> Result<Self> {
        let bin_path = resolve_binary(workspace_root)?;

        let mut child = Command::new(&bin_path)
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| anyhow!("failed to spawn rml-lsp at {}: {}", bin_path.display(), e))?;

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
                            log::debug!("LSP notification: {}", not.method);
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

fn resolve_binary(workspace_root: &Path) -> Result<PathBuf> {
    if let Ok(path) = std::env::var("RML_LSP_PATH") {
        let p = PathBuf::from(path);
        if p.exists() {
            return Ok(p);
        }
    }
    for target_dir in ["target", "crates/lsp/target"] {
        for profile in ["debug", "release"] {
            for bin in ["rml-lsp.exe", "rml-lsp"] {
                let p = workspace_root.join(target_dir).join(profile).join(bin);
                if p.exists() {
                    return Ok(p);
                }
            }
        }
    }
    Ok(PathBuf::from("rml-lsp"))
}
