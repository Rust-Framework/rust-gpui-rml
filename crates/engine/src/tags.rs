//! HTML 标签到 GPUI 元素构造调用的映射表
//!
//! 详见文档 §2.2 标签映射。

use std::collections::HashMap;
use std::sync::OnceLock;

/// 内置 HTML 标签枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BuiltinTag {
    Div,
    Span,
    P,
    H1,
    H2,
    H3,
    H4,
    H5,
    H6,
    Button,
    Input,
    TextArea,
    Ul,
    Ol,
    Li,
    Img,
    A,
    Label,
    Br,
}

impl BuiltinTag {
    /// 返回该标签在 GPUI 中的构造调用代码（作为字符串）
    ///
    /// 注意：调用链可能用到 `Styled` trait 的方法（如 `text_size`/`flex`/`flex_col`），
    /// codegen 需在生成代码顶部 `use gpui::Styled;` 才能编译。
    pub fn codegen_ctor(self) -> &'static str {
        match self {
            // 容器类
            BuiltinTag::Div => "gpui::div()",
            BuiltinTag::Span => "gpui::div()",
            BuiltinTag::P => "gpui::div().text_sm().text_color(rml_core::theme::color(\"--text-muted\"))",
            // 标题：直接 text_size 设置 px 大小（紧凑现代默认值）
            // h1=32px / h2=18px / h3=24px / h4=20px / h5=18px / h6=16px
            BuiltinTag::H1 => "gpui::div().text_size(gpui::px(32.))",
            BuiltinTag::H2 => "gpui::div().text_size(gpui::px(18.))",
            BuiltinTag::H3 => "gpui::div().text_size(gpui::px(24.))",
            BuiltinTag::H4 => "gpui::div().text_size(gpui::px(20.))",
            BuiltinTag::H5 => "gpui::div().text_size(gpui::px(18.))",
            BuiltinTag::H6 => "gpui::div().text_size(gpui::px(16.))",
            // 表单类：原生轨简化为 div + class（扩展轨用 <Button>/<Input>）
            BuiltinTag::Button => "gpui::div()",
            BuiltinTag::Input => "gpui::div()",
            BuiltinTag::TextArea => "gpui::div()",
            // 列表：默认垂直排列
            BuiltinTag::Ul => "gpui::div().flex().flex_col()",
            BuiltinTag::Ol => "gpui::div().flex().flex_col()",
            BuiltinTag::Li => "gpui::div()",
            // 其他
            BuiltinTag::Img => "gpui::div()",
            BuiltinTag::A => "gpui::div()",
            BuiltinTag::Label => "gpui::div()",
            // <br>：用 hidden() 产生零尺寸占位
            BuiltinTag::Br => "gpui::div().hidden()",
        }
    }

    /// 是否为自闭合标签
    pub fn is_self_closing(self) -> bool {
        matches!(self, BuiltinTag::Input | BuiltinTag::Img | BuiltinTag::Br)
    }

    /// 标签文本大小（仅 h1~h6 有意义，其他返回 0.0）
    pub fn text_size(self) -> f32 {
        match self {
            BuiltinTag::H1 => 32.0,
            BuiltinTag::H2 => 18.0,
            BuiltinTag::H3 => 24.0,
            BuiltinTag::H4 => 20.0,
            BuiltinTag::H5 => 18.0,
            BuiltinTag::H6 => 16.0,
            _ => 0.0,
        }
    }
}

static TAG_MAP: OnceLock<HashMap<&'static str, BuiltinTag>> = OnceLock::new();

fn build_tag_map() -> HashMap<&'static str, BuiltinTag> {
    let mut m = HashMap::new();
    m.insert("div", BuiltinTag::Div);
    m.insert("span", BuiltinTag::Span);
    m.insert("p", BuiltinTag::P);
    m.insert("h1", BuiltinTag::H1);
    m.insert("h2", BuiltinTag::H2);
    m.insert("h3", BuiltinTag::H3);
    m.insert("h4", BuiltinTag::H4);
    m.insert("h5", BuiltinTag::H5);
    m.insert("h6", BuiltinTag::H6);
    m.insert("button", BuiltinTag::Button);
    m.insert("input", BuiltinTag::Input);
    m.insert("textarea", BuiltinTag::TextArea);
    m.insert("ul", BuiltinTag::Ul);
    m.insert("ol", BuiltinTag::Ol);
    m.insert("li", BuiltinTag::Li);
    m.insert("img", BuiltinTag::Img);
    m.insert("a", BuiltinTag::A);
    m.insert("label", BuiltinTag::Label);
    m.insert("br", BuiltinTag::Br);
    m
}

