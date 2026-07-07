//! RML 编译器入口
//!
//! 串起 parse → validate → codegen，输出 Rust 源码字符串。

pub mod accordion;
pub mod alert;
pub mod avatar;
pub mod badge;
pub mod card;
pub mod code_editor;
pub mod codegen;
pub mod component;
pub mod description_list;
pub mod event;
pub mod expr;
pub mod icon;
pub mod input;
pub mod kbd;
pub mod label;
pub mod menu;
pub mod popover;
pub mod props_registry;
pub mod radio_group;
pub mod separator;
pub mod source_map;
pub mod tab_bar;
pub mod tabs;
pub mod tag;
pub mod table;
pub mod tooltip;
pub mod tree;
pub mod user_component;
pub mod validator;

use crate::css::StyleSheet;
use crate::parser;
use crate::parser::Span;
use crate::compiler::source_map::SourceMap;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;

/// 字段校验规则（Phase B-3.2：`#[validate]` 宏）
///
/// 由 `crates/engine/src/build/scanner.rs` 从 `.rml.rs` 的 `#[validate(...)]` 属性提取，
/// 经 `CodegenCtx.field_validations` 传递给 codegen，生成校验代码写入 `__rml_state.field_errors`。
#[derive(Debug, Clone)]
pub enum ValidationRule {
    /// 非空校验（String 非空、数字非零）
    Required,
    /// 字符串长度范围（min/max 任一可省略）
    Length { min: Option<i64>, max: Option<i64> },
    /// 数值范围（min/max 任一可省略）
    Range { min: Option<f64>, max: Option<f64> },
    /// 正则匹配（pattern 为正则表达式字符串）
    Regex(String),
    /// 自定义校验函数（函数名，签名 `fn(&str) -> Option<SharedString>`）
    Custom(String),
}

/// 字段校验规则集
///
/// 一个字段可声明多个规则，按声明顺序执行。任一失败则写入错误状态，不赋值、不 bump_version。
/// `custom_message` 为 `Some` 时覆盖所有失败分支的默认错误消息。
///
/// Phase B-3.3：支持 `validator_type` 接口式校验（`#[validate(MyValidator)]`）。
/// 与 `rules`/`custom_message` 互斥——IValidate 已封装完整校验逻辑（含消息）。
#[derive(Debug, Clone, Default)]
pub struct ValidationRuleSet {
    /// 规则列表（按声明顺序）
    pub rules: Vec<ValidationRule>,
    /// 自定义错误消息（覆盖默认消息）
    pub custom_message: Option<String>,
    /// IValidate 类型名（Phase B-3.3：`#[validate(MyValidator)]`）
    ///
    /// 为 `Some` 时，`rules` 与 `custom_message` 必须为空（互斥）。
    /// codegen 通过 `MyValidator::default().valid_with_view(value, this)` 调用。
    pub validator_type: Option<String>,
}

/// `<input model={field} oninput={fn} onchange={fn} />` 的 handler 映射（Phase B-3）
///
/// 由 `collect_model_input_handlers` 从 AST 收集，codegen 在 `gen_input_state_impl`
/// 的 `cx.subscribe` 回调内，model 反向同步之后、`cx.notify()` 之前调用 handler。
#[derive(Debug, Clone, Default)]
pub struct InputHandlers {
    /// `oninput={method}` 的方法名
    pub on_input: Option<String>,
    /// `onchange={method}` 的方法名
    pub on_change: Option<String>,
}

/// 用户自定义组件信息（`#[component]` 标注的 struct）
///
/// 由 build.rs 从所有 `.rml.rs` 文件扫描收集，注入 `CodegenCtx.user_components`。
/// codegen 在遇到 `<CounterCase />` 等用户组件标签时，生成
/// `self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()`。
#[derive(Debug, Clone, Default)]
pub struct UserComponentInfo {
    /// struct 名（如 "CounterCase"）
    pub struct_name: String,
    /// snake_case 字段名（如 "counter_case"，父视图中的 `Option<Entity<CounterCase>>` 字段名）
    pub entity_field: String,
    /// `#[component(slots = [...])]` 声明的具名插槽列表
    ///
    /// 父视图 codegen 据此分离 `<template slot="x">` 子节点并注入到对应 slot setter。
    /// 空 Vec 表示组件不接受任何插槽。
    pub slots: Vec<String>,
    /// 所有 pub 字段名 → 类型字符串（如 `"title" → "SharedString"`、`"count" → "i32"`）
    ///
    /// 由 build.rs 从 `StructMetadata.field_types` 拷贝。codegen 在 `gen_user_component` 中
    /// 处理 `<CaseDocPage title={...}>` 等属性绑定时，据此生成类型转换代码
    /// （`String`/`SharedString` → `.into()`/`.clone()`，`i32` → `parse()`，`Vec<_>` → `.clone()`）。
    pub field_types: HashMap<String, String>,
    /// 所有 `#[computed]` 方法名列表
    ///
    /// 由 build.rs 从 `StructMetadata.computed_methods` 拷贝。codegen 在处理绑定属性时
    /// 据此区分 `self.rml_sample()`（方法调用）与 `self.title`（字段访问）。
    pub computed_methods: Vec<String>,
}

