//! `ICommand` trait + `RelayCommand` —— 命令系统契约与 WPF 风格命令对象
//!
//! 对齐 WPF `ICommand`：`execute` + `can_execute`，接口纯净无编译器元信息。
//!
//! `#[command]` 标记的方法可被 `.rml` 中的 `on*` 事件绑定调用。
//! 命令方法签名：`fn(&mut self, ev: &Event, cx: &mut Context<Self>)`
//! 或带参数：`fn(&mut self, param: T, ev: &Event, cx: &mut Context<Self>)`
//!
//! `#[command]` 宏是 pass-through（仅校验签名），codegen 直接调用命令方法保留事件类型安全。
//!
//! `ICommand` trait 是 object-safe 的，可作为 ViewModel 字段类型（`Arc<dyn ICommand>`），
//! 用于声明式 `<MenuItem command={field} />` 绑定等 MVVM 场景。
//! `RelayCommand` 是框架提供的默认 `ICommand` 实现（WPF `RelayCommand`/`DelegateCommand` 等价物），
//! 持有 `WeakEntity<T>` + 闭包，在 `execute` 时 upgrade 并 update ViewModel。

use gpui::{App, Context};

/// 命令基础 trait（对齐 WPF `ICommand`）。
///
/// Object-safe：可存储为 `Arc<dyn ICommand>` / `Box<dyn ICommand>`，
/// 作为 ViewModel 字段在 RML 中通过 `command={field}` 绑定到控件 click。
///
/// 命令对象需自行持有 `WeakEntity<T>` 来更新 ViewModel 状态（参见 `RelayCommand`）。
///
/// `parameter` 类型擦除为 `&dyn Any`，实现方按需 downcast。
///
/// # 与 `#[command]` 方法的关系
///
/// - `#[command]` 方法：codegen 生成的事件绑定直接调用强类型方法（绕过 trait，保留类型安全）
/// - `ICommand` trait：用于声明式 `command={field}` 绑定、快捷键、命令面板等动态调度场景
pub trait ICommand: 'static {
    /// 执行命令（WPF: `Execute`）
    ///
    /// `parameter` 类型擦除，实现方按需 `downcast_ref`/`downcast_mut`。
    /// 无参数命令可忽略 `parameter`。
    fn execute(&self, parameter: &dyn std::any::Any, cx: &mut App);

    /// 是否可执行（WPF: `CanExecute`）
    ///
    /// 返回 `false` 时 UI 层应禁用对应控件（如按钮 disabled）。
    /// 默认实现返回 `true`。
    ///
    /// 简单状态检查可直接在此返回；需要访问 ViewModel 状态的命令
    /// 可在 `execute` 内通过 `WeakEntity::upgrade()` 检查并提前返回。
    fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool {
        true
    }
}

/// WPF `RelayCommand` / `DelegateCommand` 等价物。
///
/// 持有 `WeakEntity<T>` + 闭包，在 `execute` 时 upgrade 弱引用并 `update` ViewModel。
/// 适用于 MVVM 声明式绑定：ViewModel 持有 `Arc<dyn ICommand>` 字段，
/// 在 RML 中通过 `command={field}` 绑定到 `<MenuItem>` 等控件。
///
/// # 用法
///
/// ```rust,ignore
/// pub struct MainWindow {
///     save_command: Arc<dyn ICommand>,
/// }
///
/// impl ILifecycle for MainWindow {
///     fn on_loaded(&mut self, cx: &mut Context<Self>) {
///         self.save_command = Arc::new(
///             RelayCommand::new(cx, |this, cx| this.save(cx))
///         );
///     }
/// }
/// ```
///
/// RML 声明式绑定：
/// ```xml
/// <MenuItem label="Save" command={save_command} />
/// ```
pub struct RelayCommand {
    action: Box<dyn Fn(&mut App) + 'static>,
    can_run: Option<Box<dyn Fn() -> bool + 'static>>,
}

