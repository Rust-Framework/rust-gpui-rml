//! 主题系统 —— `ThemeState` Global + `ThemeExt` 扩展
//!
//! 开发体验与 [`crate::i18n`] 完全对齐:
//! - `cx.set_style("styles.css")` 加载全局样式 CSS 的 `:root` 变量作为基础值（颜色 + 长度 + 数字）
//! - `cx.set_theme("dark")` 加载/切换主题(主题变量覆盖基础变量)
//! - `cx.theme_color("--primary")` 取当前生效颜色(基础 + 主题)
//! - `theme_color_static("--primary")` 供 `#[computed]` 等无 `App` 上下文场景使用
//! - `rml::theme::length("--spacing")` / `rml::theme::number("--opacity")` 取非颜色变量
//!
//! 框架内置 light/dark 两套默认主题色,即使 `assets/themes/light.css` 和 `dark.css` 为空
//! 也能获得现代化的亮暗配色。主题 CSS 文件可选择性覆盖内置颜色实现定制。
//!
//! 变量优先级（高 → 低）: 主题 CSS 文件 > 框架内置默认 > `set_style` 基础变量。
//!
//! 启用 `gpui-component` feature 后,`set_theme` 还会同步 gpui-component 原生主题
//! （Button/Input 等组件的配色），无需开发者手动调用。
//!
//! 主题文件格式(`assets/themes/dark.css`):
//! ```css
//! :root {
//!     --primary: #007bff;
//!     --text: #333333;
//!     --spacing: 8px;
//!     --opacity: 0.5;
//! }
//! ```
//! 解析 `:root` 块中的所有变量: `#hex` 识别为颜色, `Npx`/`Npt` 识别为长度, 纯数字识别为数字。

use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::RwLock;

use gpui::{App, AppContext, BorrowAppContext, Context, Global, Pixels, Rgba, SharedString};

/// 线程内同步主题颜色快照,供 `#[computed]` 等无 `App` 上下文场景使用
static ACTIVE_THEME_COLORS: RwLock<Option<HashMap<String, Rgba>>> = RwLock::new(None);

fn sync_active_theme(colors: &HashMap<String, Rgba>) {
    if let Ok(mut guard) = ACTIVE_THEME_COLORS.write() {
        *guard = Some(colors.clone());
    }
}

/// 无 `App` 上下文时取主题颜色(依赖 `sync_active_theme` 维护的快照)
///
/// 未找到变量时返回透明黑色作为 fallback。
pub fn theme_color_static(name: &str) -> Rgba {
    ACTIVE_THEME_COLORS
        .read()
        .ok()
        .and_then(|guard| guard.as_ref()?.get(name).copied())
        .unwrap_or(Rgba {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 0.0,
        })
}

/// 取当前主题颜色(供 codegen 生成的样式代码调用)
///
/// 在 `Render::render` 闭包中无 `App` 上下文,通过此自由函数读取
/// `ACTIVE_THEME_COLORS` 快照。与 `theme_color_static` 等价,提供更简洁的调用名。
pub fn color(name: &str) -> Rgba {
    theme_color_static(name)
}

/// 运行时 CSS 变量值（支持主题切换）
///
/// `:root` 块中的变量按值类型分类:
/// - `#hex` → `Color`（颜色变量,由 `color()` 查询）
/// - `Npx`/`Npt` → `Length`（长度变量,由 `length()` 查询）
/// - 纯数字 → `Number`（数字变量,由 `number()` 查询）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ThemeVar {
    /// 颜色变量 `--primary: #007bff`
    Color(Rgba),
    /// 长度变量 `--spacing: 8px`（已归一化为像素值）
    Length(f32),
    /// 数字变量 `--opacity: 0.5`
    Number(f32),
}

/// 线程内同步主题变量快照,供 `#[computed]` 等无 `App` 上下文场景使用
static ACTIVE_THEME_VARS: RwLock<Option<HashMap<String, ThemeVar>>> = RwLock::new(None);

fn sync_active_vars(vars: &HashMap<String, ThemeVar>) {
    if let Ok(mut guard) = ACTIVE_THEME_VARS.write() {
        *guard = Some(vars.clone());
    }
}

/// 无 `App` 上下文时取主题变量(依赖 `sync_active_vars` 维护的快照)
pub fn theme_var_static(name: &str) -> Option<ThemeVar> {
    ACTIVE_THEME_VARS
        .read()
        .ok()
        .and_then(|guard| guard.as_ref()?.get(name).copied())
}

/// 运行时查询长度变量(供 codegen 生成的样式代码调用)
///
/// 未找到变量或变量非 `Length` 类型时返回 `px(0.0)`。
pub fn length(name: &str) -> Pixels {
    match theme_var_static(name) {
        Some(ThemeVar::Length(n)) => gpui::px(n),
        _ => gpui::px(0.0),
    }
}

/// 运行时查询数字变量(供 codegen 生成的样式代码调用)
///
/// 未找到变量或变量非 `Number` 类型时返回 `0.0`。
pub fn number(name: &str) -> f32 {
    match theme_var_static(name) {
        Some(ThemeVar::Number(n)) => n,
        _ => 0.0,
    }
}

/// 默认主题资源目录(相对 `assets/` 根)
pub const DEFAULT_THEMES_DIR: &str = "assets/themes";

/// 内部 Global:当前主题与颜色表
#[derive(Debug, Clone)]
pub struct ThemeState {
    theme: String,
    dir: String,
    /// 全局基础颜色(由 `set_style` 加载,作为主题颜色的默认值)
    base_colors: HashMap<String, Rgba>,
    /// 当前生效颜色(base_colors + 主题颜色覆盖)
    colors: HashMap<String, Rgba>,
    themes: HashMap<String, HashMap<String, Rgba>>,
    /// 全局基础非颜色变量(由 `set_style` 加载,作为主题变量的默认值)
    base_vars: HashMap<String, ThemeVar>,
    /// 当前生效非颜色变量(base_vars + 主题变量覆盖)
    vars: HashMap<String, ThemeVar>,
    /// 各主题的非颜色变量表(按主题名索引)
    theme_var_sets: HashMap<String, HashMap<String, ThemeVar>>,
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            theme: String::new(),
            dir: DEFAULT_THEMES_DIR.to_string(),
            base_colors: HashMap::new(),
            colors: HashMap::new(),
            themes: HashMap::new(),
            base_vars: HashMap::new(),
            vars: HashMap::new(),
            theme_var_sets: HashMap::new(),
        }
    }
}

