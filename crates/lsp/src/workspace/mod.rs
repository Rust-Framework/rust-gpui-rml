//! Roslyn Workspace 抽象：持有所有打开文档 + 项目索引

pub mod document;
pub mod project_index;

use std::collections::HashMap;

use lsp_types::Url;

use crate::semantics::model::SemanticModel;
use crate::syntax::parse::parse_document;
use crate::workspace::document::Document;
use crate::workspace::project_index::ProjectIndex;

/// Roslyn Workspace 等价物
pub struct Workspace {
    documents: HashMap<Url, Document>,
    index: ProjectIndex,
}

impl Workspace {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
            index: ProjectIndex::new(),
        }
    }

    /// 打开文档：解析 + 语义分析
    pub fn open_document(&mut self, uri: Url, text: &str, version: i32) {
        let tree = parse_document(text);
        let semantic = SemanticModel::analyze_with_uri(&tree, &self.index, &uri);
        let doc = Document::new(uri.clone(), version, tree, semantic);
        self.documents.insert(uri, doc);
    }

    /// 更新文档：重新解析 + 语义重算
    pub fn update_document(&mut self, uri: &Url, text: &str, version: i32) {
        let tree = parse_document(text);
        let semantic = SemanticModel::analyze_with_uri(&tree, &self.index, uri);
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.version = version;
            doc.tree = tree;
            doc.semantic = semantic;
        } else {
            let doc = Document::new(uri.clone(), version, tree, semantic);
            self.documents.insert(uri.clone(), doc);
        }
    }

    /// 关闭文档
    pub fn close_document(&mut self, uri: &Url) {
        self.documents.remove(uri);
    }

    /// 获取文档
    pub fn document(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }

    /// .rml.rs 变更时重扫 StructMetadata，刷新 index
    pub fn refresh_codebehind(&mut self, uri: &Url, text: &str) {
        self.index.refresh_codebehind(uri, text);
    }

    /// 注册 .rml ↔ .rml.rs 配对
    pub fn register_pair(&mut self, rml_uri: Url, rml_rs_uri: Url) {
        self.index.register_pair(rml_uri, rml_rs_uri);
    }

    /// 自动配对 .rml → .rml.rs（同目录同名约定）
    pub fn auto_pair(&mut self, rml_uri: &Url) -> Option<Url> {
        self.index.auto_pair(rml_uri)
    }

    /// 获取 .rml 对应的 code-behind URI
    pub fn codebehind_uri(&self, rml_uri: &Url) -> Option<&Url> {
        self.index.codebehind_uri(rml_uri)
    }

    /// 项目索引引用（供 features 查询）
    pub fn index(&self) -> &ProjectIndex {
        &self.index
    }
}

impl Default for Workspace {
    fn default() -> Self {
        Self::new()
    }
}
