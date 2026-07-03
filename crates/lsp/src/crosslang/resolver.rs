//! 绑定路径解析器
//!
//! 将 RML 绑定表达式（如 `user.name`、`items[0]`、`count + 1`）解析为
//! 根标识符 + 成员访问链，供 coordinator 查询对应的 Rust 符号。

/// 绑定路径解析结果
#[derive(Debug, Clone)]
pub struct BindingPath {
    /// 根标识符（第一个 identifier token）
    pub root: String,
    /// 成员访问链（`.` 之后的标识符）
    pub members: Vec<String>,
}

/// 解析绑定表达式，提取根标识符和成员访问链
///
/// - `count` → `{ root: "count", members: [] }`
/// - `user.name` → `{ root: "user", members: ["name"] }`
/// - `items[0]` → `{ root: "items", members: [] }`（索引访问不加入 members）
/// - `count + 1` → `{ root: "count", members: [] }`（只取第一个标识符）
/// - `format!("...")` → `{ root: "format", members: [] }`
pub fn parse_binding_path(expr: &str) -> Option<BindingPath> {
    let trimmed = expr.trim();
    let root = extract_ident(trimmed)?;
    let rest = &trimmed[root.len()..];

    let mut members = Vec::new();
    let mut pos = 0;
    let bytes = rest.as_bytes();
    while pos < bytes.len() {
        // 跳过 `[...]` 索引访问
        if bytes[pos] == b'[' {
            if let Some(end) = rest[pos..].find(']') {
                pos += end + 1;
                continue;
            } else {
                break;
            }
        }
        // `.member` 访问
        if bytes[pos] == b'.' {
            pos += 1;
            if let Some(member) = extract_ident(&rest[pos..]) {
                members.push(member.clone());
                pos += member.len();
            } else {
                break;
            }
            continue;
        }
        // 其他字符（运算符、括号等）→ 停止
        break;
    }

    Some(BindingPath { root, members })
}

/// 从字符串开头提取第一个 identifier
fn extract_ident(s: &str) -> Option<String> {
    let mut ident = String::new();
    for ch in s.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else {
            break;
        }
    }
    if ident.is_empty() {
        None
    } else {
        Some(ident)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_ident() {
        let p = parse_binding_path("count").unwrap();
        assert_eq!(p.root, "count");
        assert!(p.members.is_empty());
    }

    #[test]
    fn dot_access() {
        let p = parse_binding_path("user.name").unwrap();
        assert_eq!(p.root, "user");
        assert_eq!(p.members, vec!["name"]);
    }

    #[test]
    fn chained_access() {
        let p = parse_binding_path("user.address.city").unwrap();
        assert_eq!(p.root, "user");
        assert_eq!(p.members, vec!["address", "city"]);
    }

    #[test]
    fn index_access() {
        let p = parse_binding_path("items[0]").unwrap();
        assert_eq!(p.root, "items");
        assert!(p.members.is_empty());
    }

    #[test]
    fn index_then_dot() {
        let p = parse_binding_path("items[0].name").unwrap();
        assert_eq!(p.root, "items");
        assert_eq!(p.members, vec!["name"]);
    }

    #[test]
    fn expression_only_root() {
        let p = parse_binding_path("count + 1").unwrap();
        assert_eq!(p.root, "count");
        assert!(p.members.is_empty());
    }

    #[test]
    fn empty_returns_none() {
        assert!(parse_binding_path("").is_none());
        assert!(parse_binding_path("  ").is_none());
    }
}
