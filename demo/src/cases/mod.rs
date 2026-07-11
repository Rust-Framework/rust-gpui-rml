//! RML 案例库模块

pub mod common;

#[path = "overflow_test_case.rml.rs"]
pub mod overflow_test_case;

#[path = "welcome_case.rml.rs"]
pub mod welcome_case;
#[path = "counter_case.rml.rs"]
pub mod counter_case;
#[path = "two_way_case.rml.rs"]
pub mod two_way_case;
#[path = "button_case.rml.rs"]
pub mod button_case;
#[path = "accordion_case.rml.rs"]
pub mod accordion_case;
#[path = "tab_bar_case.rml.rs"]
pub mod tab_bar_case;
#[path = "avatar_case.rml.rs"]
pub mod avatar_case;
#[path = "i18n_case.rml.rs"]
pub mod i18n_case;
#[path = "menu_context_case.rml.rs"]
pub mod menu_context_case;
#[path = "menu_dropdown_case.rml.rs"]
pub mod menu_dropdown_case;
#[path = "menu_editor_case.rml.rs"]
pub mod menu_editor_case;
#[path = "menu_features_case.rml.rs"]
pub mod menu_features_case;
#[path = "menu_custom_case.rml.rs"]
pub mod menu_custom_case;
#[path = "status_bar_case.rml.rs"]
pub mod status_bar_case;
#[path = "table_case.rml.rs"]
pub mod table_case;
#[path = "description_list_case.rml.rs"]
pub mod description_list_case;

// M1 任务 1.3：17 个组件独立 demo
#[path = "badge_case.rml.rs"]
pub mod badge_case;
#[path = "label_case.rml.rs"]
pub mod label_case;
#[path = "separator_case.rml.rs"]
pub mod separator_case;
#[path = "tag_case.rml.rs"]
pub mod tag_case;
#[path = "progress_case.rml.rs"]
pub mod progress_case;
#[path = "progress_circle_case.rml.rs"]
pub mod progress_circle_case;
#[path = "button_group_case.rml.rs"]
pub mod button_group_case;
#[path = "avatar_group_case.rml.rs"]
pub mod avatar_group_case;
#[path = "card_case.rml.rs"]
pub mod card_case;
#[path = "title_bar_case.rml.rs"]
pub mod title_bar_case;
#[path = "native_status_bar_case.rml.rs"]
pub mod native_status_bar_case;
#[path = "checkbox_case.rml.rs"]
pub mod checkbox_case;
#[path = "switch_case.rml.rs"]
pub mod switch_case;
#[path = "input_case.rml.rs"]
pub mod input_case;
#[path = "tree_case.rml.rs"]
pub mod tree_case;
#[path = "slider_case.rml.rs"]
pub mod slider_case;
#[path = "code_editor_case.rml.rs"]
pub mod code_editor_case;
#[path = "alert_case.rml.rs"]
pub mod alert_case;

// Phase 4：6 个框架能力专项案例
#[path = "expression_case.rml.rs"]
pub mod expression_case;
#[path = "conditional_case.rml.rs"]
pub mod conditional_case;
#[path = "list_case.rml.rs"]
pub mod list_case;
#[path = "template_slot_case.rml.rs"]
pub mod template_slot_case;
#[path = "slot_scope_case.rml.rs"]
pub mod slot_scope_case;
#[path = "validation_case.rml.rs"]
pub mod validation_case;
#[path = "theme_case.rml.rs"]
pub mod theme_case;

#[path = "css_priority_case.rml.rs"]
pub mod css_priority_case;
#[path = "css_functions_case.rml.rs"]
pub mod css_functions_case;

// M1'.10：6 个指令专项 demo
#[path = "else_case.rml.rs"]
pub mod else_case;
#[path = "once_case.rml.rs"]
pub mod once_case;
#[path = "html_case.rml.rs"]
pub mod html_case;
#[path = "key_case.rml.rs"]
pub mod key_case;
#[path = "show_case.rml.rs"]
pub mod show_case;
#[path = "ref_case.rml.rs"]
pub mod ref_case;

// M2'.1：Icon 组件 demo
#[path = "icon_case.rml.rs"]
pub mod icon_case;

// M2'.2：Kbd 组件 demo
#[path = "kbd_case.rml.rs"]
pub mod kbd_case;

// M2'.3：Tooltip 通用属性 demo
#[path = "tooltip_case.rml.rs"]
pub mod tooltip_case;

// M2'.4：Popover 容器 demo
#[path = "popover_case.rml.rs"]
pub mod popover_case;

// M3'.6：Tab Preview demo（右键菜单 + 预览模式 + 双击 promote）
#[path = "tab_preview_case.rml.rs"]
pub mod tab_preview_case;

// Phase 1：8 个基础无状态组件 demo
#[path = "spinner_case.rml.rs"]
pub mod spinner_case;
#[path = "link_case.rml.rs"]
pub mod link_case;
#[path = "collapsible_case.rml.rs"]
pub mod collapsible_case;
#[path = "group_box_case.rml.rs"]
pub mod group_box_case;
#[path = "pagination_case.rml.rs"]
pub mod pagination_case;
#[path = "radio_case.rml.rs"]
pub mod radio_case;
#[path = "stepper_case.rml.rs"]
pub mod stepper_case;
#[path = "rating_case.rml.rs"]
pub mod rating_case;
#[path = "otp_input_case.rml.rs"]
pub mod otp_input_case;
#[path = "number_input_case.rml.rs"]
pub mod number_input_case;
#[path = "color_picker_case.rml.rs"]
pub mod color_picker_case;
#[path = "calendar_case.rml.rs"]
pub mod calendar_case;
#[path = "date_picker_case.rml.rs"]
pub mod date_picker_case;
#[path = "select_case.rml.rs"]
pub mod select_case;
#[path = "combobox_case.rml.rs"]
pub mod combobox_case;
#[path = "virtual_list_case.rml.rs"]
pub mod virtual_list_case;
#[path = "resizable_case.rml.rs"]
pub mod resizable_case;
#[path = "settings_case.rml.rs"]
pub mod settings_case;

// P0-1：用户组件事件绑定 demo
#[path = "user_component_event_case.rml.rs"]
pub mod user_component_event_case;

// P0-2：content 绑定 demo（IVisual/AnyElement/ToString）
#[path = "content_binding_case.rml.rs"]
pub mod content_binding_case;

// 基础能力补齐：焦点事件 demo
#[path = "focus_event_case.rml.rs"]
pub mod focus_event_case;
