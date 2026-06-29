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
use syn::visit::Visit;
use syn::{Expr, ExprField, File, ImplItem, Item, Type, Visibility};
use walkdir::WalkDir;

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
    /// 所有 pub 字段名（与 IModel::rml_fields 一致），供 codegen 生成 `__rml_bump_version` match 臂
    pub observable_fields: Vec<String>,
    /// 所有 `#[computed]` 方法名，供 codegen 生成 `__rml_computed_deps_version` match 臂
    pub computed_methods: Vec<String>,
    /// 每个 `#[computed]` 方法 → 依赖的 pub 字段列表（通过 `self.<field>` 访问检测）
    pub computed_deps: HashMap<String, Vec<String>>,
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

    // 第一遍：收集所有 #[window]/#[component] 标注的 struct 的 pub 字段名
    for item in &file.items {
        if let Item::Struct(s) = item {
            let is_component_struct = s
                .attrs
                .iter()
                .any(|a| a.path().is_ident("window") || a.path().is_ident("component"));
            if !is_component_struct {
                continue;
            }
            let struct_name = s.ident.to_string();
            let mut meta = StructMetadata::default();
            for f in &s.fields {
                if matches!(f.vis, Visibility::Public(_)) {
                    if let Some(name) = &f.ident {
                        meta.observable_fields.push(name.to_string());
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
                    // 收集方法体的 self.<ident> 依赖
                    let mut visitor = ComputedDepVisitor::default();
                    visitor.visit_block(&method.block);
                    meta.computed_methods.push(method_name.clone());
                    meta.computed_deps.insert(method_name, visitor.deps);
                }
            }
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
        // 扫描宏 token 字符串中的 self.<ident> 模式
        // 覆盖 format!("{}", self.count)、vec![self.x] 等场景
        let macro_str = node.mac.tokens.to_string();
        scan_self_field_accesses(&macro_str, &mut self.deps);
        // 继续递归（嵌套宏）
        syn::visit::visit_expr_macro(self, node);
    }

    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        // 兜底：直接遇到的 Macro 节点（如 stmt 位置的宏调用）
        let macro_str = node.tokens.to_string();
        scan_self_field_accesses(&macro_str, &mut self.deps);
        syn::visit::visit_macro(self, node);
    }
}

/// 在字符串中扫描 `self.<ident>` 模式，将识别到的字段名加入 `deps`
///
/// 边界检查：`self` 前必须是非标识符字符（避免匹配 `myself.foo`）。
fn scan_self_field_accesses(s: &str, deps: &mut Vec<String>) {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i + 5 <= bytes.len() {
        // 边界检查：前一个字符不能是标识符字符
        let boundary_ok = i == 0 || !is_ident_byte(bytes[i - 1]);
        if boundary_ok && &bytes[i..i + 5] == b"self." {
            // 提取标识符
            let start = i + 5;
            let mut end = start;
            while end < bytes.len() && is_ident_byte(bytes[end]) {
                end += 1;
            }
            if end > start {
                if let Ok(name) = std::str::from_utf8(&bytes[start..end]) {
                    if !deps.contains(&name.to_string()) {
                        deps.push(name.to_string());
                    }
                }
            }
            i = end;
        } else {
            i += 1;
        }
    }
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
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
    fn scans_pub_fields_only() {
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
}
