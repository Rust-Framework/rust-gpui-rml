//! Avatar 封装 —— 基于 gpui-component 的 Avatar / AvatarGroup
//!
//! RML `<Avatar>` 编译为 `rml_ui::Avatar::new().<setters>...`：
//! - `src` 属性 → `Avatar::src`（图片源）
//! - `name` 属性 / 文本子节点 → `Avatar::name`（用户名，无图片时回退为首字母）
//! - `placeholder` 属性 → `Avatar::placeholder(IconName::...)`（占位图标）
//! - `small`/`xsmall`/`large` → Sizable 尺寸
//!
//! RML `<AvatarGroup>` 编译为 `rml_ui::AvatarGroup::new().<setters>.child(Avatar)...`：
//! - `limit` 属性 → `AvatarGroup::limit`（最大显示数量）
//! - `ellipsis` 标志 → `AvatarGroup::ellipsis()`（超限省略标记）
//! - 子节点必须是 `<Avatar>` 元素

pub use gpui_component::avatar::{Avatar, AvatarGroup};
