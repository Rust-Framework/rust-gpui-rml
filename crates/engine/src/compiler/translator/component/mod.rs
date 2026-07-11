//! 扩展组件 translator
//!
//! 每个扩展组件独占一个 translator 文件，遵循"一个 rs 文件 = 一个组件 / 一个职责"原则：
//! - `stateless`：通用 Stateless / StatelessNoId 组件（Button / Avatar / Card 等，通过 component_lookup + setter 通用分发）
//! - `stateful`：通用 Stateful 组件（Input / TextInput / Slider 等）
//! - `tree` / `code_editor`：从 Stateful 抽出的特殊构造器组件
//! - `tabs` / `tab_bar` / `table` / `description_list` / `popover` / `hover_card` / `sheet` / `dialog` / `accordion`：容器组件
//! - `label` / `separator` / `icon` / `kbd` / `tag` / `alert` / `radio_group` / `activity_bar`：特殊构造组件
//!
//! 本模块保留 `<component content={...}>` 透明容器 translator。

pub mod accordion;
pub mod activity_bar;
pub mod alert;
pub mod alert_dialog;
pub mod code_editor;
pub mod description_list;
pub mod dialog;
pub mod field;
pub mod form;
pub mod hover_card;
pub mod icon;
pub mod kbd;
pub mod label;
pub mod notification;
pub mod otp_input;
pub mod popover;
pub mod radio_group;
pub mod resizable;
pub mod scroll;
pub mod separator;
pub mod settings;
pub mod sheet;
pub mod sidebar;
pub mod sidebar_menu;
pub mod sidebar_menu_item;
pub mod stateful;
pub mod stateless;
pub mod stepper;
pub mod tab_bar;
pub mod table;
pub mod tabs;
pub mod tag;
pub mod tree;
pub mod virtual_list;

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::apply_css_styles;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

/// `<component content={...}>` 透明容器 translator
///
/// 将 `content` 属性的表达式作为任意元素直接嵌入，用于需要命令式构造子树的场景。
/// 不带 `content` 属性的 `<component>` 根节点由 `root::ComponentRootTranslator` 处理。
#[derive(Debug)]
pub struct ComponentTranslator;

impl IRmlTranslator for ComponentTranslator {
    fn tag(&self) -> &'static str {
        "*component-transparent"
    }

    fn matches(&self, elem: &Element) -> bool {
        elem.tag == "component"
            && elem.attributes.iter().any(|a| {
                matches!(a, Attribute::Bind { name, .. } if name == "content")
            })
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        _id_counter: &mut usize,
        _loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let content = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Bind { name, expr, .. } if name == "content" => Some(expr.as_str()),
                _ => None,
            })
            .ok_or_else(|| CodegenError {
                message: "<component> must have content={...} attribute".to_string(),
                span: Some(elem.span),
            })?;

        let mut code = content.to_string();
        if let Some(sheet) = &ctx.stylesheet {
            let style_code = apply_css_styles(elem, &elem.tag, sheet, parents);
            if !style_code.is_empty() {
                code.push_str(&style_code);
            }
        }
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("component", "Component", ComponentCategory::Layout)
    }
}

/// 注册所有扩展组件 translator
pub fn register_all(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    stateless::register(registry);
    stateful::register(registry);
    tree::register(registry);
    code_editor::register(registry);
    tabs::register(registry);
    tab_bar::register(registry);
    table::register(registry);
    description_list::register(registry);
    popover::register(registry);
    hover_card::register(registry);
    sheet::register(registry);
    dialog::register(registry);
    field::register(registry);
    form::register(registry);
    alert_dialog::register(registry);
    accordion::register(registry);
    label::register(registry);
    notification::register(registry);
    scroll::register(registry);
    separator::register(registry);
    icon::register(registry);
    kbd::register(registry);
    tag::register(registry);
    alert::register(registry);
    radio_group::register(registry);
    stepper::register(registry);
    otp_input::register(registry);
    virtual_list::register(registry);
    resizable::register(registry);
    settings::register(registry);
    sidebar::register(registry);
    sidebar_menu::register(registry);
    sidebar_menu_item::register(registry);
    activity_bar::register(registry);
    registry.register(ComponentTranslator);
}
