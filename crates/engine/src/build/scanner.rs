//! `.rml` 文件扫描 + `.rml.rs` code-behind syn 解析
//!
//! - `scan(dirs) -> Vec<PathBuf>`：递归扫描 `.rml` 文件
//! - `scan_struct_metadata(rml_rs_path) -> HashMap<String, StructMetadata>`：
//!   syn 解析 `.rml.rs`，提取每个 `#[window]`/`#[component]` 标注 struct 的
//!   pub 字段名 + 每个 `#[computed]` 方法的依赖字段列表
//!
//! Phase B-2：build.rs 调用 `scan_struct_metadata` 收集元信息，
//! 传入 `CodegenCtx` 供 codegen 生成版本管理方法。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use quote::quote;
use syn::visit::Visit;
use syn::{Expr, ExprField, File, Ident, ImplItem, Item, Lit, LitStr, ReturnType, Token, Type};
use walkdir::WalkDir;

use crate::compiler::{ValidationRule, ValidationRuleSet};

/// 递归扫描给定目录列表中的所有 `.rml` 文件，返回排序后的路径列表。
///
/// 不存在的目录会被静默跳过（避免可选模板目录报错）。
pub fn scan(dirs: &[PathBuf]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    for dir in dirs {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("rml") {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files
}

/// 单个 struct 的元信息（由 build.rs 扫描 `.rml.rs` 提取）
#[derive(Debug, Default, Clone)]
pub struct StructMetadata {
    /// 所有 pub 字段名（与 IModel::rml_fields 一致），供双向绑定与 computed 依赖扫描
    pub observable_fields: Vec<String>,
    /// 全部用户字段名（pub + private），与 #[component] 注入的版本计数器对齐
    pub version_fields: Vec<String>,
    /// 所有 `#[computed]` 方法名，供 codegen 生成 `__rml_computed_deps_version` match 臂
    pub computed_methods: Vec<String>,
    /// 每个 `#[computed]` 方法 → 依赖的 pub 字段列表（通过 `self.<field>` 访问检测）
    pub computed_deps: HashMap<String, Vec<String>>,
    /// 每个 `#[computed]` 方法 → 返回类型字符串（如 `"i32"`、`"Vec<TabItem>"`）
    ///
    /// codegen 生成的包装方法需要显式标注返回类型以调用
    /// `ComputedCache::get_or_compute::<T, _>(...)`。
    pub computed_returns: HashMap<String, String>,
    /// 每个 pub 字段 → 类型字符串（如 `"i32"`、`"String"`、`"SharedString"`）
    ///
    /// Phase B-3：codegen 的 `gen_model_input` 据此生成类型转换代码
    /// （`i32` → `parse::<i32>()`、`String` → `into()`）。
    pub field_types: HashMap<String, String>,
    /// 每个 pub 字段 → 校验规则集（Phase B-3.2：`#[validate]` 宏）
    ///
    /// scanner 从字段属性 `#[validate(...)]` 提取规则，codegen 据此在 parse 成功后、
    /// 赋值前生成规则校验链。
    pub field_validations: HashMap<String, ValidationRuleSet>,
    /// 是否为 `#[component]` 标注的 struct（用户自定义组件）
    ///
    /// build.rs 据此收集 `user_components` 注册表，供 codegen 在
    /// `component_lookup` 未命中时生成 `self.<field>.as_ref().expect(...).clone()`。
    pub is_component: bool,
}

/// 扫描 `.rml.rs` code-behind 文件，提取所有 `#[window]`/`#[component]` 标注 struct 的元信息。
///
/// 返回 `HashMap<struct_name, StructMetadata>`。如果文件不存在或解析失败，返回空 map。
///
/// # 流程
///
/// 1. 解析 `.rml.rs` 为 `syn::File`
/// 2. 第一遍：收集所有 `#[window]`/`#[component]` 标注的 struct 的 pub 字段名
/// 3. 第二遍：扫描 impl 块中的 `#[computed]` 方法，用 `syn::visit::Visit` 提取方法体内的
///    `self.<ident>` 访问作为依赖
pub fn scan_struct_metadata(rml_rs_path: &Path) -> HashMap<String, StructMetadata> {
    let mut result: HashMap<String, StructMetadata> = HashMap::new();

    let source = match std::fs::read_to_string(rml_rs_path) {
        Ok(s) => s,
        Err(_) => return result,
    };

    let file: File = match syn::parse_str(&source) {
        Ok(f) => f,
        Err(_) => return result,
    };

    // 第一遍：收集所有 #[window]/#[component] 标注 struct 的用户字段名
    for item in &file.items {
        if let Item::Struct(s) = item {
            let has_window = s.attrs.iter().any(|a| a.path().is_ident("window"));
            let has_component = s.attrs.iter().any(|a| a.path().is_ident("component"));
            if !has_window && !has_component {
                continue;
            }
            let struct_name = s.ident.to_string();
            let mut meta = StructMetadata::default();
            meta.is_component = has_component;
            for f in &s.fields {
                if let Some(name) = &f.ident {
                    let name_str = name.to_string();
                    if !name_str.starts_with("__rml_") {
                        meta.version_fields.push(name_str.clone());
                    }
                    let is_public = matches!(f.vis, syn::Visibility::Public(_));
                    if is_public {
                        meta.observable_fields.push(name_str.clone());
                    }
                    // 提取字段类型字符串（清理 token 间空格：`Vec < TabItem >` → `Vec<TabItem>`)
                    let ty = &f.ty;
                    let ty_str = quote!(#ty).to_string();
                    let cleaned = ty_str.split_whitespace().collect::<String>();
                    meta.field_types.insert(name_str.clone(), cleaned);

                    // Phase B-3.2：解析 #[validate(...)] 属性
                    for attr in &f.attrs {
                        if attr.path().is_ident("validate") {
                            match attr.parse_args::<ValidateArgs>() {
                                Ok(args) => {
                                    let rule_set: ValidationRuleSet = args.into();
                                    meta.field_validations.insert(name_str.clone(), rule_set);
                                }
                                Err(e) => {
                                    // 解析失败：警告但不阻塞编译
                                    println!(
                                        "cargo:warning=RML: failed to parse #[validate] on field {}: {}",
                                        name_str, e
                                    );
                                }
                            }
                        }
                    }
                }
            }
            result.insert(struct_name, meta);
        }
    }

    // 第二遍：扫描 impl 块中的 #[computed] 方法
    for item in &file.items {
        if let Item::Impl(impl_block) = item {
            // 获取 impl 的目标类型名（如 MainWindow）
            let ty_name = type_name(&impl_block.self_ty);
            let Some(meta) = result.get_mut(&ty_name) else {
                continue;
            };
            for impl_item in &impl_block.items {
                if let ImplItem::Fn(method) = impl_item {
                    let is_computed = method.attrs.iter().any(|a| a.path().is_ident("computed"));
                    if !is_computed {
                        continue;
                    }
                    let method_name = method.sig.ident.to_string();
                    // 提取返回类型字符串（codegen 包装方法需显式标注）
                    let return_type = return_type_str(&method.sig.output);
                    // 收集方法体的 self.<ident> 依赖
                    let mut visitor = ComputedDepVisitor::default();
                    visitor.visit_block(&method.block);
                    meta.computed_methods.push(method_name.clone());
                    meta.computed_deps.insert(method_name.clone(), visitor.deps);
                    meta.computed_returns.insert(method_name, return_type);
                }
            }
        }
    }

    result
}

/// 从 `ReturnType` 提取类型字符串（去除 `->` 与空格）
///
/// - `-> i32` → `"i32"`
/// - `-> Vec<TabItem>` → `"Vec<TabItem>"`
/// - 无返回类型（`-> ()` 隐式）→ `"()"`
fn return_type_str(output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => {
            // 用 quote!.to_string() 保留源码形式（含泛型参数）
            let s = quote!(#ty).to_string();
            // 清理 token 间空格：`Vec < TabItem >` → `Vec<TabItem>`
            s.split_whitespace().collect::<String>()
        }
    }
}

/// 从 `Type` 提取最内层类型名（如 `MainWindow`、`MyWidget`）
fn type_name(ty: &Type) -> String {
    if let Type::Path(p) = ty {
        if let Some(seg) = p.path.segments.last() {
            return seg.ident.to_string();
        }
    }
    String::new()
}

/// `#[computed]` 方法体依赖访问器
///
/// 检测 `self.<ident>` 字段读取模式（如 `self.count`、`self.user.name` 中的 `self.count`）。
/// 仅收集直接字段名，不递归进入嵌套表达式（如 `self.items[0].name` 只收集 `items`）。
///
/// ## 宏参数扫描
///
/// syn 不会解析宏参数（如 `format!("{}", self.count)`）中的表达式，
/// 因此 `visit_expr_macro` 单独扫描宏的 token 字符串，提取 `self.<ident>` 模式。
#[derive(Default)]
struct ComputedDepVisitor {
    deps: Vec<String>,
}

impl<'ast> Visit<'ast> for ComputedDepVisitor {
    fn visit_expr_field(&mut self, node: &'ast ExprField) {
        // 检测 self.<ident> 模式：base 是 `self` 标识符
        if let Expr::Path(syn::ExprPath { path, .. }) = &*node.base {
            if path.is_ident("self") {
                if let syn::Member::Named(ident) = &node.member {
                    let name = ident.to_string();
                    if !self.deps.contains(&name) {
                        self.deps.push(name);
                    }
                }
            }
        }
        // 递归遍历子表达式（捕获 `self.a.b`、`self.f()` 等中的其他 self 访问）
        syn::visit::visit_expr_field(self, node);
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        // 优先用 syn 解析宏参数为逗号分隔的表达式列表
        // （覆盖 format!("...", a, b)、println!(...)、vec![a, b] 等典型宏）
        if let Ok(parsed) = node
            .mac
            .parse_body_with(syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        {
            for expr in &parsed {
                self.visit_expr(expr);
            }
        } else {
            // 兜底：字符串扫描（容忍 `self . field` 形式的空格）
            let macro_str = node.mac.tokens.to_string();
            scan_self_field_accesses(&macro_str, &mut self.deps);
        }
        // 继续递归（嵌套宏）
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        // 兜底：直接遇到的 Macro 节点（如 stmt 位置的宏调用）
        if let Ok(parsed) = node
            .parse_body_with(syn::punctuated::Punctuated::<syn::Expr, syn::Token![,]>::parse_terminated)
        {
            for expr in &parsed {
                self.visit_expr(expr);
            }
        } else {
            let macro_str = node.tokens.to_string();
            scan_self_field_accesses(&macro_str, &mut self.deps);
        }
        syn::visit::visit_macro(self, node);
    }
}

/// 在字符串中扫描 `self.<ident>` 模式，将识别到的字段名加入 `deps`
///
/// 兜底实现：当 `parse_body_with` 解析失败时使用。容忍 `self . field` 形式的空格
/// （syn token stream 字符串化时会在 punct 周围插入空格）。
///
/// 边界检查：`self` 前必须是非标识符字符（避免匹配 `myself.foo`）。
fn scan_self_field_accesses(s: &str, deps: &mut Vec<String>) {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i + 4 <= chars.len() {
        // 匹配 "self" 标识符
        if chars[i] == 's'
            && chars[i + 1] == 'e'
            && chars[i + 2] == 'l'
            && chars[i + 3] == 'f'
            // 边界检查：前一个字符不能是标识符字符
            && (i == 0 || !is_ident_char(chars[i - 1]))
        {
            let mut j = i + 4;
            // 跳过空白
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            // 期望 '.'
            if j < chars.len() && chars[j] == '.' {
                j += 1;
                // 跳过空白
                while j < chars.len() && chars[j].is_whitespace() {
                    j += 1;
                }
                // 提取标识符
                let start = j;
                while j < chars.len() && is_ident_char(chars[j]) {
                    j += 1;
                }
                if j > start {
                    let name: String = chars[start..j].iter().collect();
                    if !deps.contains(&name) {
                        deps.push(name);
                    }
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

// ──────────────────────────────────────────────────────────────────────────
//  Phase B-3.2：#[validate(...)] 属性解析器
// ──────────────────────────────────────────────────────────────────────────

/// `#[validate(...)]` 属性参数解析器
///
/// 解析逗号分隔的规则列表，如 `required, length(min = 3, max = 20), message = "..."`。
/// 由 scanner 调用 `attr.parse_args::<ValidateArgs>()` 解析属性参数。
///
/// Phase B-3.3：支持 `#[validate(MyValidator)]` 接口式校验。
/// `MyValidator` 为实现 `rml_core::validate::IValidate` 的类型名（单标识符）。
/// 与规则式（required/length/range/regex/custom）+ message 互斥。
struct ValidateArgs {
    rules: Vec<ValidationRule>,
    custom_message: Option<String>,
    validator_type: Option<String>,
}

impl syn::parse::Parse for ValidateArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut rules = Vec::new();
        let mut custom_message = None;
        let mut validator_type: Option<String> = None;
        let mut last_ident_span = None; // 推断为 Option<proc_macro2::Span>

        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            last_ident_span = Some(ident.span());
            match ident.to_string().as_str() {
                "required" => {
                    rules.push(ValidationRule::Required);
                }
                "length" | "range" => {
                    // 解析 (min = N, max = M)，min/max 任一可省略
                    let content;
                    syn::parenthesized!(content in input);
                    let mut min: Option<f64> = None;
                    let mut max: Option<f64> = None;
                    while !content.is_empty() {
                        let key: Ident = content.parse()?;
                        let _: Token![=] = content.parse()?;
                        let val: Lit = content.parse()?;
                        let num = match &val {
                            Lit::Int(i) => i.base10_parse::<f64>().ok(),
                            Lit::Float(f) => f.base10_parse::<f64>().ok(),
                            _ => None,
                        };
                        match key.to_string().as_str() {
                            "min" => min = num,
                            "max" => max = num,
                            _ => {}
                        }
                        if content.peek(Token![,]) {
                            let _: Token![,] = content.parse()?;
                        }
                    }
                    if ident.to_string() == "length" {
                        // length 的 min/max 转为 i64（字符串长度）
                        rules.push(ValidationRule::Length {
                            min: min.map(|v| v as i64),
                            max: max.map(|v| v as i64),
                        });
                    } else {
                        // range 的 min/max 保持 f64
                        rules.push(ValidationRule::Range { min, max });
                    }
                }
                "regex" | "custom" | "message" => {
                    let _: Token![=] = input.parse()?;
                    let val: LitStr = input.parse()?;
                    let s = val.value();
                    match ident.to_string().as_str() {
                        "regex" => rules.push(ValidationRule::Regex(s)),
                        "custom" => rules.push(ValidationRule::Custom(s)),
                        "message" => custom_message = Some(s),
                        _ => {}
                    }
                }
                other => {
                    // Phase B-3.3：未知标识符识别为 IValidate 类型名
                    if validator_type.is_some() {
                        return Err(syn::Error::new(
                            ident.span(),
                            format!("duplicate validator type: only one IValidate type allowed, got: {}", other),
                        ));
                    }
                    validator_type = Some(other.to_string());
                }
            }
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }

        // Phase B-3.3：互斥校验——IValidate 类型与规则式 + message 不可混用
        if validator_type.is_some() {
            if !rules.is_empty() {
                return Err(syn::Error::new(
                    last_ident_span.unwrap(),
                    "cannot mix IValidate type with rule-based validators (required/length/range/regex/custom)",
                ));
            }
            if custom_message.is_some() {
                return Err(syn::Error::new(
                    last_ident_span.unwrap(),
                    "cannot mix IValidate type with message override (use IValidate::message() instead)",
                ));
            }
        }

        Ok(ValidateArgs { rules, custom_message, validator_type })
    }
}

impl From<ValidateArgs> for ValidationRuleSet {
    fn from(args: ValidateArgs) -> Self {
        ValidationRuleSet {
            rules: args.rules,
            custom_message: args.custom_message,
            validator_type: args.validator_type,
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_temp_rml_rs(content: &str) -> std::path::PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "rml_test_{}.rml.rs",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    #[test]
    fn scans_all_named_fields() {
        let path = write_temp_rml_rs(
            r#"
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
    pub name: String,
    _private: bool,
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let main_window = meta.get("MainWindow").unwrap();
        // 仅 pub 字段参与 observable 版本追踪；私有字段仍可被 #[computed] 读取
        assert_eq!(main_window.observable_fields, vec!["count", "name"]);
    }

    #[test]
    fn scans_computed_method_deps() {
        let path = write_temp_rml_rs(
            r#"
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
    pub name: String,
}

impl MainWindow {
    #[computed]
    pub fn doubled(&self) -> i32 {
        self.count * 2
    }

    #[computed]
    pub fn display(&self) -> String {
        format!("{} {}", self.count, self.name)
    }
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let main_window = meta.get("MainWindow").unwrap();
        assert_eq!(main_window.computed_methods, vec!["doubled", "display"]);
        assert_eq!(
            main_window.computed_deps.get("doubled"),
            Some(&vec!["count".to_string()])
        );
        assert_eq!(
            main_window.computed_deps.get("display"),
            Some(&vec!["count".to_string(), "name".to_string()])
        );
    }

    #[test]
    fn ignores_non_component_structs() {
        let path = write_temp_rml_rs(
            r#"
pub struct NotAComponent {
    pub x: i32,
}

#[component]
pub struct MyWidget {
    pub label: String,
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        assert!(meta.get("NotAComponent").is_none());
        let widget = meta.get("MyWidget").unwrap();
        assert_eq!(widget.observable_fields, vec!["label"]);
    }

    #[test]
    fn missing_file_returns_empty() {
        let path = std::path::PathBuf::from("/nonexistent/file.rml.rs");
        let meta = scan_struct_metadata(&path);
        assert!(meta.is_empty());
    }

    #[test]
    fn no_computed_returns_empty_deps() {
        let path = write_temp_rml_rs(
            r#"
#[window]
pub struct Empty {
    pub x: i32,
}

impl Empty {
    #[computed]
    pub fn constant(&self) -> i32 {
        42
    }
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let empty = meta.get("Empty").unwrap();
        assert_eq!(empty.computed_methods, vec!["constant"]);
        assert_eq!(empty.computed_deps.get("constant"), Some(&vec![]));
    }

    #[test]
    fn captures_chained_self_access() {
        // self.a.b 应同时识别 self.a 和后续访问
        let path = write_temp_rml_rs(
            r#"
#[window]
pub struct MainWindow {
    pub user: String,
    pub count: i32,
}

impl MainWindow {
    #[computed]
    pub fn summary(&self) -> String {
        let _ = self.user.len();
        let _ = self.count;
        String::new()
    }
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let m = meta.get("MainWindow").unwrap();
        let deps = m.computed_deps.get("summary").unwrap();
        assert!(deps.contains(&"user".to_string()));
        assert!(deps.contains(&"count".to_string()));
    }

    #[test]
    fn extracts_computed_return_types() {
        let path = write_temp_rml_rs(
            r#"
#[window]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[computed]
    pub fn doubled(&self) -> i32 {
        self.count * 2
    }

    #[computed]
    pub fn items(&self) -> Vec<String> {
        vec![self.count.to_string()]
    }

    #[computed]
    pub fn no_return(&self) {
        let _ = self.count;
    }
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let m = meta.get("MainWindow").unwrap();
        assert_eq!(m.computed_returns.get("doubled"), Some(&"i32".to_string()));
        assert_eq!(
            m.computed_returns.get("items"),
            Some(&"Vec<String>".to_string())
        );
        assert_eq!(
            m.computed_returns.get("no_return"),
            Some(&"()".to_string())
        );
    }

    #[test]
    fn scans_field_types() {
        let path = write_temp_rml_rs(
            r#"
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
    pub name: String,
    pub age: u32,
    pub score: f64,
    _private: bool,
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let m = meta.get("MainWindow").unwrap();
        assert_eq!(m.field_types.get("count"), Some(&"i32".to_string()));
        assert_eq!(m.field_types.get("name"), Some(&"String".to_string()));
        assert_eq!(m.field_types.get("age"), Some(&"u32".to_string()));
        assert_eq!(m.field_types.get("score"), Some(&"f64".to_string()));
        // 私有字段仍记录类型与版本追踪，但不参与 pub observable 绑定
        assert_eq!(m.field_types.get("_private"), Some(&"bool".to_string()));
        assert!(!m.observable_fields.contains(&"_private".to_string()));
        assert!(m.version_fields.contains(&"_private".to_string()));
    }

    #[test]
    fn scans_generic_field_types() {
        let path = write_temp_rml_rs(
            r#"
#[component]
pub struct MyWidget {
    pub items: Vec<String>,
    pub optional: Option<i32>,
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let m = meta.get("MyWidget").unwrap();
        assert_eq!(m.field_types.get("items"), Some(&"Vec<String>".to_string()));
        assert_eq!(
            m.field_types.get("optional"),
            Some(&"Option<i32>".to_string())
        );
    }
}
