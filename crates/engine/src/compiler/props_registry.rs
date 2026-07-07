//! 组件属性映射注册表（框架开发规范）
//!
//! ## 目的
//!
//! RML 框架在 .rml → Rust 代码翻译（codegen）时，需要把 .rml 属性映射到
//! gpui-component 组件的 builder 方法。本注册表作为**单一信源**，列出每个
//! 组件支持的所有属性，确保 codegen 翻译齐全，避免属性被静默丢弃。
//!
//! ## 维护规则
//!
//! 添加新组件或新属性时，**必须同步**：
//! 1. 在本文件 `COMPONENT_PROPS` / `SHELL_PROPS` 中登记
//! 2. 在 `component_bind_setter` / `component_static_setter` / `component_event_setter`
//!    或 `shell.rs` 的 bind 属性处理中添加对应 match 分支
//! 3. 运行 `cargo test -p rust-rml-engine --test props_registry_complete` 验证一致性
//!
//! ## 属性分类
//!
//! - **static**：静态属性 `label="..."` / `primary=""`，由 `component_static_setter` 处理
//! - **bind**：绑定属性 `value={field}`，由 `component_bind_setter` 处理
//! - **event**：事件属性（声明式 `on-click={fn}` kebab-case，normalize 后内部 `on_click` snake_case），
//!   由 `component_event_setter` 处理
//! - **shell**：窗口外壳属性 `title="..."` / `tabs={...}`，由 `shell.rs` 处理
//!
//! 注册表合并所有分类，按组件列出完整属性清单。条目统一使用 normalize 后的 snake_case 形式。

// ──────────────────────────────────────────────────────────────────────────
// 通用属性（所有 Stateless / Stateful 组件共享）
// ──────────────────────────────────────────────────────────────────────────

/// 通用静态属性（来自 `component_static_setter` 的通用 match 分支）
///
/// 这些属性对所有 Stateless/Stateful 组件生效，不区分 tag。
pub const COMMON_STATIC_PROPS: &[&str] = &[
    // 文本类
    "label", "placeholder", "tooltip",
    // Button variant
    "primary", "secondary", "danger", "success", "warning", "info", "ghost", "link", "text",
    // Sizable 尺寸（替代旧 small/xsmall/large 布尔标志）
    "size",
    // 状态
    "compact", "loading", "disabled", "selected",
    // StyledExt 字体权重（h_flex/v_flex 已废弃，改用 display="flex" + flex-direction）
    "font_thin", "font_extralight", "font_light", "font_normal", "font_medium",
    "font_semibold", "font_bold", "font_extrabold", "font_black",
];

/// 通用绑定属性（来自 `component_bind_setter` 的通用 match 分支）
pub const COMMON_BIND_PROPS: &[&str] = &[
    "content", "value", "disabled", "selected", "checked", "label", "size",
];

/// 通用事件属性（来自 `component_event_setter` 的通用 match 分支）
///
/// 声明式 `on-click`（kebab-case），normalize 后内部 `on_click`（snake_case）。
pub const COMMON_EVENT_PROPS: &[&str] = &[
    "on_click",
];

/// 归一化样式属性（对所有元素与组件生效，由 `style_attr::apply_style_attr` 处理）
///
/// 列表对齐 `css/mapper.rs` 支持的 CSS 子集。
/// normalize 后为 snake_case 形式（如 `flex-direction` → `flex_direction`）。
pub const STYLE_ATTR_PROPS: &[&str] = &[
    // 盒模型
    "width", "height",
    "padding", "padding_top", "padding_right", "padding_bottom", "padding_left",
    "margin", "margin_top", "margin_right", "margin_bottom", "margin_left",
    "border_radius",
    "border", "border_color", "border_top", "border_right", "border_bottom", "border_left",
    // 文本
    "font_size", "font_weight", "font_family",
    "text_align", "line_height", "white_space",
    "color", "background", "background_color",
    // Flexbox
    "display", "flex_direction", "flex_wrap",
    "justify_content", "align_items", "flex", "gap",
    "min_width", "max_width", "min_height", "max_height",
    // 视觉效果
    "opacity", "overflow", "overflow_x", "overflow_y",
];

