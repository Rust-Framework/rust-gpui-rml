//! 组件内部状态容器
//!
//! `#[component]` / `#[window]` 宏为用户结构体注入单一字段
//! `__rml_state: RmlState`，统一承载框架运行时所需的全部状态：
//!
//! - 字段版本追踪（替代旧 `__rml_<field>_version: AtomicU64` 每字段一个的设计）
//! - `#[computed]` 缓存
//! - `<input value={field}>` 双向绑定所需的 `InputState` entity 暂存与正向同步版本
//! - 字段校验错误状态
//! - `on_loaded` 一次性初始化守卫
//! - 窗口句柄（供 `IWindow::handle` / 对话框 `close` 使用）
//! - 具名插槽渲染闭包
//!
//! 设计目标：把原本散落在用户结构体中的 7+ 类 `__rml_*` 仪式字段收敛为单一字段，
//! 让 IDE 自动补全与 rustdoc 只显示一个入口，消除视觉噪声。

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use gpui::{AnyWindowHandle, AppContext, Entity, SharedString};

use crate::InputState;
use crate::window::actions::{IWindowActions, NotificationKind};

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

    /// `<input value={field}>` 绑定的 `InputState` entity，按字段名索引
    ///
    /// 惰性初始化：首次 `__rml_get_or_init_input_state(field)` 时创建并订阅。
    pub input_states: HashMap<String, Entity<InputState>>,

    /// 每个字段上次正向同步到 `InputState` 的版本号
    ///
    /// render 时对比 `get_version(field)` 与此值，不同则调用 `InputState::set_value`。
    pub input_state_versions: HashMap<String, u64>,

    /// StateBridge 绑定的 State Entity，按 (bridge_key, field) 索引
    ///
    /// 类型擦除存储（与 `ref_entities` 同模式）：`Entity<T>` 自身 `Clone + Send + Sync + 'static`
    /// （不依赖 `T` 的 `Send`/`Sync`），可安全存储为 `Box<dyn Any + Send + Sync>`
    /// 并通过 `downcast_ref::<Entity<T>>()` 取回。
    ///
    /// key 为 bridge_key（如 "slider"），value 为字段名→Entity 的映射。
    /// 惰性初始化：首次 `__rml_get_or_init_<suffix>_state(field)` 时创建并订阅。
    pub state_bridge_entities: HashMap<&'static str, HashMap<String, Box<dyn std::any::Any + Send + Sync>>>,

    /// StateBridge 正向同步版本号，key 为 `"<bridge_key>:<field>"`
    pub state_bridge_versions: HashMap<String, u64>,

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

    /// `once` 指令的数据快照缓存
    ///
    /// key 为编译期生成的唯一标识（`once_0`、`once_1`、...），value 为类型擦除的快照值。
    /// 首次渲染时调用 `once_get_or_init` 初始化；后续渲染直接读取缓存。
    ///
    /// 设计说明：`AnyElement` 内部为 `ArenaBox`，arena 每帧 reset，不可跨帧缓存。
    /// 因此 `once` 不能缓存元素本身，而是缓存元素的数据依赖（字段值），
    /// 后续渲染用快照数据重建元素，达到"冻结首次渲染状态"的语义。
    ///
    /// 使用 `Mutex` 提供内部可变性，使 `once_get_or_init` 只需 `&self`，
    /// 从而在 slot 闭包（仅有 `&self`）内也能使用 `once` 指令。
    /// `Mutex` 而非 `RefCell` 以满足 `Sync` 约束（`RmlState: Send + Sync`）。
    pub once_cache: Mutex<HashMap<&'static str, Box<dyn std::any::Any + Send + Sync>>>,

    /// `ref` 指令的元素实体缓存
    ///
    /// key 为 `ref="name"` 中的 name，value 为类型擦除的 `Entity<T>` 句柄。
    /// 由 `get_or_init_ref` 在首次渲染时惰性创建并存储，
    /// 由 `__rml_populate_refs`（宏生成）在渲染后填充到 `ElementRef<T>` 字段。
    ///
    /// 设计说明：`Entity<T>` 自身 `Clone + Send + Sync + 'static`（不依赖 `T` 的
    /// `Send`/`Sync`，因为内部仅持有 `Weak<RwLock<EntityRefCounts>>`，不持有 `T`）。
    /// 故可作为 `Box<dyn Any + Send + Sync>` 类型擦除存储，并通过
    /// `downcast_ref::<Entity<T>>()` 取回，同时保持 `RmlState: Send + Sync`
    /// 以满足 `IModel: 'static + Send + Sync` 约束。
    pub ref_entities: HashMap<String, Box<dyn std::any::Any + Send + Sync>>,

    /// 已订阅事件标识集合
    ///
    /// 防止重复 subscribe 导致内存泄漏（每次 render 都会重新评估事件订阅代码）。
    /// subscription 句柄用 `detach()` 让其随 entity 生命周期自动销毁，
    /// 不在此字段中保存（Subscription 非 Sync，无法存入 RmlState）。
    ///
    /// key 形式：`<ref_name>:<event_name>`，如 `input_state:on_change`。
    ///
    /// 设计说明：用 `Mutex` 而非 `RefCell` 以满足 `Sync` 约束，
    /// `Mutex<HashSet<String>>` 是 `Send + Sync`，保持 `RmlState: Send + Sync`。
    pub subscribed_events: Mutex<HashSet<String>>,

    /// `on-focus`/`on-blur` 事件的 FocusHandle 缓存
    ///
    /// GPUI 的 on_focus/on_blur 是 Context 级 API（非元素 builder 方法），
    /// 需要 FocusHandle 引用来注册监听器。FocusHandle 在首次渲染时创建并缓存，
    /// 后续渲染复用同一 handle，用 `.track_focus(&handle)` 关联到元素。
    ///
    /// key 为编译期生成的唯一标识（`focus_0`、`focus_1`、...）。
    ///
    /// 使用 `Mutex` 提供内部可变性，使 `get_or_init_focus_handle` 只需 `&self`，
    /// 从而在 slot 闭包（仅有 `&self` via `__rml_self_ref`）内也能使用。
    pub focus_handles: Mutex<HashMap<String, gpui::FocusHandle>>,

    /// 待处理通知队列（延迟到 render 时通过 Window 发送）
    ///
    /// 命令回调（`#[command]`）内无 `Window` 参数，调用 `self.notify_info(msg)` 入队，
    /// render 首帧 `drain_notifications(window, cx)` 消费并调用 `window.notify_*`。
    pub pending_notifications: Vec<(NotificationKind, SharedString)>,
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

    /// `once` 指令的数据快照访问入口
    ///
    /// 首次调用时执行 `init` 闭包生成快照并存入缓存；后续调用直接返回缓存的克隆。
    /// 泛型 `T` 由闭包返回类型推断，调用方需保证同一 key 的 `T` 一致。
    ///
    /// 约束：`T: 'static + Send + Sync + Clone`。`Clone` 是因为每次渲染都需要一份
    /// 独立的快照值（元素构建过程可能 move 字段）。
    ///
    /// 签名为 `&self`（通过 `Mutex` 内部可变性），使 slot 闭包内也能调用。
    pub fn once_get_or_init<T: 'static + Send + Sync + Clone>(
        &self,
        key: &'static str,
        init: impl FnOnce() -> T,
    ) -> T {
        let cache = self.once_cache.lock().unwrap();
        if let Some(boxed) = cache.get(key) {
            if let Some(v) = boxed.downcast_ref::<T>() {
                return v.clone();
            }
        }
        drop(cache);
        let v = init();
        self.once_cache
            .lock()
            .unwrap()
            .insert(key, Box::new(v.clone()));
        v
    }

    /// `ref` 指令的元素实体访问入口
    ///
    /// 首次调用时通过 `ctor` 闭包创建 `Entity<T>` 并存入 `ref_entities` 缓存；
    /// 后续调用直接返回缓存的克隆。泛型 `T` 由闭包返回类型推断。
    ///
    /// 由 Stateful 组件 codegen 在 `ref="name"` 时生成调用：
    /// ```ignore
    /// Input::new(&self.__rml_state.get_or_init_ref(
    ///     "input1",
    ///     _window,
    ///     &mut *cx,
    ///     |w, c| rml_ui::InputState::new(w, c),
    /// ))
    /// ```
    ///
    /// 由宏生成的 `__rml_populate_refs()` 在渲染后从同一缓存取出 `Entity<T>`，
    /// 注入到用户声明的 `ElementRef<T>` 字段（字段名需与 ref name 一致）。
    ///
    /// 约束：`T: 'static`。`Entity<T>` 自身 `Clone + Send + Sync + 'static`
    /// （不依赖 `T` 的 `Send`/`Sync`），可安全存储为 `Box<dyn Any + Send + Sync>`
    /// 并通过 `downcast_ref::<Entity<T>>()` 取回。
    ///
    /// 即便 `InputState` 内部含 `Vec<Subscription>`（`Subscription` 含
    /// `Box<dyn FnOnce()>` 非 `Sync`）导致 `InputState` 不是 `Sync`，
    /// `Entity<InputState>` 仍为 `Send + Sync` —— 因为句柄内部仅持有
    /// `Weak<RwLock<EntityRefCounts>>`，不持有 `T`。
    pub fn get_or_init_ref<T: 'static>(
        &mut self,
        name: &str,
        window: &mut gpui::Window,
        cx: &mut gpui::App,
        ctor: impl FnOnce(&mut gpui::Window, &mut gpui::Context<T>) -> T,
    ) -> gpui::Entity<T> {
        if let Some(boxed) = self.ref_entities.get(name) {
            if let Some(entity) = boxed.downcast_ref::<gpui::Entity<T>>() {
                return entity.clone();
            }
        }
        let entity = cx.new(move |cx| ctor(window, cx));
        self.ref_entities
            .insert(name.to_string(), Box::new(entity.clone()));
        entity
    }

    // ── StateBridge 存取 ──────────────────────────────────────────

    /// 检查 StateBridge entity 是否已初始化
    pub fn has_state_bridge(&self, bridge_key: &str, field: &str) -> bool {
        self.state_bridge_entities
            .get(bridge_key)
            .map(|m| m.contains_key(field))
            .unwrap_or(false)
    }

    /// 获取 StateBridge entity（类型擦除 → downcast 回 `Entity<T>`）
    ///
    /// 由生成的 `__rml_get_or_init_<suffix>_state` 方法调用。
    pub fn get_state_bridge<T: 'static>(&self, bridge_key: &str, field: &str) -> Option<Entity<T>> {
        self.state_bridge_entities
            .get(bridge_key)?
            .get(field)?
            .downcast_ref::<Entity<T>>()
            .cloned()
    }

    /// 插入 StateBridge entity
    pub fn insert_state_bridge<T: 'static>(
        &mut self,
        bridge_key: &'static str,
        field: String,
        entity: Entity<T>,
    ) {
        self.state_bridge_entities
            .entry(bridge_key)
            .or_insert_with(HashMap::new)
            .insert(field, Box::new(entity));
    }

    /// 读取 StateBridge 正向同步版本号
    pub fn get_state_bridge_version(&self, bridge_key: &str, field: &str) -> u64 {
        let key = format!("{}:{}", bridge_key, field);
        self.state_bridge_versions.get(&key).copied().unwrap_or(0)
    }

    /// 设置 StateBridge 正向同步版本号
    pub fn set_state_bridge_version(&mut self, bridge_key: &str, field: &str, version: u64) {
        let key = format!("{}:{}", bridge_key, field);
        self.state_bridge_versions.insert(key, version);
    }

    /// 检查指定事件是否已订阅
    ///
    /// 用于防止 render 时重复 subscribe 同一事件导致内存泄漏。
    /// key 形式：`<ref_name>:<event_name>`，如 `input_state:on_change`。
    pub fn is_event_subscribed(&self, key: &str) -> bool {
        self.subscribed_events
            .lock()
            .map(|set| set.contains(key))
            .unwrap_or(false)
    }

    /// 标记事件已订阅
    ///
    /// 由 codegen 在 `cx.subscribe(...).detach()` 之后调用。
    /// 用 `&self` 而非 `&mut self`：通过 `Mutex` 提供内部可变性，
    /// 使 codegen 在只读 `&self` 上下文中也能更新订阅状态。
    pub fn mark_event_subscribed(&self, key: String) {
        if let Ok(mut set) = self.subscribed_events.lock() {
            set.insert(key);
        }
    }

    /// 获取或初始化焦点事件所需的 `FocusHandle`
    ///
    /// GPUI 的 `on_focus`/`on_blur` 是 `Context<T>` 级 API，需要 `&FocusHandle` 参数。
    /// 首次调用时通过 `cx.focus_handle()` 创建并缓存到 `focus_handles`，
    /// 后续渲染复用同一 handle，用 `.track_focus(&handle)` 关联到元素。
    ///
    /// 签名为 `&self`（通过 `Mutex` 内部可变性），使 slot 闭包内也能调用。
    pub fn get_or_init_focus_handle(
        &self,
        key: &str,
        cx: &mut gpui::App,
    ) -> gpui::FocusHandle {
        let cache = self.focus_handles.lock().unwrap();
        if let Some(handle) = cache.get(key) {
            return handle.clone();
        }
        drop(cache);
        let handle = cx.focus_handle();
        self.focus_handles
            .lock()
            .unwrap()
            .insert(key.to_string(), handle.clone());
        handle
    }

    // ── 延迟通知 ──────────────────────────────────────────────────

    /// 入队一条信息通知（命令回调内无 Window，延迟到 render 时发送）
    pub fn notify_info(&mut self, message: impl Into<SharedString>) {
        self.pending_notifications
            .push((NotificationKind::Info, message.into()));
    }

    /// 入队一条成功通知
    pub fn notify_success(&mut self, message: impl Into<SharedString>) {
        self.pending_notifications
            .push((NotificationKind::Success, message.into()));
    }

    /// 入队一条警告通知
    pub fn notify_warning(&mut self, message: impl Into<SharedString>) {
        self.pending_notifications
            .push((NotificationKind::Warning, message.into()));
    }

    /// 入队一条错误通知
    pub fn notify_error(&mut self, message: impl Into<SharedString>) {
        self.pending_notifications
            .push((NotificationKind::Error, message.into()));
    }

    /// 消费待处理通知队列，通过 Window 发送
    ///
    /// 由 codegen 在 render 方法首帧调用（`window` 在 render 作用域可用）。
    pub fn drain_notifications(&mut self, window: &mut gpui::Window, cx: &mut gpui::App) {
        while let Some((kind, msg)) = self.pending_notifications.pop() {
            window.show_notification(msg, kind, cx);
        }
    }
}