/// 代码生成上下文
#[derive(Debug, Clone, Default)]
pub struct CodegenCtx {
    /// 视图结构体名（如 "Counter"）
    pub view_struct_name: String,
    /// 视图模块路径（如 "my_app::views::counter"）
    pub view_module_path: String,
    /// 全局样式表（由 build.rs 加载所有 `.css` 文件合并而成）
    ///
    /// codegen 在遇到 `class="..."` 属性时查询此样式表，
    /// 将匹配的 CSS 规则转换为 GPUI 样式方法调用。
    /// 为 None 时（如单元测试）class 属性不生成样式代码。
    pub stylesheet: Option<StyleSheet>,
    /// 计算属性方法名列表（由 build.rs 扫描 `.rml.rs` 文件中的 `#[computed]` 收集）
    ///
    /// 当插值 `{name}` 中的 `name` 在此列表中时，codegen 生成 `self.name()`（方法调用）
    /// 而非 `self.name`（字段访问）。
    pub computed_methods: Vec<String>,
    /// 当前 struct 的所有 pub 字段名（Phase B-2：observable 字段追踪）
    ///
    /// 由 build.rs 通过 syn 扫描 `.rml.rs` 提取，与 `IModel::rml_fields` 一致。
    /// codegen 据此生成 `__rml_bump_version`/`__rml_get_version` 的 match 臂。
    pub observable_fields: Vec<String>,
    /// 全部用户字段名（pub + private），供版本计数 match 臂生成
    pub version_fields: Vec<String>,
    /// 每个 `#[computed]` 方法 → 依赖的 pub 字段列表（Phase B-2：缓存依赖追踪）
    ///
    /// 由 build.rs 通过 `syn::visit::Visit` 扫描 `#[computed]` 方法体中的
    /// `self.<field>` 访问收集。codegen 据此生成 `__rml_computed_deps_version` 方法，
    /// 对每个 computed 方法 sum 其依赖字段的版本号作为缓存键。
    pub computed_deps: HashMap<String, Vec<String>>,
    /// 每个 `#[computed]` 方法 → 返回类型字符串（Phase B-2：缓存包装方法签名）
    ///
    /// codegen 生成的包装方法需要显式标注返回类型以调用
    /// `ComputedCache::get_or_compute::<T, _>(...)`。
    /// 由 build.rs 从 `method.sig.output` 提取（如 `"i32"`、`"Vec<TabItem>"`）。
    pub computed_returns: HashMap<String, String>,
    /// 每个 pub 字段 → 类型字符串（Phase B-3：双向绑定类型转换）
    ///
    /// codegen 的 `gen_model_input` 据此生成类型转换代码：
    /// `i32`/`u32` 等 → `state.value().parse::<T>().unwrap_or(0)`，
    /// `String`/`SharedString` → `state.value().into()`。
    pub field_types: HashMap<String, String>,
    /// 每个 pub 字段 → 校验规则集（Phase B-3.2：`#[validate]` 宏）
    ///
    /// 由 scanner 从 `.rml.rs` 的 `#[validate(...)]` 属性提取。
    /// codegen 的 `gen_field_assign_expr` 据此在 parse 成功后、赋值前
    /// 生成规则校验链（range/length/required/regex/custom）。
    pub field_validations: HashMap<String, ValidationRuleSet>,
    /// RML 中声明 `model={field}` 的字段名（双向绑定 input 专用）
    pub model_fields: Vec<String>,
    /// `model={field | Converter}` 的 converter 映射（Phase B-2：双向绑定 convert_back）
    ///
    /// key 为字段名，value 为 converter 类型名（如 "Currency"）。
    /// codegen 的 `gen_field_assign_expr` 据此在反向绑定时调用
    /// `ConverterName::default().convert_back(&value)` 替代裸 `parse`。
    pub model_converters: HashMap<String, String>,
    /// `<input model={field} oninput={fn} onchange={fn} />` 的 handler 映射（Phase B-3）
    ///
    /// 由 `collect_model_input_handlers` 从 AST 收集。codegen 的 `gen_input_state_impl`
    /// 据此在 `cx.subscribe` 回调内、model 反向同步之后、`cx.notify()` 之前调用用户 handler。
    pub model_input_handlers: HashMap<String, InputHandlers>,
    /// 用户自定义组件注册表（`#[component]` 标注的 struct）
    ///
    /// 由 build.rs 从所有 `.rml.rs` 文件扫描收集，key 为 struct 名（如 "CounterCase"）。
    /// codegen 在 `gen_component` 中 `component_lookup` 未命中时查此表，
    /// 生成 `self.<entity_field>.as_ref().expect(...).clone()` 嵌入用户组件。
    pub user_components: HashMap<String, UserComponentInfo>,
    /// 是否标注 `#[contributehost]`（注册 host slot）
    pub is_contributehost: bool,
    /// 生命周期钩子（Phase B-3：`#[on_loaded]`/`#[on_unloaded]` 自动联动）
    ///
    /// 由 build.rs 扫描 `.rml.rs` impl 块中的 `#[on_loaded]`/`#[on_unloaded]` 标注方法收集，
    /// codegen 据此生成 `impl ILifecycle for <View>` 自动联动。
    pub lifecycle_hooks: crate::build::scanner::LifecycleHooks,
    /// 是否已存在手动 `impl ILifecycle for <Type>` 块
    ///
    /// 若为 `true` 且 `lifecycle_hooks` 非空：codegen 跳过自动生成并发出 warning
    /// （避免重复 impl 导致编译错误）。
    pub has_manual_lifecycle_impl: bool,
    /// 严格模式：将 codegen 期间检测到的「已注册但无映射」类 warning 升级为 error。
    ///
    /// 由 build.rs 的 `Builder.strict(true)` 设置（默认 true）。
    /// 单元测试中默认 false，便于隔离测试单条 setter 路径而不触发其他路径的 error。
    pub strict: bool,
    /// slot 闭包内引用父视图数据时的 self 别名（Phase 2：slot 闭包捕获父视图数据）
    ///
    /// 由 `gen_user_component` 在生成 slot 内容前 clone ctx 并设置为
    /// `Some("__rml_self_ref".to_string())`。表达式生成函数据此把 `self.xxx`
    /// 替换为 `__rml_self_ref.xxx`，绕过 slot 闭包的生命周期限制。
    /// 默认 `None`，行为不变（生成 `self.xxx`）。
    pub self_alias: Option<String>,
    /// sourcemap 收集器（codegen 透传 AST span → 生成代码位置）
    ///
    /// 由 `compile()` 在调用 codegen 前创建空实例并传入；codegen 在生成关键代码片段
    /// （元素构造、属性 setter、事件绑定等）时调用 `ctx.source_map.borrow_mut().record(...)`。
    /// build.rs 将其序列化为 `.rml.map` 文件供 dap crate 消费。
    ///
    /// 使用 `RefCell` 包裹以保持 `&CodegenCtx` 不可变借用在 codegen 全链路传播，
    /// 同时允许 sourcemap 在生成过程中增量记录。
    pub source_map: RefCell<SourceMap>,
}

