//! LanguageClient —— 高内聚语言服务客户端
//!
//! 封装 LSP server 进程 + IPC + provider 工厂 + grammar 注册。
//! 一个实例服务一个 LSP server；server 可支持多种语言（rust+rml 一体化）。
//!
//! ## 使用示例
//!
//! ```ignore
//! use rust_rml_client::LanguageClient;
//!
//! // 启动 rust+rml 一体化语言服务（注册 grammars + spawn rml-lsp + initialize 握手）
//! let client = LanguageClient::unified(&workspace_root)?;
//!
//! // 打开文档 + 安装 providers 到 InputState
//! // language_id 从 URI 扩展名自动推断（.rs → "rust"，.rml → "rml"）
//! client.open_document(&uri, &text);
//! client.install_providers(&mut state, uri);
//!
//! // 直访底层 LSP（formatting / rename / references 等不常用方法）
//! let rx = client.lsp().formatting(&uri);
//! ```

use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Result;
use gpui_component::highlighter::{LanguageConfig, LanguageRegistry};
use gpui_component::input::InputState;
use lsp_types::Uri;

use crate::language_profile::{LanguageDescriptor, LanguageProfile};
use crate::lsp_client::LspClient;
use crate::providers::{
    LspCompletionProvider, LspDefinitionProvider, LspHoverProvider, LspSemanticTokensProvider,
};

pub struct LanguageClient {
    profile: LanguageProfile,
    lsp: Arc<LspClient>,
}

impl LanguageClient {
    /// 通用构造：按 profile 启动 LSP server 并完成 initialize 握手。
    ///
    /// 内部步骤：
    /// 1. 遍历 `profile.languages`，为每个带 grammar 的语言注册 tree-sitter grammar
    /// 2. spawn LSP server 子进程（profile 驱动二进制解析）
    /// 3. LSP initialize 握手（缓存 semantic tokens legend）
    pub fn new(profile: LanguageProfile, workspace_root: &Path) -> Result<Self> {
        // 1. 注册所有 tree-sitter grammars
        for lang in &profile.languages {
            if let Some(grammar) = &lang.grammar {
                LanguageRegistry::singleton().register(
                    lang.language_id.as_ref(),
                    &LanguageConfig::new(
                        lang.language_id.clone(),
                        grammar.language.clone(),
                        grammar.injection_languages.clone(),
                        grammar.highlights,
                        grammar.injections,
                        grammar.locals,
                    ),
                );
            }
        }

        // 2. + 3. spawn LSP server + initialize
        let lsp = Arc::new(LspClient::spawn(&profile, workspace_root)?);

        Ok(Self { profile, lsp })
    }

    /// rust+rml 一体化便捷构造 —— `LanguageProfile::unified()` 预设
    ///
    /// 使用 crates\lsp 定制 rust-analyzer，单一 server 同时处理 rust 和 rml。
    pub fn unified(workspace_root: &Path) -> Result<Self> {
        Self::new(LanguageProfile::unified(), workspace_root)
    }

    /// 打开文档（language_id 从 URI 扩展名自动推断）
    pub fn open_document(&self, uri: &Uri, text: &str) {
        let language_id = self
            .detect_language(uri)
            .map(|d| d.language_id.as_ref())
            .unwrap_or("rml");
        self.lsp.open_document(uri, text, language_id);
    }

    /// 文档变更通知
    pub fn change_document(&self, uri: &Uri, text: &str) {
        self.lsp.change_document(uri, text);
    }

    /// 一行安装所有 LSP providers 到 `InputState`（绑定到指定 URI）
    ///
    /// 安装 completion / hover / definition / semantic_tokens 四个 provider，
    /// semantic_tokens 仅在 server 声明了 legend 时安装。
    pub fn install_providers(&self, state: &mut InputState, uri: Uri) {
        state.lsp.completion_provider =
            Some(Rc::new(LspCompletionProvider::new(self.lsp.clone(), uri.clone())));
        state.lsp.hover_provider =
            Some(Rc::new(LspHoverProvider::new(self.lsp.clone(), uri.clone())));
        state.lsp.definition_provider =
            Some(Rc::new(LspDefinitionProvider::new(self.lsp.clone(), uri.clone())));
        if let Some(legend) = self.lsp.semantic_tokens_legend() {
            state.lsp.semantic_tokens_provider =
                Some(Rc::new(LspSemanticTokensProvider::new(self.lsp.clone(), uri, legend)));
        }
    }

    /// 直访底层 `LspClient`（formatting / rename / references / document_symbol 等不常用方法）
    pub fn lsp(&self) -> &LspClient {
        &self.lsp
    }

    /// 语言 profile
    pub fn profile(&self) -> &LanguageProfile {
        &self.profile
    }

    /// 从 URI 扩展名推断对应的 `LanguageDescriptor`
    ///
    /// 遍历 `profile.languages`，匹配 `file_extensions`。未匹配时返回 None。
    fn detect_language(&self, uri: &Uri) -> Option<&LanguageDescriptor> {
        let path = uri.path().as_str();
        for lang in &self.profile.languages {
            for ext in &lang.file_extensions {
                if path.ends_with(&format!(".{ext}")) {
                    return Some(lang);
                }
            }
        }
        None
    }
}