/// 查找标签名对应的 `BuiltinTag`
pub fn lookup(tag: &str) -> Option<BuiltinTag> {
    TAG_MAP.get_or_init(build_tag_map).get(tag).copied()
}

/// 判断标签是否为内置 HTML 标签（小写）
pub fn is_builtin(tag: &str) -> bool {
    lookup(tag).is_some()
}

/// 将 kebab-case 扩展组件标签规范化为 PascalCase。
///
/// 通用规则：`context-menu` → `ContextMenu`，`menu-item` → `MenuItem`。
/// 已是 PascalCase、snake_case（`status_bar`）或无连字符的标签原样返回。
pub fn normalize_component_tag(tag: &str) -> String {
    if !tag.contains('-') {
        return tag.to_string();
    }
    tag.split('-')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => {
                    let mut s = String::new();
                    s.extend(first.to_uppercase());
                    s.push_str(chars.as_str());
                    s
                }
            }
        })
        .collect()
}

/// 将标签规范化为组件属性注册表中的标准名（PascalCase）。
///
/// 在 `normalize_component_tag`（kebab-case → PascalCase）基础上，额外处理小写别名：
/// `accordion` → `Accordion`、`item` → `AccordionItem`。
/// 供 `props_registry` 的属性查询使用，使 `<accordion>` / `<item>` / `<accordion-item>`
/// 都能命中 `COMPONENT_PROPS` 中以 PascalCase 登记的条目，无需重复登记。
pub fn canonical_tag(tag: &str) -> String {
    let normalized = normalize_component_tag(tag);
    match normalized.as_str() {
        "accordion" => "Accordion".to_string(),
        "item" => "AccordionItem".to_string(),
        "tab_bar" => "TabBar".to_string(),
        "tab" => "Tab".to_string(),
        "table" => "Table".to_string(),
        "column" => "Column".to_string(),
        _ => normalized,
    }
}

/// 查询扩展组件：先原始标签，再 kebab-case 规范化后查询。
pub fn component_lookup_resolved(tag: &str) -> Option<ComponentTag> {
    component_lookup(tag).or_else(|| component_lookup(&normalize_component_tag(tag)))
}

/// 判断标签是否为扩展组件（PascalCase 或 kebab-case，不含特殊小写 `menu`/`status_bar`）
pub fn is_component(tag: &str) -> bool {
    if is_special_lowercase_component(tag) {
        return false;
    }
    let normalized = normalize_component_tag(tag);
    normalized
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false)
        && component_lookup(&normalized).is_some()
}

/// 扩展组件中的 lowercase 标签（如 `menu`、`status_bar`、`accordion`），在 `component_lookup` 中注册
pub fn is_special_lowercase_component(tag: &str) -> bool {
    component_lookup(tag).is_some() && !tag.contains('-') && {
        !tag
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false)
    }
}

/// 是否为扩展轨组件（含 PascalCase、kebab-case、特殊小写）
pub fn is_extension_component(tag: &str) -> bool {
    is_component(tag) || is_special_lowercase_component(tag)
}

// ──────────────────────────────────────────────────────────────────────────
//  根节点标记：`<window>` / `<modern_window>` / `<tab_window>` / `<dialog>` / `<component>`
//
//  RML 根节点必须是这几种之一。编译器从根节点属性提取窗口/对话框配置，
//  生成 `impl IWindow`（仅 `<window>`/`<modern_window>`/`<tab_window>`）
//  或对话框方法（`<dialog>`）+ `impl Render`。
//  这些不是普通 HTML 标签，不参与 `BuiltinTag` 查找。
// ──────────────────────────────────────────────────────────────────────────

/// RML 根节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootTag {
    /// `<window>`：基础窗口（透明标题栏，`WindowChrome::Transparent`）
    Window,
    /// `<modern_window>`：现代窗口（自绘 TitleBar/Menu/StatusBar）
    ModernWindow,
    /// `<tab_window>`：TabBar 标题栏 + 可调整插槽高级窗口
    TabWindow,
    /// `<dialog>`：模态对话框（非独立 OS 窗口，依赖父窗口的 Root 层渲染）
    ///
    /// 复用 `#[window]` 宏标注的结构体（获得 `__rml_window_handle` 字段），
    /// 但 codegen 不生成 `impl IWindow`，而是生成 `open(window, cx)` / `close(cx)`
    /// 方法，封装 gpui-component 的 `Dialog` 组件。
    DialogWindow,
    /// `<component>`：可复用组件（无窗口操作）
    Component,
}

