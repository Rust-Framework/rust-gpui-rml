//! RML prelude
//!
//! 使用方式：`use rml::prelude::*;`

pub use rml_core::prelude::*;
pub use rml_macros::*;

pub use crate::compiler::{compile, CodegenCtx, CompileError, CompileOutput};
