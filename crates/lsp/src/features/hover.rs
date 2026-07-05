//! 悬停功能：标签/属性文档
//!
//! 悬停标签时，从 `tags::component_lookup` + `props_registry` 拼出文档；
//! 悬停属性时给出属性类别（static/bind/event）与适用标签。

use lsp_types::{Hover, HoverContents, MarkedString};

use rust_rml_engine::compiler::props_registry;
use rust_rml_engine::tags;

use crate::features::ast_util::find_element_at_offset;
use crate::server::conv;
use crate::workspace::Workspace;

/// 执行悬停查询
pub fn hover(
    uri: &lsp_types::Url,
    position: lsp_types::Position,
    workspace: &Workspace,
) -> Option<Hover> {
    let doc = workspace.document(uri)?;
    let tree = &doc.tree;
    let source = tree.text();
    let line_starts = &tree.line_starts;
    let byte_offset = conv::position_to_byte_offset(position, source, line_starts);

    let root = tree.root.as_ref()?;
    let elem = find_element_at_offset(root, byte_offset)?;

    // 悬停在标签名上：给出标签文档
    let range = conv::span_to_range(elem.span, source, line_starts);

    let content = format_tag_hover(&elem.tag);
    Some(Hover {
        range: Some(range),
        contents: HoverContents::Scalar(MarkedString::String(content)),
    })
}

/// 生成标签的悬停文档
fn format_tag_hover(tag: &str) -> String {
    let mut lines = Vec::new();

    if tags::is_root_tag(tag) {
        lines.push(format!("# Root element: `<{}>`", tag));
        match tag {
            "window" => lines.push("Basic window with transparent title bar.".into()),
            "modern_window" => lines.push("Modern window with self-drawn TitleBar/Menu/StatusBar.".into()),
            "tab_window" => lines.push("Advanced window with TabBar title bar and resizable slots.".into()),
            "dialog" => lines.push("Modal dialog (not a separate OS window).".into()),
            "component" => lines.push("Reusable component (no window operations).".into()),
            _ => {}
        }
        if let Some(shell_props) = props_registry::shell_props_for(tag) {
            lines.push(String::new());
            lines.push("## Shell attributes".into());
            for prop in shell_props {
                lines.push(format!("- `{}`", prop));
            }
        }
    } else if tags::lookup(tag).is_some() {
        lines.push(format!("# HTML element: `<{}>`", tag));
        lines.push("Built-in HTML tag mapped to gpui::div().".into());
    } else if tags::component_lookup(tag).is_some() {
        lines.push(format!("# Component: `<{}>`", tag));
        lines.push("gpui-component extension.".into());

        let (statics, binds, events) = props_registry::props_for(tag);
        if !statics.is_empty() {
            lines.push(String::new());
            lines.push("## Static attributes".into());
            for prop in &statics {
                lines.push(format!("- `{}`", prop));
            }
        }
        if !binds.is_empty() {
            lines.push(String::new());
            lines.push("## Bind attributes".into());
            for prop in &binds {
                lines.push(format!("- `{{{{{}}}}}`", prop));
            }
        }
        if !events.is_empty() {
            lines.push(String::new());
            lines.push("## Event attributes".into());
            for prop in &events {
                lines.push(format!("- `{}`", prop));
            }
        }
    } else {
        lines.push(format!("# `<{}>`", tag));
        lines.push("Unknown tag.".into());
    }

    lines.join("\n")
}