/// 判断标签是否为 RML 根节点标记
pub fn is_root_tag(tag: &str) -> bool {
    matches!(
        tag,
        "window" | "modern_window" | "tab_window" | "dialog" | "component"
    )
}

/// 查找根节点类型
pub fn root_tag_lookup(tag: &str) -> Option<RootTag> {
    match tag {
        "window" => Some(RootTag::Window),
        "modern_window" => Some(RootTag::ModernWindow),
        "tab_window" => Some(RootTag::TabWindow),
        "dialog" => Some(RootTag::DialogWindow),
        "component" => Some(RootTag::Component),
        _ => None,
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  扩展组件：gpui-component 路由表（双轨制组件策略的「扩展轨」）
//
//  当 .rml 中出现 PascalCase 标签（如 <Button>），codegen 会查询本表，
//  若命中则生成 `rml_ui::<Type>::new(...)` 调用，否则报「未知组件」错误。
// 详见开发规划 §2.5 Layer 5。
// ──────────────────────────────────────────────────────────────────────────

/// 扩展组件的构造模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentKind {
    /// 无状态组件：构造调用形如 `Button::new(id)`
    /// `id: impl Into<ElementId>` — 由 codegen 自动分配 `("rml_el", N)` 元组
    Stateless,
    /// 无状态无参组件：构造调用形如 `TitleBar::new()` / `StatusBar::new()`
    /// 用于无 `ElementId` 参数的 RenderOnce 组件
    StatelessNoId,
    /// 有状态组件：构造调用形如 `Input::new(&self.<field>)`
    /// 需要视图中持有对应 state entity 字段（如 `Entity<InputState>`）
    Stateful { state_field: &'static str },
    /// Entity 引用组件：从 Host 的 `Entity<T>` 字段直接 clone
    /// 配合 `ref="field_name"` 指令指定字段名
    /// 生成 `self.<field>.as_ref().expect("init in on_loaded").clone()`
    EntityRef,
    /// 无状态组件，子节点通过 `.item(|item| ...)` 闭包式 builder 注入。
    ///
    /// 构造调用形如 `Accordion::new(id)`，与 `Stateless` 一致；
    /// 但子节点处理不同：每个 `<AccordionItem>` 子节点生成
    /// `.item(|__rml_item: rml_ui::AccordionItem| __rml_item.<setters>.child(...))`。
    /// 子节点 tag 名（如 `AccordionItem`）由 codegen 在 `StatelessWithItems` 分支
    /// 通过 `is_item_builder_tag` 识别。
    StatelessWithItems,
}

/// 扩展组件的元信息
#[derive(Debug, Clone, Copy)]
pub struct ComponentTag {
    /// 类型路径，如 `rml_ui::Button`
    pub ctor_path: &'static str,
    pub kind: ComponentKind,
}

/// 查询扩展组件元信息（仅查内置 gpui-component 路由表）
///
/// 注意：用户自定义组件（`@component` 标注的 struct）由 codegen 在另一条路径处理，
/// 不在本表查询范围内。
pub fn component_lookup(tag: &str) -> Option<ComponentTag> {
    match tag {
        "Button" => Some(ComponentTag {
            ctor_path: "rml_ui::Button",
            kind: ComponentKind::Stateless,
        }),
        "ButtonGroup" => Some(ComponentTag {
            ctor_path: "rml_ui::ButtonGroup",
            kind: ComponentKind::Stateless,
        }),
        "Badge" => Some(ComponentTag {
            ctor_path: "rml_ui::Badge",
            kind: ComponentKind::Stateless,
        }),
        "Checkbox" => Some(ComponentTag {
            ctor_path: "rml_ui::Checkbox",
            kind: ComponentKind::Stateless,
        }),
        "Label" => Some(ComponentTag {
            ctor_path: "rml_ui::Label",
            kind: ComponentKind::Stateless,
        }),
        "Separator" => Some(ComponentTag {
            ctor_path: "rml_ui::Separator",
            kind: ComponentKind::Stateless,
        }),
        "Tag" => Some(ComponentTag {
            ctor_path: "rml_ui::Tag",
            kind: ComponentKind::Stateless,
        }),
        "Progress" => Some(ComponentTag {
            ctor_path: "rml_ui::Progress",
            kind: ComponentKind::Stateless,
        }),
        "ProgressCircle" => Some(ComponentTag {
            ctor_path: "rml_ui::ProgressCircle",
            kind: ComponentKind::Stateless,
        }),
        "Slider" => Some(ComponentTag {
            ctor_path: "rml_ui::Slider",
            kind: ComponentKind::Stateless,
        }),
        "Switch" => Some(ComponentTag {
            ctor_path: "rml_ui::Switch",
            kind: ComponentKind::Stateless,
        }),
        "Input" => Some(ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "input_state",
            },
        }),
        "TextInput" => Some(ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "input_state",
            },
        }),
        // CodeEditor：基于 Input 的代码编辑器，自动应用 mono 字体 + size_full
        // 字段必须为 Option<Entity<InputState>>，在 on_loaded 中延迟初始化
        "CodeEditor" => Some(ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "editor_state",
            },
        }),
        // 窗口外壳组件（RenderOnce，无 ElementId 参数）
        // TitleBar / NativeStatusBar 来自 gpui-component，供用户手动组装标题栏/状态栏
        // 注：ModernWindowShell 不在此路由表中——它是 `<modern_window>` 根元素的内部实现，
        // 由 codegen 直接生成包裹代码，不作为用户可用的 `<ModernWindowShell>` 标签
        "TitleBar" => Some(ComponentTag {
            ctor_path: "rml_ui::TitleBar",
            kind: ComponentKind::StatelessNoId,
        }),
        // gpui-component 原生状态栏容器（手动 .left() / .right() 组装）
        "NativeStatusBar" | "StatusBar" => Some(ComponentTag {
            ctor_path: "rml_ui::NativeStatusBar",
            kind: ComponentKind::StatelessNoId,
        }),
        "ActivityBar" => Some(ComponentTag {
            ctor_path: "rml_ui::ActivityBar",
            kind: ComponentKind::EntityRef,
        }),
        "Tree" => Some(ComponentTag {
            ctor_path: "rml_ui::Tree",
            kind: ComponentKind::Stateful {
                state_field: "tree_state",
            },
        }),
        // MVVM / 声明式菜单栏（ui crate MenuBar；声明式由 compiler/menu/ 生成 children）
        "MenuBar" => Some(ComponentTag {
            ctor_path: "rml_ui::MenuBar",
            kind: ComponentKind::Stateless,
        }),
        "menu" => Some(ComponentTag {
            ctor_path: "rml_ui::MenuBar",
            kind: ComponentKind::Stateless,
        }),
        "status_bar" => Some(ComponentTag {
            ctor_path: "rml_ui::StatusBar",
            kind: ComponentKind::StatelessNoId,
        }),
        // Accordion：闭包式 builder，子节点为 <AccordionItem> / <item>
        "Accordion" | "accordion" => Some(ComponentTag {
            ctor_path: "rml_ui::Accordion",
            kind: ComponentKind::StatelessWithItems,
        }),
        // Avatar：无参构造 RenderOnce 叶子组件（无 ParentElement，无 .label()）
        "Avatar" => Some(ComponentTag {
            ctor_path: "rml_ui::Avatar",
            kind: ComponentKind::StatelessNoId,
        }),
        // AvatarGroup：无参构造 RenderOnce 容器（.child(Avatar)）
        "AvatarGroup" => Some(ComponentTag {
            ctor_path: "rml_ui::AvatarGroup",
            kind: ComponentKind::StatelessNoId,
        }),
        // Card：Ant Design 风格卡片容器，需 id 支持 hoverable 悬浮效果
        "Card" => Some(ComponentTag {
            ctor_path: "rml_ui::Card",
            kind: ComponentKind::Stateless,
        }),
        // TabBar：标签栏容器，子节点为 <Tab>（直接 .child(Tab::new()...)，非闭包）
        "TabBar" | "tab_bar" => Some(ComponentTag {
            ctor_path: "rml_ui::TabBar",
            kind: ComponentKind::StatelessWithItems,
        }),
        // Table：WPF DataGrid 风格声明式表格，子节点为 <Column> / <template slot="...">
        "Table" | "table" => Some(ComponentTag {
            ctor_path: "rml_ui::Table",
            kind: ComponentKind::StatelessWithItems,
        }),
        _ => None,
    }
}

