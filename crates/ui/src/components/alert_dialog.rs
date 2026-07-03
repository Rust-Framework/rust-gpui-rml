//! AlertDialog 封装 —— 基于 gpui-component 的 AlertDialog
//!
//! RML `<dialog>` 元素编译为 `window.open_alert_dialog(...)` 调用：
//! - `title` 属性 → `AlertDialog::title`
//! - `width` 属性 → `AlertDialog::width`
//! - 子元素 → `AlertDialog::content`
//!
//! AlertDialog 默认居中显示，内置 ESC 关闭、关闭按钮，比 Dialog 更精简。
//! 业务代码（如登录表单的提交按钮）通过 `WindowExt::close_dialog` 关闭对话框。

pub use gpui_component::dialog::{
    AlertDialog, DialogAction, DialogButtonProps, DialogClose, DialogContent, DialogDescription,
    DialogFooter, DialogHeader, DialogTitle,
};
