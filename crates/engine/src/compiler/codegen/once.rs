//! `once` 指令代码生成
//!
//! `once` 冻结元素的数据依赖：首次渲染时快照所有字段引用值，后续渲染使用快照数据
//! 重建元素（AnyElement 不可跨帧缓存，必须每帧重建，但用快照数据而非当前数据）。
//!
//! ## 实现策略
//!
//! 1. 递归调用 `gen_element`（移除 Once 指令）生成元素代码
//! 2. 遍历元素 AST 收集所有字段引用（`collect_element_fields`）
//! 3. 对元素代码做字符串替换：`self.X` / `self.X()` → `__once_data_{id}.N`
//!    - 跳过字符串字面量内的匹配（避免误替换文本内容）
//!    - 词边界检查（避免 `self.count` 匹配 `self.counter`）
//!    - computed 方法先替换（更长模式 `self.X()`），普通字段后替换
//! 4. 快照代码单独生成（不经过替换），避免 `self.X.clone()` 被错误替换
//!
//! ## 嵌套 once
//!
//! 嵌套 once 元素各自生成独立的快照块，使用唯一变量名 `__once_data_{id}` 避免遮蔽。
//! 外层 once 跳过内层 once 子元素的字段收集（内层自行处理）。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::compiler::expr;
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node, TextSegment};

use super::node::{gen_element, GenResult};

