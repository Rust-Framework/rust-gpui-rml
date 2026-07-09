//! 原生 HTML 标签公共引擎
//!
//! `BuiltinMeta` 描述单个原生标签的元信息，`BuiltinTranslator` 将其接入统一
//! `IRmlTranslator` 接口；实际转译逻辑委托 `builtin_engine`，避免每个标签重复
//! 实现 id、CSS、属性、子节点、if/show/each 等通用流程。

use super::{ComponentCategory, IRmlTranslator, PrinterCtx, TranslatorMetadata};
use crate::compiler::codegen::attribute::{apply_bind_attr, apply_css_styles, apply_static_attr};
use crate::compiler::codegen::node::gen_node_impl;
use crate::compiler::codegen::text::gen_expr_code;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::css::ParentInfo;
use crate::parser::ast::{Attribute, Directive, Element, Node};

/// 原生 HTML 标签元信息
#[derive(Debug)]
pub struct BuiltinMeta {
    pub tag: &'static str,
    pub display_name: &'static str,
    pub category: ComponentCategory,
    /// GPUI 构造器调用代码，如 `"gpui::div()"`
    pub ctor: &'static str,
    /// 是否可作为容器包含子元素
    pub is_container: bool,
    /// 是否为 void / 自闭合标签
    pub is_self_closing: bool,
    /// 是否实现 GPUI `Styled` trait（anchored/deferred 等非 Styled 元素为 false）
    pub is_styled: bool,
}

impl BuiltinMeta {
    /// 转换为设计时元数据
    pub const fn to_metadata(&self) -> TranslatorMetadata {
        TranslatorMetadata::new(self.tag, self.display_name, self.category)
            .container(self.is_container)
    }
}

/// 原生标签 translator 公共包装
#[derive(Debug)]
pub struct BuiltinTranslator {
    pub meta: &'static BuiltinMeta,
}

impl IRmlTranslator for BuiltinTranslator {
    fn tag(&self) -> &'static str {
        self.meta.tag
    }

    fn to_rust(
        &self,
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
    ) -> Result<(String, bool), CodegenError> {
        builtin_engine::translate(elem, ctx, id_counter, loop_vars, parents, self.meta.ctor, self.meta.is_styled)
    }

    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, super::PrintError> {
        builtin_engine::print(elem, ctx)
    }

    fn metadata(&self) -> TranslatorMetadata {
        self.meta.to_metadata()
    }
}

/// 原生标签转译引擎
pub mod builtin_engine {
    use super::*;

