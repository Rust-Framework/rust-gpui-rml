//! Resizable 专用 translator —— h_resizable/v_resizable 函数构造 + 面板子节点处理
//!
//! Resizable 构造器为函数 `h_resizable(id)` / `v_resizable(id)`，非 `ResizablePanelGroup::new(id)`。
//! 核心要点：
//! - `direction` 属性选择 h/v 构造器（默认 horizontal）
//! - ResizablePanelGroup 不实现 Styled，CSS 样式不作用于 `<resizable>`
//! - ResizablePanel 实现 Styled + ParentElement，CSS 样式和子节点作用于 `<resizable-panel>`
//! - 非 panel 子节点通过 `From<T: Into<AnyElement>> for ResizablePanel` 自动包裹
//!
//! ## 生成代码示例
//!
//! ```ignore
//! rml_ui::h_resizable(("rml_resizable", 0usize))
//!     .size(gpui::px(400.))
//!     .child(rml_ui::resizable_panel().size(gpui::px(200.)).child(...))
//!     .child(rml_ui::resizable_panel().child(...))
//! ```

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::append_css_class_styles;
use crate::compiler::codegen::gen_node;
use crate::compiler::setters::{
    component_bind_rust_expr, component_event_setter, component_static_setter,
};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element, EventHandler, Node};
use crate::tags;

// ──────────────────────────────────────────────────────────────────────────
//  ResizableTranslator：<resizable> 面板组
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ResizableTranslator;

impl IRmlTranslator for ResizableTranslator {
    fn tag(&self) -> &'static str {
        "Resizable"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Resizable"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        _parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let id_val = *id_counter;
        *id_counter += 1;

        // 1. direction 属性 → 选择 h/v 构造器（默认 horizontal）
        let direction =
            extract_static_attr(elem, "direction").unwrap_or_else(|| "horizontal".to_string());
        let ctor_fn = match direction.as_str() {
            "vertical" | "v" => "rml_ui::v_resizable",
            _ => "rml_ui::h_resizable",
        };

        let mut code = format!("{}((\"rml_resizable\", {}usize))", ctor_fn, id_val);

        // 2. 不应用 CSS class 样式（ResizablePanelGroup 不实现 Styled）

        let resolved = tags::normalize_component_tag(&elem.tag);
        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        // 3. 应用属性（跳过 direction，特殊处理 size/height/width/on_resize）
        // ResizablePanelGroup 不实现 Styled，height/width 不能走 CSS 样式路径
        // 对 horizontal：height → .size()；对 vertical：width → .size()
        let cross_axis_attr = if direction == "vertical" { "width" } else { "height" };
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    if name == "direction" {
                        continue;
                    }
                    if name == "size" || name == cross_axis_attr {
                        if let Some(px_code) = parse_px_value(value) {
                            code.push_str(&format!(".size({})", px_code));
                        }
                        continue;
                    }
                    // 跳过非交叉轴的 height/width（ResizablePanelGroup 不实现 Styled）
                    if name == "height" || name == "width" {
                        continue;
                    }
                    if let Some(setter) = component_static_setter(name, value, &resolved) {
                        code.push_str(&setter);
                    }
                }
                Attribute::Bind { name, expr, .. } => {
                    if name == "size" || name == cross_axis_attr {
                        let rust_expr = component_bind_rust_expr(expr, &lv, &computed);
                        code.push_str(&format!(".size({})", rust_expr));
                        continue;
                    }
                    if name == "height" || name == "width" {
                        continue;
                    }
                    if let Some(setter) =
                        crate::compiler::setters::component_bind_setter(name, expr, &lv, &computed, &resolved)
                    {
                        code.push_str(&setter);
                    }
                }
                Attribute::Event { name, handler, .. } => {
                    if name == "on_resize" {
                        code.push_str(&gen_on_resize_handler(handler));
                        continue;
                    }
                    if let Some(setter) = component_event_setter(name, handler, &resolved) {
                        code.push_str(&setter);
                    }
                }
            }
        }

        // 4. 处理子节点：通过 gen_node 委托
        // <resizable-panel> 子节点由 ResizablePanelTranslator 生成 resizable_panel()...
        // 非 panel 子节点需显式 .into_any_element() 后通过 From<AnyElement> for ResizablePanel 包裹
        for child in &elem.children {
            let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
            let is_panel =
                matches!(child, Node::Element(e) if tags::canonical_tag(&e.tag) == "ResizablePanel");
            let wrapped_code = if is_panel {
                child_code
            } else {
                format!("{}.into_any_element()", child_code)
            };
            if is_iter {
                code.push_str(&format!("\n            .children({})", wrapped_code));
            } else {
                code.push_str(&format!("\n            .child({})", wrapped_code));
            }
        }

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Resizable", "Resizable", ComponentCategory::Layout)
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  ResizablePanelTranslator：<resizable-panel> 面板
// ──────────────────────────────────────────────────────────────────────────

#[derive(Debug)]
pub struct ResizablePanelTranslator;

