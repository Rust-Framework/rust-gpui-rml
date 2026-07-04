//! 组件内部状态容器
//!
//! `#[component]` / `#[window]` 宏为用户结构体注入单一字段
//! `__rml_state: RmlState`，统一承载框架运行时所需的全部状态：
//!
//! - 字段版本追踪（替代旧 `__rml_<field>_version: AtomicU64` 每字段一个的设计）
//! - `#[computed]` 缓存
//! - `<input model={field}>` 双向绑定所需的 `InputState` entity 暂存与正向同步版本
//! - 字段校验错误状态
//! - `on_loaded` 一次性初始化守卫
//! - 窗口句柄（供 `IWindow::handle` / 对话框 `close` 使用）
//! - 具名插槽渲染闭包
//!
//! 设计目标：把原本散落在用户结构体中的 7+ 类 `__rml_*` 仪式字段收敛为单一字段，
//! 让 IDE 自动补全与 rustdoc 只显示一个入口，消除视觉噪声。

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use gpui::{AnyWindowHandle, Entity, SharedString};

use crate::InputState;

/// 组件运行时状态容器
///
/// 由 `#[component]` / `#[window]` 宏注入为 `__rml_state: RmlState` 私有字段。
/// 通过 `Default` 初始化空状态，按需惰性填充。
///
/// 线程安全：所有字段均 `Send + Sync`，满足 `IModel: 'static + Send + Sync`。
#[derive(Default)]
pub struct RmlState {
    /// `#[computed]` 方法结果缓存（按方法名 + 依赖版本号键控）
    pub computed_cache: rml_core::computed_cache::ComputedCache,

    /// `<input model={field}>` 绑定的 `InputState` entity，按字段名索引
    ///
    /// 惰性初始化：首次 `__rml_get_or_init_input_state(field)` 时创建并订阅。
    pub input_states: HashMap<String, Entity<InputState>>,

    /// 每个字段上次正向同步到 `InputState` 的版本号
    ///
    /// render 时对比 `get_version(field)` 与此值，不同则调用 `InputState::set_value`。
    pub input_state_versions: HashMap<String, u64>,

    /// 字段校验错误状态
    ///
    /// `None` = 校验通过，`Some(msg)` = 校验失败（红色边框 + tooltip 显示 msg）。
    pub field_errors: HashMap<String, Option<SharedString>>,

    /// 可观察字段版本计数器
    ///
    /// 替代旧 `__rml_<field>_version: AtomicU64` 每字段一个的设计。
    /// 惰性插入：首次 `bump_version(field)` 时 `or_insert_with(|| AtomicU64::new(0))`。
    pub field_versions: HashMap<String, AtomicU64>,

    /// `on_loaded` 一次性初始化守卫
    pub loaded: bool,

    /// 窗口句柄（由 `IWindow::set_handle` / 对话框 `open` 设置）
    pub window_handle: Option<AnyWindowHandle>,

    /// 具名插槽渲染闭包
    ///
    /// 替代旧 `__rml_slot_<name>: Option<SlotRenderer>` 每插槽一个的设计。
    /// key 为插槽名（`'static str`，来自 `#[component(slots = [...])]` 声明）。
    pub slots: HashMap<&'static str, rml_core::slot::SlotRenderer>,
}

impl RmlState {
    /// 将指定字段的版本号 +1（由 `#[command]` 宏注入）
    ///
    /// 取 `&mut self` 以使用 `HashMap::entry` API 惰性插入。
    /// 所有调用点（`#[command]` 包装、双向绑定反向同步）均持有 `&mut self`。
    pub fn bump_version(&mut self, field: &str) {
        self.field_versions
            .entry(field.to_string())
            .or_insert_with(|| AtomicU64::new(0))
            .fetch_add(1, Ordering::Relaxed);
    }

    /// 读取字段当前版本号
    ///
    /// 未追踪字段返回 0（等价于旧设计 `_ => 0` 默认分支）。
    pub fn get_version(&self, field: &str) -> u64 {
        self.field_versions
            .get(field)
            .map(|a| a.load(Ordering::Relaxed))
            .unwrap_or(0)
    }

    /// 注入具名插槽渲染闭包
    pub fn set_slot(&mut self, name: &'static str, renderer: rml_core::slot::SlotRenderer) {
        self.slots.insert(name, renderer);
    }

    /// 查询具名插槽渲染闭包
    pub fn slot(&self, name: &str) -> Option<&rml_core::slot::SlotRenderer> {
        self.slots.get(name)
    }
}
