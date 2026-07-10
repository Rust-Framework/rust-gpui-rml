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
    /// 是否标注 `#[contributehost]`（注册 host slot）
    pub is_contributehost: bool,
    /// `#[component(slots = ["header", "footer", ...])]` 声明的具名插槽列表
    ///
    /// build.rs 据此填充 `UserComponentInfo.slots`，供 codegen 在父视图中
    /// 分离 `<template slot="x">` 子节点并校验 slot 名合法性。
    pub slots: Vec<String>,
    /// 事件回调字段名 → handler 类型名（P0-1：用户组件事件绑定）
    ///
    /// 由 scanner 扫描 `pub on_click: Option<rml_core::event::ClickHandler>` 等字段提取，
    /// key 为字段名（如 "on_click"），value 为 handler 类型名（如 "ClickHandler"）。
    /// build.rs 据此填充 `UserComponentInfo.event_fields`，供 `gen_prop_assign`
    /// 在父视图 `<MyComp on-click={handler} />` 时生成闭包并注入到子组件字段。
    pub event_fields: HashMap<String, String>,
    /// 所有 `#[command]` 标注方法名（供 LSP 命令补全/诊断）
    pub commands: Vec<String>,
    /// 生命周期钩子（Phase B-3：`#[on_loaded]`/`#[on_unloaded]` 自动联动）
    ///
    /// scanner 扫描 impl 块中的 `#[on_loaded]`/`#[on_unloaded]` 标注方法，
    /// codegen 据此生成 `impl ILifecycle` 自动联动，无需用户手动 impl。
    pub lifecycle_hooks: LifecycleHooks,
    /// 是否已存在手动 `impl ILifecycle for <Type>` 块
    ///
    /// 若为 `true` 且 `lifecycle_hooks` 非空：codegen 跳过自动生成并发出 warning
    /// （避免重复 impl 导致编译错误）。若为 `true` 且 `lifecycle_hooks` 为空：无操作。
    pub has_manual_lifecycle_impl: bool,
}

/// 生命周期钩子元信息（Phase B-3：`#[on_loaded]`/`#[on_unloaded]` 自动联动）
///
/// scanner 扫描 `.rml.rs` impl 块中标注 `#[on_loaded]`/`#[on_unloaded]` 的方法名，
/// codegen 据此生成 `impl ILifecycle for <View>`，在 trait 方法中调用用户方法。
#[derive(Debug, Default, Clone)]
pub struct LifecycleHooks {
    /// `#[on_loaded]` 标注的方法名（至多一个；多次标注以最后一个为准）
    pub on_loaded: Option<String>,
    /// `#[on_unloaded]` 标注的方法名（至多一个；多次标注以最后一个为准）
    pub on_unloaded: Option<String>,
}

impl LifecycleHooks {
    /// 是否存在任何钩子（决定 codegen 是否生成 `impl ILifecycle`）
    pub fn has_any(&self) -> bool {
        self.on_loaded.is_some() || self.on_unloaded.is_some()
    }
}

/// 扫描 `.rml.rs` code-behind 文件，提取所有 `#[window]`/`#[component]` 标注 struct 的元信息。
///
/// 返回 `HashMap<struct_name, StructMetadata>`。如果文件不存在或解析失败，返回空 map。
///
/// 文件读取后委托给 [`parse_struct_metadata`]，后者为纯函数，可处理内存中的源码字符串
/// （供 LSP 处理未保存缓冲区）。
pub fn scan_struct_metadata(rml_rs_path: &Path) -> HashMap<String, StructMetadata> {
    let source = match std::fs::read_to_string(rml_rs_path) {
        Ok(s) => s,
        Err(_) => return HashMap::new(),
    };
    parse_struct_metadata(&source)
}

