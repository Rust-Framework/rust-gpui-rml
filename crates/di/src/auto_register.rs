//! 自动注册机制 —— `ctor` + 全局注册表
//!
//! 扩展 crate 经 `#[ctor::ctor]` 函数调用 `auto_register(closure)` 注册服务，
//! `configure` 阶段经 `apply_auto_registrations` 应用所有注册。
//!
//! `Fn`（非 `FnOnce`）+ 非 drain 式读取，支持多次 build。

use std::sync::{Mutex, OnceLock};

use crate::collection::ServiceCollection;

type RegisterFn = Box<dyn Fn(&mut ServiceCollection) + Send + Sync>;

static AUTO_REGISTRATIONS: OnceLock<Mutex<Vec<RegisterFn>>> = OnceLock::new();

/// 注册自动注册闭包。通常在 `#[ctor::ctor]` 函数中调用。
///
/// 闭包为 `Fn`（非 `FnOnce`），支持多次 build（如测试场景）。
pub fn auto_register(f: impl Fn(&mut ServiceCollection) + Send + Sync + 'static) {
    AUTO_REGISTRATIONS
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(Box::new(f));
}

/// 应用所有自动注册。`configure` 内部调用。
pub fn apply_auto_registrations(collection: &mut ServiceCollection) {
    if let Some(registry) = AUTO_REGISTRATIONS.get() {
        for f in registry.lock().unwrap().iter() {
            f(collection);
        }
    }
}
