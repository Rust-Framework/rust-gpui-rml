//! Translator 注册表
//!
//! 按标签名索引所有 `IRmlTranslator` 实现，提供统一查询与分类能力。

use super::{ComponentCategory, IRmlTranslator, TranslatorMetadata};
use crate::parser::ast::Element;
use std::collections::HashMap;
use std::sync::Arc;

/// Translator 注册表
///
/// 内部使用 `Arc` 保证 `Clone` 低成本，便于存入 `CodegenCtx`。
#[derive(Debug, Clone)]
pub struct TranslatorRegistry {
    translators: Arc<HashMap<&'static str, Arc<dyn IRmlTranslator>>>,
}

impl Default for TranslatorRegistry {
    /// 默认注册表包含所有内置 translator
    fn default() -> Self {
        Self::builtin()
    }
}

impl TranslatorRegistry {
    /// 创建空注册表
    pub fn empty() -> Self {
        Self {
            translators: Arc::new(HashMap::new()),
        }
    }

    /// 创建内置注册表
    ///
    /// 当前 Phase 0 仅包含最小示例 translator，后续阶段逐步填充所有原生标签、
    /// 扩展组件、根节点 translator。
    pub fn builtin() -> Self {
        let mut reg = Self::empty();
        super::builtin::register_all(&mut reg);
        reg
    }

    /// 注册一个 translator
    pub fn register<T: IRmlTranslator + 'static>(&mut self, translator: T) {
        let map = Arc::make_mut(&mut self.translators);
        map.insert(translator.tag(), Arc::new(translator));
    }

    /// 按标签名精确查询
    pub fn get(&self, tag: &str) -> Option<&dyn IRmlTranslator> {
        self.translators.get(tag).map(|b| b.as_ref())
    }

    /// 按元素匹配 translator
    ///
    /// 遍历所有 translator，返回第一个 `matches(elem)` 为 true 的实现。
    pub fn resolve(&self, elem: &Element) -> Option<&dyn IRmlTranslator> {
        self.translators.values().find(|t| t.matches(elem)).map(|b| b.as_ref())
    }

    /// 返回所有已注册标签名
    pub fn all_tags(&self) -> Vec<&'static str> {
        self.translators.keys().copied().collect()
    }

    /// 按分类返回 translator 列表
    pub fn by_category(&self, cat: ComponentCategory) -> Vec<&dyn IRmlTranslator> {
        self.translators
            .values()
            .filter(|t| t.metadata().category == cat)
            .map(|b| b.as_ref())
            .collect()
    }

    /// 返回某标签的元数据
    pub fn metadata(&self, tag: &str) -> Option<TranslatorMetadata> {
        self.get(tag).map(|t| t.metadata())
    }

    /// 判断是否包含某标签
    pub fn contains(&self, tag: &str) -> bool {
        self.translators.contains_key(tag)
    }
}
