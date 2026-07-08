//! 节点代码生成 —— `gen_node` + `gen_element`
//!
//! 为单个 AST 节点（元素/文本/插值/混合文本）生成构建代码。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};
use crate::tags;
use crate::compiler::component as comp;
use crate::compiler::menu;

use super::attribute::apply_css_styles;
use super::text::{gen_expr_code, gen_mixed_text};

/// codegen 结果：元素代码 + 是否迭代器
pub type GenResult = (String, bool);

/// 为单个节点生成构建代码，返回 (代码, 是否迭代器)
///
/// 公共入口，无父链（顶层调用）。内部委托 `gen_node_impl` 传递空父链。
pub fn gen_node(
    node: &Node,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<GenResult, CodegenError> {
    gen_node_impl(node, ctx, depth, id_counter, loop_vars, &[])
}

/// 带父链的节点生成（供 `gen_element` 递归子节点时调用）
pub(crate) fn gen_node_impl(
    node: &Node,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<GenResult, CodegenError> {
    let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
    match node {
        Node::Element(elem) => {
            let (code, is_iter) = gen_element(elem, ctx, depth, id_counter, loop_vars, parents)?;
            // 注入 sourcemap 行内标记：后处理扫描时记录 (elem.span, rust_line, rust_col)
            // 标记格式 /*__rml_sm:S:E*/，S/E 为 AST span 字节偏移
            let marked = format!("/*__rml_sm:{}:{}*/{}", elem.span.start, elem.span.end, code);
            Ok((marked, is_iter))
        }
        Node::Text(text) => Ok((format!("{:?}", text), false)),
        Node::Interpolation { expr, span } => {
            let code = format!("format!(\"{{}}\", {})", gen_expr_code(expr, &lv, &computed));
            let marked = format!("/*__rml_sm:{}:{}*/{}", span.start, span.end, code);
            Ok((marked, false))
        }
        Node::MixedText(segments) => {
            // MixedText 无整体 span（segments 各自带 span），此处不加标记
            Ok((gen_mixed_text(segments, &lv, &computed), false))
        }
    }
}

pub(crate) fn gen_element(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<GenResult, CodegenError> {
    // 0. 处理 once 指令：数据快照（必须在所有其他处理之前）
    if elem.directives.iter().any(|d| matches!(d, Directive::Once { .. })) {
        return super::once::gen_once_element(elem, ctx, depth, id_counter, loop_vars, parents);
    }

    // 0b. 处理 html 指令：降级为 Label 文本节点
    //
    // GPUI 无原生 HTML 渲染能力，`html={raw}` 指令降级为 `rml_ui::Label::new(raw)` 文本节点。
    // 元素的其他属性和子节点被忽略（html 指令的语义是"用 raw 替换整个元素内容"）。
    // if/show/each 指令仍然生效（控制是否渲染 / 迭代 Label）。
    if let Some(html_expr) = elem.directives.iter().find_map(|d| match d {
        Directive::Html { expr, .. } => Some(expr.clone()),
        _ => None,
    }) {
        let each_clause = elem.directives.iter().find_map(|d| match d {
            Directive::Each { clause: c, .. } => Some(c.clone()),
            _ => None,
        });

        // each 引入循环变量，html 表达式在循环作用域内求值
        let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
        if let Some(clause) = &each_clause {
            child_loop_vars.push(clause.item.clone());
            if let Some(idx) = &clause.index {
                child_loop_vars.push(idx.clone());
            }
        }
        let lv: Vec<&str> = child_loop_vars.iter().map(|s| s.as_str()).collect();
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

        let rust_expr = gen_expr_code(&html_expr, &lv, &computed);
        let label_code = format!("rml_ui::Label::new({})", rust_expr);

        // each 包裹：对每个迭代项渲染一个 Label
        if let Some(clause) = each_clause {
            let iter_expr = if loop_vars.iter().any(|lv| {
                clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv))
            }) {
                clause.iterable.clone()
            } else {
                format!("{}.{}", crate::compiler::expr::current_self_alias().unwrap_or("self"), clause.iterable)
            };
            return Ok((
                format!(
                    "{}.iter().map(|{}| {{ {} }})",
                    iter_expr, clause.item, label_code
                ),
                true,
            ));
        }

        // if/show 条件包裹：与 gen_element 末尾的 if/show 处理语义一致
        let cond: Option<String> = elem.directives.iter().find_map(|d| match d {
            Directive::If { expr: c, .. } | Directive::Show { expr: c, .. } => Some(c.clone()),
            _ => None,
        });
        if let Some(cond) = cond {
            let cond_code = gen_expr_code(&cond, &lv, &computed);
            let cond_code = cond_code
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.to_string())
                .unwrap_or(cond_code);
            return Ok((
                format!(
                    "if {} {{ {}.into_any_element() }} else {{ gpui::Empty.into_any_element() }}",
                    cond_code, label_code
                ),
                false,
            ));
        }

        return Ok((label_code, false));
    }

    let tag = &elem.tag;

    // 透明容器：<component content={expr} /> 直接嵌入表达式，不创建元素包装。
    // 用于在 RML 模板中注入动态 AnyElement/impl IntoElement（类似 WPF ContentControl）。
    // 表达式可引用 _window/cx（render 方法作用域内可用）。
    // 支持 `each` 指令：<component each={s in status} content={s.render(_window, cx)} />
    if tag == "component" {
        let content_expr = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr, .. } = attr {
                if name == "content" {
                    return Some(expr.clone());
                }
            }
            None
        });
        if let Some(expr) = content_expr {
            // `<component content={...} />` 表达式可引用 render 方法作用域内的 _window/cx，
            // 将它们加入 loop_vars 避免被加 self. 前缀
            let mut scope_vars: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
            for v in ["_window", "cx"] {
                if !scope_vars.contains(&v) {
                    scope_vars.push(v);
                }
            }

            // 检测 each 指令 — 必须在生成 code 前将 loop 变量加入 scope_vars，
            // 否则 gen_expr_code 会把 `group` 误加 `self.` 前缀变成 `self.group`
            let each_clause = elem.directives.iter().find_map(|d| match d {
                Directive::Each { clause: c, .. } => Some(c.clone()),
                _ => None,
            });
            if let Some(clause) = &each_clause {
                if !scope_vars.contains(&clause.item.as_str()) {
                    scope_vars.push(clause.item.as_str());
                }
                if let Some(idx) = &clause.index {
                    if !scope_vars.contains(&idx.as_str()) {
                        scope_vars.push(idx.as_str());
                    }
                }
            }

            let lv: Vec<&str> = scope_vars;
            let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
            let code = crate::compiler::codegen::gen_expr_code(&expr, &lv, &computed);

            if let Some(clause) = each_clause {
                // iterable 可能是 self.field 或 loop_var.field
                let iter_expr = if loop_vars.iter().any(|lv| {
                    clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv))
                }) {
                    clause.iterable.clone()
                } else {
                    format!("{}.{}", crate::compiler::expr::current_self_alias().unwrap_or("self"), clause.iterable)
                };
                let iter_code = format!(
                    "{iter_expr}.iter().map(|{}| {})",
                    clause.item, code
                );
                return Ok((iter_code, true));
            }
            return Ok((code, false));
        }
        return Err(CodegenError {
            message: "<component> 标签必须提供 content={expr} 属性".to_string(),
            span: Some(elem.span),
        });
    }

    // <slot> 占位符：组件模板内声明插槽渲染位置（Vue 风格 `<slot name="header" />`）。
    //
    // slot 渲染闭包存储在 `self.__rml_state.slots: HashMap<&'static str, SlotRenderer>`，
    // codegen 通过 `self.__rml_state.slot(<name>)` 查询并调用闭包即时生成 element：
    //   `self.__rml_state.slot("name").map(|f| f(&NullSlotScope::new("name"), _window, cx)).unwrap_or(gpui::Empty)`
    //
    // 闭包首参 `&dyn ISlotScope` 由插槽宿主构造传入；自定义组件默认传 `NullSlotScope`，
    // 仅向 slot 内容暴露插槽名，不提供父容器操控权（如 resizable）。
    //
    // 返回 is_iter=false（直接是 AnyElement，不需要 .children() 包裹）。
    // 无 name 属性的 `<slot />` 对应 "default" 插槽。
    if tag == "slot" {
        let slot_name = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Static { name, value, .. } if name == "name" => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "default".to_string());
        return Ok((
            format!(
                "self.__rml_state.slot({slot_name:?}).map_or(gpui::Empty.into_any_element(), |f| f(&rml_core::slot::NullSlotScope::new({slot_name:?}), _window, cx))",
                slot_name = slot_name
            ),
            false,
        ));
    }

    // 菜单容器标签（context-menu / dropdown-menu / menu-bar / app-menu-bar）
    if menu::is_menu_container(tag) {
        let code = menu::gen_menu_element(elem, ctx, depth, id_counter, loop_vars)?;
        return Ok((code, false));
    }

    // 用户自定义 #[component]（PascalCase，如 WelcomeCase）
    if ctx.user_components.contains_key(tag) {
        let mut code = comp::gen_component(elem, ctx, depth, id_counter, loop_vars)?;
        if let Some(sheet) = &ctx.stylesheet {
            code.push_str(&apply_css_styles(elem, tag, sheet, parents));
        }
        return Ok((code, false));
    }

    // 扩展组件（PascalCase、kebab-case 或特殊小写标签 menu/status-bar）
    if tags::is_extension_component(tag) {
        let mut code = comp::gen_component(elem, ctx, depth, id_counter, loop_vars)?;
        if let Some(sheet) = &ctx.stylesheet {
            code.push_str(&apply_css_styles(elem, tag, sheet, parents));
        }
        return Ok((code, false));
    }

    // 原生 HTML 标签通过注册表路由到对应 translator
    if let Some(translator) = ctx.registry.resolve(elem) {
        return translator.to_rust(elem, ctx, id_counter, loop_vars, parents);
    }

    Err(CodegenError {
        message: format!("unknown tag: <{}>", tag),
        span: Some(elem.span),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser;

    fn minimal_ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".to_string(),
            registry: crate::compiler::translator::TranslatorRegistry::builtin(),
            ..Default::default()
        }
    }

    /// 生成单个节点的代码（便捷封装）
    fn gen(node: &Node) -> Result<GenResult, CodegenError> {
        let mut counter = 0usize;
        gen_node(node, &minimal_ctx(), 0, &mut counter, &[])
    }

    // ─── if/else 配对：核心场景 ───

    #[test]
    fn if_else_pair_generates_conditional_expression() {
        // <div><div if={x}></div><div else></div></div>
        let root = parser::parse(r#"<div><div if={x}></div><div else></div></div>"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(!is_iter, "outer div should not be an iterator");
        assert!(
            code.contains("if ") && code.contains("} else {"),
            "expected if/else conditional expression, got: {}",
            code
        );
        // 不应回退到 gpui::Empty（else 分支必须有真实元素）
        assert!(
            !code.contains("gpui::Empty.into_any_element()"),
            "if/else pair should not fall back to Empty, got: {}",
            code
        );
    }

    #[test]
    fn if_else_pair_emits_single_child_call() {
        // <div><div if={x}></div><div else></div></div>
        let root = parser::parse(r#"<div><div if={x}></div><div else></div></div>"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        // if/else 配对应作为单个 .child(if ... else ...) 注入，不应出现两个 .child()
        let child_call_count = code.matches(".child(").count();
        assert_eq!(
            child_call_count,
            1,
            "expected single .child() call for if/else pair, got {}: {}",
            child_call_count,
            code
        );
    }

    #[test]
    fn if_else_pair_preserves_inner_content() {
        // <div><div if={x}>visible</div><div else>hidden</div></div>
        let root =
            parser::parse(r#"<div><div if={x}>visible</div><div else>hidden</div></div>"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains("visible") && code.contains("hidden"),
            "expected both branches' content to appear, got: {}",
            code
        );
    }

    // ─── if/else 配对：错误场景 ───

    #[test]
    fn standalone_else_returns_error() {
        // <div><div else></div></div>
        let root = parser::parse(r#"<div><div else></div></div>"#).unwrap();
        let result = gen(&root);
        assert!(result.is_err(), "expected error for standalone else");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("`else` 指令必须紧跟在 `if` 指令之后"),
            "expected standalone else error, got: {}",
            err.message
        );
    }

    #[test]
    fn else_without_adjacent_if_returns_error() {
        // <div><div></div><div else></div></div>  (if 与 else 之间有非 if 兄弟)
        let root = parser::parse(r#"<div><div></div><div else></div></div>"#).unwrap();
        let result = gen(&root);
        assert!(result.is_err(), "expected error for non-adjacent else");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("`else` 指令必须紧跟在 `if` 指令之后"),
            "expected standalone else error, got: {}",
            err.message
        );
    }

    #[test]
    fn if_else_with_each_returns_error() {
        // <div><div if={x} each={i in items}></div><div else></div></div>
        let root = parser::parse(
            r#"<div><div if={x} each={i in items}></div><div else></div></div>"#,
        )
        .unwrap();
        let result = gen(&root);
        assert!(result.is_err(), "expected error for if/else with each");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("`if`/`else` 配对不支持 `each`"),
            "expected if/else+each rejection, got: {}",
            err.message
        );
    }

    // ─── if 不带 else：保持现有行为 ───

    #[test]
    fn if_without_else_falls_back_to_empty() {
        // <div><div if={x}></div></div>
        let root = parser::parse(r#"<div><div if={x}></div></div>"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains("if ") && code.contains("gpui::Empty.into_any_element()"),
            "expected if with Empty fallback, got: {}",
            code
        );
    }

    // ─── 多重 if/else 配对 ───

    #[test]
    fn multiple_if_else_pairs_as_siblings() {
        // <div><div if={x}></div><div else></div><div if={y}></div><div else></div></div>
        let root = parser::parse(
            r#"<div><div if={x}></div><div else></div><div if={y}></div><div else></div></div>"#,
        )
        .unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        let else_count = code.matches("} else {").count();
        assert_eq!(
            else_count, 2,
            "expected 2 if/else pairs (2 `}} else {{`), got {}: {}",
            else_count, code
        );
        // 两个 if/else 配对应作为两个 .child() 调用（外加外层 div 自身，但外层 div 不调用 .child）
        let child_call_count = code.matches(".child(").count();
        assert_eq!(
            child_call_count, 2,
            "expected 2 .child() calls for 2 if/else pairs, got {}: {}",
            child_call_count, code
        );
    }

    #[test]
    fn orphan_else_after_consumed_pair_returns_error() {
        // <div><div if={x}></div><div else></div><div else></div></div>
        // 第一个 if/else 配对成功，第二个 else 无前置 if → 报错
        let root = parser::parse(
            r#"<div><div if={x}></div><div else></div><div else></div></div>"#,
        )
        .unwrap();
        let result = gen(&root);
        assert!(result.is_err(), "expected error for orphan else");
        let err = result.unwrap_err();
        assert!(
            err.message.contains("`else` 指令必须紧跟在 `if` 指令之后"),
            "expected orphan else error, got: {}",
            err.message
        );
    }

    // ─── html 指令：降级为 Label 文本节点 ───

    #[test]
    fn html_basic_generates_label() {
        // <div html={raw} /> → rml_ui::Label::new(self.raw)
        let root = parser::parse(r#"<div html={raw} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(!is_iter, "html should not be an iterator");
        assert!(
            code.contains("rml_ui::Label::new(self.raw)"),
            "expected Label::new(self.raw), got: {}",
            code
        );
    }

    #[test]
    fn html_with_member_access() {
        // <div html={user.bio} /> → rml_ui::Label::new(self.user.bio)
        let root = parser::parse(r#"<div html={user.bio} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains("rml_ui::Label::new(self.user.bio)"),
            "expected Label::new(self.user.bio), got: {}",
            code
        );
    }

    #[test]
    fn html_with_string_literal() {
        // <div html={"hello"} /> → rml_ui::Label::new("hello")
        let root = parser::parse(r#"<div html={"hello"} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains("rml_ui::Label::new(\"hello\")"),
            "expected Label::new(\"hello\"), got: {}",
            code
        );
    }

    #[test]
    fn html_with_if_directive() {
        // <div if={show} html={raw} /> → if self.show { Label::new(self.raw) } else { Empty }
        let root = parser::parse(r#"<div if={show} html={raw} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(!is_iter, "html+if should not be an iterator");
        assert!(
            code.contains("if self.show"),
            "expected if self.show conditional, got: {}",
            code
        );
        assert!(
            code.contains("rml_ui::Label::new(self.raw)"),
            "expected Label::new(self.raw) inside if branch, got: {}",
            code
        );
        assert!(
            code.contains("gpui::Empty"),
            "expected Empty in else branch, got: {}",
            code
        );
    }

    #[test]
    fn html_with_each_directive() {
        // <li each={item in items} html={item.html} /> → self.items.iter().map(|item| { Label::new(item.html) })
        let root = parser::parse(r#"<li each={item in items} html={item.html} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(is_iter, "html+each should be an iterator");
        assert!(
            code.contains("self.items.iter().map(|item|"),
            "expected iter().map(|item|) wrapper, got: {}",
            code
        );
        assert!(
            code.contains("rml_ui::Label::new(item.html)"),
            "expected Label::new(item.html) inside map, got: {}",
            code
        );
    }

    #[test]
    fn html_ignores_other_attributes_and_children() {
        // <div html={raw} class="card"><span>ignored</span></div>
        // → 只生成 Label::new(self.raw)，忽略 class 和子节点
        let root =
            parser::parse(r#"<div html={raw} class="card"><span>ignored</span></div>"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains("rml_ui::Label::new(self.raw)"),
            "expected only Label::new(self.raw), got: {}",
            code
        );
        assert!(
            !code.contains("ignored"),
            "expected children to be ignored, got: {}",
            code
        );
    }

    // ─── key 指令：基于 key 哈希生成稳定 element_id ───

    #[test]
    fn key_basic_generates_stable_id_from_field() {
        // <div key={item.id} /> → .id(("rml_key", rml_core::element_id::from_key(&self.item.id)))
        let root = parser::parse(r#"<div key={item.id} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&self.item.id))"#),
            "expected key-based stable id, got: {}",
            code
        );
        // 不应回退到递增整数 id
        assert!(
            !code.contains("\"rml_el\""),
            "key directive should not fall back to event-style incremental id, got: {}",
            code
        );
    }

    #[test]
    fn key_in_each_uses_loop_var_not_self() {
        // <li each={item in items} key={item.id} />
        // → key 表达式在 each 作用域内求值，引用循环变量 item 而非 self.item
        let root = parser::parse(r#"<li each={item in items} key={item.id} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(is_iter, "each should produce an iterator");
        assert!(
            code.contains("self.items.iter().map(|item|"),
            "expected iter().map(|item|) wrapper, got: {}",
            code
        );
        // key 表达式应引用循环变量 item.id，而非 self.item.id
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&item.id))"#),
            "expected key to use loop var item.id, got: {}",
            code
        );
        assert!(
            !code.contains("self.item.id"),
            "key in each should not reference self.item.id, got: {}",
            code
        );
    }

    #[test]
    fn key_with_string_literal() {
        // <div key={"static-key"} /> → .id(("rml_key", rml_core::element_id::from_key(&"static-key")))
        let root = parser::parse(r#"<div key={"static-key"} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&"static-key"))"#),
            "expected key with string literal, got: {}",
            code
        );
    }

    #[test]
    fn key_with_numeric_literal() {
        // <div key={42} /> → .id(("rml_key", rml_core::element_id::from_key(&42)))
        let root = parser::parse(r#"<div key={42} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&42))"#),
            "expected key with numeric literal, got: {}",
            code
        );
    }

    #[test]
    fn ref_takes_priority_over_key() {
        // <div ref="input1" key={item.id} /> → ref 优先，使用 rml_ref:input1
        let root = parser::parse(r#"<div ref="input1" key={item.id} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(r#".id("rml_ref:input1")"#),
            "ref should take priority over key, got: {}",
            code
        );
        // 不应同时生成 key id
        assert!(
            !code.contains("rml_key"),
            "ref should suppress key id generation, got: {}",
            code
        );
    }

    #[test]
    fn key_takes_priority_over_event_handler() {
        // <div key={item.id} onclick={handler} /> → key 优先于事件处理器 id
        let root = parser::parse(r#"<div key={item.id} onclick={handler} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&self.item.id))"#),
            "key should take priority over event handler, got: {}",
            code
        );
        // 不应回退到递增整数 id
        assert!(
            !code.contains("\"rml_el\""),
            "key should suppress event-style incremental id, got: {}",
            code
        );
    }

    #[test]
    fn key_without_other_id_sources_alone_in_tree() {
        // <div><span key={k} /></div> → 子元素只用 key id，外层 div 无 id
        let root = parser::parse(r#"<div><span key={k} /></div>"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(r#"("rml_key", rml_core::element_id::from_key(&self.k))"#),
            "expected key-based id for span, got: {}",
            code
        );
    }

    // ─── show 指令：保留布局空间，隐藏视觉（与 if 区分） ───

    #[test]
    fn show_alone_generates_invisible_when() {
        // <div show={visible} /> → gpui::div()....when(!self.visible, |d| d.invisible())
        let root = parser::parse(r#"<div show={visible} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(!is_iter, "show alone should not be an iterator");
        assert!(
            code.contains(".when(!self.visible, |d| d.invisible())"),
            "expected .when(!self.visible, |d| d.invisible()), got: {}",
            code
        );
        // 不应回退到 if/else 条件渲染（show 必须始终渲染元素）
        assert!(
            !code.contains("gpui::Empty.into_any_element()"),
            "show should always render element (no Empty fallback), got: {}",
            code
        );
    }

    #[test]
    fn show_with_each_applies_invisible_per_item() {
        // <li each={item in items} show={item.visible} />
        // → self.items.iter().map(|item| { div()....when(!item.visible, |d| d.invisible()) })
        let root = parser::parse(r#"<li each={item in items} show={item.visible} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(is_iter, "each should produce an iterator");
        assert!(
            code.contains("self.items.iter().map(|item|"),
            "expected iter().map(|item|) wrapper, got: {}",
            code
        );
        assert!(
            code.contains(".when(!item.visible, |d| d.invisible())"),
            "expected per-item .when(!item.visible, |d| d.invisible()) inside map, got: {}",
            code
        );
    }

    #[test]
    fn if_takes_priority_over_show() {
        // <div if={cond} show={visible} /> → if 优先，使用条件渲染（show 被忽略）
        let root = parser::parse(r#"<div if={cond} show={visible} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains("if self.cond {"),
            "expected if to take priority (conditional render), got: {}",
            code
        );
        assert!(
            code.contains("gpui::Empty.into_any_element()"),
            "expected Empty in else branch for if, got: {}",
            code
        );
        // 不应生成 show 的 invisible() 调用
        assert!(
            !code.contains("invisible()"),
            "show should be ignored when if is present, got: {}",
            code
        );
    }

    #[test]
    fn if_with_each_applies_conditional_per_item() {
        // <li each={item in items} if={item.active} />
        // → self.items.iter().map(|item| { if item.active { elem } else { Empty } })
        let root = parser::parse(r#"<li each={item in items} if={item.active} />"#).unwrap();
        let (code, is_iter) = gen(&root).expect("codegen should succeed");
        assert!(is_iter, "each should produce an iterator");
        assert!(
            code.contains("self.items.iter().map(|item|"),
            "expected iter().map(|item|) wrapper, got: {}",
            code
        );
        assert!(
            code.contains("if item.active {"),
            "expected per-item if inside map, got: {}",
            code
        );
        assert!(
            code.contains("gpui::Empty.into_any_element()"),
            "expected Empty in else branch, got: {}",
            code
        );
    }

    #[test]
    fn show_with_member_access() {
        // <div show={user.visible} /> → .when(!self.user.visible, |d| d.invisible())
        let root = parser::parse(r#"<div show={user.visible} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(".when(!self.user.visible, |d| d.invisible())"),
            "expected .when(!self.user.visible, |d| d.invisible()), got: {}",
            code
        );
    }

    #[test]
    fn show_with_string_literal_condition() {
        // <div show={true} /> → .when(!true, |d| d.invisible()) — 字面量条件
        // 注意：show={true} 始终可见，show={false} 始终不可见，但 codegen 不做常量折叠
        let root = parser::parse(r#"<div show={true} />"#).unwrap();
        let (code, _) = gen(&root).expect("codegen should succeed");
        assert!(
            code.contains(".when(!true, |d| d.invisible())"),
            "expected .when(!true, |d| d.invisible()) for literal, got: {}",
            code
        );
    }
}
