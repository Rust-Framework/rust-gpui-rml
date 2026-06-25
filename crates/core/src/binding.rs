//! 绑定路径与绑定上下文
//!
//! 编译期解析 `{user.name}` 等绑定表达式为 `BindingPath`，
//! 运行时通过 `IBindingContext` 建立订阅关系。
//! 详见文档 §3.6 绑定引擎原理。

/// 绑定路径段
#[derive(Debug, Clone, PartialEq)]
pub enum BindingSegment {
    /// ViewModel 字段
    Field(String),
    /// 嵌套字段访问（`a.b` 中的 `b`）
    Member(String),
    /// 索引访问（`items[0]`）
    Index(usize),
    /// 方法调用（`items.len()`）
    MethodCall(String),
}

/// 绑定路径，由编译期从 `{a.b.c}` 解析而来
#[derive(Debug, Clone, PartialEq)]
pub struct BindingPath {
    pub segments: Vec<BindingSegment>,
}

impl BindingPath {
    /// 从点分字符串创建绑定路径（`"user.name"` → `[Field("user"), Member("name")]`）
    pub fn parse(expr: &str) -> Self {
        let segments = expr
            .split('.')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .enumerate()
            .map(|(i, s)| {
                if i == 0 {
                    BindingSegment::Field(s)
                } else {
                    BindingSegment::Member(s)
                }
            })
            .collect();
        Self { segments }
    }

    /// 根路径字段名
    pub fn root_field(&self) -> Option<&str> {
        match self.segments.first()? {
            BindingSegment::Field(s) | BindingSegment::Member(s) => Some(s),
            _ => None,
        }
    }
}

/// 绑定上下文 trait（运行时由 View 持有，供绑定引擎使用）
///
/// MVP 阶段为标记 trait，阶段二扩展为完整的订阅管理接口。
pub trait IBindingContext {
    /// 标记绑定已建立
    fn rml_bind(&mut self, path: &BindingPath);
}