// ──────────────────────────────────────────────────────────────────────────
// 组件专用属性
// ──────────────────────────────────────────────────────────────────────────

/// 每个组件的专用属性清单（不含通用属性）
///
/// key = canonical_tag 规范化后的 PascalCase 名（如 "StatusBar"），
/// 或保留的小写无连字符别名（如 "menu"）。
/// value = 专用属性列表
pub static COMPONENT_PROPS: &[(&str, &[&str])] = &[
    // Input / TextInput 专用（声明式 on-change，内部 on_change）
    ("Input", &["on_change"]),
    ("TextInput", &["on_change"]),
    // Tree 专用（Stateful 组件，数据由 TreeState Entity 提供，不支持 items 绑定）
    ("Tree", &["on_activate", "on_select"]),
    // MenuBar / StatusBar 不支持 items 绑定（框架不定义 IMenuItem/IStatusBarItem 数据结构）
    // 业务侧经命令式 render_menu_bar() / render_status_bar() 构建
    // Accordion 专用
    ("Accordion", &["multiple", "bordered", "on_toggle_click", "open_ixs"]),
    // AccordionItem 专用（item builder 子标签，不在 component_lookup 中）
    ("AccordionItem", &["title", "open", "icon"]),
    // Avatar 专用（placeholder 已在 COMMON_STATIC_PROPS）
    ("Avatar", &["src", "name"]),
    // AvatarGroup 专用
    ("AvatarGroup", &["limit", "ellipsis"]),
    // Badge 专用（Number/Dot/Icon 三种 variant：count/max 为 Number variant 参数，dot 切换 Dot variant，icon 切换 Icon variant）
    ("Badge", &["count", "max", "dot", "icon"]),
    // Card 专用（Ant Design 风格卡片，title/extra/cover/footer/bordered/borderless/hoverable）
    ("Card", &["title", "extra", "cover", "footer", "bordered", "borderless", "hoverable"]),
    // Tag 专用（variant 属性 primary/secondary/danger/success/warning/info 已在 COMMON_STATIC_PROPS，
    // outline 为 Tag 专属描边样式）
    ("Tag", &["outline"]),
    // Separator 专用（无 new() 构造器，通过 vertical/dashed 选择 horizontal/vertical/dashed 构造）
    ("Separator", &["vertical", "dashed"]),
    // Tabs 专用（WPF TabControl：header + body，全量属性含 on_close/bordered）
    ("Tabs", &[
        "selected_index", "on_click", "on_close", "on_close_all", "on_close_others", "on_promote",
        "prefix", "suffix", "last_empty_space",
        "menu", "track_scroll",
        "bordered",
        "underline", "pill", "flat", "outline", "segmented",
    ]),
    // TabBar 专用（原生形态：纯 header，不含 on_close*/bordered）
    ("TabBar", &[
        "selected_index", "on_click",
        "prefix", "suffix", "last_empty_space",
        "menu", "track_scroll",
        "underline", "pill", "flat", "outline", "segmented",
    ]),
    // Tab 专用（统一底层为 TabItem：label→title, icon→title_icon, body 通过子节点注入）
    // <tab-item> 已弃用移除，统一用 <tab>
    ("Tab", &[
        "label", "icon", "disabled", "selected", "prefix", "suffix", "on_click",
        "closable", "preview",
        "underline", "pill", "flat", "outline", "segmented",
    ]),
    // Table 专用（WPF DataGrid 风格表格）
    ("Table", &["columns", "rows", "delegate", "bordered", "borderless", "stripe"]),
    // Column 专用（item builder 子标签，不在 component_lookup 中）
    ("Column", &["key", "title", "width", "align", "field"]),
    // DescriptionList 专用（无 ElementId 容器，vertical/bordered/columns/label_width/items）
    ("DescriptionList", &["vertical", "bordered", "columns", "label_width", "items"]),
    // DescriptionItem 专用（item builder 子标签，label 为构造器参数，value/span 为 setter）
    ("DescriptionItem", &["label", "value", "span"]),
    // Popover 专用（浮动气泡容器：trigger slot + content 子节点 + anchor 定位 + default_open 非受控初始）
    // 受控模式（open + on_open_change）需要特殊回调签名适配，待需求出现时再添加
    ("Popover", &["anchor", "mouse_button", "appearance", "overlay_closable", "default_open"]),
    // Icon 专用（RenderOnce 无 ElementId，name/path 为构造器参数，size 走通用 Sizable）
    ("Icon", &["name", "path"]),
    // Kbd 专用（RenderOnce 无 ElementId，key 为构造器参数，outline/appearance 为 setter）
    ("Kbd", &["key", "outline", "appearance"]),
    // Breadcrumb 专用（RenderOnce 无 ElementId，items 数据绑定 + on_select 同级选择回调）
    ("Breadcrumb", &["items", "on_select"]),
    // Alert 专用（variant 关联函数 + message 构造器参数）
    // info/success/warning/error 已在 COMMON_STATIC_PROPS（Button variant 集合复用）
    // on_close 走 event 分类（前缀 "on"）
    ("Alert", &["variant", "message", "title", "banner", "visible", "icon", "on_close"]),
    // ── Phase 1 基础无状态组件 ──
    // Spinner：.icon(impl Into<Icon>)，color 暂不支持（Hsla 解析复杂），size 走通用 Sizable
    ("Spinner", &["icon"]),
    // Skeleton：.secondary() 布尔切换次级颜色
    ("Skeleton", &["secondary"]),
    // Link：.href(impl Into<SharedString>)，disabled/on_click 走通用
    ("Link", &["href"]),
    // Collapsible：.open(bool) 控制展开，content slot 待后续支持
    ("Collapsible", &["open"]),
    // GroupBox：.title(impl IntoElement)，normal/fill/outline 为 variant 关联方法
    ("GroupBox", &["title", "normal", "fill", "outline", "variant"]),
    // Pagination：.current_page(usize)/.total_pages(usize)/.visible_pages(usize)/.compact()
    // on_click 签名为 Fn(&usize, ...)，走 event 分类但需专属代码生成
    ("Pagination", &["current_page", "total_pages", "visible_pages", "compact", "on_click"]),
    // Radio：.label()/.checked()/.disabled() 走通用，tab_index/tab_stop 为 Radio 专属
    // on_click 签名为 Fn(&bool, ...)，已在 component_event_setter 中处理（is_bool_event）
    ("Radio", &["tab_index", "tab_stop", "on_click"]),
    // RadioGroup：.selected_index(Option<usize>)/.disabled(bool)
    // horizontal/layout 控制 vertical/horizontal 构造器选择
    // on_click 签名为 Fn(&usize, ...)，需专属代码生成
    ("RadioGroup", &["selected_index", "horizontal", "vertical", "layout", "on_click"]),
];