impl Global for ThemeState {}

impl ThemeState {
    pub fn theme(&self) -> &str {
        &self.theme
    }

    pub fn dir(&self) -> &str {
        &self.dir
    }

    pub fn color(&self, name: &str) -> Option<Rgba> {
        self.colors.get(name).copied()
    }

    /// 查询非颜色变量(合并 base + theme,theme 优先)
    pub fn var(&self, name: &str) -> Option<ThemeVar> {
        self.vars.get(name).copied()
    }

    pub fn set_dir(&mut self, dir: impl Into<String>) {
        self.dir = dir.into();
    }

    /// 设置全局基础颜色(由 `set_style` 调用),并重新合并当前主题颜色与变量
    pub fn set_base_colors(&mut self, base: HashMap<String, Rgba>) {
        self.base_colors = base;
        self.recompute();
    }

    /// 设置全局基础非颜色变量(由 `set_style` 调用),并重新合并当前主题变量
    pub fn set_base_vars(&mut self, vars: HashMap<String, ThemeVar>) {
        self.base_vars = vars;
        self.recompute_vars();
    }

    /// 加载主题颜色表;若当前未设置主题或与当前主题同名,则激活
    pub fn load_theme(&mut self, name: impl Into<String>, theme_colors: HashMap<String, Rgba>) {
        let name = name.into();
        self.themes.insert(name.clone(), theme_colors.clone());
        if self.theme.is_empty() || self.theme == name {
            self.theme = name;
            self.recompute();
        }
    }

    /// 加载主题非颜色变量表;若当前主题同名,则激活
    pub fn load_theme_vars(&mut self, name: impl Into<String>, theme_vars: HashMap<String, ThemeVar>) {
        let name = name.into();
        self.theme_var_sets.insert(name.clone(), theme_vars);
        if self.theme == name {
            self.recompute_vars();
        }
    }

    /// 切换到已加载的主题;成功返回 true
    pub fn switch_theme(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.themes.contains_key(&name) {
            self.theme = name;
            self.recompute();
            true
        } else {
            false
        }
    }

    /// 重新计算合并颜色:base_colors 为底,当前主题颜色覆盖
    fn recompute_colors(&mut self) {
        let theme_colors = self.themes.get(&self.theme).cloned().unwrap_or_default();
        let mut merged = self.base_colors.clone();
        for (k, v) in theme_colors {
            merged.insert(k, v);
        }
        self.colors = merged;
        sync_active_theme(&self.colors);
    }

    /// 重新计算合并非颜色变量:base_vars 为底,当前主题变量覆盖
    fn recompute_vars(&mut self) {
        let theme_vars = self.theme_var_sets.get(&self.theme).cloned().unwrap_or_default();
        let mut merged = self.base_vars.clone();
        for (k, v) in theme_vars {
            merged.insert(k, v);
        }
        self.vars = merged;
        sync_active_vars(&self.vars);
    }

    /// 重新计算颜色与变量(主题切换/基础值变更时调用)
    fn recompute(&mut self) {
        self.recompute_colors();
        self.recompute_vars();
    }
}

/// 确保 `ThemeState` Global 已注册
pub fn ensure_theme(cx: &mut App) {
    if !cx.has_global::<ThemeState>() {
        cx.set_global(ThemeState::default());
    }
}

// ─── 框架内置默认主题色 ───