impl IRmlTranslator for ResizablePanelTranslator {
    fn tag(&self) -> &'static str {
        "ResizablePanel"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "ResizablePanel"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let resolved = tags::normalize_component_tag(&elem.tag);
        let component = tags::component_lookup_resolved(&elem.tag).ok_or_else(|| CodegenError {
            message: format!("unknown component: <{}>", elem.tag),
            span: Some(elem.span),
        })?;

        // 1. 生成 resizable_panel() 函数调用（非 ::new()）
        let mut code = format!("{}()", component.ctor_path);

        // 2. CSS class 样式（ResizablePanel 实现 Styled）
        append_css_class_styles(&mut code, elem, &elem.tag, ctx.stylesheet.as_ref(), parents);

        let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        // 3. 应用属性（特殊处理 size/size_range/visible）
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    if name == "size" {
                        if let Some(px_code) = parse_px_value(value) {
                            code.push_str(&format!(".size({})", px_code));
                        }
                        continue;
                    }
                    if name == "visible" {
                        code.push_str(&format!(".visible({})", parse_bool(value)));
                        continue;
                    }
                    if name == "size_range" {
                        if let Some(range_code) = parse_size_range(value) {
                            code.push_str(&format!(".size_range({})", range_code));
                        }
                        continue;
                    }
                    if let Some(setter) = component_static_setter(name, value, &resolved) {
                        code.push_str(&setter);
                    }
                }
                Attribute::Bind { name, expr, .. } => {
                    if name == "size" {
                        let rust_expr = component_bind_rust_expr(expr, &lv, &computed);
                        code.push_str(&format!(".size({})", rust_expr));
                        continue;
                    }
                    if name == "visible" {
                        let rust_expr = component_bind_rust_expr(expr, &lv, &computed);
                        code.push_str(&format!(".visible({})", rust_expr));
                        continue;
                    }
                    if let Some(setter) =
                        crate::compiler::setters::component_bind_setter(name, expr, &lv, &computed, &resolved)
                    {
                        code.push_str(&setter);
                    }
                }
                Attribute::Event { name, handler, .. } => {
                    if let Some(setter) = component_event_setter(name, handler, &resolved) {
                        code.push_str(&setter);
                    }
                }
            }
        }

        // 4. 处理子节点（ResizablePanel 实现 ParentElement）
        for child in &elem.children {
            let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
            if is_iter {
                code.push_str(&format!("\n            .children({})", child_code));
            } else {
                code.push_str(&format!("\n            .child({})", child_code));
            }
        }

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("ResizablePanel", "ResizablePanel", ComponentCategory::Layout)
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  辅助函数
// ──────────────────────────────────────────────────────────────────────────

/// 从元素属性中提取 static 字符串值
fn extract_static_attr(elem: &Element, name: &str) -> Option<String> {
    elem.attributes.iter().find_map(|attr| {
        if let Attribute::Static { name: n, value, .. } = attr {
            if n == name {
                return Some(value.clone());
            }
        }
        None
    })
}

/// 解析像素值字符串 → Rust 代码
/// "400px" → "gpui::px(400.)"
/// "400"   → "gpui::px(400.)"
fn parse_px_value(value: &str) -> Option<String> {
    let v = value.trim().trim_end_matches("px").trim();
    let n: f64 = v.parse().ok()?;
    // 确保 float literal（gpui::px 接受 f32，400 会推断为整数导致类型不匹配）
    let n_str = if n.fract() == 0.0 {
        format!("{:.0}.", n)
    } else {
        format!("{}", n)
    };
    Some(format!("gpui::px({})", n_str))
}

/// 解析布尔值
fn parse_bool(value: &str) -> &'static str {
    if value.is_empty() || value.eq_ignore_ascii_case("true") {
        "true"
    } else {
        "false"
    }
}

/// 解析尺寸范围字符串 → Rust 代码
/// "100px..500px" → "gpui::px(100.)..gpui::px(500.)"
fn parse_size_range(value: &str) -> Option<String> {
    let parts: Vec<&str> = value.split("..").collect();
    if parts.len() != 2 {
        return None;
    }
    let start = parse_px_value(parts[0])?;
    let end = parse_px_value(parts[1])?;
    Some(format!("{}..{}", start, end))
}

/// 生成 on_resize 事件处理器代码
///
/// on_resize 回调签名：`Fn(&Entity<ResizableState>, &mut Window, &mut App) + 'static`
/// 用户方法签名约定：`fn on_resize(&mut self, state: &Entity<ResizableState>, cx: &mut Context<Self>)`
fn gen_on_resize_handler(handler: &EventHandler) -> String {
    let method = match handler {
        EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
        EventHandler::WithArgs(m, _) => m,
        EventHandler::ClosureField(_) => "",
    };
    format!(
        ".on_resize({{\n                    \
         let weak = cx.weak_entity();\n                    \
         move |state: &gpui::Entity<rml_ui::ResizableState>, _window: &mut gpui::Window, app: &mut gpui::App| {{\n                        \
         if let Some(entity) = weak.upgrade() {{\n                            \
         entity.update(app, |this, cx| {{ this.{}(state, cx); }});\n                        \
         }}\n                    \
         }}\n                \
         }})",
        method
    )
}

/// 注册 Resizable translator
pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(ResizableTranslator);
    registry.register(ResizablePanelTranslator);
}
