//! 项目索引：.rml ↔ .rml.rs 配对 + StructMetadata 缓存
//!
//! 复用 `engine::build::scanner::parse_struct_metadata`（内存纯函数入口），
//! 不重复实现 syn 解析。

use std::collections::HashMap;

use lsp_types::Url;
use rust_rml_engine::build::scanner::{parse_struct_metadata, StructMetadata};

/// 项目索引
pub struct ProjectIndex {
    /// rml_rs_uri → StructMetadata 表（按 struct_name 索引）
    metadata: HashMap<Url, HashMap<String, StructMetadata>>,
    /// .rml uri → 配对的 .rml.rs uri（同目录同名约定）
    rml_to_codebehind: HashMap<Url, Url>,
}

impl ProjectIndex {
    pub fn new() -> Self {
        Self {
            metadata: HashMap::new(),
            rml_to_codebehind: HashMap::new(),
        }
    }

    /// 从 .rml.rs 源码刷新 StructMetadata 缓存
    pub fn refresh_codebehind(&mut self, rml_rs_uri: &Url, source: &str) {
        let parsed = parse_struct_metadata(source);
        self.metadata.insert(rml_rs_uri.clone(), parsed);
    }

    /// 注册 .rml ↔ .rml.rs 配对
    pub fn register_pair(&mut self, rml_uri: Url, rml_rs_uri: Url) {
        self.rml_to_codebehind.insert(rml_uri, rml_rs_uri);
    }

    /// 获取 .rml 对应 code-behind 的 StructMetadata
    ///
    /// 若无配对或未刷新，返回 None（调用方降级为空补全列表）。
    pub fn metadata_for(&self, rml_uri: &Url) -> Option<&HashMap<String, StructMetadata>> {
        let rml_rs_uri = self.rml_to_codebehind.get(rml_uri)?;
        self.metadata.get(rml_rs_uri)
    }

    /// 获取 .rml 对应 code-behind 的所有 struct 名称
    pub fn struct_names_for(&self, rml_uri: &Url) -> Vec<&str> {
        self.metadata_for(rml_uri)
            .map(|m| m.keys().map(|s| s.as_str()).collect())
            .unwrap_or_default()
    }
}

impl Default for ProjectIndex {
    fn default() -> Self {
        Self::new()
    }
}
