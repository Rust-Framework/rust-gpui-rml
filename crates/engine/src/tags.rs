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
    Code,
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
            // 标题：text_size + font_weight（h1=BOLD, h2-h6=SEMIBOLD）
            // 严格遵循 HTML 标准 H1 > H2 > H3 > H4 > H5 > H6
            // h1=32px / h2=24px / h3=20px / h4=18px / h5=16px / h6=14px
            BuiltinTag::H1 => "gpui::div().text_size(gpui::px(32.)).font_weight(gpui::FontWeight::BOLD)",
            BuiltinTag::H2 => "gpui::div().text_size(gpui::px(24.)).font_weight(gpui::FontWeight::SEMIBOLD)",
            BuiltinTag::H3 => "gpui::div().text_size(gpui::px(20.)).font_weight(gpui::FontWeight::SEMIBOLD)",
            BuiltinTag::H4 => "gpui::div().text_size(gpui::px(18.)).font_weight(gpui::FontWeight::SEMIBOLD)",
            BuiltinTag::H5 => "gpui::div().text_size(gpui::px(16.)).font_weight(gpui::FontWeight::SEMIBOLD)",
            BuiltinTag::H6 => "gpui::div().text_size(gpui::px(14.)).font_weight(gpui::FontWeight::SEMIBOLD)",
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
            // <code>：内联代码，等宽字体
            BuiltinTag::Code => "gpui::div()",
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
            BuiltinTag::H2 => 24.0,
            BuiltinTag::H3 => 20.0,
            BuiltinTag::H4 => 18.0,
            BuiltinTag::H5 => 16.0,
            BuiltinTag::H6 => 14.0,
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
    m.insert("code", BuiltinTag::Code);
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
/// 已是 PascalCase 或无连字符的标签原样返回。
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
        // 小写无连字符别名（normalize_component_tag 不会转为 PascalCase，需手动映射）
        "accordion" => "Accordion".to_string(),
        "item" => "AccordionItem".to_string(),
        "tab" => "Tab".to_string(),
        "tabs" => "Tabs".to_string(),
        "table" => "Table".to_string(),
        "column" => "Column".to_string(),
        "popover" => "Popover".to_string(),
        "descriptions" => "DescriptionList".to_string(),
        "description" => "DescriptionItem".to_string(),
        "separator" => "DescriptionSeparator".to_string(),
        "breadcrumb" => "Breadcrumb".to_string(),
        // kebab-case 形式（tab-bar / tab-item）由 normalize_component_tag 自动转为 PascalCase
        _ => normalized,
    }
}

/// 查询扩展组件：先原始标签，再 kebab-case 规范化后查询。
pub fn component_lookup_resolved(tag: &str) -> Option<ComponentTag> {
    component_lookup(tag).or_else(|| component_lookup(&normalize_component_tag(tag)))
}

/// 判断标签是否为扩展组件（PascalCase 或 kebab-case，不含特殊小写 `menu`/`accordion`）
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

/// 扩展组件中的 lowercase 标签（如 `menu`、`accordion`），在 `component_lookup` 中注册
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

/// 返回组件在 CSS 选择器中可隐式匹配的类名
///
/// RML 组件标签（如 `<Card>`、`<button-group>`）在 CSS 匹配时被视为带有与其
/// 小写标签名相同的 class，使 `.card`、`.button-group` 等类选择器可直接命中
/// 对应组件，无需在每个组件上显式写 `class="card"`。
pub fn implicit_class_for(tag: &str) -> Option<String> {
    if is_extension_component(tag) {
        Some(tag.to_ascii_lowercase())
    } else {
        None
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  根节点标记：`<window>` / `<modern-window>` / `<tab-window>` / `<dialog>` / `<component>`
//
//  RML 根节点必须是这几种之一。编译器从根节点属性提取窗口/对话框配置，
//  生成 `impl IWindow`（仅 `<window>`/`<modern-window>`/`<tab-window>`）
//  或对话框方法（`<dialog>`）+ `impl Render`。
//  这些不是普通 HTML 标签，不参与 `BuiltinTag` 查找。
// ──────────────────────────────────────────────────────────────────────────

/// RML 根节点类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootTag {
    /// `<window>`：基础窗口（透明标题栏，`WindowChrome::Transparent`）
    Window,
    /// `<modern-window>`：现代窗口（自绘 TitleBar/Menu/StatusBar）
    ModernWindow,
    /// `<tab-window>`：TabBar 标题栏 + 可调整插槽高级窗口
    TabWindow,
    /// `<dialog>`：模态对话框（非独立 OS 窗口，依赖父窗口的 Root 层渲染）
    ///
    /// 复用 `#[window]` 宏标注的结构体（获得 `__rml_state.window_handle` 字段），
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
        "window" | "modern-window" | "tab-window" | "dialog" | "component"
    )
}

