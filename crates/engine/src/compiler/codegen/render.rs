//! `impl Render` 生成 —— 从根节点子节点生成 render 方法

use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Element, Node};
use crate::tags;

use super::node::gen_node;
use super::shell;

/// 窗口壳包裹类型
pub(super) enum ShellWrap {
    None,
    Modern,
    Tab,
}

/// 从根节点的子节点生成 `impl Render`
///
/// 单个子节点：直接使用其代码。多个子节点：包裹在 `gpui::div()` 中。
/// 零子节点：使用 `gpui::div()` 作为占位。
pub(super) fn gen_render_impl_from_children(
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
    out.push_str("        if !self.__rml_loaded {\n");
    out.push_str("            self.__rml_loaded = true;\n");
    out.push_str("            rml_core::lifecycle::ILifecycle::on_loaded(self, _window, cx);\n");
    if ctx.contribution_bindings {
        out.push_str("            self.__rml_attach_contribution_bindings(cx);\n");
    }
    out.push_str("        }\n");
    out.push_str("        use gpui::{ParentElement, InteractiveElement, StatefulInteractiveElement, IntoElement, Styled};\n");
    out.push_str("        use rml_ui::{ContextMenuExt, DropdownMenu, PopupMenuItem, Side, h_flex};\n");
    out.push_str("        use rml_ui::{ActiveTheme, ButtonVariants, Disableable, Sizable, Selectable, StyledExt};\n");
    out.push_str("        use rml::runtime::event_flow::convert as rml_convert;\n");

    let mut id_counter: usize = 0;
    let empty: Vec<String> = Vec::new();

    let (slot_menu, slot_title, slot_footer, slot_left, slot_right, slot_bottom, slot_tabs, body_children) =
        if matches!(shell, ShellWrap::Tab | ShellWrap::Modern) {
            shell::partition_slot_children(&elem.children)
        } else {
            (None, None, None, None, None, None, Vec::new(), elem.children.clone())
        };

    let body = if body_children.is_empty() {
        "gpui::div()".to_string()
    } else if body_children.len() == 1 {
        let (code, _) = gen_node(&body_children[0], ctx, 0, &mut id_counter, &empty)?;
        code
    } else {
        let mut code = String::from("gpui::div()");
        for child in &body_children {
            let (child_code, _) = gen_node(child, ctx, 0, &mut id_counter, &empty)?;
            code.push_str(&format!("\n            .child({})", child_code));
        }
        code
    };

    let slot_menu_code = slot_menu
        .as_ref()
        .map(|node| gen_node(node, ctx, 0, &mut id_counter, &empty).map(|(c, _)| c))
        .transpose()?;
    let slot_title_code = slot_title
        .as_ref()
        .map(|node| gen_node(node, ctx, 0, &mut id_counter, &empty).map(|(c, _)| c))
        .transpose()?;
    let slot_footer_code = slot_footer
        .as_ref()
        .map(|node| gen_node(node, ctx, 0, &mut id_counter, &empty).map(|(c, _)| c))
        .transpose()?;
    let slot_left_code = slot_left
        .as_ref()
        .map(|node| gen_node(node, ctx, 0, &mut id_counter, &empty).map(|(c, _)| c))
        .transpose()?;
    let slot_right_code = slot_right
        .as_ref()
        .map(|node| gen_node(node, ctx, 0, &mut id_counter, &empty).map(|(c, _)| c))
        .transpose()?;
    let slot_bottom_code = slot_bottom
        .as_ref()
        .map(|node| gen_node(node, ctx, 0, &mut id_counter, &empty).map(|(c, _)| c))
        .transpose()?;

    // slot_tabs：对每个 <Tab> 子节点调 tab_bar::tab::gen_tab_child 生成代码
    // （模板定制模式，与 tabs={Vec<TabItem>} 简单模式互斥）
    let slot_tabs_codes: Vec<String> = slot_tabs
        .iter()
        .map(|node| {
            if let Node::Element(tab_elem) = node {
                crate::compiler::tab_bar::tab::gen_tab_child(tab_elem, ctx, &mut id_counter, &empty)
            } else {
                Err(CodegenError {
                    message: format!(
                        "<template slot=\"tabs\"> 仅支持 <Tab> 子节点，得到 {:?}",
                        node
                    ),
                })
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
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
            slot_menu_code.as_deref(),
            slot_title_code.as_deref(),
            slot_footer_code.as_deref(),
        )?,
        ShellWrap::Tab => shell::gen_tab_window_wrapper(
            elem,
            ctx,
            &body,
            slot_menu_code.as_deref(),
            slot_title_code.as_deref(),
            slot_footer_code.as_deref(),
            slot_left_code.as_deref(),
            slot_right_code.as_deref(),
            slot_bottom_code.as_deref(),
            slot_tabs_ref.as_deref(),
        )?,
        ShellWrap::None => body,
    };

    // 窗口根节点自动注入 Dialog/Sheet/Notification 渲染层。
    // `<component>` 与 `<dialog>` 不注入（dialog 自身是 layer 内的 child）。
    let with_layers = if matches!(shell, ShellWrap::Modern | ShellWrap::Tab)
        || root_tag_is_window(elem)
    {
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

    out.push_str(&format!("        {}\n", with_layers));
    out.push_str("    }\n");
    out.push_str("}\n");

    Ok(out)
}

/// 判断元素是否为窗口根节点（window/modern_window/tab_window）
fn root_tag_is_window(elem: &Element) -> bool {
    matches!(
        tags::root_tag_lookup(&elem.tag),
        Some(tags::RootTag::Window | tags::RootTag::ModernWindow | tags::RootTag::TabWindow)
    )
}
