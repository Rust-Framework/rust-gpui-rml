//! 语义层（Roslyn SemanticModel 等价物）
//!
//! 惰性解析绑定路径/命令名 → 产出语义诊断。

pub mod binder;
pub mod diagnostics;
pub mod model;
pub mod tokens;