fn rgba_from_hex(hex: u32) -> Rgba {
    Rgba {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

/// 内置 light 主题默认 CSS 变量颜色
fn builtin_light_colors() -> HashMap<String, Rgba> {
    let mut m = HashMap::new();
    m.insert("--primary".to_string(), rgba_from_hex(0x007acc));
    m.insert("--background".to_string(), rgba_from_hex(0xffffff));
    m.insert("--surface".to_string(), rgba_from_hex(0xf3f4f6));
    m.insert("--surface-variant".to_string(), rgba_from_hex(0xf9fafb));
    m.insert("--code-bg".to_string(), rgba_from_hex(0xf1f3f5));
    m.insert("--text".to_string(), rgba_from_hex(0x111827));
    m.insert("--text-muted".to_string(), rgba_from_hex(0x6b7280));
    m.insert("--border".to_string(), rgba_from_hex(0xe5e7eb));
    m.insert("--success".to_string(), rgba_from_hex(0x059669));
    m.insert("--warning".to_string(), rgba_from_hex(0xd97706));
    m.insert("--error".to_string(), rgba_from_hex(0xdc2626));
    m.insert("--info".to_string(), rgba_from_hex(0x2563eb));
    m.insert("--primary-foreground".to_string(), rgba_from_hex(0xffffff));
    m.insert("--title-bar".to_string(), rgba_from_hex(0xf3f4f6));
    m.insert("--status-bar".to_string(), rgba_from_hex(0xf3f4f6));
    m.insert("--chrome-surface".to_string(), rgba_from_hex(0xf3f4f6));
    m.insert("--editor-surface".to_string(), rgba_from_hex(0xffffff));
    m
}

/// 内置 dark 主题默认 CSS 变量颜色
fn builtin_dark_colors() -> HashMap<String, Rgba> {
    let mut m = HashMap::new();
    m.insert("--primary".to_string(), rgba_from_hex(0x007acc));
    m.insert("--background".to_string(), rgba_from_hex(0x222427));
    m.insert("--surface".to_string(), rgba_from_hex(0x2a2b30));
    m.insert("--surface-variant".to_string(), rgba_from_hex(0x1a1b1d));
    m.insert("--code-bg".to_string(), rgba_from_hex(0x1a1b1d));
    m.insert("--text".to_string(), rgba_from_hex(0xd4d4d8));
    m.insert("--text-muted".to_string(), rgba_from_hex(0x8e8e93));
    m.insert("--border".to_string(), rgba_from_hex(0x374151));
    m.insert("--success".to_string(), rgba_from_hex(0x4ec9b0));
    m.insert("--warning".to_string(), rgba_from_hex(0xcca700));
    m.insert("--error".to_string(), rgba_from_hex(0xf44747));
    m.insert("--info".to_string(), rgba_from_hex(0x3794ff));
    m.insert("--primary-foreground".to_string(), rgba_from_hex(0xffffff));
    m.insert("--title-bar".to_string(), rgba_from_hex(0x1a1b1d));
    m.insert("--status-bar".to_string(), rgba_from_hex(0x1a1b1d));
    m.insert("--chrome-surface".to_string(), rgba_from_hex(0x1a1b1d));
    m.insert("--editor-surface".to_string(), rgba_from_hex(0x222427));
    m
}

/// 按主题名返回内置默认颜色; 未知主题返回空表
fn builtin_theme_colors(theme: &str) -> HashMap<String, Rgba> {
    match theme {
        "dark" => builtin_dark_colors(),
        "light" => builtin_light_colors(),
        _ => HashMap::new(),
    }
}

/// 以内置默认为底,叠加 CSS 文件提供的颜色与变量,返回合并结果
fn merge_theme_with_builtin(
    theme: &str,
    dir: &str,
) -> (HashMap<String, Rgba>, HashMap<String, ThemeVar>) {
    let mut colors = builtin_theme_colors(theme);
    let mut vars = HashMap::new();
    if let Ok((css_colors, css_vars)) = load_theme_vars_embedded(theme, dir) {
        for (k, v) in css_colors {
            colors.insert(k, v);
        }
        for (k, v) in css_vars {
            vars.insert(k, v);
        }
    }
    (colors, vars)
}

// ─── gpui-component 原生主题同步（feature-gated） ───

#[cfg(feature = "gpui-component")]
fn apply_builtin_gpui_theme(theme: &str, cx: &mut App) {
    match theme {
        "dark" => apply_dark_theme_config(cx),
        "light" => apply_light_theme_config(cx),
        _ => {}
    }
}

#[cfg(feature = "gpui-component")]
fn apply_dark_theme_config(cx: &mut App) {
    use gpui::px;

    gpui_component::theme::Theme::sync_scrollbar_appearance(cx);
    let t = gpui_component::theme::Theme::global_mut(cx);

    t.highlight_theme = gpui_component::highlighter::HighlightTheme::default_dark();

    t.background = gpui::rgb(0x222427).into();
    t.secondary = gpui::rgb(0x1a1b1d).into();
    t.muted = gpui::rgb(0x2a2b30).into();
    t.title_bar = gpui::rgb(0x1a1b1d).into();
    t.title_bar_border = gpui::rgb(0x0f1012).into();
    t.sidebar = gpui::rgb(0x1a1b1d).into();
    t.sidebar_accent = gpui::rgb(0x2a2b30).into();
    t.tab_bar = gpui::transparent_black();
    t.tab_bar_segmented = gpui::rgb(0x1a1b1d).into();
    t.tab_foreground = gpui::rgb(0xd4d4d8).into();
    t.tab_active = gpui::rgb(0x222427).into();
    t.tab_active_foreground = gpui::rgb(0xffffff).into();
    t.colors.list = gpui::rgb(0x25262a).into();
    t.input = gpui::rgb(0x2e3035).into();

    t.list_hover = gpui::rgb(0x33353a).into();
    t.list_active = gpui::rgb(0x2a2b30).into();
    t.list_active_border = gpui::transparent_black();
    t.selection = gpui::rgb(0x264f78).into();
    t.secondary_hover = gpui::rgb(0x3a3b40).into();
    t.secondary_active = gpui::rgb(0x4a4b50).into();
    t.secondary_foreground = gpui::rgb(0xd4d4d8).into();
    t.primary_hover = gpui::rgb(0x1a8ad4).into();

    t.foreground = gpui::rgb(0xd4d4d8).into();
    t.caret = gpui::rgb(0xffffff).into();
    t.muted_foreground = gpui::rgb(0x8e8e93).into();
    t.link = gpui::rgb(0x3794ff).into();
    t.accent_foreground = gpui::rgb(0xd4d4d8).into();

    t.primary = gpui::rgb(0x007acc).into();
    t.primary_foreground = gpui::rgb(0xffffff).into();
    t.success = gpui::rgb(0x4ec9b0).into();
    t.warning = gpui::rgb(0xcca700).into();
    t.danger = gpui::rgb(0xf44747).into();
    t.info = gpui::rgb(0x3794ff).into();

    t.status_bar = gpui::rgb(0x1a1b1d).into();

    t.scrollbar = gpui::transparent_black();
    t.scrollbar_thumb = gpui::rgb(0x555555).into();
    t.scrollbar_thumb_hover = gpui::rgb(0x666666).into();

    t.border = gpui::rgb(0x374151).into();
    t.drag_border = gpui::rgb(0x007acc).into();
    t.popover = gpui::rgb(0x2a2b32).into();
    t.popover_foreground = gpui::rgb(0xd4d4d8).into();
    t.accent = gpui::rgb(0x094771).into();
    t.ring = gpui::rgb(0x007acc).into();

    t.transparent = gpui::transparent_black();
    t.window_border = gpui::transparent_black();
    t.font_size = px(14.);
    t.scrollbar_show = gpui_component::scroll::ScrollbarShow::Scrolling;

    t.tokens = gpui_component::theme::ThemeTokens::from(&t.colors);
}

#[cfg(feature = "gpui-component")]
fn apply_light_theme_config(cx: &mut App) {
    use gpui::px;

    gpui_component::theme::Theme::sync_scrollbar_appearance(cx);
    let t = gpui_component::theme::Theme::global_mut(cx);

    t.highlight_theme = gpui_component::highlighter::HighlightTheme::default_light();

    t.background = gpui::rgb(0xffffff).into();
    t.secondary = gpui::rgb(0xf3f4f6).into();
    t.muted = gpui::rgb(0xf9fafb).into();
    t.title_bar = gpui::rgb(0xf3f4f6).into();
    t.title_bar_border = gpui::rgb(0xe5e7eb).into();
    t.sidebar = gpui::rgb(0xf3f4f6).into();
    t.sidebar_accent = gpui::rgb(0xe5e7eb).into();
    t.tab_bar = gpui::rgb(0xf3f4f6).into();
    t.tab_bar_segmented = gpui::rgb(0xf3f4f6).into();
    t.tab_foreground = gpui::rgb(0x374151).into();
    t.tab_active = gpui::rgb(0xffffff).into();
    t.tab_active_foreground = gpui::rgb(0x111827).into();
    t.colors.list = gpui::rgb(0xf9fafb).into();
    t.input = gpui::rgb(0xffffff).into();

    t.list_hover = gpui::rgb(0xe5e7eb).into();
    t.list_active = gpui::rgb(0xd1d5db).into();
    t.list_active_border = gpui::transparent_black();
    t.selection = gpui::rgb(0xadd6ff).into();
    t.secondary_hover = gpui::rgb(0xd1d5db).into();
    t.secondary_active = gpui::rgb(0x9ca3af).into();
    t.secondary_foreground = gpui::rgb(0x374151).into();
    t.primary_hover = gpui::rgb(0x1a8ad4).into();

    t.foreground = gpui::rgb(0x111827).into();
    t.caret = gpui::rgb(0x000000).into();
    t.muted_foreground = gpui::rgb(0x6b7280).into();
    t.link = gpui::rgb(0x2563eb).into();
    t.accent_foreground = gpui::rgb(0x374151).into();

    t.primary = gpui::rgb(0x007acc).into();
    t.primary_foreground = gpui::rgb(0xffffff).into();
    t.success = gpui::rgb(0x059669).into();
    t.warning = gpui::rgb(0xd97706).into();
    t.danger = gpui::rgb(0xdc2626).into();
    t.info = gpui::rgb(0x2563eb).into();

    t.status_bar = gpui::rgb(0xf3f4f6).into();

    t.scrollbar = gpui::rgb(0xf3f4f6).into();
    t.scrollbar_thumb = gpui::rgb(0x9ca3af).into();
    t.scrollbar_thumb_hover = gpui::rgb(0x6b7280).into();

    t.border = gpui::rgb(0xe5e7eb).into();
    t.drag_border = gpui::rgb(0x007acc).into();
    t.popover = gpui::rgb(0xffffff).into();
    t.popover_foreground = gpui::rgb(0x111827).into();
    t.accent = gpui::rgb(0xdbeafe).into();
    t.ring = gpui::rgb(0x007acc).into();

    t.transparent = gpui::transparent_black();
    t.window_border = gpui::rgb(0xe5e7eb).into();
    t.font_size = px(14.);
    t.scrollbar_show = gpui_component::scroll::ScrollbarShow::Scrolling;

    t.tokens = gpui_component::theme::ThemeTokens::from(&t.colors);
}

/// `Context` / `App` 主题扩展
pub trait ThemeExt {
    /// 指定主题目录并加载主题(同时设置默认目录供后续 `set_theme` 使用)
    fn use_theme_with_dir(&mut self, theme: impl AsRef<str>, dir: impl AsRef<str>);
    /// 加载(若未缓存)并切换到指定主题,刷新窗口
    fn set_theme(&mut self, theme: impl AsRef<str>);
    /// 从嵌入资源加载全局样式 CSS,提取 `:root` 颜色变量作为基础颜色
    ///
    /// 基础颜色作为主题颜色的默认值:当 `set_theme` 的主题未定义某变量时,
    /// `theme_color` / `color` 回退到 `set_style` 设置的基础颜色。
    /// `path` 为相对 `assets/` 根的路径(如 `"styles.css"`)。
    fn set_style(&mut self, path: impl AsRef<str>);
    /// 取当前主题下的颜色
    fn theme_color(&self, name: &str) -> Rgba;
    /// 当前主题名
    fn current_theme(&self) -> SharedString;
}

impl ThemeExt for App {
    fn use_theme_with_dir(&mut self, theme: impl AsRef<str>, dir: impl AsRef<str>) {
        let theme = theme.as_ref().to_string();
        let dir = dir.as_ref().to_string();
        ensure_theme(self);
        let (colors, vars) = merge_theme_with_builtin(&theme, &dir);
        self.update_global::<ThemeState, _>(|state, _| {
            state.set_dir(&dir);
            state.load_theme(&theme, colors);
            state.load_theme_vars(&theme, vars);
        });
        #[cfg(feature = "gpui-component")]
        apply_builtin_gpui_theme(&theme, self);
    }

    fn set_theme(&mut self, theme: impl AsRef<str>) {
        let theme = theme.as_ref().to_string();
        ensure_theme(self);
        let has_theme = self
            .read_global(|state: &ThemeState, _| state.themes.contains_key(&theme));
        if !has_theme {
            let dir = self.read_global(|state: &ThemeState, _| state.dir().to_string());
            let (colors, vars) = merge_theme_with_builtin(&theme, &dir);
            self.update_global::<ThemeState, _>(|state, _| {
                state.load_theme(&theme, colors);
                state.load_theme_vars(&theme, vars);
            });
        }
        let mut switched = false;
        self.update_global::<ThemeState, _>(|state, _| {
            switched = state.switch_theme(&theme);
        });
        if switched {
            #[cfg(feature = "gpui-component")]
            apply_builtin_gpui_theme(&theme, self);
            self.refresh_windows();
        }
    }

    fn set_style(&mut self, path: impl AsRef<str>) {
        let path = path.as_ref();
        let path = path.strip_prefix("assets/").unwrap_or(path);
        let css = match crate::assets::load_str(path) {
            Some(css) => css,
            None => {
                eprintln!("RML: style asset not embedded: {}", path);
                return;
            }
        };
        // 无 :root 块时返回空(样式文件可仅含类规则,变量由主题文件提供)
        let (base_colors, base_vars) = parse_theme_vars(css).unwrap_or_default();
        ensure_theme(self);
        self.update_global::<ThemeState, _>(|state, _| {
            state.set_base_colors(base_colors);
            state.set_base_vars(base_vars);
        });
        self.refresh_windows();
    }

    fn theme_color(&self, name: &str) -> Rgba {
        if self.has_global::<ThemeState>() {
            self.global::<ThemeState>()
                .color(name)
                .unwrap_or(Rgba {
                    r: 0.0,
                    g: 0.0,
                    b: 0.0,
                    a: 0.0,
                })
        } else {
            theme_color_static(name)
        }
    }

    fn current_theme(&self) -> SharedString {
        if self.has_global::<ThemeState>() {
            self.global::<ThemeState>().theme().into()
        } else {
            SharedString::default()
        }
    }
}

impl<T> ThemeExt for Context<'_, T> {
    fn use_theme_with_dir(&mut self, theme: impl AsRef<str>, dir: impl AsRef<str>) {
        ThemeExt::use_theme_with_dir(BorrowMut::<App>::borrow_mut(self), theme, dir);
    }

    fn set_theme(&mut self, theme: impl AsRef<str>) {
        ThemeExt::set_theme(BorrowMut::<App>::borrow_mut(self), theme);
    }

    fn set_style(&mut self, path: impl AsRef<str>) {
        ThemeExt::set_style(BorrowMut::<App>::borrow_mut(self), path);
    }

    fn theme_color(&self, name: &str) -> Rgba {
        ThemeExt::theme_color(Borrow::<App>::borrow(self), name)
    }

    fn current_theme(&self) -> SharedString {
        ThemeExt::current_theme(Borrow::<App>::borrow(self))
    }
}

/// 从嵌入资源加载并解析主题 CSS 颜色表
///
/// `dir` 为相对 cwd 的资源目录(如 `"assets/themes"`),内部去掉 `"assets/"` 前缀
/// 得到嵌入资源 key(如 `"themes/{theme}.css"`)。
/// 若资源未嵌入或 `assets` 模块未初始化,返回 Err。
pub fn load_theme_colors_embedded(
    theme: &str,
    dir: &str,
) -> Result<HashMap<String, Rgba>, String> {
    let (colors, _vars) = load_theme_vars_embedded(theme, dir)?;
    Ok(colors)
}

/// 从嵌入资源加载并解析主题 CSS 全部变量(颜色 + 非颜色)
///
/// 返回 `(colors, vars)`:颜色变量为 `Rgba`,非颜色变量为 `ThemeVar`。
pub fn load_theme_vars_embedded(
    theme: &str,
    dir: &str,
) -> Result<(HashMap<String, Rgba>, HashMap<String, ThemeVar>), String> {
    // 嵌入资源 key 是相对 assets/ 根的路径,去掉 "assets/" 前缀
    let sub_dir = dir.strip_prefix("assets/").unwrap_or(dir);
    let path = format!("{}/{}.css", sub_dir.trim_end_matches('/'), theme);
    let css = crate::assets::load_str(&path)
        .ok_or_else(|| format!("theme asset not embedded: {}", path))?;
    parse_theme_vars(css)
}

/// 解析主题 CSS,提取 `:root` 块中的 `#hex` 颜色变量
///
/// 仅识别 `--name: #hexvalue;` 形式的声明;非颜色变量忽略。
/// 支持 `#rgb`、`#rrggbb`、`#rrggbbaa` 三种 hex 格式。
///
/// 内部委托 [`parse_theme_vars`],仅返回颜色部分(向后兼容)。
pub fn parse_theme_css(css: &str) -> Result<HashMap<String, Rgba>, String> {
    let (colors, _vars) = parse_theme_vars(css)?;
    Ok(colors)
}

/// 解析主题 CSS,提取 `:root` 块中的所有变量(颜色 + 非颜色)
///
/// - `#hex` 值识别为颜色 → `HashMap<String, Rgba>`
/// - `Npx`/`Npt` 值识别为长度 → `HashMap<String, ThemeVar::Length>`（pt 按 4/3 换算为 px）
/// - 纯数字值识别为数字 → `HashMap<String, ThemeVar::Number>`
/// - 其他值忽略
pub fn parse_theme_vars(
    css: &str,
) -> Result<(HashMap<String, Rgba>, HashMap<String, ThemeVar>), String> {
    let mut colors = HashMap::new();
    let mut vars = HashMap::new();
    let root_block = extract_root_block(css)
        .ok_or_else(|| "no :root block found in theme css".to_string())?;
    for decl in root_block.split(';') {
        let decl = decl.trim();
        if !decl.starts_with("--") {
            continue;
        }
        if let Some(colon_idx) = decl.find(':') {
            let name = decl[..colon_idx].trim().to_string();
            let value = decl[colon_idx + 1..].trim();
            if let Some(rgba) = parse_css_color(value) {
                colors.insert(name, rgba);
            } else if let Some(v) = parse_theme_var_value(value) {
                vars.insert(name, v);
            }
            // 不识别的值忽略
        }
    }
    Ok((colors, vars))
}

/// 解析非颜色变量值: `Npx`/`Npt`/`Nem`/`Nrem` → Length, 纯数字 → Number
///
/// 单位换算与 mapper.rs 的 `length_method` 保持一致：
/// - `px` → 原样
/// - `pt` → n * 4/3（1pt ≈ 1.333px）
/// - `em`/`rem` → n * 16.0（基准字号 16px）
fn parse_theme_var_value(s: &str) -> Option<ThemeVar> {
    let s = s.trim();
    // px 长度
    if let Some(rest) = s.strip_suffix("px") {
        if let Ok(n) = rest.trim().parse::<f32>() {
            return Some(ThemeVar::Length(n));
        }
    }
    // pt 长度(1pt = 4/3 px)
    if let Some(rest) = s.strip_suffix("pt") {
        if let Ok(n) = rest.trim().parse::<f32>() {
            return Some(ThemeVar::Length(n * 4.0 / 3.0));
        }
    }
    // em/rem 长度(基准字号 16px)，先匹配 rem 再匹配 em，避免 "1rem" 误匹配 "em"
    if let Some(rest) = s.strip_suffix("rem").or_else(|| s.strip_suffix("em")) {
        if let Ok(n) = rest.trim().parse::<f32>() {
            return Some(ThemeVar::Length(n * 16.0));
        }
    }
    // 纯数字
    if let Ok(n) = s.parse::<f32>() {
        return Some(ThemeVar::Number(n));
    }
    None
}

/// 提取 `:root { ... }` 块内容
fn extract_root_block(css: &str) -> Option<&str> {
    let mut pos = 0;
    let bytes = css.as_bytes();
    while pos < bytes.len() {
        // 跳过注释
        if pos + 1 < bytes.len() && bytes[pos] == b'/' && bytes[pos + 1] == b'*' {
            pos += 2;
            while pos + 1 < bytes.len() {
                if bytes[pos] == b'*' && bytes[pos + 1] == b'/' {
                    pos += 2;
                    break;
                }
                pos += 1;
            }
            continue;
        }
        // 检测 :root
        if bytes[pos] == b':'
            && css[pos..].starts_with(":root")
            && pos + 5 <= bytes.len()
        {
            let after = &css[pos + 5..];
            let after_trimmed = after.trim_start();
            if let Some(open) = after_trimmed.find('{') {
                let block_start = after.len() - after_trimmed.len() + open + 1;
                let abs_start = pos + 5 + block_start;
                // 找到匹配的 }
                let mut depth = 1;
                let mut i = abs_start;
                while i < bytes.len() && depth > 0 {
                    if bytes[i] == b'{' {
                        depth += 1;
                    } else if bytes[i] == b'}' {
                        depth -= 1;
                        if depth == 0 {
                            return Some(&css[abs_start..i]);
                        }
                    }
                    i += 1;
                }
            }
        }
        pos += 1;
    }
    None
}

/// 解析 hex 颜色(#rgb / #rrggbb / #rrggbbaa)
fn parse_hex_color(s: &str) -> Option<Rgba> {
    let s = s.trim();
    if !s.starts_with('#') {
        return None;
    }
    let hex = &s[1..];
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            (r, g, b, 255u8)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            (r, g, b, 255u8)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            (r, g, b, a)
        }
        _ => return None,
    };
    Some(Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    })
}