/// 代码生成错误
///
/// 由 codegen / event / component 模块共用，定义在此避免循环依赖。
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
    /// 错误对应的 `.rml` 源码区间（可选）
    ///
    /// 由 codegen 报错路径透传 AST 节点的 `span`，便于上层（build.rs / LSP）
    /// 定位到具体源码位置。`None` 表示无法定位（如合成节点或代码逻辑错误）。
    pub span: Option<Span>,
}

impl CodegenError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            span: None,
        }
    }

    /// 附带源码区间
    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Codegen error: {}", self.message)?;
        if let Some(span) = self.span {
            write!(f, " (span {}..{})", span.start, span.end)?;
        }
        Ok(())
    }
}

impl std::error::Error for CodegenError {}

/// 编译错误
#[derive(Debug)]
pub enum CompileError {
    Parse(parser::ParseError),
    Validate(validator::ValidationError),
    Codegen(CodegenError),
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CompileError::Parse(e) => write!(f, "{}", e),
            CompileError::Validate(e) => write!(f, "{}", e),
            CompileError::Codegen(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for CompileError {}

impl From<parser::ParseError> for CompileError {
    fn from(e: parser::ParseError) -> Self {
        CompileError::Parse(e)
    }
}
impl From<validator::ValidationError> for CompileError {
    fn from(e: validator::ValidationError) -> Self {
        CompileError::Validate(e)
    }
}
impl From<CodegenError> for CompileError {
    fn from(e: CodegenError) -> Self {
        CompileError::Codegen(e)
    }
}

/// 编译 `.rml` 源码为 Rust 源码字符串 + sourcemap
///
/// # 参数
/// - `source`: `.rml` 文件内容
/// - `ctx`: 代码生成上下文（含视图结构名）
///
/// # 返回
/// `CompileOutput`，包含生成的 `impl Render for <View>` 代码块字符串与 sourcemap。
/// sourcemap 由 codegen 在生成过程中透传 AST span 收集，可持久化为 `.rml.map`。
pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<CompileOutput, CompileError> {
    let root = parser::parse(source)?;
    validator::validate(&root, &ctx.user_components)?;
    let mut ctx = ctx.clone();
    ctx.model_fields = codegen::collect_model_fields(&root);
    ctx.model_converters = codegen::collect_model_converters(&root);
    ctx.model_input_handlers = codegen::collect_model_input_handlers(&root);
    let code = codegen::codegen(&root, &ctx)?;
    Ok(CompileOutput {
        code,
        source_map: ctx.source_map.into_inner(),
    })
}

/// 编译输出
///
/// 由 `compile()` 返回，包含生成的 Rust 代码与源映射。
#[derive(Debug, Clone)]
pub struct CompileOutput {
    /// 生成的 `impl Render for <View>` 代码块字符串
    pub code: String,
    /// `.rml` 字节区间 → 生成代码 (line, col) 的源映射
    pub source_map: crate::compiler::source_map::SourceMap,
}

