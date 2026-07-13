//! RML 终端组件：基于 alacritty_terminal + portable-pty 的嵌入式终端。
//!
//! 提供可在 GPUI 应用中嵌入的终端模拟器，支持：
//! - PTY 进程管理（跨平台，基于 portable-pty）
//! - VTE 解析与渲染（基于 alacritty_terminal）
//! - 键盘/鼠标输入、选择、滚动
//! - 可配置的颜色调色板、字体、尺寸
//!
//! # 快速开始
//!
//! ```ignore
//! use rml_ui_term::{TerminalView, TerminalConfig};
//!
//! let terminal = cx.new(|cx| TerminalView::spawn_default(cx));
//! ```

pub mod clipboard;
pub mod colors;
pub mod config;
pub mod event;
pub mod input;
pub mod layout;
pub mod mouse;
pub mod pty;
pub mod render;
pub mod scroll;
pub mod state;
pub mod view;

pub use clipboard::Clipboard;
pub use config::{ColorPalette, ColorPaletteBuilder};
pub use event::{GpuiEventProxy, TerminalEvent};
pub use pty::{default_shell, shell_for_schema, spawn_terminal, PtyHandles};
pub use render::TerminalRenderer;
pub use scroll::TerminalScrollHandle;
pub use state::TerminalState;
pub use view::{
    BellCallback, ClipboardStoreCallback, ExitCallback, KeyHandler, ResizeCallback,
    TerminalConfig, TerminalView, TitleCallback,
};

/// GPUI [`key_context`](gpui::Div::key_context) id set on [`TerminalView`] while it is focused.
pub const TERMINAL_KEY_CONTEXT: &str = "Terminal";

/// Keymap context for workbench shortcuts that must not run while a terminal has focus.
pub const MENU_SHORTCUT_CONTEXT: &str = "!Terminal";
