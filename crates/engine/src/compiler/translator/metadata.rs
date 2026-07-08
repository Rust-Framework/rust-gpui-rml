//! Translator 设计时与校验元数据
//!
//! 为每个 RML 标签提供统一的描述信息，供 validator、LSP 补全、可视化设计器使用。

/// 组件分类
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ComponentCategory {
    /// 布局容器（div、flex、grid 等）
    Layout,
    /// 表单控件（input、button、checkbox 等）
    Form,
    /// 数据展示（table、tree、description-list 等）
    Data,
    /// 反馈组件（alert、progress、popover 等）
    Feedback,
    /// 导航组件（tabs、breadcrumb、menu 等）
    Navigation,
    /// 通用容器组件（card、group-box 等）
    Container,
    /// 原子/基础组件（text、icon、separator 等）
    Primitive,
    /// 根节点（window / modern-window / tab-window / dialog / component）
    Root,
    /// 用户自定义组件
    User,
    /// 未分类
    Uncategorized,
}

/// 单个 translator 的元数据
#[derive(Debug, Clone)]
pub struct TranslatorMetadata {
    /// 该 translator 处理的 canonical 标签名
    pub tag: &'static str,
    /// 显示名称（用于设计器面板）
    pub display_name: &'static str,
    /// 组件分类
    pub category: ComponentCategory,
    /// 是否为容器（可包含子元素）
    pub is_container: bool,
    /// 可接受的直接子标签白名单；空切片表示接受任意合法子节点
    pub allowed_children: &'static [&'static str],
    /// 可接受的 slot 名白名单（仅对 shell 根节点/容器有意义）
    pub allowed_slots: &'static [&'static str],
    /// 拖拽放入时的默认占位子节点标签名
    pub default_child: Option<&'static str>,
    /// 默认属性列表（属性名 → 默认值 RML 字符串）
    pub default_attrs: &'static [(&'static str, &'static str)],
    /// Stateful 组件需要在 ViewModel 中预置的 state/entity 字段名提示
    pub state_field_hint: Option<&'static str>,
    /// EntityRef 组件是否需要 `ref="name"` 才能工作
    pub requires_ref: bool,
    /// 是否为根节点
    pub is_root: bool,
    /// 该组件是否由用户自定义（#[component] 标注）
    pub is_user: bool,
}

impl TranslatorMetadata {
    /// 创建最小元数据，其他字段取默认值
    pub const fn new(tag: &'static str, display_name: &'static str, category: ComponentCategory) -> Self {
        Self {
            tag,
            display_name,
            category,
            is_container: false,
            allowed_children: &[],
            allowed_slots: &[],
            default_child: None,
            default_attrs: &[],
            state_field_hint: None,
            requires_ref: false,
            is_root: false,
            is_user: false,
        }
    }

    /// 链式设置 is_container
    pub const fn container(mut self, value: bool) -> Self {
        self.is_container = value;
        self
    }

    /// 链式设置 allowed_children
    pub const fn children(mut self, value: &'static [&'static str]) -> Self {
        self.allowed_children = value;
        self
    }

    /// 链式设置 allowed_slots
    pub const fn slots(mut self, value: &'static [&'static str]) -> Self {
        self.allowed_slots = value;
        self
    }

    /// 链式设置 default_child
    pub const fn default_child(mut self, value: &'static str) -> Self {
        self.default_child = Some(value);
        self
    }

    /// 链式设置 default_attrs
    pub const fn attrs(mut self, value: &'static [(&'static str, &'static str)]) -> Self {
        self.default_attrs = value;
        self
    }

    /// 链式设置 state_field_hint
    pub const fn state(mut self, value: &'static str) -> Self {
        self.state_field_hint = Some(value);
        self
    }

    /// 链式设置 requires_ref
    pub const fn requires_ref(mut self, value: bool) -> Self {
        self.requires_ref = value;
        self
    }

    /// 链式设置 is_root
    pub const fn root(mut self, value: bool) -> Self {
        self.is_root = value;
        self
    }

    /// 链式设置 is_user
    pub const fn user(mut self, value: bool) -> Self {
        self.is_user = value;
        self
    }
}