/// 查找根节点类型
pub fn root_tag_lookup(tag: &str) -> Option<RootTag> {
    match tag {
        "window" => Some(RootTag::Window),
        "modern-window" => Some(RootTag::ModernWindow),
        "tab-window" => Some(RootTag::TabWindow),
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
    /// 有状态组件：构造调用形如 `Input::new(&entity)`
    ///
    /// 配合 `ref="name"` 指令，codegen 生成 `__rml_state.get_or_init_ref(...)` 调用，
    /// 惰性创建 `Entity<T>` 并缓存到 `RmlState.ref_entities`，无需用户在 `on_loaded`
    /// 中手动创建 entity。同时宏侧生成 `__rml_populate_refs()`，将 entity 注入
    /// 用户声明的 `ElementRef<T>` 字段（字段名需与 ref name 一致）。
    ///
    /// - `state_field`：默认字段名（无 ref 时回退使用 `self.<state_field>.as_ref().expect(...)`，
    ///   兼容用户自定义 `Option<Entity<T>>` + 手动 `on_loaded` 初始化的旧用法）
    /// - `state_ctor`：state 构造闭包表达式字符串，签名 `(window, cx) -> T`。
    ///   闭包返回类型推断 T，避免要求所有 state 类型构造函数签名一致
    ///   （`InputState::new(w, c)` / `SliderState::new()` / `TreeState::new(c)` 三者签名不同，
    ///   闭包适配器统一为 `|w, c| ...`）。
    Stateful {
        state_field: &'static str,
        state_ctor: &'static str,
    },
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
    /// 是否为容器组件（实现 `ParentElement`，支持 `.child(...)` 接收元素子节点）。
    ///
    /// `true`：codegen 将元素子节点生成为 `.child(...)` / `.children(...)` 调用。
    /// `false`：仅支持单个文本子节点作为 label，元素子节点被拒绝。
    pub container: bool,
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
            container: false,
        }),
        // Alert：variant 关联函数 + message 构造器参数，委托到 compiler/alert 专属处理
        // PascalCase: <Alert>，小写别名: <alert>（参考 Accordion 模式）
        "Alert" | "alert" => Some(ComponentTag {
            ctor_path: "rml_ui::Alert",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "ButtonGroup" => Some(ComponentTag {
            ctor_path: "rml_ui::ButtonGroup",
            kind: ComponentKind::Stateless,
            container: true,
        }),
        "Badge" => Some(ComponentTag {
            ctor_path: "rml_ui::Badge",
            kind: ComponentKind::StatelessNoId,
            container: true,
        }),
        "Checkbox" => Some(ComponentTag {
            ctor_path: "rml_ui::Checkbox",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "Label" => Some(ComponentTag {
            ctor_path: "rml_ui::Label",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "Separator" => Some(ComponentTag {
            ctor_path: "rml_ui::Separator",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        // DescriptionList：无 ElementId 容器，子节点为 <description>/<separator>
        "DescriptionList" | "descriptions" => Some(ComponentTag {
            ctor_path: "rml_ui::DescriptionList",
            kind: ComponentKind::StatelessWithItems,
            container: false,
        }),
        "Tag" => Some(ComponentTag {
            ctor_path: "rml_ui::Tag",
            kind: ComponentKind::Stateless,
            container: true,
        }),
        "Progress" => Some(ComponentTag {
            ctor_path: "rml_ui::Progress",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "ProgressCircle" => Some(ComponentTag {
            ctor_path: "rml_ui::ProgressCircle",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "Slider" => Some(ComponentTag {
            ctor_path: "rml_ui::Slider",
            kind: ComponentKind::Stateful {
                state_field: "slider_state",
                state_ctor: "|_w, _c| rml_ui::SliderState::new()",
            },
            container: false,
        }),
        "Switch" => Some(ComponentTag {
            ctor_path: "rml_ui::Switch",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "Input" => Some(ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "input_state",
                state_ctor: "|w, c| rml_ui::InputState::new(w, c)",
            },
            container: false,
        }),
        "TextInput" => Some(ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "input_state",
                state_ctor: "|w, c| rml_ui::InputState::new(w, c)",
            },
            container: false,
        }),
        // CodeEditor：基于 Input 的代码编辑器，自动应用 mono 字体 + 默认高度 360px
        // 声明 h-full 可让编辑器填满父容器（如 LSP 编辑器工作区）
        // 字段必须为 Option<Entity<InputState>>，在 on_loaded 中延迟初始化
        "CodeEditor" => Some(ComponentTag {
            ctor_path: "rml_ui::Input",
            kind: ComponentKind::Stateful {
                state_field: "editor_state",
                state_ctor: "|w, c| rml_ui::InputState::new(w, c)",
            },
            container: false,
        }),
        // 窗口外壳组件（RenderOnce，无 ElementId 参数）
        // TitleBar / NativeStatusBar 来自 gpui-component，供用户手动组装标题栏/状态栏
        // 注：ModernWindowShell 不在此路由表中——它是 `<modern-window>` 根元素的内部实现，
        // 由 codegen 直接生成包裹代码，不作为用户可用的 `<ModernWindowShell>` 标签
        "TitleBar" => Some(ComponentTag {
            ctor_path: "rml_ui::TitleBar",
            kind: ComponentKind::StatelessNoId,
            container: true,
        }),
        // gpui-component 原生状态栏容器（手动 .left() / .right() 组装）
        // PascalCase: <NativeStatusBar>，kebab-case: <native-status-bar>
        "NativeStatusBar" | "native-status-bar" => Some(ComponentTag {
            ctor_path: "rml_ui::NativeStatusBar",
            kind: ComponentKind::StatelessNoId,
            container: true,
        }),
        "ActivityBar" => Some(ComponentTag {
            ctor_path: "rml_ui::ActivityBar",
            kind: ComponentKind::EntityRef,
            container: false,
        }),
        "Tree" => Some(ComponentTag {
            ctor_path: "rml_ui::Tree",
            kind: ComponentKind::Stateful {
                state_field: "tree_state",
                state_ctor: "|_w, c| rml_ui::TreeState::new(c)",
            },
            container: false,
        }),
        // MVVM / 声明式菜单栏（ui crate MenuBar；声明式由 compiler/menu/ 生成 children）
        "MenuBar" | "menu-bar" => Some(ComponentTag {
            ctor_path: "rml_ui::MenuBar",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        "menu" => Some(ComponentTag {
            ctor_path: "rml_ui::MenuBar",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        // Accordion：闭包式 builder，子节点为 <AccordionItem> / <item>
        "Accordion" | "accordion" => Some(ComponentTag {
            ctor_path: "rml_ui::Accordion",
            kind: ComponentKind::StatelessWithItems,
            container: false,
        }),
        // Avatar：无参构造 RenderOnce 叶子组件（无 ParentElement，无 .label()）
        "Avatar" => Some(ComponentTag {
            ctor_path: "rml_ui::Avatar",
            kind: ComponentKind::StatelessNoId,
            container: false,
        }),
        // AvatarGroup：无参构造 RenderOnce 容器（.child(Avatar)）
        "AvatarGroup" => Some(ComponentTag {
            ctor_path: "rml_ui::AvatarGroup",
            kind: ComponentKind::StatelessNoId,
            container: true,
        }),
        // Breadcrumb：无参构造 RenderOnce 叶子组件（.items(Vec<BreadcrumbItem>)）
        // PascalCase: <Breadcrumb>，kebab-case: <breadcrumb>
        "Breadcrumb" | "breadcrumb" => Some(ComponentTag {
            ctor_path: "rml_ui::Breadcrumb",
            kind: ComponentKind::StatelessNoId,
            container: false,
        }),
        // Card：Ant Design 风格卡片容器，需 id 支持 hoverable 悬浮效果
        "Card" => Some(ComponentTag {
            ctor_path: "rml_ui::Card",
            kind: ComponentKind::Stateless,
            container: true,
        }),
        // Tabs：WPF TabControl 风格标签容器，header + body 切换
        // PascalCase: <Tabs>，kebab-case: <tabs>
        "Tabs" | "tabs" => Some(ComponentTag {
            ctor_path: "rml_ui::Tabs",
            kind: ComponentKind::StatelessWithItems,
            container: false,
        }),
        // TabBar：原生 gpui-component 形态标签栏（纯 header，无 body）
        // PascalCase: <TabBar>，kebab-case: <tab-bar>
        "TabBar" | "tab-bar" => Some(ComponentTag {
            ctor_path: "rml_ui::TabBar",
            kind: ComponentKind::StatelessWithItems,
            container: false,
        }),
        // Icon：RenderOnce 无 ElementId，构造器接受 IconName 或 path 字符串
        // 由专属 compiler/icon 模块处理 name/path 属性提取
        "Icon" => Some(ComponentTag {
            ctor_path: "rml_ui::Icon",
            kind: ComponentKind::StatelessNoId,
            container: false,
        }),
        // Kbd：RenderOnce 无 ElementId，构造器接受 Keystroke
        // 由专属 compiler/kbd 模块处理 key 属性提取（Keystroke::parse）
        "Kbd" => Some(ComponentTag {
            ctor_path: "rml_ui::Kbd",
            kind: ComponentKind::StatelessNoId,
            container: false,
        }),
        // Table：WPF DataGrid 风格声明式表格，子节点为 <Column> / <template slot="...">
        "Table" | "table" => Some(ComponentTag {
            ctor_path: "rml_ui::Table",
            kind: ComponentKind::StatelessWithItems,
            container: false,
        }),
        // Popover：浮动气泡容器，子节点通过 slot="trigger" 路由到 .trigger()，
        // 其余子节点作为 content 注入 .child()。anchor/mouse_button/appearance 等
        // 专用属性由 compiler/popover 模块处理。
        "Popover" | "popover" => Some(ComponentTag {
            ctor_path: "rml_ui::Popover",
            kind: ComponentKind::StatelessWithItems,
            container: false,
        }),
        // Phase 1 基础无状态组件
        // Spinner：RenderOnce 无 ElementId，.icon(impl Into<Icon>)/.color(Hsla)，Sizable
        "Spinner" => Some(ComponentTag {
            ctor_path: "rml_ui::Spinner",
            kind: ComponentKind::StatelessNoId,
            container: false,
        }),
        // Skeleton：RenderOnce 无 ElementId，.secondary()，Styled
        "Skeleton" => Some(ComponentTag {
            ctor_path: "rml_ui::Skeleton",
            kind: ComponentKind::StatelessNoId,
            container: false,
        }),
        // Link：构造器 Link::new(id)，.href()/.disabled()/.on_click()，ParentElement 容器
        "Link" => Some(ComponentTag {
            ctor_path: "rml_ui::Link",
            kind: ComponentKind::Stateless,
            container: true,
        }),
        // Collapsible：RenderOnce 无 ElementId，.open(bool)/.content(element)，ParentElement 容器
        "Collapsible" => Some(ComponentTag {
            ctor_path: "rml_ui::Collapsible",
            kind: ComponentKind::StatelessNoId,
            container: true,
        }),
        // GroupBox：RenderOnce 无 ElementId，.title(element)/variant(.normal/.fill/.outline)，ParentElement 容器
        "GroupBox" => Some(ComponentTag {
            ctor_path: "rml_ui::GroupBox",
            kind: ComponentKind::StatelessNoId,
            container: true,
        }),
        // Pagination：构造器 Pagination::new(id)，.current_page(usize)/.total_pages(usize)/.on_click(&usize)
        "Pagination" => Some(ComponentTag {
            ctor_path: "rml_ui::Pagination",
            kind: ComponentKind::Stateless,
            container: false,
        }),
        // Radio：构造器 Radio::new(id)，.label()/.checked()/.disabled()/.on_click(&bool)，ParentElement 容器
        "Radio" => Some(ComponentTag {
            ctor_path: "rml_ui::Radio",
            kind: ComponentKind::Stateless,
            container: true,
        }),
        // RadioGroup：构造器 RadioGroup::vertical(id)/horizontal(id)（new 为私有），
        // .selected_index(Option<usize>)/.disabled(bool)/.on_click(&usize)
        // 子节点为 <Radio>，通过 .child(impl Into<Radio>) 注入
        // 委托到 compiler/radio_group 专属处理（与 Separator/Tag 模式一致）
        "RadioGroup" | "radio-group" => Some(ComponentTag {
            ctor_path: "rml_ui::RadioGroup",
            kind: ComponentKind::Stateless,
            container: true,
        }),
        _ => None,
    }
}

/// 判断标签是否为 `StatelessWithItems` 组件的子项 builder
///
/// Accordion 支持三种形式：`AccordionItem`（PascalCase）、`item`（短标签）、`accordion-item`（kebab-case）。
/// Tabs/TabBar 共用 `Tab`（PascalCase）、`tab`（短标签）两种子项形式。
/// 仅在 `<accordion>`/`<tabs>`/`<tab-bar>` 内合法，不在 `component_lookup` 中注册
/// （避免被误用为顶层扩展组件），在 validator 和 codegen 中通过此函数识别。
///
/// 注：`<tab-item>` 已弃用并移除——RML 架构保持干净整洁，统一用 `<tab>` 即可。
pub fn is_item_builder_tag(tag: &str) -> bool {
    matches!(
        tag,
        "AccordionItem" | "item" | "Tab" | "tab" | "Column" | "column"
            | "DescriptionItem" | "description" | "DescriptionSeparator" | "separator"
    ) || normalize_component_tag(tag) == "AccordionItem"
        || normalize_component_tag(tag) == "Tab"
        || normalize_component_tag(tag) == "Column"
        || normalize_component_tag(tag) == "DescriptionItem"
        || normalize_component_tag(tag) == "DescriptionSeparator"
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
    }

    #[test]
    fn kebab_normalizes_to_pascal() {
        assert_eq!(normalize_component_tag("status-bar"), "StatusBar");
        assert_eq!(normalize_component_tag("tab-bar"), "TabBar");
        assert_eq!(normalize_component_tag("native-status-bar"), "NativeStatusBar");
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
    fn canonical_tag_kebab_normalizes_to_pascal() {
        // kebab-case tag 由 normalize_component_tag 自动转 PascalCase
        assert_eq!(canonical_tag("tab-bar"), "TabBar");
        assert_eq!(canonical_tag("status-bar"), "StatusBar");
        assert_eq!(canonical_tag("native-status-bar"), "NativeStatusBar");
        assert_eq!(canonical_tag("tab-item"), "TabItem");
    }

    #[test]
    fn canonical_tag_preserves_special_lowercase() {
        // menu 是唯一保留的小写无连字符别名
        assert_eq!(canonical_tag("menu"), "menu");
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
    fn component_lookup_tabs() {
        let tag = component_lookup("Tabs").expect("Tabs should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::Tabs");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
    }

    #[test]
    fn component_lookup_tabs_kebab() {
        // kebab-case <tabs> 直接命中 component_lookup
        let tag = component_lookup("tabs").expect("tabs should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::Tabs");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
        assert!(component_lookup_resolved("tabs").is_some());
    }

    #[test]
    fn component_lookup_tab_bar() {
        let tag = component_lookup("TabBar").expect("TabBar should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::TabBar");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
    }

    #[test]
    fn component_lookup_tab_bar_kebab() {
        // kebab-case <tab-bar> 直接命中 component_lookup
        let tag = component_lookup("tab-bar").expect("tab-bar should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::TabBar");
        assert_eq!(tag.kind, ComponentKind::StatelessWithItems);
        assert!(component_lookup_resolved("tab-bar").is_some());
        // tab-bar 含连字符，不属于 special_lowercase，但仍是 extension_component
        assert!(!is_special_lowercase_component("tab-bar"));
        assert!(is_extension_component("tab-bar"));
    }

    #[test]
    fn component_lookup_native_status_bar_kebab() {
        // <native-status-bar> → rml_ui::NativeStatusBar（gpui-component 原生）
        let tag = component_lookup("native-status-bar").expect("native-status-bar should be registered");
        assert_eq!(tag.ctor_path, "rml_ui::NativeStatusBar");
        assert_eq!(tag.kind, ComponentKind::StatelessNoId);
        let tag_pascal = component_lookup("NativeStatusBar").expect("NativeStatusBar should be registered");
        assert_eq!(tag_pascal.ctor_path, "rml_ui::NativeStatusBar");
    }

    #[test]
    fn canonical_tag_maps_tab_bar_kebab() {
        assert_eq!(canonical_tag("tabs"), "Tabs");
        assert_eq!(canonical_tag("tab-bar"), "TabBar");
        assert_eq!(canonical_tag("tab"), "Tab");
        assert_eq!(canonical_tag("Tabs"), "Tabs");
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
