//! RML 编译器入口
//!
//! 串起 parse → validate → codegen，输出 Rust 源码字符串。

pub mod codegen;
pub mod component;
pub mod event;
pub mod expr;
pub mod menu;
pub mod validator;

use crate::css::StyleSheet;
use crate::parser;
use std::collections::HashMap;
use std::fmt;

/// 字段校验规则（Phase B-3.2：`#[validate]` 宏）
///
/// 由 `crates/engine/src/build/scanner.rs` 从 `.rml.rs` 的 `#[validate(...)]` 属性提取，
/// 经 `CodegenCtx.field_validations` 传递给 codegen，生成校验代码写入 `__rml_field_errors`。
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
    /// 用户自定义组件注册表（`#[component]` 标注的 struct）
    ///
    /// 由 build.rs 从所有 `.rml.rs` 文件扫描收集，key 为 struct 名（如 "CounterCase"）。
    /// codegen 在 `gen_component` 中 `component_lookup` 未命中时查此表，
    /// 生成 `self.<entity_field>.as_ref().expect(...).clone()` 嵌入用户组件。
    pub user_components: HashMap<String, UserComponentInfo>,
    /// 是否标注 `#[contributehost]`（首次 render 自动 attach 贡献订阅）
    pub is_contributehost: bool,
}

/// 代码生成错误
///
/// 由 codegen / event / component 模块共用，定义在此避免循环依赖。
#[derive(Debug, Clone)]
pub struct CodegenError {
    pub message: String,
}

impl fmt::Display for CodegenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Codegen error: {}", self.message)
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

/// 编译 `.rml` 源码为 Rust 源码字符串
///
/// # 参数
/// - `source`: `.rml` 文件内容
/// - `ctx`: 代码生成上下文（含视图结构名）
///
/// # 返回
/// 生成的 `impl Render for <View>` 代码块字符串
pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<String, CompileError> {
    let root = parser::parse(source)?;
    validator::validate(&root)?;
    let mut ctx = ctx.clone();
    ctx.model_fields = codegen::collect_model_fields(&root);
    let code = codegen::codegen(&root, &ctx)?;
    Ok(code)
}