/// 查询组件的所有已注册属性（通用 + 专用）
///
/// 返回 (static_props, bind_props, event_props) 三元组（owned Vec）。
/// 若组件未在 COMPONENT_PROPS 登记，仅返回通用属性。
///
/// 通过 `canonical_tag()` 规范化标签：kebab-case → PascalCase（如 `menu-bar` → `MenuBar`），
/// 小写别名 → PascalCase（如 `accordion` → `Accordion`、`item` → `AccordionItem`）。
pub fn props_for(tag: &str) -> (Vec<&'static str>, Vec<&'static str>, Vec<&'static str>) {
    // 查找专用属性（canonical_tag 统一处理 kebab-case 和小写别名）
    let canonical = crate::tags::canonical_tag(tag);
    let extra: &[&str] = COMPONENT_PROPS
        .iter()
        .find(|(t, _)| *t == canonical.as_str())
        .map(|(_, props)| *props)
        .unwrap_or(&[]);

    // 把专用属性按前缀分类（on* → event，其余 → bind）
    let mut bind_props: Vec<&'static str> = COMMON_BIND_PROPS.to_vec();
    let mut event_props: Vec<&'static str> = COMMON_EVENT_PROPS.to_vec();
    for prop in extra {
        if prop.starts_with("on") {
            if !event_props.contains(prop) {
                event_props.push(prop);
            }
        } else if !bind_props.contains(prop) {
            bind_props.push(prop);
        }
    }

    (COMMON_STATIC_PROPS.to_vec(), bind_props, event_props)
}

