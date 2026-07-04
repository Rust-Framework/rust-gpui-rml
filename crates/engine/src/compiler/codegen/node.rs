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
        Node::Element(elem) => gen_element(elem, ctx, depth, id_counter, loop_vars, parents),
        Node::Text(text) => Ok((format!("{:?}", text), false)),
        Node::Interpolation(expr_str) => {
            Ok((
                format!("format!(\"{{}}\", {})", gen_expr_code(expr_str, &lv, &computed)),
                false,
            ))
        }
        Node::MixedText(segments) => {
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
            Attribute::Static { name, value } if name == "class" => Some(value.clone()),
            _ => None,
        })
        .unwrap_or_default();
    let classes: Vec<String> = class_value.split_whitespace().map(|s| s.to_string()).collect();
    let id: Option<String> = elem
        .attributes
        .iter()
        .find_map(|attr| match attr {
            Attribute::Static { name, value } if name == "id" => Some(value.clone()),
            _ => None,
        });
    ParentInfo {
        tag: elem.tag.clone(),
        classes,
        id,
    }
}

fn gen_element(
    elem: &Element,
    ctx: &CodegenCtx,
    depth: usize,
    id_counter: &mut usize,
    loop_vars: &[String],
    parents: &[ParentInfo],
) -> Result<GenResult, CodegenError> {
    let tag = &elem.tag;

    // 透明容器：<component content={expr} /> 直接嵌入表达式，不创建元素包装。
    // 用于在 RML 模板中注入动态 AnyElement/impl IntoElement（类似 WPF ContentControl）。
    // 表达式可引用 _window/cx（render 方法作用域内可用）。
    // 支持 `each` 指令：<component each={s in status} content={s.render(_window, cx)} />
    if tag == "component" {
        let content_expr = elem.attributes.iter().find_map(|attr| {
            if let Attribute::Bind { name, expr } = attr {
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
            let lv: Vec<&str> = scope_vars;
            let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
            let code = crate::compiler::codegen::gen_expr_code(&expr, &lv, &computed);

            // 检测 each 指令
            let each_clause = elem.directives.iter().find_map(|d| match d {
                Directive::Each(c) => Some(c.clone()),
                _ => None,
            });
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
        });
    }

    // <slot> 占位符：组件模板内声明插槽渲染位置（Vue 风格 `<slot name="header" />`）。
    //
    // slot 字段类型为 `Option<SlotRenderer>`（`Box<dyn Fn(&mut Window, &mut App) -> AnyElement + Send + Sync>`），
    // codegen 调用闭包即时生成 element：
    //   `self.__rml_slot_<name>.as_ref().map(|f| f(window, cx)).unwrap_or(gpui::Empty)`
    //
    // 返回 is_iter=false（直接是 AnyElement，不需要 .children() 包裹）。
    // 无 name 属性的 `<slot />` 对应 "default" 插槽。
    if tag == "slot" {
        let slot_name = elem
            .attributes
            .iter()
            .find_map(|a| match a {
                Attribute::Static { name, value } if name == "name" => Some(value.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "default".to_string());
        return Ok((
            format!(
                "self.__rml_slot_{}.as_ref().map_or(gpui::Empty.into_any_element(), |f| f(_window, cx))",
                slot_name
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
        let code = comp::gen_component(elem, ctx, depth, id_counter, loop_vars)?;
        return Ok((code, false));
    }

    // 扩展组件（PascalCase、kebab-case 或特殊小写标签 menu/status-bar）
    if tags::is_extension_component(tag) {
        let code = comp::gen_component(elem, ctx, depth, id_counter, loop_vars)?;
        return Ok((code, false));
    }

    // model 指令：input/textarea 的双向绑定
    let model_field = elem.directives.iter().find_map(|d| match d {
        Directive::Model { field: f, .. } => Some(f.clone()),
        _ => None,
    });

    if model_field.is_some() && (tag == "input" || tag == "textarea") {
        let code = super::binding::gen_model_input(elem, ctx, id_counter, model_field.unwrap())?;
        return Ok((code, false));
    }

    let builtin = tags::lookup(tag).ok_or_else(|| CodegenError {
        message: format!("unknown tag: <{}>", tag),
    })?;

    let each_clause = elem.directives.iter().find_map(|d| match d {
        Directive::Each(c) => Some(c.clone()),
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

    // 1. 生成元素构造调用
    let mut code = String::from(builtin.codegen_ctor());

    // 2. ref 指令或事件处理器：生成 .id()
    let ref_name: Option<&str> = elem.directives.iter().find_map(|d| match d {
        Directive::Ref(name) => Some(name.as_str()),
        _ => None,
    });

    let has_events = elem
        .attributes
        .iter()
        .any(|attr| matches!(attr, Attribute::Event { .. }));

    if let Some(name) = ref_name {
        code.push_str(&format!(".id({:?})", format!("rml_ref:{}", name)));
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
            Attribute::Static { name, value } => {
                code.push_str(&apply_static_attr(name, value));
            }
            Attribute::Bind { name, expr } => {
                let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
                code.push_str(&apply_bind_attr(name, expr, &lv, &computed));
            }
            Attribute::Event { name, handler } => {
                code.push_str(&event::apply_event(name, handler, ctx));
            }
        }
    }

    // 4. 处理子节点（构建父链：当前元素 → child_parents）
    let current_parent = build_parent_info(elem);
    let mut child_parents = parents.to_vec();
    child_parents.push(current_parent);

    for child in &elem.children {
        let (child_code, is_iter) = gen_node_impl(child, ctx, depth + 1, id_counter, &child_loop_vars, &child_parents)?;
        if is_iter {
            code.push_str(&format!("\n            .children({})", child_code));
        } else {
            code.push_str(&format!("\n            .child({})", child_code));
        }
    }

    // 5. 处理 each 指令：将元素包装在迭代器中
    if let Some(clause) = each_clause {
        let iter_code = format!(
            "self.{}.iter().map(|{}| {{\n                {}\n            }})",
            clause.iterable, clause.item, code
        );
        return Ok((iter_code, true));
    }

    // 6. 处理 if/show 指令：条件渲染
    let cond: Option<String> = elem.directives.iter().find_map(|d| match d {
        Directive::If(c) => Some(c.clone()),
        Directive::Show(c) => Some(c.clone()),
        _ => None,
    });

    if let Some(cond) = cond {
        let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();
        let cond_code = gen_expr_code(&cond, &lv, &computed);
        let cond_code = cond_code
            .strip_prefix('(')
            .and_then(|s| s.strip_suffix(')'))
            .map(|s| s.to_string())
            .unwrap_or(cond_code);
        Ok((
            format!(
                "if {} {{ {}.into_any_element() }} else {{ gpui::Empty.into_any_element() }}",
                cond_code, code
            ),
            false,
        ))
    } else {
        Ok((code, false))
    }
}
