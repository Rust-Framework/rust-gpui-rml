//! Grid / GridItem translator
//!
//! RML `<Grid>` 编译为 `Grid`（声明式等宽网格布局容器）。
//! RML `<GridItem>` 编译为 `GridItem`（Grid 子项，控制 col-span/row-span/col-start 等）。
//!
//! ## 属性
//!
//! Grid: `columns="3"` / `rows="2"` (static, u16)
//! GridItem: `col-span="2"` / `row-span="3"` / `col-start="1"` / `col-end="4"` / `row-start="2"` / `row-end="5"` (static)

use super::super::{ComponentCategory, IRmlTranslator, PrintError, PrinterCtx, TranslatorMetadata};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::Element;
use crate::tags;

#[derive(Debug)]
pub struct GridTranslator;

impl IRmlTranslator for GridTranslator {
    fn tag(&self) -> &'static str {
        "Grid"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "Grid"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::grid::gen_grid(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("Grid", "Grid", ComponentCategory::Layout).container(true)
    }
}

#[derive(Debug)]
pub struct GridItemTranslator;

impl IRmlTranslator for GridItemTranslator {
    fn tag(&self) -> &'static str {
        "GridItem"
    }

    fn matches(&self, elem: &Element) -> bool {
        tags::canonical_tag(&elem.tag) == "GridItem"
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        let code = crate::compiler::components::grid::gen_grid_item(
            elem, ctx, id_counter, loop_vars, parents,
        )?;
        Ok((code, false))
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError> {
        super::super::utils::print_element(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new("GridItem", "GridItem", ComponentCategory::Layout).container(true)
    }
}

pub fn register(registry: &mut crate::compiler::translator::TranslatorRegistry) {
    registry.register(GridTranslator);
    registry.register(GridItemTranslator);
}
