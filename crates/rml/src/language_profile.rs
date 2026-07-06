//! LanguageProfile / LanguageDescriptor / TreeSitterGrammar / DebugProfile —— 语言服务配置预设
//!
//! RML 框架的 LSP/DAP 体系是 rust+rml 一体化设计：`crates\lsp` 通过直接引入
//! rust-analyzer 源码定制开发，单一 server 同时处理 rust 和 rml。
//!
//! - `LanguageProfile` 描述一个 LSP server（二进制 + 启动参数 + 支持的语言列表）
//! - `LanguageDescriptor` 描述该 server 支持的某一种语言（language_id + 扩展名 + grammar）
//! - `LanguageProfile::unified()` —— rust+rml 一体化预设（crates\lsp 定制 rust-analyzer）

use gpui::SharedString;

/// Tree-sitter 语法包（可选 —— 内置语言无需提供）
#[derive(Clone)]
pub struct TreeSitterGrammar {
    pub language: tree_sitter::Language,
    pub highlights: &'static str,
    pub injections: &'static str,
    pub locals: &'static str,
    pub injection_languages: Vec<SharedString>,
}

/// 单语言描述 —— LSP server 支持的某一种语言
#[derive(Clone)]
pub struct LanguageDescriptor {
    /// LSP 协议 language id（"rust" / "rml"）
    pub language_id: SharedString,
    /// 文件扩展名列表（["rs"] / ["rml"]）
    pub file_extensions: Vec<SharedString>,
    /// Tree-sitter 语法包（None = 依赖 gpui-component 内置）
    pub grammar: Option<TreeSitterGrammar>,
}

/// 语言服务配置 —— 描述如何启动并与一个 LSP server 交互
///
/// 一个 server 可支持多种语言（rust+rml 一体化），通过 `languages` 字段声明。
#[derive(Clone)]
pub struct LanguageProfile {
    /// LSP server 二进制名（"rml-lsp" —— crates\lsp 定制 rust-analyzer）
    pub server_binary: String,
    /// LSP server 启动参数（["--stdio"]）
    pub server_args: Vec<String>,
    /// 覆盖二进制路径的环境变量名（"RML_LSP_PATH"）
    pub server_path_env: Option<&'static str>,
    /// 相对 workspace_root 的二进制搜索路径（["target", "crates/lsp/target"]）
    pub server_search_paths: Vec<&'static str>,
    /// 该 server 支持的所有语言
    pub languages: Vec<LanguageDescriptor>,
}

impl LanguageProfile {
    /// rust+rml 一体化预设 —— 使用 crates\lsp 定制 rust-analyzer
    ///
    /// 单一 server 同时处理 rust 和 rml：
    /// - rust: gpui-component 内置 Rust grammar（`tree-sitter-languages` feature）
    /// - rml: crates/rml 自带 tree-sitter grammar
    pub fn unified() -> Self {
        Self {
            server_binary: "rml-lsp".to_string(),
            server_args: vec!["--stdio".to_string()],
            server_path_env: Some("RML_LSP_PATH"),
            server_search_paths: vec!["target", "crates/lsp/target"],
            languages: vec![
                LanguageDescriptor {
                    language_id: "rust".into(),
                    file_extensions: vec!["rs".into()],
                    grammar: None,
                },
                LanguageDescriptor {
                    language_id: "rml".into(),
                    file_extensions: vec!["rml".into()],
                    grammar: Some(TreeSitterGrammar {
                        language: tree_sitter::Language::new(crate::grammar::language()),
                        highlights: crate::grammar::HIGHLIGHTS_QUERY,
                        injections: crate::grammar::INJECTIONS_QUERY,
                        locals: "",
                        injection_languages: vec!["rust".into()],
                    }),
                },
            ],
        }
    }
}

/// 调试适配器配置 —— 描述如何启动一个 DAP debug adapter
///
/// RML 编译到 Rust，lldb-vscode 同时调试两者，故无需 language_id 字段。
#[derive(Clone)]
pub struct DebugProfile {
    pub adapter_binary: String,
    pub adapter_args: Vec<String>,
    pub adapter_path_env: Option<&'static str>,
}

impl DebugProfile {
    /// rust+rml 一体化调试预设 —— 使用 lldb-vscode
    pub fn unified() -> Self {
        Self {
            adapter_binary: "lldb-vscode".to_string(),
            adapter_args: vec![],
            adapter_path_env: Some("LLDB_VSCODE_PATH"),
        }
    }
}
