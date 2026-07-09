//! `DropdownMenu` codegen

use crate::compiler::menu::hoist::MenuHoist;
use crate::compiler::menu::item::gen_popup_menu_body;
use crate::compiler::menu::popup::anchor_from_elem;
use crate::compiler::menu::{gen_trigger_children, partition_menu_children};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::Element;

pub fn gen_dropdown_menu(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<String, CodegenError> {
    let (triggers, items) = partition_menu_children(&elem.children);
    if triggers.is_empty() {
        return Err(CodegenError {
            message: "DropdownMenu requires a trigger child (e.g. Button)".to_string(),
            span: Some(elem.span),
        });
    }
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
    let anchor = anchor_from_elem(elem);

    Ok(format!(
        "{{\n            let __rml_menu_weak = cx.weak_entity();\n            {hoist_lets}\n            {trigger_code}.dropdown_menu_with_anchor({anchor}, move |menu, window, cx| {{\n            {body}\n        }})\n        }}"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;

    #[test]
    fn dropdown_with_anchor() {
        let src = r#"<DropdownMenu anchor="TopRight">
            <Button label="Options" ghost="" />
            <MenuSeparator />
            <MenuItem label="Exit" onclick={on_exit} />
        </DropdownMenu>"#;
        let root = parser::parse(src).unwrap();
        let crate::parser::ast::Node::Element(elem) = root else { panic!() };
        let ctx = CodegenCtx::default();
        let mut id = 0;
        let code = gen_dropdown_menu(&elem, &ctx, 0, &mut id, &[]).unwrap();
        assert!(code.contains("dropdown_menu_with_anchor"));
        assert!(code.contains("Anchor::TopRight"));
        assert!(code.contains("menu.separator()"));
    }
}
