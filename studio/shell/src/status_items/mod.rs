//! 状态栏贡献项 —— 经 `#[contribute]` 注册到 `studio.shell` 的 status 槽位。
//!
//! 扩展方式：新增 `#[contribute(kind = "status")]` struct 即可向状态栏添加新项，
//! 无需修改 shell 主代码。
//!
//! 每个状态栏项独占一个文件（遵循"一个 rs 文件 = 一个职责"约束）。

#[path = "status_encoding.rml.rs"]
pub mod status_encoding;
#[path = "status_language.rml.rs"]
pub mod status_language;
#[path = "status_ready.rml.rs"]
pub mod status_ready;
#[path = "status_theme.rml.rs"]
pub mod status_theme;

pub use status_encoding::StatusEncoding;
pub use status_language::StatusLanguage;
pub use status_ready::StatusReady;
pub use status_theme::StatusTheme;

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
