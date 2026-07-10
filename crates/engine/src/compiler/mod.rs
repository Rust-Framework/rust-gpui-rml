//! RML 编译器入口
//!
//! 串起 parse → validate → codegen，输出 Rust 源码字符串。
//!
//! ## 子模块
//!
//! - `codegen`：代码生成（元素 → Rust 代码）
//! - `components`：扩展组件 codegen 实现
//! - `context`：编译上下文类型（`CodegenCtx`/`CodegenError`/`CompileError`）与 `compile()` 入口
//! - `event`：事件处理器代码生成
//! - `expr`：表达式转换
//! - `printer`：生成代码格式化
//! - `props_registry`：组件属性映射注册表
//! - `setters`：通用属性 setter
//! - `source_map`：sourcemap 收集
//! - `tooltip`：tooltip 属性处理
//! - `translator`：translator 注册表与 builtin/component translator
//! - `validator`：AST 校验

pub mod codegen;
pub mod components;
pub mod context;
pub mod event;
pub mod expr;
pub mod printer;
pub mod props_registry;
pub mod setters;
pub mod source_map;
pub mod style_directive;
pub mod tooltip;
pub mod twoway;
pub mod translator;
pub mod validator;

pub use context::{
    compile, CodegenCtx, CodegenError, CompileError, CompileOutput, InputHandlers,
    UserComponentInfo, ValidationRule, ValidationRuleSet,
};
pub use style_directive::scan_style_directives;
