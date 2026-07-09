//! `<img>` translator —— 映射到 GPUI 原生 `gpui::img()`
//!
//! GPUI `img()` 构造器要求传入 `source: impl Into<ImageSource>`，不是 builder 模式。
//! 因此 `<img src="x">` 的 src 属性在转译时被提取为 `gpui::img("x")` 的构造参数，
//! 而非通过 `.src()` 链式调用。

use super::{BuiltinMeta, BuiltinTranslator, ComponentCategory, IRmlTranslator};
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Element};

const META: &BuiltinMeta = &BuiltinMeta {
    tag: "img",
    display_name: "Image",
    category: ComponentCategory::Primitive,
    ctor: "gpui::img(\"\")",
    is_container: false,
    is_self_closing: true,
    is_styled: true,
};

#[derive(Debug)]
pub struct ImgTranslator;

impl IRmlTranslator for ImgTranslator {
    fn tag(&self) -> &'static str {
        META.tag
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        // 提取 src 属性作为 img() 构造参数
        let src = elem
            .attributes
            .iter()
            .find_map(|attr| match attr {
                Attribute::Static { name, value, .. } if name == "src" => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_default();
        let ctor = format!("gpui::img({:?})", src);
        super::meta::builtin_engine::translate(
            elem, ctx, id_counter, loop_vars, parents, &ctor, true,
        )
    }

    fn to_rml(
        &self,
        elem: &Element,
        ctx: &super::PrinterCtx,
    ) -> Result<String, super::PrintError> {
        BuiltinTranslator { meta: META }.to_rml(elem, ctx)
    }

    fn metadata(&self) -> super::TranslatorMetadata {
        META.to_metadata()
    }
}
