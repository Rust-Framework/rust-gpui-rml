//! `#[command]` 实现
//!
//! Phase B-2：自动注入字段版本号 bump 与 `cx.notify()`，用户无需手写。
//!
//! ## 行为
//!
//! - 用 `syn::visit::Visit` 遍历方法体，识别所有 `self.<ident> = ...` 和
//!   `self.<ident> += ...` 等赋值/复合赋值操作（完整覆盖 `=`/`+=`/`-=`/`*=`/`/=`
//!   /`%=`/`&=`/`|=`/`^=`/`<<=`/`>>=`）。
//! - 在每个包含字段修改的语句后注入 `self.__rml_bump_version("<field>");`。
//! - 若方法返回 `()` 且存在 `&mut Context<Self>` 参数，方法末尾追加 `<cx>.notify();`。
//! - 用户已写的 `cx.notify()` 不剥离（GPUI 多次 notify 幂等）。
//!
//! ## 限制
//!
//! - 返回类型非 `()` 的方法不注入 notify（避免改变返回值类型）；用户需手动调用。
//! - 字段修改检测基于 AST 模式匹配，不追踪借用的指针间接修改（如 `let p = &mut self.x; *p = 1;`）。

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;
use syn::{
    parse_quote, BinOp, Expr, ExprAssign, ExprBinary, ExprField, ExprPath, FnArg, ItemFn, LitStr,
    Member, Pat, ReturnType, Stmt,
};

