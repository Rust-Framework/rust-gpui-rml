//! 编辑器命令契约。

use gpui::SharedString;
use rml_core::command::ICommand;

/// 编辑器命令 —— 扩展 `ICommand`,仅添加手势(快捷键)声明。
///
/// `can_execute(ctx)` 已由 `ICommand` 提供,命令实现通过 `ctx.parameter` 的
/// `EditorContext` downcast 判断可用性,无需 `when()` 方法。
///
/// # 应用场景
///
/// - `FormatCommand` → gesture="Shift+Alt+F", can_execute 检查 EditorContext 存在
/// - `RenameCommand` → gesture="F2", can_execute 检查 cursor 存在
/// - `GoToDefinitionCommand` → gesture="F12", can_execute 检查 cursor 存在
pub trait IEditorCommand: ICommand {
    /// 手势绑定(键盘快捷键 "Shift+Alt+F" / "F12")。
    /// None = 无默认快捷键,仅经命令面板触发。
    /// 命名 gesture 而非 keybinding:未来可扩展鼠标/触摸手势。
    fn gesture(&self) -> Option<SharedString> {
        None
    }
}
