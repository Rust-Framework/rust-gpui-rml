//! `#[command]` 实现
//!
//! Phase B-2：自动注入字段版本号 bump 与 `cx.notify()`，用户无需手写。
//! Phase B-3：支持 `no_notify` 参数，让用户控制 notify 时机。
//! Phase B-3：支持 `debounce = "100ms"` 参数，窗口内重复调用只触发一次。
//!
//! ## 行为
//!
//! - 用 `syn::visit::Visit` 遍历方法体，识别所有 `self.<ident> = ...` 和
//!   `self.<ident> += ...` 等赋值/复合赋值操作（完整覆盖 `=`/`+=`/`-=`/`*=`/`/=`
//!   /`%=`/`&=`/`|=`/`^=`/`<<=`/`>>=`）。
//! - 在每个包含字段修改的语句后注入 `self.__rml_bump_version("<field>");`。
//! - 若方法返回 `()` 且存在 `&mut Context<Self>` 参数，且未指定 `no_notify`，
//!   方法末尾追加 `<cx>.notify();`。
//! - 用户已写的 `cx.notify()` 不剥离（GPUI 多次 notify 幂等）。
//!
//! ## 参数
//!
//! - 无参数：默认行为（注入 notify）
//! - `no_notify`：不注入 notify（仍注入 bump_version）
//! - `debounce = "100ms"`：debounce 时间窗口（支持 ms/s 后缀），窗口内重复调用只触发一次
//!
//! ## 限制
//!
//! - 返回类型非 `()` 的方法不注入 notify（避免改变返回值类型）；用户需手动调用。
//! - 字段修改检测基于 AST 模式匹配，不追踪借用的指针间接修改（如 `let p = &mut self.x; *p = 1;`）。

use proc_macro2::TokenStream;
use quote::quote;
use syn::visit::Visit;
use syn::{
    parse_quote, BinOp, Expr, ExprAssign, ExprBinary, ExprField, ExprMethodCall, ExprPath, FnArg,
    Ident, ItemFn, LitStr, Member, Pat, ReturnType, Stmt, Token,
};

/// `#[command]` 参数
#[derive(Default)]
struct CommandArgs {
    no_notify: bool,
    /// debounce 时间窗口（毫秒）。None 表示不启用 debounce。
    debounce_ms: Option<u64>,
}

impl syn::parse::Parse for CommandArgs {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let mut args = CommandArgs::default();
        while !input.is_empty() {
            let ident: Ident = input.parse()?;
            match ident.to_string().as_str() {
                "no_notify" => args.no_notify = true,
                "debounce" => {
                    let _: Token![=] = input.parse()?;
                    let lit: LitStr = input.parse()?;
                    let s = lit.value();
                    args.debounce_ms = Some(parse_duration_ms(&s).ok_or_else(|| {
                        syn::Error::new(
                            lit.span(),
                            format!(
                                "invalid debounce duration: {:?} (expected like \"100ms\" or \"2s\")",
                                s
                            ),
                        )
                    })?);
                }
                other => {
                    return Err(syn::Error::new(
                        ident.span(),
                        format!("unknown #[command] argument: {}", other),
                    ))
                }
            }
            // 允许逗号分隔（可选）
            if input.peek(Token![,]) {
                let _: Token![,] = input.parse()?;
            }
        }
        Ok(args)
    }
}

