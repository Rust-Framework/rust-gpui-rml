//! 断点管理
//!
//! 纯数据管理：维护源文件 → 断点列表的映射，提供增删改查。
//! 与引擎的同步（调用 `DebugEngine::set_breakpoints`）由 `DebugSession` 编排，
//! 本模块不直接接触引擎。

use std::collections::HashMap;

use lsp_types::Url;

use crate::engine::Breakpoint;

/// 断点管理器：按源文件组织断点
#[derive(Default)]
pub struct BreakpointManager {
    /// 源文件 → 断点列表
    breakpoints: HashMap<Url, Vec<Breakpoint>>,
}

impl BreakpointManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// 替换指定源文件的所有断点
    pub fn set(&mut self, source: &Url, breakpoints: Vec<Breakpoint>) {
        self.breakpoints.insert(source.clone(), breakpoints);
    }

    /// 在指定源文件追加一个断点
    pub fn add(&mut self, bp: Breakpoint) {
        self.breakpoints
            .entry(bp.source.clone())
            .or_default()
            .push(bp);
    }

    /// 移除指定源文件某行的断点，返回是否移除成功
    pub fn remove(&mut self, source: &Url, line: u32) -> bool {
        if let Some(list) = self.breakpoints.get_mut(source) {
            let before = list.len();
            list.retain(|bp| bp.line != line);
            return list.len() != before;
        }
        false
    }

    /// 切换指定源文件某行断点的启用状态；断点不存在时返回 None
    pub fn toggle(&mut self, source: &Url, line: u32) -> Option<bool> {
        let list = self.breakpoints.get_mut(source)?;
        let bp = list.iter_mut().find(|bp| bp.line == line)?;
        bp.enabled = !bp.enabled;
        Some(bp.enabled)
    }

    /// 清空指定源文件的所有断点
    pub fn clear(&mut self, source: &Url) {
        self.breakpoints.remove(source);
    }

    /// 获取指定源文件的断点列表（只读）
    pub fn get(&self, source: &Url) -> &[Breakpoint] {
        self.breakpoints
            .get(source)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 获取所有已启用断点的扁平迭代
    pub fn all_enabled(&self) -> impl Iterator<Item = &Breakpoint> {
        self.breakpoints
            .values()
            .flatten()
            .filter(|bp| bp.enabled)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bp(source: &str, line: u32) -> Breakpoint {
        Breakpoint {
            source: Url::parse(source).unwrap(),
            line,
            condition: None,
            hit_condition: None,
            log_message: None,
            enabled: true,
        }
    }

    #[test]
    fn add_and_get() {
        let mut mgr = BreakpointManager::new();
        let url = Url::parse("file:///foo.rml").unwrap();
        mgr.add(bp("file:///foo.rml", 10));
        mgr.add(bp("file:///foo.rml", 20));
        assert_eq!(mgr.get(&url).len(), 2);
    }

    #[test]
    fn remove_by_line() {
        let mut mgr = BreakpointManager::new();
        let url = Url::parse("file:///foo.rml").unwrap();
        mgr.add(bp("file:///foo.rml", 10));
        assert!(mgr.remove(&url, 10));
        assert!(mgr.get(&url).is_empty());
        assert!(!mgr.remove(&url, 99));
    }

    #[test]
    fn toggle_disables_then_enables() {
        let mut mgr = BreakpointManager::new();
        let url = Url::parse("file:///foo.rml").unwrap();
        mgr.add(bp("file:///foo.rml", 5));
        assert_eq!(mgr.toggle(&url, 5), Some(false));
        assert_eq!(mgr.toggle(&url, 5), Some(true));
        assert_eq!(mgr.toggle(&url, 99), None);
    }

    #[test]
    fn all_enabled_filters_disabled() {
        let mut mgr = BreakpointManager::new();
        mgr.add(bp("file:///a.rml", 1));
        mgr.add(bp("file:///b.rml", 2));
        let mut b3 = bp("file:///c.rml", 3);
        b3.enabled = false;
        mgr.add(b3);
        let mut enabled: Vec<u32> = mgr.all_enabled().map(|bp| bp.line).collect();
        enabled.sort();
        assert_eq!(enabled, vec![1, 2]);
    }
}
