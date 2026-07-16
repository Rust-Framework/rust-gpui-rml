//! ActivityBar 面板注册表 —— `ctor` + 全局工厂列表
//!
//! 扩展 crate 经 `#[ctor::ctor]` 调用 `register_activity_panel(factory)` 注册面板工厂,
//! Host 在 `on_loaded` 中经 `get_activity_panels()` 枚举所有已注册面板。
//!
//! 与 `studio_core::di::auto_register` 同构: `Fn`（非 `FnOnce`）+ 非 drain 式读取,支持多次查询。

use std::sync::{Arc, Mutex, OnceLock};

use rml_core::contribution::IContribution;

type PanelFactory = Box<dyn Fn() -> Arc<dyn IContribution> + Send + Sync>;

static PANEL_REGISTRY: OnceLock<Mutex<Vec<PanelFactory>>> = OnceLock::new();

/// 注册活动栏面板工厂。通常在 `#[ctor::ctor]` 函数中调用。
///
/// 工厂为 `Fn`（非 `FnOnce`）,支持多次查询（如窗口重建场景）。
pub fn register_activity_panel(f: impl Fn() -> Arc<dyn IContribution> + Send + Sync + 'static) {
    PANEL_REGISTRY
        .get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .unwrap()
        .push(Box::new(f));
}

/// 枚举所有已注册的活动栏面板（经工厂构造）。
///
/// 返回 `Vec<Arc<dyn IContribution>>`,Host 经 `VisualActivityPanel::new` 适配为 `IActivityPanel`。
/// 未经 `register_activity_panel` 注册时返回空 Vec。
pub fn get_activity_panels() -> Vec<Arc<dyn IContribution>> {
    match PANEL_REGISTRY.get() {
        Some(registry) => registry
            .lock()
            .unwrap()
            .iter()
            .map(|f| f())
            .collect(),
        None => Vec::new(),
    }
}