/// `once` 指令入口：生成数据快照包裹的元素代码
pub fn gen_once_element(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<GenResult, CodegenError> {
    // 1. 预留唯一 ID（变量名 + 缓存键）
    let once_id = *id_counter;
    *id_counter += 1;
    let var_name = format!("__once_data_{}", once_id);
    let cache_key = format!("once_{}", once_id);

    // 2. 克隆元素并移除 Once 指令，递归生成元素代码
    let mut elem_clone = elem.clone();
    elem_clone.directives.retain(|d| !matches!(d, Directive::Once));
    let (elem_code, is_iter) = gen_element(&elem_clone, ctx, depth, id_counter, loop_vars, parents)?;

    // 3. 收集字段引用（去重，保持顺序）
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    let mut field_refs: Vec<String> = Vec::new();
    collect_element_fields(elem, &lv, &mut field_refs);
    let mut seen = std::collections::HashSet::new();
    field_refs.retain(|f| seen.insert(f.clone()));

    // 4. 无字段引用 → once 退化为 no-op
    if field_refs.is_empty() {
        return Ok((elem_code, is_iter));
    }

    // 5. 替换元素代码中的 self.X / self.X() → __once_data_{id}.N
    let substituted = apply_substitution(&elem_code, &field_refs, &computed, &var_name);

    // 6. 生成快照代码（不经过替换）
    let snapshot = generate_snapshot(&field_refs, &computed);

    // 7. 拼装最终代码块
    let code = format!(
        "{{\n                let {var} = self.__rml_state.once_get_or_init({key:?}, || {{\n                    ({snap})\n                }});\n                {code}\n            }}",
        var = var_name,
        key = cache_key,
        snap = snapshot,
        code = substituted
    );

    Ok((code, is_iter))
}

// ──────────────────────────────────────────────────────────────────────────
//  字段收集
// ──────────────────────────────────────────────────────────────────────────

/// 遍历元素 AST，收集所有顶层字段引用名
///
/// 跳过嵌套 `once` 子元素（由内层 once 自行处理其快照）。
fn collect_element_fields(elem: &Element, loop_vars: &[&str], fields: &mut Vec<String>) {
    // 构建有效作用域：loop_vars + each 引入的循环变量
    // each 的 iterable 用原始 loop_vars 收集（在循环外求值），其余用有效作用域
    let mut effective_vars: Vec<String> = loop_vars.iter().map(|s| s.to_string()).collect();
    for d in &elem.directives {
        if let Directive::Each(clause) = d {
            effective_vars.push(clause.item.clone());
            if let Some(idx) = &clause.index {
                effective_vars.push(idx.clone());
            }
        }
    }
    let effective_refs: Vec<&str> = effective_vars.iter().map(|s| s.as_str()).collect();

    // 指令：if/show/each iterable 用原始 loop_vars，model.field 直接收集
    for d in &elem.directives {
        match d {
            Directive::If(c) | Directive::Show(c) => {
                fields.extend(expr::collect_fields(c, loop_vars));
            }
            Directive::Each(clause) => {
                fields.extend(expr::collect_fields(&clause.iterable, loop_vars));
            }
            Directive::Model { field, .. } => {
                fields.push(field.clone());
            }
            Directive::Html(expr) => {
                // html 表达式在 each 作用域内求值，用 effective_refs 跳过循环变量
                fields.extend(expr::collect_fields(expr, &effective_refs));
            }
            Directive::Key(expr) => {
                // key 表达式在 each 作用域内求值，用 effective_refs 跳过循环变量
                fields.extend(expr::collect_fields(expr, &effective_refs));
            }
            _ => {}
        }
    }

    // 绑定属性：用有效作用域（each 循环变量在作用域内）
    for attr in &elem.attributes {
        if let Attribute::Bind { name, expr, .. } = attr {
            if name != "class" && name != "id" && name != "style" {
                fields.extend(expr::collect_fields(expr, &effective_refs));
            }
        }
    }

    // <component> 是透明容器，不遍历子节点
    if elem.tag == "component" {
        return;
    }

    // 子节点：用有效作用域，跳过嵌套 once
    for child in &elem.children {
        match child {
            Node::Element(child_elem) => {
                if child_elem.directives.iter().any(|d| matches!(d, Directive::Once)) {
                    continue;
                }
                collect_element_fields(child_elem, &effective_refs, fields);
            }
            Node::Interpolation(expr_str) => {
                fields.extend(expr::collect_fields(expr_str, &effective_refs));
            }
            Node::MixedText(segs) => {
                for seg in segs {
                    if let TextSegment::Interpolation(expr_str) = seg {
                        fields.extend(expr::collect_fields(expr_str, &effective_refs));
                    }
                }
            }
            Node::Text(_) => {}
        }
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  字符串替换
// ──────────────────────────────────────────────────────────────────────────

/// 对元素代码做字段替换：`self.X` / `self.X()` → `{var}.N`
///
/// 替换顺序：computed 方法（`self.X()`，更长模式）优先，普通字段（`self.X`）其次。
/// 这样避免 `self.count` 部分匹配 `self.count()`。
fn apply_substitution(
    code: &str,
    field_refs: &[String],
    computed: &[&str],
    var_name: &str,
) -> String {
    let mut result = code.to_string();

    // 1. computed 方法：替换 self.X() → var.N
    for (i, field) in field_refs.iter().enumerate() {
        if computed.contains(&field.as_str()) {
            let target = format!("self.{}()", field);
            let replacement = format!("{}.{}", var_name, i);
            result = substitute_outside_strings(&result, &target, &replacement);
        }
    }

    // 2. 普通字段：替换 self.X → var.N（词边界检查）
    for (i, field) in field_refs.iter().enumerate() {
        if !computed.contains(&field.as_str()) {
            let target = format!("self.{}", field);
            let replacement = format!("{}.{}", var_name, i);
            result = substitute_outside_strings(&result, &target, &replacement);
        }
    }

    result
}

/// 在字符串外（非字符串字面量内）替换 `target` 为 `replacement`
///
/// 跳过 `"..."` 字符串字面量（处理 `\"` 转义），避免误替换文本内容。
/// 词边界检查：前后字符不能是标识符字符（避免 `self.count` 匹配 `self.counter`）。
fn substitute_outside_strings(code: &str, target: &str, replacement: &str) -> String {
    let mut result = String::with_capacity(code.len());
    let mut i = 0;
    let mut in_string = false;
    let mut escape = false;

    while i < code.len() {
        let ch = code[i..].chars().next().unwrap();

        if in_string {
            result.push(ch);
            if escape {
                escape = false;
            } else if ch == '\\' {
                escape = true;
            } else if ch == '"' {
                in_string = false;
            }
            i += ch.len_utf8();
        } else if ch == '"' {
            in_string = true;
            result.push(ch);
            i += 1;
        } else if code[i..].starts_with(target) {
            let before_ok = i == 0 || !is_ident_byte(code.as_bytes()[i - 1]);
            let after = i + target.len();
            let after_ok = after >= code.len() || !is_ident_byte(code.as_bytes()[after]);

            if before_ok && after_ok {
                result.push_str(replacement);
                i = after;
            } else {
                result.push(ch);
                i += ch.len_utf8();
            }
        } else {
            result.push(ch);
            i += ch.len_utf8();
        }
    }
    result
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

// ──────────────────────────────────────────────────────────────────────────
//  快照生成
// ──────────────────────────────────────────────────────────────────────────

/// 生成快照元组表达式：`(self.X.clone(), self.Y().clone(), ...)`
///
/// computed 方法用 `self.X().clone()`（可能返回引用，需 clone 得到 owned 值）。
/// 普通字段用 `self.X.clone()`。
/// 单元素元组需尾随逗号 `(x,)` 才是元组而非括号表达式。
fn generate_snapshot(field_refs: &[String], computed: &[&str]) -> String {
    let parts: Vec<String> = field_refs
        .iter()
        .map(|field| {
            if computed.contains(&field.as_str()) {
                format!("self.{}().clone()", field)
            } else {
                format!("self.{}.clone()", field)
            }
        })
        .collect();

    if parts.len() == 1 {
        // 单元素：返回 `expr,`（带尾随逗号），由外层 `({snap})` 形成 1-tuple
        // 避免生成 `((expr,))` 多余括号 warning
        format!("{},", parts[0])
    } else {
        parts.join(", ")
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ─── substitute_outside_strings ───

    #[test]
    fn substitute_basic_replacement() {
        let result = substitute_outside_strings("self.count + 1", "self.count", "__once_data_0.0");
        assert_eq!(result, "__once_data_0.0 + 1");
    }

    #[test]
    fn substitute_skips_string_literals() {
        // 字符串字面量内的 self.count 不应被替换
        let code = r#"format!("{}", self.count)"#;
        let result = substitute_outside_strings(code, "self.count", "X");
        // format! 的第一个参数是字符串字面量 "{}"，不应被替换
        // 第二个参数 self.count 不是字符串，应被替换
        assert_eq!(result, r#"format!("{}", X)"#);
    }

    #[test]
    fn substitute_skips_string_with_self_prefix() {
        // 字符串内容包含 "self.count" 但不应被替换
        let code = r#"format!("self.count is {}", self.count)"#;
        let result = substitute_outside_strings(code, "self.count", "X");
        assert_eq!(result, r#"format!("self.count is {}", X)"#);
    }

    #[test]
    fn substitute_word_boundary() {
        // self.count 不应匹配 self.counter
        let code = "self.counter + self.count";
        let result = substitute_outside_strings(code, "self.count", "X");
        assert_eq!(result, "self.counter + X");
    }

    #[test]
    fn substitute_handles_escaped_quotes() {
        // 转义引号 \" 不应结束字符串
        let code = r#""self.count \" end" + self.count"#;
        let result = substitute_outside_strings(code, "self.count", "X");
        assert_eq!(result, r#""self.count \" end" + X"#);
    }

    #[test]
    fn substitute_multiple_occurrences() {
        let code = "self.count + self.count";
        let result = substitute_outside_strings(code, "self.count", "X");
        assert_eq!(result, "X + X");
    }

    // ─── apply_substitution ───

    #[test]
    fn apply_substitution_regular_field() {
        let code = "format!(\"{}\", self.count)";
        let fields = vec!["count".to_string()];
        let result = apply_substitution(code, &fields, &[], "__once_data_0");
        assert_eq!(result, "format!(\"{}\", __once_data_0.0)");
    }

    #[test]
    fn apply_substitution_computed_method() {
        // computed 方法 total → 替换 self.total() （带括号）
        let code = "format!(\"{}\", self.total())";
        let fields = vec!["total".to_string()];
        let result = apply_substitution(code, &fields, &["total"], "__once_data_0");
        assert_eq!(result, "format!(\"{}\", __once_data_0.0)");
    }

    #[test]
    fn apply_substitution_mixed_fields() {
        // count 是普通字段，total 是 computed
        let code = "format!(\"{} {}\", self.count, self.total())";
        let fields = vec!["count".to_string(), "total".to_string()];
        let result = apply_substitution(code, &fields, &["total"], "__once_data_0");
        assert_eq!(result, "format!(\"{} {}\", __once_data_0.0, __once_data_0.1)");
    }

    #[test]
    fn apply_substitution_member_access() {
        // user.name → 快照 user，替换 self.user → __once_data_0.0
        let code = "format!(\"{}\", self.user.name)";
        let fields = vec!["user".to_string()];
        let result = apply_substitution(code, &fields, &[], "__once_data_0");
        assert_eq!(result, "format!(\"{}\", __once_data_0.0.name)");
    }

    // ─── generate_snapshot ───

    #[test]
    fn snapshot_single_regular_field() {
        let fields = vec!["count".to_string()];
        let result = generate_snapshot(&fields, &[]);
        // 单元素返回 `expr,`（带尾随逗号），由外层 `({snap})` 形成 1-tuple
        assert_eq!(result, "self.count.clone(),");
    }

    #[test]
    fn snapshot_single_computed_method() {
        let fields = vec!["total".to_string()];
        let result = generate_snapshot(&fields, &["total"]);
        assert_eq!(result, "self.total().clone(),");
    }

    #[test]
    fn snapshot_multiple_fields() {
        let fields = vec!["count".to_string(), "name".to_string()];
        let result = generate_snapshot(&fields, &[]);
        assert_eq!(result, "self.count.clone(), self.name.clone()");
    }

    #[test]
    fn snapshot_mixed_fields() {
        let fields = vec!["count".to_string(), "total".to_string()];
        let result = generate_snapshot(&fields, &["total"]);
        assert_eq!(result, "self.count.clone(), self.total().clone()");
    }

    // ─── collect_element_fields ───

    fn collect_fields_from_rml(rml: &str) -> Vec<String> {
        let root = crate::parser::parse(rml).unwrap();
        match root {
            crate::parser::ast::Node::Element(elem) => {
                let mut fields = Vec::new();
                collect_element_fields(&elem, &[], &mut fields);
                fields
            }
            _ => panic!("expected element root"),
        }
    }

    #[test]
    fn collect_text_interpolation() {
        let fields = collect_fields_from_rml("<div>{count}</div>");
        assert_eq!(fields, vec!["count"]);
    }

    #[test]
    fn collect_bind_attribute() {
        let fields = collect_fields_from_rml(r#"<div value={user.name}></div>"#);
        assert_eq!(fields, vec!["user"]);
    }

    #[test]
    fn collect_if_directive() {
        let fields = collect_fields_from_rml(r#"<div if={count > 0}></div>"#);
        assert_eq!(fields, vec!["count"]);
    }

    #[test]
    fn collect_each_iterable() {
        let fields = collect_fields_from_rml(r#"<li each={item in items}>{item.name}</li>"#);
        assert_eq!(fields, vec!["items"]);
    }

    #[test]
    fn collect_nested_children() {
        let fields = collect_fields_from_rml(r#"<div><span>{count}</span><p>{name}</p></div>"#);
        assert_eq!(fields, vec!["count", "name"]);
    }

    #[test]
    fn collect_skips_nested_once() {
        // 嵌套 once 子元素的字段由内层 once 处理，外层不收集
        let fields = collect_fields_from_rml(r#"<div once><span once>{count}</span>{name}</div>"#);
        assert_eq!(fields, vec!["name"]);
    }

    #[test]
    fn collect_model_directive() {
        let fields = collect_fields_from_rml(r#"<input model={text} />"#);
        assert_eq!(fields, vec!["text"]);
    }

    #[test]
    fn collect_deduplicates() {
        // collect_element_fields 收集所有引用（含重复），去重在 gen_once_element 中进行
        let fields = collect_fields_from_rml(r#"<div>{count} {count}</div>"#);
        assert_eq!(fields, vec!["count", "count"]);
    }

    #[test]
    fn collect_skips_class_id_style() {
        // class/id/style 绑定形式在 codegen 中被丢弃，不应收集
        let fields = collect_fields_from_rml(r#"<div class={theme} id={elem_id} style={color}></div>"#);
        assert_eq!(fields, Vec::<String>::new());
    }

    // ─── 端到端 codegen 测试 ───

    fn gen_code(rml: &str) -> String {
        use crate::compiler::CodegenCtx;
        let root = crate::parser::parse(rml).unwrap();
        let elem = match root {
            crate::parser::ast::Node::Element(e) => e,
            _ => panic!("expected element"),
        };
        let ctx = CodegenCtx {
            view_struct_name: "TestView".to_string(),
            ..Default::default()
        };
        let mut id_counter = 0;
        let (code, _) =
            gen_element(&elem, &ctx, 0, &mut id_counter, &[], &[]).unwrap();
        code
    }

    #[test]
    fn once_basic_freezes_field() {
        let code = gen_code(r#"<div once>{count}</div>"#);
        assert!(
            code.contains("once_get_or_init"),
            "expected once_get_or_init call, got: {}",
            code
        );
        assert!(
            code.contains("__once_data_0.0"),
            "expected __once_data_0.0 reference, got: {}",
            code
        );
        assert!(
            code.contains("self.count.clone()"),
            "expected self.count.clone() in snapshot, got: {}",
            code
        );
        // 替换后的代码不应再包含 self.count（除非在快照中）
        let after_snapshot = code.split("}).").nth(1).unwrap_or("");
        assert!(
            !after_snapshot.contains("self.count"),
            "expected self.count to be replaced after snapshot, got: {}",
            code
        );
    }

    #[test]
    fn once_no_fields_is_noop() {
        // 无字段引用 → once 退化为普通元素
        let code = gen_code(r#"<div once>static text</div>"#);
        assert!(
            !code.contains("once_get_or_init"),
            "expected no snapshot for static content, got: {}",
            code
        );
    }

    #[test]
    fn once_computed_method() {
        use crate::compiler::CodegenCtx;
        let root = crate::parser::parse(r#"<div once>{total}</div>"#).unwrap();
        let elem = match root {
            crate::parser::ast::Node::Element(e) => e,
            _ => panic!(),
        };
        let ctx = CodegenCtx {
            view_struct_name: "TestView".to_string(),
            computed_methods: vec!["total".to_string()],
            ..Default::default()
        };
        let mut id_counter = 0;
        let (code, _) =
            gen_element(&elem, &ctx, 0, &mut id_counter, &[], &[]).unwrap();
        assert!(
            code.contains("self.total().clone()"),
            "expected self.total().clone() in snapshot, got: {}",
            code
        );
        assert!(
            code.contains("__once_data_0.0"),
            "expected __once_data_0.0 reference, got: {}",
            code
        );
    }

    #[test]
    fn once_with_each() {
        let code = gen_code(r#"<li each={item in items} once>{item.name}</li>"#);
        assert!(
            code.contains("once_get_or_init"),
            "expected once_get_or_init, got: {}",
            code
        );
        assert!(
            code.contains("self.items.clone()"),
            "expected self.items.clone() in snapshot, got: {}",
            code
        );
        assert!(
            code.contains("__once_data_0.0.iter()"),
            "expected __once_data_0.0.iter() for frozen list, got: {}",
            code
        );
    }

    #[test]
    fn once_nested_independent_snapshots() {
        let code = gen_code(r#"<div once><span once>{count}</span>{name}</div>"#);
        // 外层快照 name，内层快照 count
        assert!(
            code.contains("self.name.clone()"),
            "expected self.name.clone() in outer snapshot, got: {}",
            code
        );
        assert!(
            code.contains("self.count.clone()"),
            "expected self.count.clone() in inner snapshot, got: {}",
            code
        );
        // 两个独立的变量名
        assert!(
            code.contains("__once_data_0"),
            "expected __once_data_0 for outer, got: {}",
            code
        );
        assert!(
            code.contains("__once_data_1"),
            "expected __once_data_1 for inner, got: {}",
            code
        );
    }

    // ─── once + html 组合 ───

    #[test]
    fn once_with_html_freezes_field() {
        // <div once html={user.bio} /> → once 快照 user，Label 用快照值
        let code = gen_code(r#"<div once html={user.bio} />"#);
        assert!(
            code.contains("once_get_or_init"),
            "expected once_get_or_init call, got: {}",
            code
        );
        // 快照应包含 user
        assert!(
            code.contains("self.user.clone()"),
            "expected self.user.clone() in snapshot, got: {}",
            code
        );
        // Label 应使用快照变量而非 self.user
        assert!(
            code.contains("__once_data_0.0.bio"),
            "expected __once_data_0.0.bio in Label, got: {}",
            code
        );
        // 替换后的代码不应再包含 self.user.bio（self.user 仅出现在快照中）
        let after_snapshot = code.split("}).").nth(1).unwrap_or("");
        assert!(
            !after_snapshot.contains("self.user.bio"),
            "expected self.user.bio to be replaced after snapshot, got: {}",
            code
        );
    }

    #[test]
    fn once_with_html_and_each_freezes_iterable() {
        // <li once each={item in items} html={item.html} />
        // once 快照 items（each iterable），html 表达式中的 item 是循环变量不快照
        let code = gen_code(r#"<li once each={item in items} html={item.html} />"#);
        assert!(
            code.contains("once_get_or_init"),
            "expected once_get_or_init, got: {}",
            code
        );
        assert!(
            code.contains("self.items.clone()"),
            "expected self.items.clone() in snapshot, got: {}",
            code
        );
        // 迭代应使用快照变量
        assert!(
            code.contains("__once_data_0.0.iter()"),
            "expected __once_data_0.0.iter() for frozen list, got: {}",
            code
        );
    }

    #[test]
    fn once_with_each_and_key_freezes_iterable_only() {
        // <li once each={item in items} key={item.id} />
        // once 快照 items（each iterable），key 表达式中的 item 是循环变量不快照
        let code = gen_code(r#"<li once each={item in items} key={item.id} />"#);
        assert!(
            code.contains("once_get_or_init"),
            "expected once_get_or_init, got: {}",
            code
        );
        assert!(
            code.contains("self.items.clone()"),
            "expected self.items.clone() in snapshot, got: {}",
            code
        );
        // 迭代应使用快照变量
        assert!(
            code.contains("__once_data_0.0.iter()"),
            "expected __once_data_0.0.iter() for frozen list, got: {}",
            code
        );
        // key 表达式应基于循环变量生成稳定 id（item.id，不快照）
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&item.id))"#),
            "expected key to use loop var item.id, got: {}",
            code
        );
    }
}
