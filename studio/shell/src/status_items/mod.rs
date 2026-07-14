//! 状态栏贡献项 —— 经 `#[contribute]` 注册到 `studio.shell` 的 status 槽位。
//!
//! 扩展方式：新增 `#[contribute(kind = "status")]` struct 即可向状态栏添加新项，
//! 无需修改 shell 主代码。
//!
//! 每个状态栏项独占一个文件（遵循"一个 rs 文件 = 一个职责"约束）。

pub mod encoding;
pub mod language;
pub mod ready;
pub mod theme;

pub use encoding::StatusEncoding;
pub use language::StatusLanguage;
pub use ready::StatusReady;
pub use theme::StatusTheme;

use std::sync::Once;

use rml_core::contribution::register_visual_ability;

static STATUS_REGISTERED: Once = Once::new();

/// 注册所有状态栏项的 `IVisual` 能力 cast。
pub fn ensure_status_ready_registered() {
    STATUS_REGISTERED.call_once(|| {
        register_visual_ability::<StatusReady>();
        register_visual_ability::<StatusEncoding>();
        register_visual_ability::<StatusLanguage>();
        register_visual_ability::<StatusTheme>();
    });
}