/// 解析 CSS 颜色值：`#hex` / `rgb()` / `rgba()` / `hsl()` / `hsla()`
///
/// 与 mapper.rs 的 `function_to_color` 逻辑保持一致，但返回 GPUI 的 `Rgba`（f32 0.0-1.0）。
/// 用于 `parse_theme_vars` 解析 `:root` 中的颜色变量。
fn parse_css_color(s: &str) -> Option<Rgba> {
    let s = s.trim();
    if let Some(rgba) = parse_hex_color(s) {
        return Some(rgba);
    }
    let open = s.find('(')?;
    let close = s.rfind(')')?;
    if close <= open {
        return None;
    }
    let name = s[..open].trim();
    let args_str = &s[open + 1..close];
    let raw_args: Vec<&str> = args_str.split(',').map(|a| a.trim()).collect();

    match name {
        "rgb" if raw_args.len() == 3 => {
            let r = parse_rgb_channel(raw_args[0])?;
            let g = parse_rgb_channel(raw_args[1])?;
            let b = parse_rgb_channel(raw_args[2])?;
            Some(rgba_from_u8(r, g, b, 255))
        }
        "rgba" if raw_args.len() == 4 => {
            let r = parse_rgb_channel(raw_args[0])?;
            let g = parse_rgb_channel(raw_args[1])?;
            let b = parse_rgb_channel(raw_args[2])?;
            let a = parse_alpha(raw_args[3])?;
            Some(rgba_from_u8(r, g, b, a))
        }
        "hsl" if raw_args.len() == 3 => {
            let h = raw_args[0].parse::<f32>().ok()?;
            let sat = parse_percent_or_number(raw_args[1])?;
            let light = parse_percent_or_number(raw_args[2])?;
            let (r, g, b) = hsl_to_rgb(h, sat, light);
            Some(rgba_from_u8(r, g, b, 255))
        }
        "hsla" if raw_args.len() == 4 => {
            let h = raw_args[0].parse::<f32>().ok()?;
            let sat = parse_percent_or_number(raw_args[1])?;
            let light = parse_percent_or_number(raw_args[2])?;
            let a = parse_alpha(raw_args[3])?;
            let (r, g, b) = hsl_to_rgb(h, sat, light);
            Some(rgba_from_u8(r, g, b, a))
        }
        _ => None,
    }
}