/// 判断标签是否为 `StatelessWithItems` 组件的子项 builder
///
/// Accordion 支持三种形式：`AccordionItem`（PascalCase）、`item`（短标签）、`accordion-item`（kebab-case）。
/// TabBar 支持三种形式：`Tab`（PascalCase）、`tab`（短标签）、`tab`（已是短形式）。
/// 仅在 `<accordion>`/`<tab_bar>` 内合法，不在 `component_lookup` 中注册
/// （避免被误用为顶层扩展组件），在 validator 和 codegen 中通过此函数识别。
pub fn is_item_builder_tag(tag: &str) -> bool {
    matches!(
        tag,
        "AccordionItem" | "item" | "Tab" | "tab" | "Column" | "column"
    ) || normalize_component_tag(tag) == "AccordionItem"
        || normalize_component_tag(tag) == "Tab"
        || normalize_component_tag(tag) == "Column"
}

#[cfg(test)]
mod normalize_tests {
    use super::*;

    #[test]
    fn kebab_to_pascal() {
        assert_eq!(normalize_component_tag("context-menu"), "ContextMenu");
        assert_eq!(normalize_component_tag("dropdown-menu"), "DropdownMenu");
        assert_eq!(normalize_component_tag("menu-bar"), "MenuBar");
        assert_eq!(normalize_component_tag("menu-item"), "MenuItem");
        assert_eq!(normalize_component_tag("menu-separator"), "MenuSeparator");
        assert_eq!(normalize_component_tag("app-menu-bar"), "AppMenuBar");
    }

