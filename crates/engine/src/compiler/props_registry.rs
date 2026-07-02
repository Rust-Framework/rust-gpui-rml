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
//! - **event**：事件属性 `onclick={fn}`，由 `component_event_setter` 处理
//! - **shell**：窗口外壳属性 `title="..."` / `tabs={...}`，由 `shell.rs` 处理
//!
//! 注册表合并所有分类，按组件列出完整属性清单。

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
    // Sizable 尺寸
    "small", "xsmall", "large",
    // 状态
    "compact", "loading", "disabled", "selected",
    // StyledExt 字体权重
    "font_thin", "font_extralight", "font_light", "font_normal", "font_medium",
    "font_semibold", "font_bold", "font_extrabold", "font_black",
    // StyledExt 布局
    "h_flex", "v_flex",
];

/// 通用绑定属性（来自 `component_bind_setter` 的通用 match 分支）
pub const COMMON_BIND_PROPS: &[&str] = &[
    "content", "value", "disabled", "selected", "checked", "label",
];

/// 通用事件属性（来自 `component_event_setter` 的通用 match 分支）
pub const COMMON_EVENT_PROPS: &[&str] = &[
    "onclick",
];

// ──────────────────────────────────────────────────────────────────────────
// 组件专用属性
// ──────────────────────────────────────────────────────────────────────────

/// 每个组件的专用属性清单（不含通用属性）
///
/// key = RML 标签名（PascalCase 或 kebab-case），value = 专用属性列表
pub static COMPONENT_PROPS: &[(&str, &[&str])] = &[
    // Input / TextInput 专用
    ("Input", &["onchange"]),
    ("TextInput", &["onchange"]),
    // Tree 专用
    ("Tree", &["items", "on_activate", "on_select"]),
    // MenuBar / menu / status_bar 专用（items 绑定）
    ("MenuBar", &["items"]),
    ("menu", &["items"]),
    ("status_bar", &["items"]),
];

/// 查询组件的所有已注册属性（通用 + 专用）
///
/// 返回 (static_props, bind_props, event_props) 三元组。
/// 若组件未在 COMPONENT_PROPS 登记，仅返回通用属性。
pub fn props_for(tag: &str) -> (&'static [&'static str], &'static [&'static str], &'static [&'static str]) {
    let extra: &[&str] = COMPONENT_PROPS
        .iter()
        .find(|(t, _)| *t == tag)
        .map(|(_, props)| *props)
        .unwrap_or(&[]);

    // 把专用属性按前缀分类（on* → event，其余 → bind）
    let mut bind_extra: Vec<&'static str> = Vec::new();
    let mut event_extra: Vec<&'static str> = Vec::new();
    for prop in extra {
        if prop.starts_with("on") {
            event_extra.push(prop);
        } else {
            bind_extra.push(prop);
        }
    }

    // 合并通用 + 专用（专用优先，避免重复）
    let _ = (bind_extra, event_extra); // 当前实现仅返回通用，专用属性通过 is_prop_registered 查询
    (COMMON_STATIC_PROPS, COMMON_BIND_PROPS, COMMON_EVENT_PROPS)
}

/// 判断属性是否在组件的已注册清单中（通用 + 专用）
///
/// 供 codegen 在 bind_setter / static_setter 未命中时调用，
/// 判断属性是"已知但未映射"（需补全 setter 逻辑）还是"完全未知"（用户拼写错误）。
pub fn is_prop_registered(tag: &str, attr: &str) -> bool {
    // 通用属性
    if COMMON_STATIC_PROPS.contains(&attr)
        || COMMON_BIND_PROPS.contains(&attr)
        || COMMON_EVENT_PROPS.contains(&attr)
    {
        return true;
    }

    // 组件专用属性
    if let Some((_, props)) = COMPONENT_PROPS.iter().find(|(t, _)| *t == tag) {
        return props.contains(&attr);
    }

    false
}

// ──────────────────────────────────────────────────────────────────────────
// 窗口外壳属性
// ──────────────────────────────────────────────────────────────────────────

/// 窗口外壳组件的可绑定属性（由 `shell.rs` 处理）
///
/// key = RML 根标签名，value = 该 shell 支持的所有属性
pub static SHELL_PROPS: &[(&str, &[&str])] = &[
    ("tab_window", &[
        "title", "width", "height", "startup", "icon",
        "tabs", "selected_tab", "show_chrome",
        "left_size", "right_size", "bottom_size",
        "on_tab_click", "on_chrome_toggle",
    ]),
    ("modern_window", &[
        "title", "width", "height", "startup", "icon",
        "menu", "footer",
    ]),
    ("window", &[
        "title", "width", "height", "startup", "icon",
    ]),
    ("dialog", &[
        "title", "width", "height",
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
        assert!(is_prop_registered("Input", "onchange"));
        assert!(is_prop_registered("Tree", "items"));
        assert!(is_prop_registered("Tree", "on_activate"));
        assert!(is_prop_registered("MenuBar", "items"));
        assert!(is_prop_registered("menu", "items"));
        assert!(is_prop_registered("status_bar", "items"));
    }

    #[test]
    fn common_props_recognized_for_any_tag() {
        assert!(is_prop_registered("Button", "label"));
        assert!(is_prop_registered("Button", "primary"));
        assert!(is_prop_registered("Button", "onclick"));
        assert!(is_prop_registered("Badge", "disabled"));
    }

    #[test]
    fn unknown_props_not_registered() {
        assert!(!is_prop_registered("Button", "nonexistent_prop"));
        assert!(!is_prop_registered("Input", "on_foo"));
    }

    #[test]
    fn shell_props_recognized() {
        assert!(is_shell_prop_registered("tab_window", "tabs"));
        assert!(is_shell_prop_registered("tab_window", "on_tab_click"));
        assert!(is_shell_prop_registered("tab_window", "left_size"));
        assert!(is_shell_prop_registered("modern_window", "menu"));
        assert!(is_shell_prop_registered("modern_window", "footer"));
        assert!(is_shell_prop_registered("window", "title"));
    }

    #[test]
    fn shell_props_for_returns_list() {
        let props = shell_props_for("tab_window").expect("tab_window should be registered");
        assert!(props.contains(&"tabs"));
        assert!(props.contains(&"show_chrome"));
    }

    #[test]
    fn unknown_shell_tag_returns_none() {
        assert!(shell_props_for("nonexistent_shell").is_none());
        assert!(!is_shell_prop_registered("nonexistent_shell", "title"));
    }

    /// 验证 COMPONENT_PROPS 中的每个 tag 都在 tags::component_lookup 中注册
    /// （避免注册表与路由表不一致）
    #[test]
    fn component_props_tags_align_with_routing_table() {
        use crate::tags;
        for (tag, _) in COMPONENT_PROPS {
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