/// 从 u8 通道构造 `Rgba`（f32 0.0-1.0）
fn rgba_from_u8(r: u8, g: u8, b: u8, a: u8) -> Rgba {
    Rgba {
        r: r as f32 / 255.0,
        g: g as f32 / 255.0,
        b: b as f32 / 255.0,
        a: a as f32 / 255.0,
    }
}

/// 解析 rgb 通道值：`255` → 255, `100%` → 255
fn parse_rgb_channel(s: &str) -> Option<u8> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let n = pct.trim().parse::<f32>().ok()?;
        return Some(clamp_u8(n * 2.55));
    }
    let n = s.parse::<f32>().ok()?;
    Some(clamp_u8(n))
}

/// 解析 alpha 值：`0.5` → 128, `50%` → 128
fn parse_alpha(s: &str) -> Option<u8> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let n = pct.trim().parse::<f32>().ok()?;
        return Some(clamp_u8(n * 2.55));
    }
    let n = s.parse::<f32>().ok()?;
    Some(clamp_u8(n * 255.0))
}

/// 解析百分比值：`50%` → 0.5, `0.5` → 0.005（CSS hsl 中 s/l 通常用百分号）
fn parse_percent_or_number(s: &str) -> Option<f32> {
    let s = s.trim();
    if let Some(pct) = s.strip_suffix('%') {
        let n = pct.trim().parse::<f32>().ok()?;
        return Some(n / 100.0);
    }
    s.parse::<f32>().ok().map(|n| n / 100.0)
}