impl RelayCommand {
    /// 从视图绑定闭包创建命令（WPF `RelayCommand(execute)` 模式）。
    ///
    /// 内部捕获 `WeakEntity<T>`，`execute` 时 upgrade 并 `update`。
    /// 闭包签名为 `Fn(&mut T, &mut Context<T>)`，与 `#[command]` 方法体一致。
    pub fn new<T, F>(cx: &Context<T>, f: F) -> Self
    where
        T: 'static,
        F: Fn(&mut T, &mut Context<T>) + 'static,
    {
        let weak = cx.weak_entity();
        Self {
            action: Box::new(move |cx: &mut App| {
                let _ = weak.update(cx, |this, cx| f(this, cx));
            }),
            can_run: None,
        }
    }

    /// 从无视图闭包创建命令（全局 Action 模式）。
    ///
    /// 不绑定任何 ViewModel，适用于纯 App 级操作（如 `cx.quit()`）。
    pub fn action<F>(f: F) -> Self
    where
        F: Fn(&mut App) + 'static,
    {
        Self {
            action: Box::new(f),
            can_run: None,
        }
    }

    /// 设置 `can_execute` 谓词（Builder 风格）。
    ///
    /// 谓词不接收上下文参数；需要检查 ViewModel 状态时，
    /// 可在闭包中捕获 `Arc<Mutex<...>>` 或使用 `Rc<RefCell<...>>`。
    pub fn can_when<F>(mut self, f: F) -> Self
    where
        F: Fn() -> bool + 'static,
    {
        self.can_run = Some(Box::new(f));
        self
    }
}

impl ICommand for RelayCommand {
    fn execute(&self, _parameter: &dyn std::any::Any, cx: &mut App) {
        (self.action)(cx);
    }

    fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool {
        self.can_run.as_ref().map_or(true, |f| f())
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysEnabled;
    struct AlwaysDisabled;

    impl ICommand for AlwaysEnabled {
        fn execute(&self, _parameter: &dyn std::any::Any, _cx: &mut App) {}
    }

    impl ICommand for AlwaysDisabled {
        fn execute(&self, _parameter: &dyn std::any::Any, _cx: &mut App) {}

        fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool {
            false
        }
    }

    #[test]
    fn default_can_execute_is_true() {
        let cmd = AlwaysEnabled;
        assert!(cmd.can_execute(&42_i32));
    }

    #[test]
    fn custom_can_execute_false() {
        let cmd = AlwaysDisabled;
        assert!(!cmd.can_execute(&42_i32));
    }

    #[test]
    fn can_execute_accepts_any_parameter_type() {
        let cmd = AlwaysEnabled;
        assert!(cmd.can_execute(&"string"));
        assert!(cmd.can_execute(&42_i64));
        assert!(cmd.can_execute(&true));
        assert!(cmd.can_execute(&vec![1, 2, 3]));
    }

    #[test]
    fn can_execute_disabled_regardless_of_parameter() {
        let cmd = AlwaysDisabled;
        assert!(!cmd.can_execute(&"text"));
        assert!(!cmd.can_execute(&42));
    }

    #[test]
    fn object_safe_arc_dyn() {
        use std::sync::Arc;
        let cmd: Arc<dyn ICommand> = Arc::new(AlwaysEnabled);
        assert!(Arc::strong_count(&cmd) >= 1);
    }

    #[test]
    fn relay_command_default_can_execute_true() {
        let cmd = RelayCommand::action(|_cx: &mut App| {});
        assert!(cmd.can_execute(&()));
    }

    #[test]
    fn relay_command_can_when_predicate() {
        let cmd = RelayCommand::action(|_cx: &mut App| {}).can_when(|| false);
        assert!(!cmd.can_execute(&()));
    }

    #[test]
    fn relay_command_can_when_true() {
        let cmd = RelayCommand::action(|_cx: &mut App| {}).can_when(|| true);
        assert!(cmd.can_execute(&()));
    }

    #[test]
    fn relay_command_as_arc_dyn_i_command() {
        use std::sync::Arc;
        // 类型级验证：RelayCommand 可转为 Arc<dyn ICommand>
        let cmd: Arc<dyn ICommand> = Arc::new(RelayCommand::action(|_cx: &mut App| {}));
        assert!(cmd.can_execute(&()));
        assert_eq!(Arc::strong_count(&cmd), 1);
    }
}
