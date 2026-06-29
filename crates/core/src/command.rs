//! `ICommand` trait —— 命令系统契约
//!
//! 对齐 WPF `ICommand`：`execute` + `can_execute`，接口纯净无编译器元信息。
//!
//! `#[command]` 标记的方法可被 `.rml` 中的 `on*` 事件绑定调用。
//! 命令方法签名：`fn(&mut self, ev: &Event, cx: &mut Context<Self>)`
//! 或带参数：`fn(&mut self, param: T, ev: &Event, cx: &mut Context<Self>)`
//!
//! `#[command]` 宏是 pass-through（仅校验签名），codegen 直接调用命令方法保留事件类型安全。
//! `ICommand::execute` 作为统一执行入口，用于快捷键、命令面板等动态调度场景，由用户按需实现。

use gpui::Context;

/// 命令基础 trait（对齐 WPF `ICommand`）。
///
/// 命令是 ViewModel 中唯一允许修改视图状态的方法。
/// 命令执行后必须调用 `cx.notify()` 触发重渲染。
///
/// `parameter` 类型擦除为 `&dyn Any`，实现方按需 downcast。
/// 这与 `#[command]` 标记方法的强类型签名正交：
/// - codegen 生成的事件绑定直接调用强类型方法（绕过 trait，保留类型安全）
/// - `ICommand::execute` 用于动态调度场景（快捷键、命令面板、脚本化测试）
///
/// `: 'static` 约束与 `IModel` 一致，确保 `Context<Self>` 可用。
pub trait ICommand: 'static {
    /// 执行命令（WPF: `Execute`）
    ///
    /// `parameter` 类型擦除，实现方按需 `downcast_ref`/`downcast_mut`。
    /// 无参数命令可忽略 `parameter`。
    ///
    /// `where Self: Sized` 约束源于 `Context<Self>` 要求 `Self: Sized`
    /// （GPUI `Context<T>` 是持有 `T` 类型实体的强类型上下文）。
    /// 这不影响实际使用——所有用户定义的 ViewModel 结构体都是 `Sized`。
    fn execute(&mut self, parameter: &dyn std::any::Any, cx: &mut Context<Self>)
    where
        Self: Sized;

    /// 是否可执行（WPF: `CanExecute`）
    ///
    /// 返回 `false` 时 UI 层应禁用对应控件（如按钮 disabled）。
    /// 默认实现返回 `true`。
    fn can_execute(&self, _parameter: &dyn std::any::Any) -> bool {
        true
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
        fn execute(&mut self, _parameter: &dyn std::any::Any, _cx: &mut Context<Self>) {
            // no-op for test
        }
    }

    impl ICommand for AlwaysDisabled {
        fn execute(&mut self, _parameter: &dyn std::any::Any, _cx: &mut Context<Self>) {
            // no-op for test
        }

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
        // 验证 parameter 类型擦除：同一命令可接受任意类型参数
        let cmd = AlwaysEnabled;
        assert!(cmd.can_execute(&"string"));
        assert!(cmd.can_execute(&42_i64));
        assert!(cmd.can_execute(&true));
        assert!(cmd.can_execute(&vec![1, 2, 3]));
    }

    #[test]
    fn can_execute_disabled_regardless_of_parameter() {
        // AlwaysDisabled 对任意参数都返回 false
        let cmd = AlwaysDisabled;
        assert!(!cmd.can_execute(&"text"));
        assert!(!cmd.can_execute(&42));
    }
}