/// 判断属性是否在组件的已注册清单中（通用 + 专用）
///
/// 供 codegen 在 bind_setter / static_setter 未命中时调用，
/// 判断属性是"已知但未映射"（需补全 setter 逻辑）还是"完全未知"（用户拼写错误）。
///
/// 通过 `canonical_tag()` 规范化标签：kebab-case → PascalCase（如 `menu-bar` → `MenuBar`），
/// 小写别名 → PascalCase（如 `accordion` → `Accordion`、`item` → `AccordionItem`）。
pub fn is_prop_registered(tag: &str, attr: &str) -> bool {
    // 归一化样式属性（对所有元素与组件生效）
    if STYLE_ATTR_PROPS.contains(&attr) {
        return true;
    }

    // 通用属性
    if COMMON_STATIC_PROPS.contains(&attr)
        || COMMON_BIND_PROPS.contains(&attr)
        || COMMON_EVENT_PROPS.contains(&attr)
    {
        return true;
    }

    // 组件专用属性：canonical_tag 统一处理 kebab-case 和小写别名
    let canonical = crate::tags::canonical_tag(tag);
    if let Some((_, props)) = COMPONENT_PROPS.iter().find(|(t, _)| *t == canonical.as_str()) {
        return props.contains(&attr);
    }

    false
}

// ──────────────────────────────────────────────────────────────────────────
// 窗口外壳属性
// ──────────────────────────────────────────────────────────────────────────

/// 窗口外壳组件的可绑定属性（由 `shell.rs` 处理）
///
/// key = RML 根标签名（kebab-case），value = 该 shell 支持的所有属性
pub static SHELL_PROPS: &[(&str, &[&str])] = &[
    ("tab-window", &[
        "title", "width", "height", "startup", "icon",
        "tabs", "selected_index", "show_chrome",
        "left_size", "right_size", "bottom_size",
        "on_tab_click", "on_tab_close",
        "on_tab_close_all", "on_tab_close_others",
        "on_chrome_toggle", "tab_item_template",
    ]),
    ("modern-window", &[
        "title", "width", "height", "startup", "icon",
        "menu", "footer",
    ]),
    ("window", &[
        "title", "width", "height", "startup", "icon",
    ]),
    ("dialog", &[
        "title", "width", "height",
    ]),
    ("component", &[
        "content",
    ]),
];

/// 判断 shell 属性是否已注册
pub fn is_shell_prop_registered(shell_tag: &str, attr: &str) -> bool {
    if let Some((_, props)) = SHELL_PROPS.iter().find(|(t, _)| *t == shell_tag) {
        return props.contains(&attr);
    }
    false
}

/// 查询 shell 的所有已注册属性
pub fn shell_props_for(shell_tag: &str) -> Option<&'static [&'static str]> {
    SHELL_PROPS
        .iter()
        .find(|(t, _)| *t == shell_tag)
        .map(|(_, props)| *props)
}

