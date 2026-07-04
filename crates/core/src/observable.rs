//! 响应式集合 —— `ObservableVec<T>`
//!
//! 提供 `&self` 可变 API 的 `Vec` 包装，内部用 `Arc<RwLock<Vec<T>>>` 共享数据 +
//! `Arc<AtomicU64>` 版本号 + 可选 `flume::Sender<()>` 通知通道。
//!
//! - **版本号**：供 `#[computed]` 依赖追踪。每次写操作 `fetch_add(1)`，
//!   `version()` 返回当前值。`Clone` 共享同一份版本号——任意副本的写操作对所有副本可见。
//! - **通知通道**：写操作后 `send(())`。业务侧用 `flume::unbounded()` 创建，
//!   背景任务 `rx.recv()` 触发 `cx.notify()`，桥接到 GPUI 重渲染。
//!
//! 所有 mutating 方法为 `&self`，满足 `IWorkbenchManager::open`、`IContributionHost::add`
//! 等 `&self` trait 约束。

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, RwLock};

/// 响应式 `Vec`：`&self` 可变 + 版本追踪 + 可选通知通道。
///
/// `Clone` 廉价（三个 `Arc` 引用），所有副本共享底层数据与版本号。
/// 写入任一副本，其他副本的 `version()` 同步递增。
pub struct ObservableVec<T> {
    inner: Arc<RwLock<Vec<T>>>,
    version: Arc<AtomicU64>,
    notify: Option<flume::Sender<()>>,
}

impl<T> ObservableVec<T> {
    /// 创建空集合（无通知通道）。
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
            version: Arc::new(AtomicU64::new(0)),
            notify: None,
        }
    }

    /// 创建空集合，附带通知通道。写操作后会 `send(())`。
    pub fn with_notify(notify: flume::Sender<()>) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Vec::new())),
            version: Arc::new(AtomicU64::new(0)),
            notify: Some(notify),
        }
    }

    /// 追加元素并触发版本递增 + 通知。
    pub fn push(&self, value: T) {
        self.inner.write().unwrap().push(value);
        self.bump();
    }

    /// 移除首个满足谓词的元素。返回是否移除成功。
    pub fn remove_where(&self, predicate: impl Fn(&T) -> bool) -> bool {
        let mut guard = self.inner.write().unwrap();
        if let Some(pos) = guard.iter().position(|x| predicate(x)) {
            guard.remove(pos);
            drop(guard);
            self.bump();
            return true;
        }
        false
    }

    /// 清空集合。若原本为空则不触发版本递增。
    pub fn clear(&self) {
        let mut guard = self.inner.write().unwrap();
        if guard.is_empty() {
            return;
        }
        guard.clear();
        drop(guard);
        self.bump();
    }

    /// 当前元素数量。
    pub fn len(&self) -> usize {
        self.inner.read().unwrap().len()
    }

    /// 是否为空。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// 当前版本号。每次写操作 `fetch_add(1)`。
    pub fn version(&self) -> u64 {
        self.version.load(Ordering::SeqCst)
    }

    fn bump(&self) {
        self.version.fetch_add(1, Ordering::SeqCst);
        if let Some(tx) = &self.notify {
            let _ = tx.send(());
        }
    }
}

impl<T: Clone> ObservableVec<T> {
    /// 快照：克隆当前所有元素。
    pub fn snapshot(&self) -> Vec<T> {
        self.inner.read().unwrap().clone()
    }

    /// 按索引获取元素（克隆）。
    pub fn get(&self, index: usize) -> Option<T> {
        self.inner.read().unwrap().get(index).cloned()
    }

    /// 返回快照迭代器（克隆后消费）。
    pub fn iter(&self) -> std::vec::IntoIter<T> {
        self.snapshot().into_iter()
    }
}

impl<T> Default for ObservableVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for ObservableVec<T> {
    /// 克隆：共享底层数据 + 版本号 + 通知通道。
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            version: Arc::clone(&self.version),
            notify: self.notify.clone(),
        }
    }
}

impl<T> std::fmt::Debug for ObservableVec<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ObservableVec")
            .field("version", &self.version())
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_increments_version() {
        let v: ObservableVec<i32> = ObservableVec::new();
        assert_eq!(v.version(), 0);
        v.push(1);
        assert_eq!(v.version(), 1);
        v.push(2);
        assert_eq!(v.version(), 2);
        assert_eq!(v.len(), 2);
    }

    #[test]
    fn clone_shares_version() {
        let v: ObservableVec<i32> = ObservableVec::new();
        let c = v.clone();
        v.push(1);
        // 副本可见同一版本号
        assert_eq!(c.version(), 1);
        assert_eq!(c.snapshot(), vec![1]);
    }

    #[test]
    fn remove_where_works() {
        let v: ObservableVec<i32> = ObservableVec::new();
        v.push(1);
        v.push(2);
        v.push(3);
        assert_eq!(v.version(), 3);
        assert!(v.remove_where(|x| *x == 2));
        assert_eq!(v.version(), 4);
        assert_eq!(v.snapshot(), vec![1, 3]);
        assert!(!v.remove_where(|x| *x == 99));
        // 未移除不 bump
        assert_eq!(v.version(), 4);
    }

    #[test]
    fn clear_noop_on_empty() {
        let v: ObservableVec<i32> = ObservableVec::new();
        v.clear();
        assert_eq!(v.version(), 0);
    }

    #[test]
    fn notify_fires_on_write() {
        let (tx, rx) = flume::unbounded();
        let v = ObservableVec::<i32>::with_notify(tx);
        v.push(1);
        assert!(rx.try_recv().is_ok());
        v.push(2);
        assert!(rx.try_recv().is_ok());
    }

    #[test]
    fn notify_skipped_on_noop() {
        let (tx, rx) = flume::unbounded();
        let v = ObservableVec::<i32>::with_notify(tx);
        v.clear(); // 空集合，不 bump，不 send
        assert!(rx.try_recv().is_err());
    }
}
