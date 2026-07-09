//! 无状态扩展组件 translator
//!
//! 处理 `ComponentKind::Stateless` 与 `ComponentKind::StatelessNoId` 组件：
//! Button、Avatar、Badge、Card、Checkbox、Collapsible、DropdownButton、GroupBox、
//! Link、Pagination、Progress、ProgressCircle、Radio、Skeleton、Spinner、
//! Switch、Text、Toggle、TitleBar、StatusBar 等。
//!
//! 构造器：
//! - Stateless：`Type::new(ElementId)`（ref 指令生成稳定 ID，否则用计数器）
//! - StatelessNoId：`Type::new()`
//!
//! 子节点处理：
//! - 容器组件（`container=true`）：所有子节点作为 `.child()` / `.children()`
//! - 非容器组件：仅单个文本子节点作为 `.label()`（Avatar 映射为 `.name()`）

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::compiler::setters::{
    component_bind_setter, component_event_setter, component_static_setter,
};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};
use crate::tags;

/// 无状态扩展组件 translator
#[derive(Debug)]
pub struct StatelessComponentTranslator;

impl IRmlTranslator for StatelessComponentTranslator {
    fn tag(&self) -> &'static str {
        "*stateless-component"
    }

    fn matches(&self, elem: &Element) -> bool {
        matches!(
            tags::component_lookup_resolved(&elem.tag).map(|c| c.kind),
            Some(tags::ComponentKind::Stateless | tags::ComponentKind::StatelessNoId)
        ) && !tags::is_menu_container(&elem.tag)
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = gen_stateless_body(elem, ctx, id_counter, loop_vars, parents)?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("*stateless-component", "Stateless Component", ComponentCategory::Layout)
    }
}

/// 生成无状态组件构造代码
fn gen_stateless_body(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<String, CodegenError> {
    let tag = &elem.tag;
    let resolved = tags::normalize_component_tag(tag);

    let component = tags::component_lookup_resolved(tag)
        .ok_or_else(|| CodegenError {
            message: format!("unknown component: <{}>", tag),
            span: Some(elem.span),
        })?;

    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    });

    let id_val = *id_counter;
    *id_counter += 1;

    let mut code = match component.kind {
        tags::ComponentKind::Stateless => {
            if let Some(name) = ref_name {
                format!("{}::new({:?})", component.ctor_path, format!("rml_ref:{}", name))
            } else {
                format!("{}::new((\"rml_el\", {}usize))", component.ctor_path, id_val)
            }
        }
        tags::ComponentKind::StatelessNoId => {
            // 无参构造：TitleBar::new() / StatusBar::new()
            format!("{}::new()", component.ctor_path)
        }
        _ => {
            return Err(CodegenError {
                message: format!(
                    "StatelessComponentTranslator does not handle component kind of <{}>",
                    tag
                ),
                span: Some(elem.span),
            });
        }
    };

    // CSS class 样式（基础层，被后续内联 style / 归一化属性覆盖）
    append_css_class_styles(&mut code, elem, tag, ctx.stylesheet.as_ref(), parents);

    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let mut label_set_by_attr = false;

    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                if let Some(setter) = component_static_setter(name, value, &resolved) {
                    code.push_str(&setter);
                    if name == "label" || (resolved == "Avatar" && name == "name") {
                        label_set_by_attr = true;
                    }
                } else {
                    crate::compiler::setters::check_missing_mapping(ctx, &resolved, name, "static")?;
                }
            }
            Attribute::Bind { name, expr, .. } => {
                if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, &resolved) {
                    code.push_str(&setter);
                    if name == "label" || (resolved == "Avatar" && name == "name") {
                        label_set_by_attr = true;
                    }
                } else {
                    crate::compiler::setters::check_missing_mapping(ctx, &resolved, name, "bind")?;
                }
            }
            Attribute::Event { name, handler, .. } => {
                if let Some(setter) = component_event_setter(name, handler, &resolved) {
                    code.push_str(&setter);
                }
            }
        }
    }

    let canonical = tags::canonical_tag(&resolved);

    if component.container {
        for child in &elem.children {
            let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
            if is_iter {
                code.push_str(&format!("\n            .children({})", child_code));
            } else {
                code.push_str(&format!("\n            .child({})", child_code));
            }
        }
    } else if !label_set_by_attr {
        let text_method = if canonical == "Avatar" { "name" } else { "label" };
        for child in &elem.children {
            if let Node::Text(text) = child {
                code.push_str(&format!(".{}({:?})", text_method, text));
                break;
            }
        }
    }

    Ok(code)
}

/// 注册无状态扩展组件 translator
pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(StatelessComponentTranslator);
}
