//! 变量作用域与求值缓存
//!
//! 按 DAP `variablesReference` 机制组织变量树：每个作用域/复合变量持有一个引用号，
//! 通过该引用号可取其子变量列表。本模块缓存引擎查询结果，避免重复请求。

use std::collections::HashMap;

use crate::engine::Variable;

/// 变量树缓存：`variables_reference` → 变量列表
#[derive(Default)]
pub struct VariableTree {
    cache: HashMap<u64, Vec<Variable>>,
}

impl VariableTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// 缓存指定引用号下的变量
    pub fn set(&mut self, variables_reference: u64, vars: Vec<Variable>) {
        self.cache.insert(variables_reference, vars);
    }

    /// 获取指定引用号下的变量（命中缓存则返回，否则 None）
    pub fn get(&self, variables_reference: u64) -> Option<&[Variable]> {
        self.cache.get(&variables_reference).map(|v| v.as_slice())
    }

    /// 是否已缓存指定引用号
    pub fn contains(&self, variables_reference: u64) -> bool {
        self.cache.contains_key(&variables_reference)
    }

    /// 清空缓存（暂停状态变化或重新启动时调用）
    pub fn clear(&mut self) {
        self.cache.clear();
    }
}
