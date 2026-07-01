//! Demo 应用自定的贡献 host_id
//!
//! 框架不预设任何 host_id；由应用按业务命名并在 Shell / 功能模块间约定。

/// ActivityBar 槽位：侧栏图标 + 活动面板
pub const ACTIVITY_BAR: &str = "demo.shell.activity-bar";

/// StatusBar 槽位：内联状态项
pub const STATUS: &str = "demo.shell.status";

/// 案例树元数据（纯数据 host，由 ViewModel 映射为 `TreeItem`）
pub const CASE_TREE: &str = "demo.shell.case-tree";
