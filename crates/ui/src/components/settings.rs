//! Settings 组件封装 —— 基于 gpui-component 的 Settings
//!
//! 多层嵌套的设置面板组件，层级结构如下：
//!
//! ```text
//! Settings
//!   SettingPage     <- 当前激活显示的页面
//!     SettingGroup
//!       SettingItem
//!         Label
//!         SettingField (Switch / Checkbox / Input / Dropdown / NumberInput / Element)
//! ```
//!
//! `Settings` 构造器为 `Settings::new(id)`，不实现 `Styled`，仅 `RenderOnce`。
//! `SettingGroup` 实现 `Styled`，支持 CSS 样式。
//!
//! ## 声明式语法
//!
//! ```rml
//! <settings id="settings" sidebar-width="280px">
//!     <setting-page title="通用" icon="settings">
//!         <setting-group title="外观">
//!             <setting-item title="暗色主题" field-type="switch"
//!                 value={is_dark} on-change={on_dark_change} />
//!         </setting-group>
//!     </setting-page>
//! </settings>
//! ```

pub use gpui_component::group_box::GroupBoxVariant;
pub use gpui_component::setting::{
    AnySettingField, NumberFieldOptions, RenderOptions, SelectIndex, SettingField,
    SettingFieldElement, SettingFieldType, SettingGroup, SettingItem, SettingPage, Settings,
};