/// 从 `.rml.rs` 源码字符串解析 `StructMetadata`（不读磁盘）。
///
/// 返回 `HashMap<struct_name, StructMetadata>`。解析失败返回空 map。
///
/// # 流程
///
/// 1. 解析 `.rml.rs` 为 `syn::File`
/// 2. 第一遍：收集所有 `#[window]`/`#[component]` 标注的 struct 的 pub 字段名
/// 3. 第二遍：扫描 impl 块中的 `#[computed]` / `#[command]` 方法，用 `syn::visit::Visit`
///    提取 `#[computed]` 方法体内的 `self.<ident>` 访问作为依赖
pub fn parse_struct_metadata(source: &str) -> HashMap<String, StructMetadata> {
    let mut result: HashMap<String, StructMetadata> = HashMap::new();

    let file: File = match syn::parse_str(source) {
        Ok(f) => f,
        Err(_) => return result,
    };

    // 第一遍：收集所有 #[window]/#[component] 标注 struct 的用户字段名
    for item in &file.items {
        if let Item::Struct(s) = item {
            let has_window = s.attrs.iter().any(|a| a.path().is_ident("window"));
            let has_component = s.attrs.iter().any(|a| a.path().is_ident("component"));
            let is_contributehost = s.attrs.iter().any(|a| a.path().is_ident("contributehost"));
            if !has_window && !has_component {
                continue;
            }
            let struct_name = s.ident.to_string();
            let mut meta = StructMetadata {
                is_component: has_component,
                is_contributehost,
                ..Default::default()
            };

            // 解析 #[component(slots = ["header", "footer", ...])] 参数
            if has_component {
                meta.slots = parse_component_slots(&s.attrs);
            }
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
                    meta.field_types.insert(name_str.clone(), cleaned.clone());

                    // P0-1：检测事件回调字段（Option<rml_core::event::XxxHandler>）
                    // 字段名需以 on_ 开头，类型为 Option<...XxxHandler>。
                    // 提取 handler 类型名（如 ClickHandler），供 gen_prop_assign 生成闭包注入。
                    if name_str.starts_with("on_") {
                        if let Some(handler_type) = parse_event_handler_field_type(&cleaned) {
                            meta.event_fields.insert(name_str.clone(), handler_type.to_string());
                        }
                    }

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

    // 第二遍：扫描 impl 块中的 #[computed] / #[command] / #[on_loaded] / #[on_unloaded]
    //         + 检测手动 `impl ILifecycle for <Type>` 块
    for item in &file.items {
        if let Item::Impl(impl_block) = item {
            // 获取 impl 的目标类型名（如 MainWindow）
            let ty_name = type_name(&impl_block.self_ty);
            let Some(meta) = result.get_mut(&ty_name) else {
                continue;
            };

            // 检测手动 `impl ILifecycle for <Type>` 块
            // impl_block.trait_ 是 Option<(Option<Not>, Path, For)>，元组 .1 为 Path
            if let Some((_, trait_path, _)) = &impl_block.trait_ {
                let trait_name = trait_path
                    .segments
                    .last()
                    .map(|seg| seg.ident.to_string())
                    .unwrap_or_default();
                if trait_name == "ILifecycle" {
                    meta.has_manual_lifecycle_impl = true;
                }
            }

            for impl_item in &impl_block.items {
                if let ImplItem::Fn(method) = impl_item {
                    let is_computed = method.attrs.iter().any(|a| a.path().is_ident("computed"));
                    let is_command = method.attrs.iter().any(|a| a.path().is_ident("command"));
                    let is_on_loaded = method.attrs.iter().any(|a| a.path().is_ident("on_loaded"));
                    let is_on_unloaded =
                        method.attrs.iter().any(|a| a.path().is_ident("on_unloaded"));
                    // 无任何相关属性：跳过
                    if !is_computed && !is_command && !is_on_loaded && !is_on_unloaded {
                        continue;
                    }
                    let method_name = method.sig.ident.to_string();
                    // #[command]：仅收集方法名（供 LSP 命令补全/诊断），无需依赖分析
                    if is_command {
                        meta.commands.push(method_name.clone());
                    }
                    // #[computed]：提取返回类型 + 收集方法体依赖
                    if is_computed {
                        let return_type = return_type_str(&method.sig.output);
                        let mut visitor = ComputedDepVisitor::default();
                        visitor.visit_block(&method.block);
                        let mut deps = visitor.deps;
                        if visitor.uses_i18n
                            && (meta.observable_fields.contains(&"i18n_version".to_string())
                                || meta.is_contributehost)
                            && !deps.contains(&"i18n_version".to_string()) {
                                deps.push("i18n_version".to_string());
                            }
                        meta.computed_methods.push(method_name.clone());
                        meta.computed_deps.insert(method_name.clone(), deps);
                        meta.computed_returns.insert(method_name.clone(), return_type);
                    }
                    // #[on_loaded] / #[on_unloaded]：记录方法名供 codegen 生成 impl ILifecycle
                    if is_on_loaded {
                        meta.lifecycle_hooks.on_loaded = Some(method_name.clone());
                    }
                    if is_on_unloaded {
                        meta.lifecycle_hooks.on_unloaded = Some(method_name.clone());
                    }
                }
            }
        }
    }

    result
}

/// 从 `ReturnType` 提取类型字符串（去除 `->` 与符号周围空格）
///
/// - `-> i32` → `"i32"`
/// - `-> Vec<TabItem>` → `"Vec<TabItem>"`
/// - `-> Vec<Arc<dyn IContribution>>` → `"Vec<Arc<dyn IContribution>>"`
///   （保留 `dyn` 与 trait 名之间的空格，仅清理符号周围空格）
/// - 无返回类型（`-> ()` 隐式）→ `"()"`
fn return_type_str(output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "()".to_string(),
        ReturnType::Type(_, ty) => {
            // 用 quote!.to_string() 保留源码形式（含泛型参数）
            let s = quote!(#ty).to_string();
            normalize_type_whitespace(&s)
        }
    }
}

/// 规范化类型字符串：移除符号周围的空格，但保留标识符之间的空格
///
/// `quote!(#ty).to_string()` 会在 token 间插入空格（如 `Vec < Arc < dyn IContribution > >`）。
/// 简单的 `split_whitespace().collect()` 会错误合并 `dyn IContribution` → `dynIContribution`。
/// 本函数仅清理符号（`<` `>` `&` `,` `;` `(` `)` `[` `]` `+` `::`）周围的空格，
/// 当空格两侧均为标识符字符时保留空格（如 `dyn Trait`、`impl Trait`）。
///
/// 此外，`proc_macro2::TokenStream::to_string()` 在某些版本下不会在 `dyn`/`impl` 关键字
/// 与紧跟的 trait 名之间插入空格（如 `dynIContribution`），本函数会补上缺失的空格。
fn normalize_type_whitespace(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_whitespace() {
            // 跳过连续空格，找到下一个非空字符
            let prev = result.chars().last();
            let mut j = i;
            while j < chars.len() && chars[j].is_whitespace() {
                j += 1;
            }
            let next = chars.get(j).copied();
            // 仅当两侧均为标识符字符时保留空格（如 `dyn IContribution`）
            let keep = matches!((prev, next),
                (Some(p), Some(n)) if is_ident_char(p) && is_ident_char(n));
            if keep {
                result.push(' ');
            }
            i = j;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    insert_keyword_spaces(&result)
}

/// 在 `dyn`/`impl` 等 type-position 关键字与紧跟的标识符之间补上缺失的空格。
///
/// `proc_macro2::TokenStream::to_string()` 可能在 `dyn` 关键字与 trait 名之间
/// 省略空格（如 `dynIContribution`），导致生成的代码无效。本函数扫描已规范化的
/// 字符串，在关键字后紧跟标识符字符处插入空格。
fn insert_keyword_spaces(s: &str) -> String {
    const KEYWORDS: &[&str] = &["dyn", "impl"];
    let chars: Vec<char> = s.chars().collect();
    let mut result = String::with_capacity(s.len());
    let mut i = 0;
    let mut prev_char: Option<char> = None;
    while i < chars.len() {
        let matched = KEYWORDS.iter().find_map(|kw| {
            let kw_chars: Vec<char> = kw.chars().collect();
            if i + kw_chars.len() > chars.len() {
                return None;
            }
            if chars[i..i + kw_chars.len()] != kw_chars[..] {
                return None;
            }
            // 关键字前必须是词边界（非标识符字符）
            if prev_char.is_some_and(is_ident_char) {
                return None;
            }
            // 关键字后必须紧跟标识符字符（说明缺少空格）
            let next = chars.get(i + kw_chars.len()).copied();
            if !next.is_some_and(is_ident_char) {
                return None;
            }
            Some(*kw)
        });
        if let Some(kw) = matched {
            result.push_str(kw);
            result.push(' ');
            i += kw.chars().count();
            prev_char = Some(' ');
        } else {
            result.push(chars[i]);
            prev_char = Some(chars[i]);
            i += 1;
        }
    }
    result
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

/// 从 struct 属性列表中解析 `#[component(slots = ["header", "footer", ...])]` 参数
///
/// 返回插槽名列表；无 `#[component]` 属性或无 `slots` 参数时返回空 Vec。
///
/// 支持形式：
/// - `#[component]` → 空
/// - `#[component(slots = ["header", "footer"])]` → ["header", "footer"]
fn parse_component_slots(attrs: &[syn::Attribute]) -> Vec<String> {
    for attr in attrs {
        if !attr.path().is_ident("component") {
            continue;
        }
        let syn::Meta::List(list) = &attr.meta else {
            continue;
        };
        // 尝试解析 tokens 为 `ident = expr_array` 形式
        let tokens = &list.tokens;
        // 用 syn 解析为 ComponentSlotsArgs
        if let Ok(args) = syn::parse2::<ComponentSlotsArgs>(tokens.clone()) {
            return args.slots;
        }
    }
    Vec::new()
}

/// 从字段类型字符串提取事件 handler 类型名（P0-1：用户组件事件绑定）
///
/// 识别 `Option<rml_core::event::ClickHandler>` / `Option<rml::event::ClickHandler>` 等
/// 类型别名形式，返回 handler 类型名（如 "ClickHandler"）。
///
/// 不识别 `Option<Box<dyn Fn(...) + Send + Sync + 'static>>` 完整 trait object 形式，
/// 用户应使用 `rml_core::event::ClickHandler` 等类型别名声明事件回调字段。
fn parse_event_handler_field_type(cleaned: &str) -> Option<&str> {
    let inner = cleaned
        .strip_prefix("Option<")
        .and_then(|s| s.strip_suffix('>'))?;
    // 取最后一段路径（如 rml_core::event::ClickHandler → ClickHandler）
    let last_segment = inner.rsplit("::").next()?;
    if last_segment.ends_with("Handler") {
        Some(last_segment)
    } else {
        None
    }
}

/// `#[component(slots = [...])]` 参数解析结构
struct ComponentSlotsArgs {
    slots: Vec<String>,
}

impl syn::parse::Parse for ComponentSlotsArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut slots = Vec::new();
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            if ident == "slots" {
                let _eq: syn::Token![=] = input.parse()?;
                let arr: syn::ExprArray = input.parse()?;
                for expr in arr.elems {
                    let lit: syn::LitStr = syn::parse2(quote! { #expr })?;
                    slots.push(lit.value());
                }
            } else {
                return Err(syn::Error::new(ident.span(), "unknown argument, expected `slots`"));
            }
            if !input.is_empty() {
                let _comma: syn::Token![,] = input.parse()?;
            }
        }
        Ok(ComponentSlotsArgs { slots })
    }
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
    uses_i18n: bool,
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

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        if node.method == "t" {
            self.uses_i18n = true;
        }
        syn::visit::visit_expr_method_call(self, node);
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
                    if ident == "length" {
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
    fn extracts_computed_return_type_with_dyn_trait() {
        // `Vec<Arc<dyn IContribution>>` 必须保留 `dyn` 与 `IContribution` 之间的空格
        // 旧实现 split_whitespace().collect() 会错误合并为 `dynIContribution`
        let path = write_temp_rml_rs(
            r#"
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[computed]
    pub fn items(&self) -> Vec<Arc<dyn IContribution>> {
        Vec::new()
    }
}
        "#,
        );
        let meta = scan_struct_metadata(&path);
        let m = meta.get("MainWindow").unwrap();
        assert_eq!(
            m.computed_returns.get("items"),
            Some(&"Vec<Arc<dyn IContribution>>".to_string())
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

    #[test]
    fn extracts_command_methods() {
        // 验证 #[command] 方法名收集 + 与 #[computed] 独立共存
        // 使用 parse_struct_metadata 纯函数入口（LSP 未保存缓冲区场景）
        let source = r#"
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[command]
    pub fn on_click(&mut self, cx: &mut Context<Self>) {
        self.count += 1;
    }

    #[command]
    pub fn on_save(&mut self, cx: &mut Context<Self>) {
    }

    #[computed]
    pub fn counter_text(&self) -> String {
        format!("{}", self.count)
    }

    pub fn helper(&self) -> i32 {
        0
    }
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("MainWindow").unwrap();
        // 两个 #[command] 方法均被收集
        assert_eq!(m.commands, vec!["on_click", "on_save"]);
        // #[computed] 仍正常工作，未受影响
        assert_eq!(m.computed_methods, vec!["counter_text"]);
        // 无标注的方法不被收集到 commands
        assert!(!m.commands.contains(&"helper".to_string()));
    }

    #[test]
    fn parse_struct_metadata_handles_invalid_source() {
        // 语法错误的源码：返回空 map，不 panic
        let meta = parse_struct_metadata("this is not rust code {{{");
        assert!(meta.is_empty());
    }

    #[test]
    fn scans_on_loaded_on_unloaded_hooks() {
        let source = r#"
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[on_loaded]
    pub fn init(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.count = 0;
    }

    #[on_unloaded]
    pub fn cleanup(&mut self, cx: &mut gpui::Context<Self>) {
        // 释放资源
    }
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("MainWindow").unwrap();
        assert_eq!(m.lifecycle_hooks.on_loaded, Some("init".to_string()));
        assert_eq!(m.lifecycle_hooks.on_unloaded, Some("cleanup".to_string()));
        assert!(!m.has_manual_lifecycle_impl);
    }

    #[test]
    fn scans_partial_hooks_only_on_loaded() {
        let source = r#"
#[component]
#[derive(Default)]
pub struct MyWidget {
    pub label: String,
}

impl MyWidget {
    #[on_loaded]
    pub fn setup(&mut self, cx: &mut gpui::Context<Self>) {}
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("MyWidget").unwrap();
        assert_eq!(m.lifecycle_hooks.on_loaded, Some("setup".to_string()));
        assert_eq!(m.lifecycle_hooks.on_unloaded, None);
        assert!(!m.has_manual_lifecycle_impl);
    }

    #[test]
    fn detects_manual_impl_lifecycle() {
        let source = r#"
#[component]
#[derive(Default)]
pub struct ManualCase {
    pub x: i32,
}

impl ManualCase {
    #[on_loaded]
    pub fn my_load(&mut self, cx: &mut gpui::Context<Self>) {}
}

impl rml_core::lifecycle::ILifecycle for ManualCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) where Self: Sized {
        self.my_load(_cx);
    }
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("ManualCase").unwrap();
        // 手动 impl + 标注同时存在：scanner 同时记录两者，codegen 负责冲突处理
        assert!(m.has_manual_lifecycle_impl);
        assert_eq!(m.lifecycle_hooks.on_loaded, Some("my_load".to_string()));
    }

    #[test]
    fn detects_manual_impl_lifecycle_without_hooks() {
        let source = r#"
#[component]
#[derive(Default)]
pub struct EmptyManualCase {
    pub x: i32,
}

impl rml_core::lifecycle::ILifecycle for EmptyManualCase {}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("EmptyManualCase").unwrap();
        assert!(m.has_manual_lifecycle_impl);
        assert!(!m.lifecycle_hooks.has_any());
    }

    #[test]
    fn no_lifecycle_hooks_when_unmarked() {
        let source = r#"
#[component]
#[derive(Default)]
pub struct PlainCase {
    pub x: i32,
}

impl PlainCase {
    pub fn helper(&mut self) {}
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("PlainCase").unwrap();
        assert!(!m.has_manual_lifecycle_impl);
        assert!(!m.lifecycle_hooks.has_any());
    }

    // ─── P0-1：事件回调字段扫描 ───

    #[test]
    fn parse_event_handler_field_type_click() {
        assert_eq!(
            parse_event_handler_field_type("Option<rml_core::event::ClickHandler>"),
            Some("ClickHandler")
        );
    }

    #[test]
    fn parse_event_handler_field_type_short_path() {
        assert_eq!(
            parse_event_handler_field_type("Option<rml::event::KeyDownHandler>"),
            Some("KeyDownHandler")
        );
    }

    #[test]
    fn parse_event_handler_field_type_bare_alias() {
        assert_eq!(
            parse_event_handler_field_type("Option<ClickHandler>"),
            Some("ClickHandler")
        );
    }

    #[test]
    fn parse_event_handler_field_type_rejects_non_handler() {
        assert_eq!(parse_event_handler_field_type("Option<i32>"), None);
        assert_eq!(parse_event_handler_field_type("Option<String>"), None);
        assert_eq!(
            parse_event_handler_field_type("Option<Box<dynFn(&ClickEvent)>>"),
            None
        );
    }

    #[test]
    fn parse_event_handler_field_type_rejects_non_option() {
        assert_eq!(parse_event_handler_field_type("ClickHandler"), None);
        assert_eq!(parse_event_handler_field_type("rml_core::event::ClickHandler"), None);
    }

    #[test]
    fn scans_event_fields_from_component_struct() {
        let source = r#"
#[component]
#[derive(Default)]
pub struct MyButton {
    pub title: SharedString,
    pub on_click: Option<rml_core::event::ClickHandler>,
    pub on_key_down: Option<rml::event::KeyDownHandler>,
    pub not_event: Option<i32>,
    pub on_hover: Option<rml_core::event::HoverHandler>,
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("MyButton").unwrap();
        assert_eq!(
            m.event_fields.get("on_click"),
            Some(&"ClickHandler".to_string())
        );
        assert_eq!(
            m.event_fields.get("on_key_down"),
            Some(&"KeyDownHandler".to_string())
        );
        assert_eq!(
            m.event_fields.get("on_hover"),
            Some(&"HoverHandler".to_string())
        );
        // not_event 不是 Handler 类型，不应被收集
        assert!(!m.event_fields.contains_key("not_event"));
    }

    #[test]
    fn scans_event_fields_ignores_non_on_prefix() {
        let source = r#"
#[component]
#[derive(Default)]
pub struct MyComp {
    pub callback: Option<rml_core::event::ClickHandler>,
}
        "#;
        let meta = parse_struct_metadata(source);
        let m = meta.get("MyComp").unwrap();
        // 字段名不以 on_ 开头，即使类型是 Handler 也不收集
        assert!(m.event_fields.is_empty());
    }
}
