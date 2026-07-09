//! `ContextMenu` codegen

use crate::compiler::menu::hoist::MenuHoist;
use crate::compiler::menu::item::gen_popup_menu_body;
use crate::compiler::menu::{gen_trigger_children, partition_menu_children};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::Element;

pub fn gen_context_menu(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let (triggers, items) = partition_menu_children(&elem.children);
    let trigger_code = gen_trigger_children(&triggers, ctx, depth, id_counter, loop_vars)?;
    let mut hoist = MenuHoist::default();
    hoist.collect_menu_items(&items, ctx, loop_vars)?;
    let hoist_lets = hoist.gen_lets(ctx);
    let body = gen_popup_menu_body(
        &items,
        Some(elem),
        ctx,
        depth,
        id_counter,
        loop_vars,
        "menu",
        &hoist,
    )?;

    Ok(format!(
        "{{\n            let __rml_menu_weak = cx.weak_entity();\n            {hoist_lets}\n            {trigger_code}.context_menu(move |menu, window, cx| {{\n            {body}\n        }})\n        }}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn kebab_context_menu_codegen() {
        let src = r#"<context-menu>
            <div>Right click</div>
            <menu-item label="Copy" onclick={on_copy} />
        </context-menu>"#;
        let root = parser::parse(src).unwrap();
        let crate::parser::ast::Node::Element(elem) = root else { panic!() };
        let ctx = CodegenCtx::default();
        let mut id = 0;
        let code = gen_context_menu(&elem, &ctx, 0, &mut id, &[]).unwrap();
        assert!(code.contains(".context_menu("));
    }
}