/// `#[command]` 入口
pub fn expand(args: TokenStream, input: TokenStream) -> TokenStream {
    let cmd_args: CommandArgs = match syn::parse2(args) {
        Ok(a) => a,
        Err(e) => return e.to_compile_error(),
    };

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

    // 若指定了 debounce，在方法体开头注入时间窗口检查
    // 仅对返回 () 的方法生效（return; 需要 () 返回类型）
    // 实现说明：用函数局部 static AtomicU64 持久化上次调用时间戳。
    //   - `#[command]` 是方法级宏，无法向结构体注入字段；
    //   - 函数局部 static 跨调用持久化、天然 Send+Sync；
    //   - 代价：同一 ViewModel 类型的多个实例共享 debounce 状态（典型 UI 单窗口场景无影响）。
    if let Some(window_ms) = cmd_args.debounce_ms {
        if let ReturnType::Default = item.sig.output {
            let debounce_check: Stmt = parse_quote! {
                {
                    static __RML_DEBOUNCE_LAST: ::std::sync::atomic::AtomicU64 =
                        ::std::sync::atomic::AtomicU64::new(0);
                    let __rml_now: u64 = ::std::time::SystemTime::now()
                        .duration_since(::std::time::UNIX_EPOCH)
                        .map(|d| d.as_millis() as u64)
                        .unwrap_or(0);
                    let __rml_last: u64 = __RML_DEBOUNCE_LAST.load(::std::sync::atomic::Ordering::Relaxed);
                    __RML_DEBOUNCE_LAST.store(__rml_now, ::std::sync::atomic::Ordering::Relaxed);
                    if __rml_last != 0 && __rml_now >= __rml_last && __rml_now - __rml_last < #window_ms {
                        return;
                    }
                }
            };
            item.block.stmts.insert(0, debounce_check);
        }
    }

    // 若检测到字段修改且有 Context 参数且方法返回 () 且未指定 no_notify，末尾追加 cx.notify()
    // 返回类型非 () 时不注入（避免改变返回值类型，用户需手动调用）
    if !all_mutated_fields.is_empty() && !cmd_args.no_notify {
        if let Some(cx) = cx_ident {
            if let ReturnType::Default = item.sig.output {
                let notify: Stmt = parse_quote! {
                    #cx.notify();
                };
                item.block.stmts.push(notify);
            }
        }
    }

    // #[command] 方法经 RML 模板绑定调用（如 on-click={on_click}），编译器无法看到引用，
    // 标记 #[allow(dead_code)] 消除误报。字段在方法内被读取，也随之消除"never read"误报。
    item.attrs.push(parse_quote! { #[allow(dead_code)] });
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

    fn visit_expr_method_call(&mut self, node: &'ast ExprMethodCall) {
        let method = node.method.to_string();
        if matches!(method.as_str(), "push" | "pop" | "clear" | "extend" | "retain" | "truncate")
        {
            if let Some(name) = self_field_name(&node.receiver) {
                if !self.mutated_fields.contains(&name) {
                    self.mutated_fields.push(name);
                }
            }
        }
        syn::visit::visit_expr_method_call(self, node);
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

/// 解析 debounce 时间字符串为毫秒数
///
/// 支持后缀：
/// - `"100ms"` → 100
/// - `"2s"` → 2000
/// - `"500"` → 500（无后缀视为毫秒）
fn parse_duration_ms(s: &str) -> Option<u64> {
    let s = s.trim();
    if let Some(num) = s.strip_suffix("ms") {
        num.trim().parse().ok()
    } else if let Some(num) = s.strip_suffix('s') {
        num.trim().parse::<u64>().ok().map(|n| n * 1000)
    } else {
        s.parse().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_duration_ms_suffix() {
        assert_eq!(parse_duration_ms("100ms"), Some(100));
        assert_eq!(parse_duration_ms("2s"), Some(2000));
        assert_eq!(parse_duration_ms("500"), Some(500));
        assert_eq!(parse_duration_ms("1ms"), Some(1));
        assert_eq!(parse_duration_ms("0ms"), Some(0));
        assert_eq!(parse_duration_ms("0s"), Some(0));
    }

    #[test]
    fn parse_duration_ms_with_spaces() {
        assert_eq!(parse_duration_ms(" 100ms "), Some(100));
        assert_eq!(parse_duration_ms("2 s"), Some(2000));
        assert_eq!(parse_duration_ms("  500  "), Some(500));
    }

    #[test]
    fn parse_duration_ms_invalid() {
        assert_eq!(parse_duration_ms("abc"), None);
        assert_eq!(parse_duration_ms("ms"), None);
        assert_eq!(parse_duration_ms("-1ms"), None);
        assert_eq!(parse_duration_ms(""), None);
        assert_eq!(parse_duration_ms("s"), None);
    }

    #[test]
    fn command_args_default_no_debounce() {
        let args = CommandArgs::default();
        assert_eq!(args.debounce_ms, None);
        assert!(!args.no_notify);
    }

    #[test]
    fn command_args_parse_no_notify() {
        let args: CommandArgs = syn::parse_str("no_notify").unwrap();
        assert!(args.no_notify);
        assert_eq!(args.debounce_ms, None);
    }

    #[test]
    fn command_args_parse_debounce_ms() {
        let args: CommandArgs = syn::parse_str("debounce = \"100ms\"").unwrap();
        assert_eq!(args.debounce_ms, Some(100));
        assert!(!args.no_notify);
    }

    #[test]
    fn command_args_parse_debounce_seconds() {
        let args: CommandArgs = syn::parse_str("debounce = \"2s\"").unwrap();
        assert_eq!(args.debounce_ms, Some(2000));
    }

    #[test]
    fn command_args_parse_debounce_and_no_notify() {
        let args: CommandArgs = syn::parse_str("no_notify, debounce = \"50ms\"").unwrap();
        assert!(args.no_notify);
        assert_eq!(args.debounce_ms, Some(50));
    }

    #[test]
    fn command_args_parse_debounce_invalid_errors() {
        let result: syn::Result<CommandArgs> = syn::parse_str("debounce = \"abc\"");
        assert!(result.is_err());
    }

    #[test]
    fn command_args_parse_unknown_arg_errors() {
        let result: syn::Result<CommandArgs> = syn::parse_str("unknown_arg");
        assert!(result.is_err());
    }
}