    /// AST → Rust 代码
    ///
    /// `ctor` 为元素构造器调用代码字符串（如 `"gpui::div()"` 或 `"gpui::img(\"foo.png\")"`），
    /// 允许调用方传入动态构造器（如 `<img>` 需将 `src` 属性作为构造参数）。
    pub fn translate(
        elem: &Element,
        ctx: &CodegenCtx,
        id_counter: &mut usize,
        loop_vars: &[String],
        parents: &[ParentInfo],
        ctor: &str,
        is_styled: bool,
    ) -> Result<(String, bool), CodegenError> {
        let tag = &elem.tag;

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
        let mut code = String::from(ctor);

        // 2. ref / key 指令或事件处理器：生成 .id()
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

        // 2b. 应用 CSS 样式（class/id 属性匹配全局样式表）—— 非 Styled 元素跳过
        if is_styled {
            if let Some(sheet) = &ctx.stylesheet {
                let style_code = apply_css_styles(elem, tag, sheet, parents);
                if !style_code.is_empty() {
                    code.push_str(&style_code);
                }
            }
        }

        // 3. 应用静态属性与绑定属性
        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    // 非 Styled 元素跳过 style 内联样式 + 归一化样式属性
                    if !is_styled
                        && (name == "style"
                            || crate::compiler::codegen::style_attr::is_style_attr(name))
                    {
                        continue;
                    }
                    code.push_str(&apply_static_attr(name, value));
                }
                Attribute::Bind { name, expr, .. } => {
                    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                    code.push_str(&apply_bind_attr(name, expr, &lv, &computed));
                }
                Attribute::Event { name, handler, .. } => {
                    code.push_str(&crate::compiler::event::apply_event(name, handler, ctx));
                }
            }
        }

        // 4. 处理子节点
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
                        0,
                        id_counter,
                        &child_loop_vars,
                        &child_parents,
                    )?;
                    let (else_code, else_is_iter) = gen_node_impl(
                        &Node::Element(else_clone),
                        ctx,
                        0,
                        id_counter,
                        &child_loop_vars,
                        &child_parents,
                    )?;

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
                gen_node_impl(child, ctx, 0, id_counter, &child_loop_vars, &child_parents)?;
            if is_iter {
                code.push_str(&format!("\n            .children({})", child_code));
            } else {
                code.push_str(&format!("\n            .child({})", child_code));
            }
            i += 1;
        }

        // 5. 处理 if / show 指令
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
            let cond_code = gen_expr_code(&cond, &lv, &computed);
            let cond_code = cond_code
                .strip_prefix('(')
                .and_then(|s| s.strip_suffix(')'))
                .map(|s| s.to_string())
                .unwrap_or(cond_code);
            code = format!("{}.when(!{}, |d| d.invisible())", code, cond_code);
        }

        // 6. 处理 each 指令
        if let Some(clause) = each_clause {
            let iter_expr = if loop_vars.iter().any(|lv| {
                clause.iterable == *lv || clause.iterable.starts_with(&format!("{}.", lv))
            }) {
                clause.iterable.clone()
            } else {
                format!(
                    "{}.{}",
                    crate::compiler::expr::current_self_alias().unwrap_or("self"),
                    clause.iterable
                )
            };
            let iter_code = format!(
                "{}.iter().map(|{}| {{\n                {}\n            }})",
                iter_expr, clause.item, code
            );
            return Ok((iter_code, true));
        }

        Ok((code, false))
    }

    /// AST → RML 源码
    pub fn print(elem: &Element, ctx: &PrinterCtx) -> Result<String, crate::compiler::translator::PrintError> {
        use crate::compiler::printer::print_children;
        use crate::parser::ast::TextSegment;

        let mut out = String::new();
        out.push_str(&ctx.indent_str());
        out.push('<');
        out.push_str(&elem.tag);

        for attr in &elem.attributes {
            match attr {
                Attribute::Static { name, value, .. } => {
                    out.push_str(&format!(
                        " {}=\"{}\"",
                        name,
                        crate::compiler::translator::utils::escape_attr_value(value)
                    ));
                }
                Attribute::Bind { name, expr, .. } => {
                    out.push_str(&format!(" {}={{{}}}", name, expr));
                }
                Attribute::Event { name, .. } => {
                    out.push_str(&format!(" {}=\"...\"", name));
                }
            }
        }

        // 指令（排除已在 codegen 中内部处理的 ref/key/html 等，保留 if/show/each/once/model）
        for d in &elem.directives {
            match d {
                Directive::If { expr, .. } => out.push_str(&format!(" if={{{}}}", expr)),
                Directive::Each { clause, .. } => {
                    out.push_str(&format!(
                        " each={{{} in {}}}",
                        clause.item, clause.iterable
                    ));
                    if let Some(idx) = &clause.index {
                        out.push_str(&format!(
                            " each={{{}, {} in {}}}",
                            clause.item, idx, clause.iterable
                        ));
                    }
                }
                Directive::Show { expr, .. } => out.push_str(&format!(" show={{{}}}", expr)),
                Directive::Model { field, converter, .. } => {
                    if let Some(conv) = converter {
                        out.push_str(&format!(" model={{{} | {}}}", field, conv));
                    } else {
                        out.push_str(&format!(" model={{{}}}", field));
                    }
                }
                Directive::Once { .. } => out.push_str(" once"),
                Directive::Html { expr, .. } => out.push_str(&format!(" html={{{}}}", expr)),
                Directive::Ref { name, .. } => out.push_str(&format!(" ref=\"{}\"", name)),
                Directive::Key { expr, .. } => out.push_str(&format!(" key={{{}}}", expr)),
                Directive::Else { .. } => {}
            }
        }

        let is_void = is_void_tag(&elem.tag);
        if elem.children.is_empty() && (ctx.self_closing || is_void) {
            out.push_str(" />");
            return Ok(out);
        }

        out.push('>');
        let child_ctx = ctx.indent();
        for child in &elem.children {
            match child {
                Node::Text(text) => {
                    out.push_str(&child_ctx.newline_indent());
                    out.push_str(text);
                }
                Node::Element(_) => {
                    out.push_str(&child_ctx.newline_indent());
                    out.push_str(&print_children(&[child.clone()], &ctx.registry, &child_ctx)?);
                }
                Node::Interpolation { expr, .. } => {
                    out.push_str(&child_ctx.newline_indent());
                    out.push_str(&format!("{{{}}}", expr));
                }
                Node::MixedText(segs) => {
                    out.push_str(&child_ctx.newline_indent());
                    for seg in segs {
                        match seg {
                            TextSegment::Literal(s) => out.push_str(s),
                            TextSegment::Interpolation { expr, .. } => out.push_str(&format!("{{{}}}", expr)),
                        }
                    }
                }
            }
        }
        out.push_str(&ctx.newline_indent());
        out.push_str("</");
        out.push_str(&elem.tag);
        out.push('>');
        Ok(out)
    }

    /// 判断原生标签是否为 void 元素
    fn is_void_tag(tag: &str) -> bool {
        matches!(tag, "input" | "img" | "br")
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
    let classes: Vec<String> = class_value.split_whitespace().map(|s| s.to_string()).collect();
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