// ──────────────────────────────────────────────────────────────────────────
// 单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn common_props_are_non_empty() {
        assert!(!COMMON_STATIC_PROPS.is_empty());
        assert!(!COMMON_BIND_PROPS.is_empty());
        assert!(!COMMON_EVENT_PROPS.is_empty());
    }

    #[test]
    fn known_component_props_recognized() {
        assert!(is_prop_registered("Input", "on_change"));
        // Tree 是 Stateful 组件，数据由 TreeState Entity 提供，不支持 items 绑定
        assert!(!is_prop_registered("Tree", "items"));
        assert!(is_prop_registered("Tree", "on_activate"));
        assert!(is_prop_registered("Tree", "on_select"));
        // MenuBar / StatusBar 不支持 items 绑定（框架不定义 IMenuItem/IStatusBarItem）
        assert!(!is_prop_registered("MenuBar", "items"));
        assert!(!is_prop_registered("menu-bar", "items"));
        assert!(!is_prop_registered("status-bar", "items"));
        assert!(!is_prop_registered("StatusBar", "items"));
    }

    #[test]
    fn common_props_recognized_for_any_tag() {
        assert!(is_prop_registered("Button", "label"));
        assert!(is_prop_registered("Button", "primary"));
        assert!(is_prop_registered("Button", "on_click"));
        assert!(is_prop_registered("Badge", "disabled"));
    }

    #[test]
    fn unknown_props_not_registered() {
        assert!(!is_prop_registered("Button", "nonexistent_prop"));
        assert!(!is_prop_registered("Input", "on_foo"));
    }

    #[test]
    fn accordion_lowercase_alias_props_registered() {
        assert!(is_prop_registered("accordion", "multiple"));
        assert!(is_prop_registered("accordion", "bordered"));
        assert!(is_prop_registered("accordion", "on_toggle_click"));
    }

    #[test]
    fn item_short_form_props_registered() {
        assert!(is_prop_registered("item", "title"));
        assert!(is_prop_registered("item", "open"));
        assert!(is_prop_registered("item", "icon"));
    }

    #[test]
    fn props_for_accordion_lowercase_and_item() {
        let (_, bind, event) = props_for("accordion");
        assert!(bind.contains(&"multiple"));
        assert!(bind.contains(&"bordered"));
        assert!(event.contains(&"on_toggle_click"));

        let (static_props, bind, _event) = props_for("item");
        assert!(bind.contains(&"title"));
        assert!(bind.contains(&"open"));
        assert!(bind.contains(&"icon"));
        // 通用属性仍可用
        assert!(static_props.contains(&"disabled"));
    }

    #[test]
    fn tabs_props_registered() {
        // Tabs（WPF TabControl）支持全量属性，含 on_close/bordered
        assert!(is_prop_registered("Tabs", "selected_index"));
        assert!(is_prop_registered("Tabs", "on_click"));
        assert!(is_prop_registered("Tabs", "on_close"));
        assert!(is_prop_registered("Tabs", "on_close_all"));
        assert!(is_prop_registered("Tabs", "on_close_others"));
        assert!(is_prop_registered("Tabs", "on_promote"));
        assert!(is_prop_registered("Tabs", "prefix"));
        assert!(is_prop_registered("Tabs", "suffix"));
        assert!(is_prop_registered("Tabs", "last_empty_space"));
        assert!(is_prop_registered("Tabs", "menu"));
        assert!(is_prop_registered("Tabs", "track_scroll"));
        assert!(is_prop_registered("Tabs", "bordered"));
        assert!(is_prop_registered("Tabs", "underline"));
        assert!(is_prop_registered("Tabs", "pill"));
        assert!(is_prop_registered("Tabs", "flat"));
        assert!(is_prop_registered("Tabs", "outline"));
        assert!(is_prop_registered("Tabs", "segmented"));
    }

    #[test]
    fn tab_bar_props_registered() {
        // TabBar（原生形态）不含 on_close*/bordered
        assert!(is_prop_registered("TabBar", "selected_index"));
        assert!(is_prop_registered("TabBar", "on_click"));
        assert!(is_prop_registered("TabBar", "prefix"));
        assert!(is_prop_registered("TabBar", "suffix"));
        assert!(is_prop_registered("TabBar", "last_empty_space"));
        assert!(is_prop_registered("TabBar", "menu"));
        assert!(is_prop_registered("TabBar", "track_scroll"));
        assert!(is_prop_registered("TabBar", "underline"));
        assert!(is_prop_registered("TabBar", "pill"));
        assert!(is_prop_registered("TabBar", "flat"));
        assert!(is_prop_registered("TabBar", "outline"));
        assert!(is_prop_registered("TabBar", "segmented"));
        // 不支持 on_close/bordered
        assert!(!is_prop_registered("TabBar", "on_close"));
        assert!(!is_prop_registered("TabBar", "on_close_all"));
        assert!(!is_prop_registered("TabBar", "bordered"));
    }

    #[test]
    fn tab_props_registered() {
        assert!(is_prop_registered("Tab", "label"));
        assert!(is_prop_registered("Tab", "icon"));
        assert!(is_prop_registered("Tab", "disabled"));
        assert!(is_prop_registered("Tab", "selected"));
        assert!(is_prop_registered("Tab", "prefix"));
        assert!(is_prop_registered("Tab", "suffix"));
        assert!(is_prop_registered("Tab", "on_click"));
        assert!(is_prop_registered("Tab", "closable"));
        // <tab-item> 已弃用移除，统一用 <tab>
        assert!(is_prop_registered("Tab", "underline"));
        assert!(is_prop_registered("Tab", "pill"));
    }

    #[test]
    fn tab_bar_kebab_alias_props_registered() {
        // <tab-bar> kebab-case 别名也应命中 TabBar 属性
        assert!(is_prop_registered("tab-bar", "selected_index"));
        assert!(is_prop_registered("tab-bar", "underline"));
    }

    #[test]
    fn tabs_kebab_alias_props_registered() {
        // <tabs> kebab-case 别名也应命中 Tabs 属性
        assert!(is_prop_registered("tabs", "selected_index"));
        assert!(is_prop_registered("tabs", "bordered"));
        assert!(is_prop_registered("tabs", "on_close"));
    }

    #[test]
    fn tab_short_form_props_registered() {
        // <tab> 短标签也应命中 Tab 属性
        assert!(is_prop_registered("tab", "label"));
        assert!(is_prop_registered("tab", "icon"));
    }

    #[test]
    fn props_for_tabs_tab_bar_and_tab() {
        // Tabs 支持 on_close/bordered
        let (_, bind, event) = props_for("Tabs");
        assert!(bind.contains(&"selected_index"));
        assert!(bind.contains(&"bordered"));
        assert!(event.contains(&"on_click"));
        assert!(event.contains(&"on_close"));

        // TabBar 不支持 on_close/bordered
        let (_, bind, event) = props_for("TabBar");
        assert!(bind.contains(&"selected_index"));
        assert!(!bind.contains(&"bordered"));
        assert!(event.contains(&"on_click"));
        assert!(!event.contains(&"on_close"));

        let (_, bind, event) = props_for("Tab");
        assert!(bind.contains(&"label"));
        assert!(bind.contains(&"icon"));
        assert!(event.contains(&"on_click"));
    }

    #[test]
    fn shell_props_recognized() {
        assert!(is_shell_prop_registered("tab-window", "tabs"));
        assert!(is_shell_prop_registered("tab-window", "on_tab_click"));
        assert!(is_shell_prop_registered("tab-window", "on_tab_close"));
        assert!(is_shell_prop_registered("tab-window", "left_size"));
        assert!(is_shell_prop_registered("tab-window", "tab_item_template"));
        assert!(is_shell_prop_registered("modern-window", "menu"));
        assert!(is_shell_prop_registered("modern-window", "footer"));
        assert!(is_shell_prop_registered("window", "title"));
    }

    #[test]
    fn shell_props_for_returns_list() {
        let props = shell_props_for("tab-window").expect("tab-window should be registered");
        assert!(props.contains(&"tabs"));
        assert!(props.contains(&"show_chrome"));
        assert!(props.contains(&"tab_item_template"));
    }

    #[test]
    fn unknown_shell_tag_returns_none() {
        assert!(shell_props_for("nonexistent_shell").is_none());
        assert!(!is_shell_prop_registered("nonexistent_shell", "title"));
    }

    /// 验证 COMPONENT_PROPS 中的每个 tag 都在 tags::component_lookup 中注册
    /// （避免注册表与路由表不一致）
    ///
    /// 例外：item builder 子标签（如 `AccordionItem`）通过 `is_item_builder_tag`
    /// 识别，不在 `component_lookup` 中注册——避免被误用为顶层扩展组件。
    #[test]
    fn component_props_tags_align_with_routing_table() {
        use crate::tags;
        for (tag, _) in COMPONENT_PROPS {
            if tags::is_item_builder_tag(tag) {
                continue;
            }
            assert!(
                tags::component_lookup(tag).is_some(),
                "COMPONENT_PROPS contains tag '{}' but tags::component_lookup returns None",
                tag
            );
        }
    }

    /// 验证 SHELL_PROPS 中的每个 tag 都是合法的根标签
    #[test]
    fn shell_props_tags_are_valid_roots() {
        use crate::tags;
        for (tag, _) in SHELL_PROPS {
            assert!(
                tags::root_tag_lookup(tag).is_some(),
                "SHELL_PROPS contains tag '{}' but tags::root_tag_lookup returns None",
                tag
            );
        }
    }
}
