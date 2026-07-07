//! 节点代码生成 —— `gen_node` + `gen_element`
//!
//! 为单个 AST 节点（元素/文本/插值/混合文本）生成构建代码。

use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};
use crate::tags;
use crate::compiler::component as comp;
use crate::compiler::event;
use crate::compiler::menu;

use super::attribute::{apply_bind_attr, apply_css_styles, apply_static_attr};
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
fn gen_node_impl(
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

/// 从 AST Element 提取 ParentInfo（用于子节点的 CSS 父链匹配）
fn build_parent_info(elem: &Element) -> ParentInfo {
    let class_value: String = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "class" => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let mut classes: Vec<String> = class_value.split_whitespace().map(|s| s.to_string()).collect();
    // 组件标签隐式携带与其小写标签名相同的 class，供后代选择器匹配
    if let Some(implicit) = tags::implicit_class_for(&elem.tag) {
        if !classes.contains(&implicit) {
            classes.push(implicit);
        }
    }
    let id: Option<String> = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value, .. } if name == "id" => Some(value.clone()),
            _ => None,
        });
    ParentInfo {
        tag: elem.tag.clone(),
        classes,
        id,
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
                format!("self.{}", clause.iterable)
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
                    format!("self.{}", clause.iterable)
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

    // model 指令：input/textarea 的双向绑定
    let model_field = elem.directives.iter().find_map(|d| match d {
        Directive::Model { field: f, .. } => Some(f.clone()),
        _ => None,
    });

    if let Some(field) = &model_field {
        if tag == "input" || tag == "textarea" {
            let code = super::binding::gen_model_input(elem, ctx, id_counter, field.clone(), parents)?;
            return Ok((code, false));
        }
    }

    let builtin = tags::lookup(tag).ok_or_else(|| CodegenError {
        message: format!("unknown tag: <{}>", tag),
        span: Some(elem.span),
    })?;

    let each_clause = elem.directives.iter().find_map(|d| match d {
        Directive::Each { clause: c, .. } => Some(c.clone()),
        _ => None,
    });

    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    if let Some(clause) = &each_clause {
        child_loop_vars.push(clause.item.clone());
        if let Some(idx) = &clause.index {
            child_loop_vars.push(idx.clone());
        }
    }
    let lv: Vec<&str> = child_loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    // 1. 生成元素构造调用
    let mut code = String::from(builtin.codegen_ctor());

    // 2. ref / key 指令或事件处理器：生成 .id()
    //
    // 优先级：ref > key > 事件处理器
    // - ref：需要特定名称供后续访问（如 self.input1.focus(cx)）
    // - key：列表渲染时基于 key 表达式生成稳定 element_id（非递增整数），
    //   使列表项重新排序后 GPUI 可正确跟踪元素状态
    // - 事件处理器：仅需元素有任意 id 即可触发交互
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        Directive::Ref { name, .. } => Some(name.as_str()),
        _ => None,
    });

    let key_expr: Option<String> = elem.directives.iter().find_map(|d| match d {
        Directive::Key { expr, .. } => Some(expr.clone()),
        _ => None,
    });

    let has_events = elem
        .attributes
        .iter()
        .any(|attr| matches!(attr, Attribute::Event { .. }));

    if let Some(name) = ref_name {
        code.push_str(&format!(".id({:?})", format!("rml_ref:{}", name)));
    } else if let Some(key) = key_expr {
        // key 表达式在 each 作用域内求值（通常引用 each 的 item，如 item.id）
        let key_code = gen_expr_code(&key, &lv, &computed);
        code.push_str(&format!(
            ".id((\"rml_key\", rml_core::element_id::from_key(&{})))",
            key_code
        ));
    } else if has_events {
        let id_val = *id_counter;
        *id_counter += 1;
        code.push_str(&format!(".id((\"rml_el\", {}usize))", id_val));
    }

    // 2b. 应用 CSS 样式（class/id 属性匹配全局样式表，支持父链后代/子选择器）
    if let Some(sheet) = &ctx.stylesheet {
        let style_code = apply_css_styles(elem, tag, sheet, parents);
        if !style_code.is_empty() {
            code.push_str(&style_code);
        }
    }

    // 3. 应用静态属性与绑定属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value, .. } => {
                code.push_str(&apply_static_attr(name, value));
            }
            Attribute::Bind { name, expr, .. } => {
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                code.push_str(&apply_bind_attr(name, expr, &lv, &computed));
            }
            Attribute::Event { name, handler, .. } => {
                code.push_str(&event::apply_event(name, handler, ctx));
            }
        }
    }

    // 4. 处理子节点（构建父链：当前元素 → child_parents）
    //
    // 使用索引遍历以支持 if/else 兄弟配对：当 if 元素紧邻 else 兄弟时，
    // 合并为单个 `if cond { if_elem } else { else_elem }` 表达式并作为单个 .child() 注入。
    let current_parent = build_parent_info(elem);
    let mut child_parents = parents.to_vec();
    child_parents.push(current_parent);

    let mut i = 0;
    while i < elem.children.len() {
        let child = &elem.children[i];

        // 4a. 检测独立 else（无前置 if 兄弟）：报错
        if let Node::Element(e) = child {
            let has_else = e.directives.iter().any(|d| matches!(d, Directive::Else { .. }));
            let has_if = e.directives.iter().any(|d| matches!(d, Directive::If { .. }));
            if has_else && !has_if {
                return Err(CodegenError {
                    message: "`else` 指令必须紧跟在 `if` 指令之后".to_string(),
                    span: Some(elem.span),
                });
            }
        }

        // 4b. 检测 if + 紧邻 else 配对
        let if_cond: Option<String> = if let Node::Element(e) = child {
            e.directives.iter().find_map(|d| match d {
                Directive::If { expr: c, .. } => Some(c.clone()),
                _ => None,
            })
        } else {
            None
        };

        if let Some(cond) = if_cond {
            let next_idx = i + 1;
            let next_has_else = next_idx < elem.children.len()
                && matches!(
                    &elem.children[next_idx],
                    Node::Element(e) if e.directives.iter().any(|d| matches!(d, Directive::Else { .. }))
                );

            if next_has_else {
                // if/else 配对成功：clone 两侧元素，分别移除 If / Else 指令后递归生成
                let (if_e, else_e) = match (&elem.children[i], &elem.children[next_idx]) {
                    (Node::Element(if_e), Node::Element(else_e)) => (if_e, else_e),
                    _ => unreachable!("next_has_else 已保证 next_idx 是 Element"),
                };

                let mut if_clone = if_e.clone();
                if_clone.directives.retain(|d| !matches!(d, Directive::If { .. }));
                let mut else_clone = else_e.clone();
                else_clone.directives.retain(|d| !matches!(d, Directive::Else { .. }));

                let (if_code, if_is_iter) = gen_node_impl(
                    &Node::Element(if_clone),
                    ctx,
                    depth + 1,
                    id_counter,
                    &child_loop_vars,
                    &child_parents,
                )?;
                let (else_code, else_is_iter) = gen_node_impl(
                    &Node::Element(else_clone),
                    ctx,
                    depth + 1,
                    id_counter,
                    &child_loop_vars,
                    &child_parents,
                )?;

                // if/else 配对不应产生迭代器（each 指令会包装为迭代器，语义不明确）
                if if_is_iter || else_is_iter {
                    return Err(CodegenError {
                        message: "`if`/`else` 配对不支持 `each` 指令，请将列表渲染与条件渲染分离"
                            .to_string(),
                        span: Some(elem.span),
                    });
                }

                let cond_code = gen_expr_code(&cond, &lv, &computed);
                let cond_code = cond_code
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .map(|s| s.to_string())
                    .unwrap_or(cond_code);

                let merged = format!(
                    "if {} {{ {}.into_any_element() }} else {{ {}.into_any_element() }}",
                    cond_code, if_code, else_code
                );
                code.push_str(&format!("\n            .child({})", merged));
                i = next_idx + 1;
                continue;
            }
        }

        // 4c. 默认行为：单独处理子节点
        let (child_code, is_iter) =
            gen_node_impl(child, ctx, depth + 1, id_counter, &child_loop_vars, &child_parents)?;
        if is_iter {
            code.push_str(&format!("\n            .children({})", child_code));
        } else {
            code.push_str(&format!("\n            .child({})", child_code));
        }
        i += 1;
    }

    // 5. 处理 if / show 指令：条件渲染
    //
    // if 与 show 的语义区分：
    // - `if={cond}`：条件渲染，cond 为 false 时元素完全不存在（不占布局空间）
    // - `show={cond}`：始终渲染元素，cond 为 false 时通过 invisible() 隐藏视觉但保留布局空间
    //
    // 若 if 与 show 同时存在，if 优先（show 被忽略）：if 为 false 时元素不存在，show 无意义。
    let if_cond: Option<String> = elem.directives.iter().find_map(|d| match d {
        Directive::If { expr: c, .. } => Some(c.clone()),
        _ => None,
    });
    let show_cond: Option<String> = if if_cond.is_some() {
        None
    } else {
        elem.directives.iter().find_map(|d| match d {
            Directive::Show { expr: c, .. } => Some(c.clone()),
            _ => None,
        })
    };

    if let Some(cond) = if_cond {
        let cond_code = gen_expr_code(&cond, &lv, &computed);
        let cond_code = cond_code
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .map(|s| s.to_string())
            .unwrap_or(cond_code);
        code = format!(
            "if {} {{ {}.into_any_element() }} else {{ gpui::Empty.into_any_element() }}",
            cond_code, code
        );
    } else if let Some(cond) = show_cond {
        // show 指令：始终渲染元素，cond 为 false 时通过 invisible() 隐藏（保留布局空间）
        //
        // invisible() 映射到 GPUI Visibility::Hidden（CSS visibility:hidden），
        // 元素参与布局但不绘制 —— 与 if（Display::None 不占空间）明确区分。
        let cond_code = gen_expr_code(&cond, &lv, &computed);
        let cond_code = cond_code
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .map(|s| s.to_string())
            .unwrap_or(cond_code);
        code = format!("{}.when(!{}, |d| d.invisible())", code, cond_code);
    }

    // 6. 处理 each 指令：将元素包装在迭代器中（最外层包装）
    //
    // each 在 if/show 之后，使条件按 item 逐项应用：
    // - `each + if`：每个 item 独立条件渲染（false 项渲染为 Empty）
    // - `each + show`：每个 item 始终渲染，按条件逐项 invisible()
    if let Some(clause) = each_clause {
        let iter_expr = if loop_vars.iter().any(|lv| {
            clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv))
        }) {
            clause.iterable.clone()
        } else {
            format!("self.{}", clause.iterable)
        };
        let iter_code = format!(
            "{}.iter().map(|{}| {{\n                {}\n            }})",
            iter_expr, clause.item, code
        );
        return Ok((iter_code, true));
    }

    Ok((code, false))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compiler::CodegenCtx;
    use crate::parser;

    fn minimal_ctx() -> CodegenCtx {
        CodegenCtx {
            view_struct_name: "TestView".to_string(),
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