/// `#[command]` 入口
pub fn expand(input: TokenStream) -> TokenStream {
    let mut item: ItemFn = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };

    // 校验：必须是方法（&self 或 &mut self 作为第一个参数）
    let has_self = item
        .sig
        .inputs
        .iter()
        .any(|arg| matches!(arg, FnArg::Receiver(_)));
    if !has_self {
        return syn::Error::new_spanned(
            &item.sig,
            "#[command] methods must take &self or &mut self as first parameter",
        )
        .to_compile_error();
    }

    // 提取 &mut Context<Self> 参数名（通常为 cx）
    let cx_ident = extract_context_param(&item.sig.inputs);

    // 处理方法体：在每个修改 self.<field> 的语句后注入 bump_version
    let mut all_mutated_fields: Vec<String> = Vec::new();
    let mut new_stmts: Vec<Stmt> = Vec::new();
    for stmt in item.block.stmts.drain(..) {
        // 用 Visitor 检测该语句内的所有字段修改
        let mut visitor = FieldMutationVisitor::default();
        visitor.visit_stmt(&stmt);

        // 保留原语句
        new_stmts.push(stmt);

        // 为每个修改的字段注入 bump_version 调用
        for field in &visitor.mutated_fields {
            if !all_mutated_fields.contains(field) {
                all_mutated_fields.push(field.clone());
            }
            let field_lit = LitStr::new(field, proc_macro2::Span::call_site());
            let bump: Stmt = parse_quote! {
                self.__rml_bump_version(#field_lit);
            };
            new_stmts.push(bump);
        }
    }
    item.block.stmts = new_stmts;

    // 若检测到字段修改且有 Context 参数且方法返回 ()，末尾追加 cx.notify()
    // 返回类型非 () 时不注入（避免改变返回值类型，用户需手动调用）
    if !all_mutated_fields.is_empty() {
        if let Some(cx) = cx_ident {
            if let ReturnType::Default = item.sig.output {
                let notify: Stmt = parse_quote! {
                    #cx.notify();
                };
                item.block.stmts.push(notify);
            }
        }
    }

    quote! { #item }
}

/// `#[command]` 方法体字段修改访问器
///
/// 检测 `self.<ident> = ...` 和 `self.<ident> += ...` 等赋值/复合赋值操作。
/// 通过 `visit_expr_assign` 和 `visit_expr_assign_op` 钩子捕获，
/// Visitor 自动递归进入 if/while/for 等嵌套块。
#[derive(Default)]
struct FieldMutationVisitor {
    mutated_fields: Vec<String>,
}

impl<'ast> Visit<'ast> for FieldMutationVisitor {
    fn visit_expr_assign(&mut self, node: &'ast ExprAssign) {
        // self.<ident> = ...
        if let Some(name) = self_field_name(&node.left) {
            if !self.mutated_fields.contains(&name) {
                self.mutated_fields.push(name);
            }
        }
        syn::visit::visit_expr_assign(self, node);
    }

    fn visit_expr_binary(&mut self, node: &'ast ExprBinary) {
        // syn 2.x：复合赋值（+=, -=, *= 等）统一为 Expr::Binary，通过 BinOp 变体区分
        if is_compound_assign_op(&node.op) {
            if let Some(name) = self_field_name(&node.left) {
                if !self.mutated_fields.contains(&name) {
                    self.mutated_fields.push(name);
                }
            }
        }
        syn::visit::visit_expr_binary(self, node);
    }
}

/// 判断 `BinOp` 是否为复合赋值运算符（+=, -=, *=, /=, %=, ^=, &=, |=, <<=, >>=）
fn is_compound_assign_op(op: &BinOp) -> bool {
    matches!(
        op,
        BinOp::AddAssign(_)
            | BinOp::SubAssign(_)
            | BinOp::MulAssign(_)
            | BinOp::DivAssign(_)
            | BinOp::RemAssign(_)
            | BinOp::BitXorAssign(_)
            | BinOp::BitAndAssign(_)
            | BinOp::BitOrAssign(_)
            | BinOp::ShlAssign(_)
            | BinOp::ShrAssign(_)
    )
}

/// 检测表达式是否为 `self.<ident>` 模式，返回字段名
fn self_field_name(expr: &Expr) -> Option<String> {
    let Expr::Field(field) = expr else {
        return None;
    };
    self_field_name_from_expr_field(field)
}

/// 从 `ExprField` 提取 `self.<ident>` 的字段名
fn self_field_name_from_expr_field(field: &ExprField) -> Option<String> {
    // base 必须是 `self` 标识符
    let Expr::Path(ExprPath { path, .. }) = &*field.base else {
        return None;
    };
    if !path.is_ident("self") {
        return None;
    }
    // member 必须是命名标识符（不支持 self.0 这类数字索引）
    if let Member::Named(ident) = &field.member {
        Some(ident.to_string())
    } else {
        None
    }
}

/// 从方法参数中提取 `&mut Context<Self>` 类型的参数名
///
/// 约定：参数形如 `cx: &mut Context<Self>`，提取 `cx` 标识符。
/// 找不到时返回 None（不注入 notify）。
fn extract_context_param(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> Option<syn::Ident> {
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = arg {
            let ty_str = quote!(#pat_type.ty).to_string();
            // 匹配 Context 类型（含 gpui::Context 或裸 Context）
            if ty_str.contains("Context") {
                if let Pat::Ident(pat_ident) = &*pat_type.pat {
                    return Some(pat_ident.ident.clone());
                }
            }
        }
    }
    None
}

// ──────────────────────────────────────────────────────────────────────────
//  以下函数保留供未来元信息生成使用（当前未使用，标记 #[allow(dead_code)]）
// ──────────────────────────────────────────────────────────────────────────

/// 从方法参数中提取事件类型名
///
/// 约定：最后一个非 Context 引用参数是事件对象（如 `&ClickEvent`）
#[allow(dead_code)]
fn extract_event_type(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> String {
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = arg {
            let ty_str = quote!(#pat_type.ty).to_string();
            if ty_str.contains("Context") {
                continue;
            }
            if let syn::Type::Reference(type_ref) = pat_type.ty.as_ref() {
                let inner = &type_ref.elem;
                let inner_str = quote!(#inner).to_string();
                return inner_str
                    .split("::")
                    .last()
                    .unwrap_or(&inner_str)
                    .trim()
                    .to_string();
            }
        }
    }
    String::new()
}

/// 提取命令参数（除 self、事件、Context 外的参数）
#[allow(dead_code)]
fn extract_params(
    inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>,
) -> Vec<(String, String)> {
    let mut params = Vec::new();
    for arg in inputs.iter() {
        if let FnArg::Typed(pat_type) = arg {
            let ty_str = quote!(#pat_type.ty).to_string();
            if ty_str.contains("Context") {
                continue;
            }
            if let syn::Type::Reference(_) = pat_type.ty.as_ref() {
                continue;
            }
            if let Pat::Ident(pat_ident) = pat_type.pat.as_ref() {
                let name = pat_ident.ident.to_string();
                let ty = ty_str.trim().to_string();
                params.push((name, ty));
            }
        }
    }
    params
}
