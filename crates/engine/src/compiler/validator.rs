//! 语义验证器
//!
//! Phase A：仅校验语法合法性（不校验 ViewModel 字段类型）。

use crate::parser::ast::{Directive, Element, Node};
use std::fmt;

#[derive(Debug, Clone)]
pub struct ValidationError {
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Validation error: {}", self.message)
    }
}

impl std::error::Error for ValidationError {}

/// 校验 AST 合法性
pub fn validate(node: &Node) -> Result<(), ValidationError> {
    validate_node(node, &mut ValidationCtx::default())
}

#[derive(Default)]
struct ValidationCtx {
    ref_names: std::collections::HashSet<String>,
}

fn validate_node(node: &Node, ctx: &mut ValidationCtx) -> Result<(), ValidationError> {
    match node {
        Node::Element(elem) => validate_element(elem, ctx),
        Node::Text(_) | Node::Interpolation(_) | Node::MixedText(_) => Ok(()),
    }
}

fn validate_element(elem: &Element, ctx: &mut ValidationCtx) -> Result<(), ValidationError> {
    // 校验指令
    let mut has_model = false;

    for d in &elem.directives {
        match d {
            Directive::Model(_) => has_model = true,
            Directive::Ref(name) => {
                if !ctx.ref_names.insert(name.clone()) {
                    return Err(ValidationError {
                        message: format!("duplicate ref name: {}", name),
                    });
                }
            }
            // Phase B-1：允许 if/each/else/once/html/key/slot/show 通过校验
            // Phase B-2 会补全 else 必须紧跟 if 的语义校验、each 子句校验等
            Directive::If(_) | Directive::Each(_) | Directive::Else | Directive::Once
            | Directive::Html(_) | Directive::Key(_) | Directive::Slot(_)
            | Directive::Show(_) => {}
        }
    }

    // model 只能用于 input/textarea
    if has_model {
        let tag = elem.tag.as_str();
        if tag != "input" && tag != "textarea" {
            return Err(ValidationError {
                message: format!("`model` directive can only be used on <input>/<textarea>, got <{}>", tag),
            });
        }
    }

    // 递归校验子节点
    for child in &elem.children {
        validate_node(child, ctx)?;
    }

    Ok(())
}