    #[test]
    fn passthrough_unchanged() {
        assert_eq!(normalize_component_tag("Button"), "Button");
        assert_eq!(normalize_component_tag("menu"), "menu");
        assert_eq!(normalize_component_tag("status_bar"), "status_bar");
    }

    #[test]
    fn menu_bar_registered_but_menu_codegen_takes_priority() {
        assert!(component_lookup_resolved("MenuBar").is_some());
        assert_eq!(
            component_lookup_resolved("MenuBar").unwrap().ctor_path,
            "rml_ui::MenuBar"
        );
        // gen_element 仍优先走 is_menu_container → compiler/menu/（处理 menu-item 子节点）
    }

    #[test]
    fn canonical_tag_maps_lowercase_aliases() {
        assert_eq!(canonical_tag("accordion"), "Accordion");
        assert_eq!(canonical_tag("item"), "AccordionItem");
    }

    #[test]
    fn canonical_tag_passthrough_pascal_and_kebab() {
        assert_eq!(canonical_tag("Accordion"), "Accordion");
        assert_eq!(canonical_tag("AccordionItem"), "AccordionItem");
        assert_eq!(canonical_tag("accordion-item"), "AccordionItem");
        assert_eq!(canonical_tag("Button"), "Button");
    }

    #[test]
    fn canonical_tag_preserves_special_lowercase() {
        assert_eq!(canonical_tag("menu"), "menu");
        assert_eq!(canonical_tag("status_bar"), "status_bar");
    }

    #[test]
    fn is_item_builder_tag_matches_all_forms() {
        assert!(is_item_builder_tag("AccordionItem"));
        assert!(is_item_builder_tag("item"));
        assert!(is_item_builder_tag("accordion-item"));
        assert!(!is_item_builder_tag("Accordion"));
        assert!(!is_item_builder_tag("div"));
    }

    #[test]
    fn component_lookup_accordion_lowercase() {
        let tag = component_lookup("accordion").expect("accordion should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::Accordion");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
        assert!(component_lookup_resolved("accordion").is_some());
        assert!(is_special_lowercase_component("accordion"));
        assert!(is_extension_component("accordion"));
    }

    #[test]
    fn component_lookup_tab_bar() {
        let tag = component_lookup("TabBar").expect("TabBar should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::TabBar");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
    }

    #[test]
    fn component_lookup_tab_bar_lowercase() {
        let tag = component_lookup("tab_bar").expect("tab_bar should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::TabBar");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
        assert!(component_lookup_resolved("tab_bar").is_some());
        assert!(is_special_lowercase_component("tab_bar"));
        assert!(is_extension_component("tab_bar"));
    }

    #[test]
    fn canonical_tag_maps_tab_bar_lowercase() {
        assert_eq!(canonical_tag("tab_bar"), "TabBar");
        assert_eq!(canonical_tag("tab"), "Tab");
        assert_eq!(canonical_tag("TabBar"), "TabBar");
        assert_eq!(canonical_tag("Tab"), "Tab");
    }

    #[test]
    fn is_item_builder_tab_matches_all_forms() {
        assert!(is_item_builder_tag("Tab"));
        assert!(is_item_builder_tag("tab"));
        assert!(!is_item_builder_tag("TabBar"));
        assert!(!is_item_builder_tag("div"));
    }
}
