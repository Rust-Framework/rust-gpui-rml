//! 原生 HTML 标签 translators
//!
//! 所有原生标签已迁移为独立的 `IRmlTranslator` 实现。
//! 每个标签独占一个文件，公共转译逻辑委托 `BuiltinTranslator` 引擎。

pub mod meta;

pub mod a;
pub mod br;
pub mod button;
pub mod code;
pub mod div;
pub mod h1;
pub mod h2;
pub mod h3;
pub mod h4;
pub mod h5;
pub mod h6;
pub mod img;
pub mod input;
pub mod label;
pub mod li;
pub mod ol;
pub mod p;
pub mod span;
pub mod textarea;
pub mod ul;

pub use meta::{BuiltinMeta, BuiltinTranslator};

pub use super::{ComponentCategory, IRmlTranslator, PrintError, TranslatorMetadata};
pub use super::ctx::PrinterCtx;

use super::TranslatorRegistry;

/// 注册所有原生 HTML 标签 translator
pub fn register_all(registry: &mut TranslatorRegistry) {
    registry.register(div::DivTranslator);
    registry.register(span::SpanTranslator);
    registry.register(p::PTranslator);
    registry.register(h1::H1Translator);
    registry.register(h2::H2Translator);
    registry.register(h3::H3Translator);
    registry.register(h4::H4Translator);
    registry.register(h5::H5Translator);
    registry.register(h6::H6Translator);
    registry.register(button::ButtonTranslator);
    registry.register(input::InputTranslator);
    registry.register(textarea::TextAreaTranslator);
    registry.register(ul::UlTranslator);
    registry.register(ol::OlTranslator);
    registry.register(li::LiTranslator);
    registry.register(img::ImgTranslator);
    registry.register(a::ATranslator);
    registry.register(label::LabelTranslator);
    registry.register(br::BrTranslator);
    registry.register(code::CodeTranslator);
}
