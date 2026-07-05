//! Element ID 辅助函数
//!
//! 提供 `key` 指令运行时支持：将任意 `Hash` 类型的 key 转换为稳定 `u64` 哈希值，
//! 用于在列表渲染中生成稳定的 `ElementId`（而非递增整数），使列表项重新排序后状态保持。

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// 将任意 `Hash` 类型的 key 转换为稳定的 `usize` 哈希值
///
/// 用于 `key` 指令：`<li each={item in items} key={item.id}>` 生成
/// `.id(("rml_key", rml_core::element_id::from_key(&item.id)))`，
/// 使列表项重新排序后 `ElementId` 保持一致，GPUI 可正确跟踪元素状态（选中、焦点等）。
///
/// 约束：`T: Hash`。大多数类型通过 `#[derive(Hash)]` 实现，包括 `u32`/`u64`/`String`/`Uuid` 等。
pub fn from_key<T: Hash>(key: &T) -> usize {
    let mut hasher = DefaultHasher::new();
    key.hash(&mut hasher);
    hasher.finish() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_produces_same_hash() {
        let a = from_key(&"user-123");
        let b = from_key(&"user-123");
        assert_eq!(a, b, "same key should produce same hash");
    }

    #[test]
    fn different_keys_produce_different_hashes() {
        let a = from_key(&"user-123");
        let b = from_key(&"user-456");
        assert_ne!(a, b, "different keys should produce different hashes");
    }

    #[test]
    fn works_with_numeric_key() {
        let a = from_key(&42u32);
        let b = from_key(&42u32);
        assert_eq!(a, b);
        assert_ne!(a, from_key(&43u32));
    }

    #[test]
    fn works_with_tuple_key() {
        let a = from_key(&(1u32, "group-a"));
        let b = from_key(&(1u32, "group-a"));
        assert_eq!(a, b);
        assert_ne!(a, from_key(&(2u32, "group-a")));
    }
}