/// HSL → RGB 转换（h: 0-360, s/l: 0.0-1.0）
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let h = h.rem_euclid(360.0) / 360.0;
    let s = s.clamp(0.0, 1.0);
    let l = l.clamp(0.0, 1.0);

    if s == 0.0 {
        let v = clamp_u8(l * 255.0);
        return (v, v, v);
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let hue_to_rgb = |t: f32| -> f32 {
        let mut t = t;
        if t < 0.0 { t += 1.0; }
        if t > 1.0 { t -= 1.0; }
        if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        }
    };

    let r = hue_to_rgb(h + 1.0 / 3.0);
    let g = hue_to_rgb(h);
    let b = hue_to_rgb(h - 1.0 / 3.0);

    (clamp_u8(r * 255.0), clamp_u8(g * 255.0), clamp_u8(b * 255.0))
}

/// 将 f32 钳位到 u8（0-255）
fn clamp_u8(n: f32) -> u8 {
    n.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hex_3_digit() {
        let c = parse_hex_color("#f00").unwrap();
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn parse_hex_6_digit() {
        let c = parse_hex_color("#007bff").unwrap();
        assert!((c.r - 0.0).abs() < 1e-6);
        assert!((c.g - 0.4823).abs() < 1e-3);
        assert!((c.b - 1.0).abs() < 1e-6);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn parse_hex_8_digit_with_alpha() {
        let c = parse_hex_color("#ff000080").unwrap();
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert!((c.a - 0.5019).abs() < 1e-3);
    }

    #[test]
    fn parse_theme_css_extracts_root_colors() {
        let css = r#"
            /* dark theme */
            :root {
                --primary-color: #007bff;
                --text-color: #333333;
                --bg-color: #f8f9fa;
            }
            .other { color: red; }
        "#;
        let colors = parse_theme_css(css).unwrap();
        assert_eq!(colors.len(), 3);
        assert!(colors.contains_key("--primary-color"));
        assert!(colors.contains_key("--text-color"));
        assert!(colors.contains_key("--bg-color"));
    }

    #[test]
    fn parse_theme_css_ignores_non_color_vars() {
        let css = r#"
            :root {
                --primary: #007bff;
                --spacing: 8px;
            }
        "#;
        let colors = parse_theme_css(css).unwrap();
        assert_eq!(colors.len(), 1);
        assert!(colors.contains_key("--primary"));
        assert!(!colors.contains_key("--spacing"));
    }

    #[test]
    fn extract_root_block_handles_nested_braces() {
        let css = ":root { --x: #fff; }";
        let block = extract_root_block(css).unwrap();
        assert!(block.contains("--x"));
    }

    #[test]
    fn theme_state_load_and_switch() {
        let mut state = ThemeState::default();
        let mut dark = HashMap::new();
        dark.insert("--bg".to_string(), Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 });
        let mut light = HashMap::new();
        light.insert("--bg".to_string(), Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 });

        state.load_theme("dark", dark);
        assert_eq!(state.theme(), "dark");
        assert!(state.color("--bg").unwrap().r < 0.5);

        state.load_theme("light", light);
        // load_theme 不改变已激活主题
        assert_eq!(state.theme(), "dark");

        assert!(state.switch_theme("light"));
        assert_eq!(state.theme(), "light");
        assert!(state.color("--bg").unwrap().r > 0.5);

        assert!(!state.switch_theme("nonexistent"));
    }

    #[test]
    fn base_colors_provide_defaults_theme_overrides() {
        let mut state = ThemeState::default();

        // set_style 加载基础颜色
        let mut base = HashMap::new();
        base.insert(
            "--bg".to_string(),
            Rgba { r: 1.0, g: 1.0, b: 1.0, a: 1.0 }, // white
        );
        base.insert(
            "--accent".to_string(),
            Rgba { r: 1.0, g: 0.0, b: 0.0, a: 1.0 }, // red
        );
        state.set_base_colors(base);

        // set_theme 加载主题颜色(仅定义 --bg,未定义 --accent)
        let mut dark = HashMap::new();
        dark.insert(
            "--bg".to_string(),
            Rgba { r: 0.0, g: 0.0, b: 0.0, a: 1.0 }, // black,覆盖基础
        );
        state.load_theme("dark", dark);

        // 主题颜色覆盖基础颜色
        assert!(state.color("--bg").unwrap().r < 0.5); // black from theme
        // 主题未定义的变量回退到基础颜色
        assert!(state.color("--accent").unwrap().r > 0.5); // red from base
    }

    // ─── ThemeVar / parse_theme_vars 测试 ───

    #[test]
    fn parse_theme_var_value_px() {
        assert_eq!(parse_theme_var_value("8px"), Some(ThemeVar::Length(8.0)));
        assert_eq!(parse_theme_var_value("0px"), Some(ThemeVar::Length(0.0)));
        assert_eq!(parse_theme_var_value("12.5px"), Some(ThemeVar::Length(12.5)));
    }

    #[test]
    fn parse_theme_var_value_pt() {
        // 1pt = 4/3 px
        let v = parse_theme_var_value("9pt").unwrap();
        match v {
            ThemeVar::Length(n) => assert!((n - 12.0).abs() < 1e-6),
            _ => panic!("expected Length"),
        }
    }

    #[test]
    fn parse_theme_var_value_number() {
        assert_eq!(parse_theme_var_value("0.5"), Some(ThemeVar::Number(0.5)));
        assert_eq!(parse_theme_var_value("10"), Some(ThemeVar::Number(10.0)));
    }

    #[test]
    fn parse_theme_var_value_rejects_unknown() {
        assert_eq!(parse_theme_var_value("red"), None);
        assert_eq!(parse_theme_var_value("50vw"), None);
        assert_eq!(parse_theme_var_value(""), None);
    }

    #[test]
    fn parse_theme_var_value_em_rem() {
        assert_eq!(parse_theme_var_value("1em"), Some(ThemeVar::Length(16.0)));
        assert_eq!(parse_theme_var_value("1rem"), Some(ThemeVar::Length(16.0)));
        assert_eq!(parse_theme_var_value("1.5em"), Some(ThemeVar::Length(24.0)));
        assert_eq!(parse_theme_var_value("0.5rem"), Some(ThemeVar::Length(8.0)));
    }

    #[test]
    fn parse_theme_vars_extracts_colors_and_vars() {
        let css = r#"
            :root {
                --primary: #007bff;
                --spacing: 8px;
                --opacity: 0.5;
            }
        "#;
        let (colors, vars) = parse_theme_vars(css).unwrap();
        assert_eq!(colors.len(), 1);
        assert!(colors.contains_key("--primary"));
        assert_eq!(vars.len(), 2);
        assert_eq!(vars.get("--spacing"), Some(&ThemeVar::Length(8.0)));
        assert_eq!(vars.get("--opacity"), Some(&ThemeVar::Number(0.5)));
    }

    #[test]
    fn parse_theme_vars_no_root_returns_err() {
        let css = ".foo { color: red; }";
        assert!(parse_theme_vars(css).is_err());
    }

    #[test]
    fn parse_theme_css_still_works_via_delegation() {
        let css = r#"
            :root {
                --primary: #007bff;
                --spacing: 8px;
            }
        "#;
        let colors = parse_theme_css(css).unwrap();
        assert_eq!(colors.len(), 1);
        assert!(colors.contains_key("--primary"));
    }

    // ─── ThemeState var 存储与查询测试 ───

    #[test]
    fn theme_state_set_base_vars_and_query() {
        let mut state = ThemeState::default();
        let mut base = HashMap::new();
        base.insert("--spacing".to_string(), ThemeVar::Length(8.0));
        base.insert("--opacity".to_string(), ThemeVar::Number(0.5));
        state.set_base_vars(base);

        assert_eq!(state.var("--spacing"), Some(ThemeVar::Length(8.0)));
        assert_eq!(state.var("--opacity"), Some(ThemeVar::Number(0.5)));
        assert_eq!(state.var("--undefined"), None);
    }

    #[test]
    fn theme_state_load_theme_vars_overrides_base() {
        let mut state = ThemeState::default();

        // 基础变量
        let mut base = HashMap::new();
        base.insert("--spacing".to_string(), ThemeVar::Length(8.0));
        base.insert("--opacity".to_string(), ThemeVar::Number(0.5));
        state.set_base_vars(base);

        // 主题变量(覆盖 --spacing,未定义 --opacity)
        let mut dark_vars = HashMap::new();
        dark_vars.insert("--spacing".to_string(), ThemeVar::Length(16.0));
        state.load_theme_vars("dark", dark_vars);

        // 未激活主题 → var 仍为基础值
        assert_eq!(state.var("--spacing"), Some(ThemeVar::Length(8.0)));

        // 激活主题(load_theme 同步激活)
        let dark_colors: HashMap<String, Rgba> = HashMap::new();
        state.load_theme("dark", dark_colors);

        // 主题变量覆盖基础变量
        assert_eq!(state.var("--spacing"), Some(ThemeVar::Length(16.0)));
        // 主题未定义的变量回退到基础
        assert_eq!(state.var("--opacity"), Some(ThemeVar::Number(0.5)));
    }

    #[test]
    fn theme_state_switch_theme_updates_vars() {
        let mut state = ThemeState::default();

        let mut base = HashMap::new();
        base.insert("--spacing".to_string(), ThemeVar::Length(8.0));
        state.set_base_vars(base);

        let dark_colors: HashMap<String, Rgba> = HashMap::new();
        let mut dark_vars = HashMap::new();
        dark_vars.insert("--spacing".to_string(), ThemeVar::Length(16.0));
        state.load_theme("dark", dark_colors);
        state.load_theme_vars("dark", dark_vars);

        let light_colors: HashMap<String, Rgba> = HashMap::new();
        let mut light_vars = HashMap::new();
        light_vars.insert("--spacing".to_string(), ThemeVar::Length(4.0));
        state.load_theme("light", light_colors);
        state.load_theme_vars("light", light_vars);

        // 当前主题为 dark
        assert_eq!(state.var("--spacing"), Some(ThemeVar::Length(16.0)));

        // 切换到 light
        assert!(state.switch_theme("light"));
        assert_eq!(state.var("--spacing"), Some(ThemeVar::Length(4.0)));

        // 切换回 dark
        assert!(state.switch_theme("dark"));
        assert_eq!(state.var("--spacing"), Some(ThemeVar::Length(16.0)));
    }

    // ─── 颜色函数解析测试 ───

    #[test]
    fn parse_css_color_rgb() {
        let c = parse_css_color("rgb(255, 0, 0)").unwrap();
        assert_eq!(c.r, 1.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 0.0);
        assert_eq!(c.a, 1.0);
    }

    #[test]
    fn parse_css_color_rgba() {
        let c = parse_css_color("rgba(0, 255, 0, 0.5)").unwrap();
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 1.0);
        assert_eq!(c.b, 0.0);
        assert!((c.a - 0.5019).abs() < 1e-3);
    }

    #[test]
    fn parse_css_color_rgb_percent() {
        let c = parse_css_color("rgb(100%, 50%, 0%)").unwrap();
        assert_eq!(c.r, 1.0);
        assert!((c.g - 0.5019).abs() < 1e-3);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn parse_css_color_hsl() {
        let c = parse_css_color("hsl(120, 100%, 50%)").unwrap();
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 1.0);
        assert_eq!(c.b, 0.0);
    }

    #[test]
    fn parse_css_color_hsla() {
        let c = parse_css_color("hsla(240, 100%, 50%, 0.5)").unwrap();
        assert_eq!(c.r, 0.0);
        assert_eq!(c.g, 0.0);
        assert_eq!(c.b, 1.0);
        assert!((c.a - 0.5019).abs() < 1e-3);
    }

    #[test]
    fn parse_css_color_hex_still_works() {
        let c = parse_css_color("#0066cc").unwrap();
        assert_eq!(c.r, 0.0);
        assert!((c.g - 0.4).abs() < 1e-3);
        assert_eq!(c.b, 0.8);
    }

    #[test]
    fn parse_css_color_rejects_invalid() {
        assert_eq!(parse_css_color("red"), None);
        assert_eq!(parse_css_color("rgb(1, 2)"), None);
        assert_eq!(parse_css_color(""), None);
    }

    #[test]
    fn parse_theme_vars_extracts_rgb_color() {
        let css = r#"
            :root {
                --primary: rgb(0, 102, 204);
                --accent: rgba(255, 0, 0, 0.5);
            }
        "#;
        let (colors, _vars) = parse_theme_vars(css).unwrap();
        assert_eq!(colors.len(), 2);
        assert!(colors.contains_key("--primary"));
        assert!(colors.contains_key("--accent"));
        let primary = colors.get("--primary").unwrap();
        assert!((primary.b - 0.8).abs() < 1e-3);
    }

    #[test]
    fn parse_theme_vars_extracts_hsl_color() {
        let css = r#"
            :root {
                --brand: hsl(280, 100%, 50%);
            }
        "#;
        let (colors, _vars) = parse_theme_vars(css).unwrap();
        assert_eq!(colors.len(), 1);
        assert!(colors.contains_key("--brand"));
    }

    #[test]
    fn parse_theme_vars_extracts_em_rem_length() {
        let css = r#"
            :root {
                --base-spacing: 1rem;
                --large-spacing: 2em;
            }
        "#;
        let (_colors, vars) = parse_theme_vars(css).unwrap();
        assert_eq!(vars.get("--base-spacing"), Some(&ThemeVar::Length(16.0)));
        assert_eq!(vars.get("--large-spacing"), Some(&ThemeVar::Length(32.0)));
    }

    #[test]
    fn hsl_to_rgb_pure_gray() {
        let (r, g, b) = hsl_to_rgb(0.0, 0.0, 0.5);
        assert_eq!((r, g, b), (128, 128, 128));
    }

    #[test]
    fn hsl_to_rgb_hue_wraparound() {
        // 480° == 120° → green
        let (r, g, b) = hsl_to_rgb(480.0, 1.0, 0.5);
        assert_eq!((r, g, b), (0, 255, 0));
    }
}
