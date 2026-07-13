//! AlertDialog 与 Dialog 共享类型 re-export
//!
//! ## AlertDialog vs Dialog 区别
//!
//! | 维度 | Dialog | AlertDialog |
//! |------|--------|-------------|
//! | RML 标签 | `<Dialog>` (PascalCase 组件) | `<AlertDialog>` (PascalCase 组件) |
//! | 根标签 | `<dialog>` (小写，RootTag::DialogWindow) | 无根标签形式 |
//! | 构造器 | `Dialog::new(cx)` | `AlertDialog::new(cx)` |
//! | close_button 默认 | `true` | `false` |
//! | overlay_closable 默认 | `true` | `false` |
//! | footer 对齐 | 右对齐 | 居中对齐 |
//! | description 方法 | 无 | `.description()` |
//! | confirm 方法 | 无 | `.confirm()` (显示取消按钮) |
//! | show_cancel 方法 | 无 | `.show_cancel(bool)` |
//! | 使用场景 | 通用模态对话框（表单、设置等） | 警示确认（删除确认、不可逆操作） |
//!
//! ## 根标签 `<dialog>` 的底层实现
//!
//! `<dialog>` 根标签（RootTag::DialogWindow）使用 `window.open_dialog()` 打开 `Dialog`
//! （非 `AlertDialog`），因为通用对话框窗口需要 `close_button(true)` 和
//! `overlay_closable(true)` 默认值，与 Dialog 默认值一致。
//!
//! ## 共享类型
//!
//! 以下类型被 Dialog 和 AlertDialog 共享，统一从此处 re-export：
//! - `DialogButtonProps`：按钮配置（ok_text/cancel_text/on_ok/on_cancel/on_close）
//! - `DialogContent` / `DialogHeader` / `DialogTitle` / `DialogDescription` / `DialogFooter`：内容构建组件
//! - `DialogClose`：关闭动作
//! - `DialogAction`：动作类型

pub use gpui_component::dialog::{
    AlertDialog, DialogAction, DialogButtonProps, DialogClose, DialogContent, DialogDescription,
    DialogFooter, DialogHeader, DialogTitle,
};
