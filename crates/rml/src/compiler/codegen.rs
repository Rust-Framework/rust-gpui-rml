//! 代码生成器
//!
//! 将 AST 转换为 Rust 源码字符串（`impl Render for <View>`）。
//! 详见文档 §10.6 代码生成。

use crate::compiler::CodegenCtx;
use crate::parser::ast::{Attribute, Directive, Element, EventHandler, Node, TextSegment};
use crate::tags;
use std::fmt;

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

/// 生成 `impl Render for <ViewStruct>` 代码块
pub fn codegen(root: &Node, ctx: &CodegenCtx) -> Result<String, CodegenError> {
    let mut out = String::new();
    let view_name = &ctx.view_struct_name;

    out.push_str(&format!(
        "impl gpui::Render for {} {{\n",
        view_name
    ));
    out.push_str(
        "    fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement {\n",
    );

    // 生成根元素构建代码
    let root_expr = gen_node(root, ctx, 0)?;
    out.push_str(&format!("        {}\n", root_expr));
    out.push_str("    }\n");
    out.push_str("}\n");

    Ok(out)
}

/// 为单个节点生成构建代码，返回一个表达式字符串
fn gen_node(node: &Node, ctx: &CodegenCtx, depth: usize) -> Result<String, CodegenError> {
    match node {
        Node::Element(elem) => gen_element(elem, ctx, depth),
        Node::Text(text) => Ok(format!(
            "gpui::div().child(gpui::Label::new({:?}))",
            text
        )),
        Node::Interpolation(expr) => {
            // 简单插值：直接读取 self.<expr>
            // 假设 expr 是字段名或方法调用
            Ok(format!(
                "gpui::div().child(gpui::Label::new(format!(\"{{}}\", self.{})))",
                expr
            ))
        }
        Node::MixedText(segments) => Ok(gen_mixed_text(segments)),
    }
}

fn gen_element(elem: &Element, ctx: &CodegenCtx, depth: usize) -> Result<String, CodegenError> {
    // 处理指令（if/each/else 在父级处理，此处跳过）
    // Phase A 简化：不处理 if/each，仅生成基础元素

    let tag = &elem.tag;

    // 判断内置标签还是自定义组件
    if tags::is_component(tag) {
        // 自定义组件（Phase B 完整实现）
        return Err(CodegenError {
            message: format!(
                "custom component <{}> not supported in Phase A",
                tag
            ),
        });
    }

    let builtin = tags::lookup(tag).ok_or_else(|| CodegenError {
        message: format!("unknown tag: <{}>", tag),
    })?;

    // 生成元素构造调用
    let mut code = String::from(builtin.codegen_ctor());

    // 应用静态属性与绑定属性
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                code.push_str(&apply_static_attr(name, value, builtin));
            }
            Attribute::Bind { name, expr } => {
                code.push_str(&apply_bind_attr(name, expr, builtin));
            }
            Attribute::Event { name, handler } => {
                code.push_str(&apply_event(name, handler, ctx));
            }
        }
    }

    // 处理指令
    for d in &elem.directives {
        match d {
            Directive::If(cond) => {
                // 简化：用 .when() 包装
                code.push_str(&format!(".when(self.{}, |el| el)", cond));
            }
            Directive::Show(cond) => {
                code.push_str(&format!(".when(self.{}, |el| el)", cond));
            }
            Directive::Model(field) => {
                // Phase A 简化：仅生成 value 绑定（不含 on_change 回写）
                code.push_str(&format!(
                    ".child(gpui::Label::new(format!(\"{{}}\", self.{})))",
                    field
                ));
            }
            Directive::Once | Directive::Else | Directive::Each(_) | Directive::Key(_)
            | Directive::Html(_) | Directive::Ref(_) | Directive::Slot(_) => {
                // Phase B 实现
            }
        }
    }

    // 处理子节点
    for child in &elem.children {
        let child_code = gen_node(child, ctx, depth + 1)?;
        code.push_str(&format!("\n            .child({})", child_code));
    }

    Ok(code)
}

fn apply_static_attr(name: &str, value: &str, _tag: tags::BuiltinTag) -> String {
    match name {
        "class" => format!(".class({:?})", value),
        "id" => format!(".id({:?})", value),
        "style" => String::new(), // Phase B
        "type" => format!(".child(gpui::Label::new({:?}))", value), // 简化
        _ => format!(".child(gpui::Label::new({:?}))", format!("{}={}", name, value)),
    }
}

fn apply_bind_attr(name: &str, expr: &str, _tag: tags::BuiltinTag) -> String {
    match name {
        "class" => format!(".class(format!(\"{{}}\", self.{}))", expr),
        "value" => format!(".child(gpui::Label::new(format!(\"{{}}\", self.{})))", expr),
        "disabled" => format!(".when(self.{}, |el| el)", expr),
        "checked" => format!(".when(self.{}, |el| el)", expr),
        _ => format!(".child(gpui::Label::new(format!(\"{{}}\", self.{})))", expr),
    }
}

fn apply_event(name: &str, handler: &EventHandler, _ctx: &CodegenCtx) -> String {
    let event_method = match name {
        "onclick" => "on_click",
        "oninput" => "on_input",
        "onchange" => "on_change",
        "onkeydown" => "on_key_down",
        "onkeyup" => "on_key_up",
        "onsubmit" => "on_submit",
        "onfocus" => "on_focus",
        "onblur" => "on_blur",
        _ => "on_click", // 默认
    };

    match handler {
        EventHandler::Ident(method) => {
            format!(
                ".{}(cx.listener(move |this, _ev: &gpui::ClickEvent, cx| {{ this.{}(&_ev.into(), cx); }}))",
                event_method, method
            )
        }
        EventHandler::MethodName(method) => {
            format!(
                ".{}(cx.listener(move |this, _ev: &gpui::ClickEvent, cx| {{ this.{}(&_ev.into(), cx); }}))",
                event_method, method
            )
        }
        EventHandler::WithArgs(method, args) => {
            // 简化：仅支持单参数
            if args.is_empty() {
                format!(
                    ".{}(cx.listener(move |this, _ev: &gpui::ClickEvent, cx| {{ this.{}(&_ev.into(), cx); }}))",
                    event_method, method
                )
            } else {
                let arg = &args[0];
                format!(
                    ".{}(cx.listener(move |this, _ev: &gpui::ClickEvent, cx| {{ let p0 = {}.clone(); this.{}(p0, &_ev.into(), cx); }}))",
                    event_method, arg, method
                )
            }
        }
    }
}

fn gen_mixed_text(segments: &[TextSegment]) -> String {
    // 构建 format! 调用
    let mut fmt_str = String::new();
    let mut args = Vec::new();
    for seg in segments {
        match seg {
            TextSegment::Literal(s) => {
                fmt_str.push_str(&s.replace('{', "{{").replace('}', "}}"));
            }
            TextSegment::Interpolation(expr) => {
                fmt_str.push_str("{}");
                args.push(format!("self.{}", expr));
            }
        }
    }
    if args.is_empty() {
        format!("gpui::div().child(gpui::Label::new({:?}))", fmt_str)
    } else {
        format!(
            "gpui::div().child(gpui::Label::new(format!({:?}, {})))",
            fmt_str,
            args.join(", ")
        )
    }
}
