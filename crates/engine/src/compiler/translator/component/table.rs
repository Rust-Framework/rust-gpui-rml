//! Table 容器组件 translator
//!
//! 薄包装 `compiler::table::gen_table`，构造 + 属性 + Column 子节点 + template slot 子节点。

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Directive, Element};
use crate::tags;

#[derive(Debug)]
pub struct TableTranslator;

impl IRmlTranslator for TableTranslator {
    fn tag(&self) -> &'static str {
        "Table"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Table"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
            Directive::Ref { name, .. } => Some(name.as_str()),
            _ => None,
        });
        let id_val = *id_counter;
        *id_counter += 1;

        let code = crate::compiler::components::table::gen_table(
            elem,
            ref_name,
            id_val,
            ctx,
            id_counter,
            loop_vars,
            parents,
        )?;

        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Table", "Table", ComponentCategory::Data).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(TableTranslator);
}
