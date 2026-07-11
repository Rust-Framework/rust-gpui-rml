//! `impl Render` 生成 —— 从根节点子节点生成 render 方法

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Element, Node};

use super::node::gen_node;
use super::shell;

/// 窗口壳包裹类型
pub(crate) enum ShellWrap {
    None,
    Window,
    Modern,
    Tab,
}

/// 从根节点的子节点生成 `impl Render`
///
/// 单个子节点：直接使用其代码。多个子节点：包裹在 `gpui::div()` 中。
/// 零子节点：使用 `gpui::div()` 作为占位。
pub(crate) fn gen_render_impl_from_children(
    elem: &Element,
    ctx: &CodegenCtx,
    shell: ShellWrap,
) -> Result<String, CodegenError> {
    let view_name = &ctx.view_struct_name;
    let mut out = String::new();

    out.push_str("#[allow(unused_imports, unused_variables, non_snake_case, dead_code)]\n");
    out.push_str(&format!("impl gpui::Render for {} {{\n", view_name));
    out.push_str(
        "    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {\n",
    );
    out.push_str("        let _rml_render_guard = rml_core::computed_cache::RenderThreadGuard::enter();\n");
    out.push_str("        if !self.__rml_state.loaded {\n");
    out.push_str("            self.__rml_state.loaded = true;\n");
    out.push_str("            rml_core::lifecycle::ILifecycle::on_loaded(self, _window, cx);\n");
    out.push_str("        }\n");
    out.push_str("        self.__rml_state.drain_notifications(_window, cx);\n");
    out.push_str("        use gpui::{ParentElement, InteractiveElement, StatefulInteractiveElement, IntoElement, Styled};\n");
    out.push_str("        use rml_ui::{ContextMenuExt, DropdownMenu, PopupMenuItem, Side, h_flex};\n");
    out.push_str("        use rml_ui::{ActiveTheme, ButtonVariants, Disableable, GroupBoxVariants, OverflowStyle, ScrollableElement, Sizable, Selectable, StyledExt};\n");
    out.push_str("        use rml::runtime::event_flow::convert as rml_convert;\n");

    let mut id_counter: usize = 0;
    let empty: Vec<String> = Vec::new();

    let mut slots = if matches!(shell, ShellWrap::Tab | ShellWrap::Modern) {
        shell::partition_slot_children(&elem.children)
    } else {
        shell::ShellSlots {
            body: elem.children.clone(),
            ..Default::default()
        }
    };

    // 过滤 <style> 元素：页面级 CSS 指令由 build.rs 在编译期处理，不参与渲染
    slots.body.retain(|node| !matches!(node, Node::Element(e) if e.tag == "style"));

    let body = if slots.body.is_empty() {
        "gpui::div()".to_string()
    } else if slots.body.len() == 1 {
        let (code, _) = gen_node(&slots.body[0], ctx, 0, &mut id_counter, &empty)?;
        code
    } else {
        let mut code = String::from("gpui::div()");
        for child in &slots.body {
            let (child_code, _) = gen_node(child, ctx, 0, &mut id_counter, &empty)?;
            code.push_str(&format!("\n            .child({})", child_code));
        }
        code
    };

    macro_rules! gen_slot_code {
        ($slot:expr) => {{
            let slot = &$slot;
            slot.as_ref()
                .map(|(n, scope_var)| {
                    let loop_vars: Vec<String> = scope_var
                        .as_ref()
                        .map(|s| vec![s.clone()])
                        .unwrap_or_default();
                    gen_node(n, ctx, 0, &mut id_counter, &loop_vars)
                        .map(|(c, _)| (c, scope_var.clone()))
                })
                .transpose()?
        }};
    }
    let slot_menu_code = gen_slot_code!(slots.menu);
    let slot_title_code = gen_slot_code!(slots.title);
    let slot_footer_code = gen_slot_code!(slots.footer);
    let slot_left_code = gen_slot_code!(slots.left);
    let slot_right_code = gen_slot_code!(slots.right);
    let slot_bottom_code = gen_slot_code!(slots.bottom);

    // slot_tabs：两种模式
    // 1) each 模式：<template slot="tabs" each={w in workbenches}><Tab title={w.name()} /></template>
    //    → codegen 单个 <Tab> 子节点（loop_vars=[w]），生成 .tab_children(self.workbenches.iter().map(|w| ...).collect())
    // 2) 列表模式：<template slot="tabs"><Tab /><Tab /></template>
    //    → codegen 每个 <Tab> 子节点，生成 .tab_children(vec![...])
    // 与 tabs={Vec<TabItem>} 简单模式互斥
    let slot_tabs_each: Option<shell::TabsEach> = if let Some(each) = &slots.tabs_each {
        let item = each.item.clone();
        let loop_vars = vec![item.clone()];
        if slots.tabs.len() != 1 {
            return Err(CodegenError {
                message: format!(
                    "<template slot=\"tabs\" each=\"...\"> 需要恰好 1 个 <Tab> 子节点，得到 {} 个",
                    slots.tabs.len()
                ),
                span: Some(elem.span),
            });
        }
        let tab_elem = match &slots.tabs[0] {
            Node::Element(e) => e,
            other => {
                return Err(CodegenError {
                    message: format!(
                        "<template slot=\"tabs\"> 仅支持 <Tab> 子节点，得到 {:?}",
                        other
                    ),
                    span: Some(elem.span),
                })
            }
        };
        let (body, _) = crate::compiler::components::tabs::tab::gen_tab_child(
            tab_elem,
            ctx,
            &mut id_counter,
            &loop_vars,
        )?;
        // 追加 .into() 将 Tab 转换为 TabItem（From<Tab> for TabItem 已定义），
        // 使 .tab_children(...).collect() 推断为 Vec<TabItem>。
        let body = format!("{}.into()", body);
        Some(shell::TabsEach {
            item: each.item.clone(),
            iterable: each.iterable.clone(),
            body,
        })
    } else {
        None
    };

    // 列表模式仅在非 each 模式下处理
    let slot_tabs_codes: Vec<String> = if slot_tabs_each.is_none() {
        slots
            .tabs
            .iter()
            .map(|node| {
                if let Node::Element(tab_elem) = node {
                    let (code, _) = crate::compiler::components::tabs::tab::gen_tab_child(
                        tab_elem,
                        ctx,
                        &mut id_counter,
                        &empty,
                    )?;
                    // 追加 .into() 将 Tab 转换为 TabItem（同 each 模式）。
                    Ok(format!("{}.into()", code))
                } else {
                    Err(CodegenError {
                        message: format!(
                            "<template slot=\"tabs\"> 仅支持 <Tab> 子节点，得到 {:?}",
                            node
                        ),
                        span: Some(elem.span),
                    })
                }
            })
            .collect::<Result<Vec<_>, _>>()?
    } else {
        Vec::new()
    };
    let slot_tabs_ref: Option<Vec<String>> = if slot_tabs_codes.is_empty() {
        None
    } else {
        Some(slot_tabs_codes)
    };

    let final_body = match shell {
        ShellWrap::Modern => shell::gen_modern_window_wrapper(
            elem,
            ctx,
            &body,
            slot_menu_code.as_ref().map(|(c, _)| c.as_str()),
            slot_title_code.as_ref().map(|(c, _)| c.as_str()),
            slot_footer_code.as_ref().map(|(c, _)| c.as_str()),
        )?,
        ShellWrap::Tab => shell::gen_tab_window_wrapper(
            elem,
            ctx,
            &body,
            shell::TabWindowSlotCodes {
                menu: slot_menu_code.as_ref().map(|(c, s)| (c.as_str(), s.as_deref())),
                title: slot_title_code.as_ref().map(|(c, s)| (c.as_str(), s.as_deref())),
                footer: slot_footer_code.as_ref().map(|(c, s)| (c.as_str(), s.as_deref())),
                left: slot_left_code.as_ref().map(|(c, s)| (c.as_str(), s.as_deref())),
                right: slot_right_code.as_ref().map(|(c, s)| (c.as_str(), s.as_deref())),
                bottom: slot_bottom_code.as_ref().map(|(c, s)| (c.as_str(), s.as_deref())),
                tabs: slot_tabs_ref.as_deref(),
                tabs_each: slot_tabs_each,
            },
        )?,
        ShellWrap::None | ShellWrap::Window => body,
    };

    // 窗口根节点自动注入 Dialog/Sheet/Notification 渲染层。
    // `<component>` 与 `<dialog>` 不注入（dialog 自身是 layer 内的 child）。
    let with_layers = if matches!(shell, ShellWrap::Window | ShellWrap::Modern | ShellWrap::Tab) {
        format!(
            "{{\n            \
             let __rml_body = {body};\n            \
             gpui::div()\n                \
             .size_full()\n                \
             .child(__rml_body)\n                \
             .children(rml_ui::Root::render_dialog_layer(_window, cx))\n                \
             .children(rml_ui::Root::render_sheet_layer(_window, cx))\n                \
             .children(rml_ui::Root::render_notification_layer(_window, cx))\n            \
             }}",
            body = final_body
        )
    } else {
        final_body
    };

    // 调用 `__rml_populate_refs()` 将本次渲染惰性创建的 `Entity<T>` 注入到
    // 用户声明的 `ElementRef<T>` 字段（由 `#[component]`/`#[window]` 宏生成）。
    // 必须在元素树构建完成后调用，确保 ref 指令的 codegen 已填充 ref_entities。
    let with_populate = format!(
        "{{\n            \
         let __rml_root = {body};\n            \
         self.__rml_populate_refs();\n            \
         __rml_root\n            \
         }}",
        body = with_layers
    );

    out.push_str(&format!("        {}\n", with_populate));
    out.push_str("    }\n");
    out.push_str("}\n");

    Ok(out)
}
