//! `ICommand` trait + `RelayCommand` + `CallContext` —— 命令贡献系统契约
//!
//! `ICommand : IContribution`——命令本身即是贡献点。命令贡献经 `register_command` 路由到 host，
//! 在 `execute`/`can_execute` 中实现点击行为。
//!
//! `CallContext` 封装 `Window` + `App`，替代旧 `(&dyn Any, &mut App)` 弱类型参数。
//!
//! `RelayCommand` 是框架提供的 `ICommand` 默认实现（WPF `RelayCommand` 等价物），
//! 持有 `WeakEntity<T>` + 闭包，用于 ViewModel 字段绑定（`command={field}`）。
//! 不作为贡献注册（dummy `id`/`name`）。

use gpui::{App, SharedString, Window};

use crate::contribution::IContribution;

/// 命令执行上下文——封装 `Window` + `App`，提供命令执行所需能力。
///
/// 替代旧 `(&dyn Any, &mut App)` 弱类型参数。命令贡献 struct 通过 `self` 携带自身状态，
/// 无需外部 parameter；`CallContext` 提供 `Window`/`App` 访问能力。
pub struct CallContext<'a> {
    pub window: &'a mut Window,
    pub app: &'a mut App,
}

impl<'a> CallContext<'a> {
    pub fn new(window: &'a mut Window, app: &'a mut App) -> Self {
        Self { window, app }
    }
}

/// 命令贡献 trait（对齐 WPF `ICommand`，继承 `IContribution`——命令本身是贡献点）。
///
/// 实现方需同时实现 `IContribution`（id/name/description/icon）和 `ICommand`（execute/can_execute）。
/// `#[contribute(command, ...)]` 宏编译期校验目标已实现 `IContribution`，路由到 `register_command`。
///
/// # 与 `#[command]` 方法的关系
///
/// - `#[command]` 方法：codegen 生成的事件绑定直接调用强类型方法（绕过 trait，保留类型安全）
/// - `ICommand` trait：用于贡献点注册、`command={field}` 绑定、快捷键、命令面板等动态调度场景
pub trait ICommand: IContribution {
    /// 执行命令（WPF: `Execute`）。
    ///
    /// `ctx` 提供 `Window`/`App` 访问能力。命令 struct 自身携带状态（`self`）。
    fn execute(&self, ctx: &mut CallContext);

    /// 是否可执行（WPF: `CanExecute`）。
    ///
    /// 返回 `false` 时 UI 层应禁用对应控件（如按钮 disabled）。
    /// 默认实现返回 `true`。
    fn can_execute(&self, _ctx: &mut CallContext) -> bool {
        true
    }
}

/// WPF `RelayCommand` / `DelegateCommand` 等价物。
///
/// 持有 `WeakEntity<T>` + 闭包，在 `execute` 时 upgrade 弱引用并 `update` ViewModel。
/// 适用于 MVVM 声明式绑定：ViewModel 持有 `Arc<dyn ICommand>` 字段，
/// 在 RML 中通过 `command={field}` 绑定到 `<MenuItem>` 等控件。
///
/// **不作为贡献注册**——`id()`/`name()` 返回 dummy 值。需注册命令贡献时手写 struct
/// 实现 `IContribution` + `ICommand`，用 `#[contribute(command, ...)]` 标记。
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
pub struct RelayCommand {
    action: Box<dyn Fn(&mut App) + Send + Sync + 'static>,
    can_run: Option<Box<dyn Fn() -> bool + Send + Sync + 'static>>,
}

impl RelayCommand {
    /// 从视图绑定闭包创建命令（WPF `RelayCommand(execute)` 模式）。
    ///
    /// 内部捕获 `WeakEntity<T>`，`execute` 时 upgrade 并 `update`。
    /// 闭包签名为 `Fn(&mut T, &mut Context<T>)`，与 `#[command]` 方法体一致。
    pub fn new<T, F>(cx: &gpui::Context<T>, f: F) -> Self
    where
        T: Send + Sync + 'static,
        F: Fn(&mut T, &mut gpui::Context<T>) + Send + Sync + 'static,
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
        F: Fn(&mut App) + Send + Sync + 'static,
    {
        Self {
            action: Box::new(f),
            can_run: None,
        }
    }

    /// 设置 `can_execute` 谓词（Builder 风格）。
    pub fn can_when<F>(mut self, f: F) -> Self
    where
        F: Fn() -> bool + Send + Sync + 'static,
    {
        self.can_run = Some(Box::new(f));
        self
    }
}

impl IContribution for RelayCommand {
    fn id(&self) -> &str {
        "__relay__"
    }

    fn name(&self) -> SharedString {
        SharedString::default()
    }
}

impl ICommand for RelayCommand {
    fn execute(&self, ctx: &mut CallContext) {
        (self.action)(ctx.app);
    }

    fn can_execute(&self, _ctx: &mut CallContext) -> bool {
        self.can_run.as_ref().map_or(true, |f| f())
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::SharedString;

    struct AlwaysEnabled;
    struct AlwaysDisabled;

    impl IContribution for AlwaysEnabled {
        fn id(&self) -> &str {
            "always_enabled"
        }
        fn name(&self) -> SharedString {
            SharedString::default()
        }
    }

    impl IContribution for AlwaysDisabled {
        fn id(&self) -> &str {
            "always_disabled"
        }
        fn name(&self) -> SharedString {
            SharedString::default()
        }
    }

    impl ICommand for AlwaysEnabled {
        fn execute(&self, _ctx: &mut CallContext) {}
    }

    impl ICommand for AlwaysDisabled {
        fn execute(&self, _ctx: &mut CallContext) {}

        fn can_execute(&self, _ctx: &mut CallContext) -> bool {
            false
        }
    }

    // CallContext 测试需要 Window/App，仅验证 trait 层级关系
    #[test]
    fn icommand_extends_icontribution() {
        // 类型级验证：ICommand: IContribution，可 upcast
        fn assert_contribution<T: IContribution>() {}
        assert_contribution::<AlwaysEnabled>();
        assert_contribution::<AlwaysDisabled>();
        assert_contribution::<RelayCommand>();
    }

    #[test]
    fn relay_command_implements_icontribution() {
        let cmd = RelayCommand::action(|_cx: &mut App| {});
        assert_eq!(cmd.id(), "__relay__");
        assert_eq!(cmd.name(), SharedString::default());
    }

    #[test]
    fn relay_command_as_arc_dyn_i_command() {
        use std::sync::Arc;
        let cmd: Arc<dyn ICommand> = Arc::new(RelayCommand::action(|_cx: &mut App| {}));
        assert_eq!(Arc::strong_count(&cmd), 1);
    }

    #[test]
    fn relay_command_as_arc_dyn_icontribution_via_upcast() {
        use std::sync::Arc;
        // trait upcasting：Arc<dyn ICommand> → Arc<dyn IContribution>
        let cmd: Arc<dyn ICommand> = Arc::new(RelayCommand::action(|_cx: &mut App| {}));
        let contrib: Arc<dyn IContribution> = cmd;
        assert_eq!(contrib.id(), "__relay__");
    }

    #[test]
    fn object_safe_arc_dyn() {
        use std::sync::Arc;
        let cmd: Arc<dyn ICommand> = Arc::new(AlwaysEnabled);
        assert!(Arc::strong_count(&cmd) >= 1);
    }
}
