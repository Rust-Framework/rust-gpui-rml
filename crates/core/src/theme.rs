//! 主题系统 —— `ThemeState` Global + `ThemeExt` 扩展
//!
//! 开发体验与 [`crate::i18n`] 完全对齐:
//! - `cx.set_style("styles.css")` 加载全局样式 CSS 的 `:root` 变量作为基础值（颜色 + 长度 + 数字）
//! - `cx.set_theme("dark")` 加载/切换主题(主题变量覆盖基础变量)
//! - `cx.theme_color("--primary")` 取当前生效颜色(基础 + 主题)
//! - `theme_color_static("--primary")` 供 `#[computed]` 等无 `App` 上下文场景使用
//! - `rml::theme::length("--spacing")` / `rml::theme::number("--opacity")` 取非颜色变量
//!
//! ## CSS 变量分层
//!
//! **基础变量**（用户 / 主题 CSS 可直接覆盖）:
//! `--primary`, `--background`, `--foreground`, `--secondary`, `--border`, `--muted`, …
//!
//! **派生变量**（[`derive_theme_colors`] 按 gpui-component 语义自动补全,主题 CSS 仍可覆盖）:
//! `--primary-hover`, `--button-secondary`, `--group-box`, `--list-hover`, `--card-bg`, …
//!
//! ## 表面层级（VS Code / Ant Design 语义, dark 示例）
//!
//! - **L0 chrome**: `--secondary` / `--title-bar` (#1a1b1d) — 标题栏、活动栏
//! - **L1 background**: `--background` (#222427) — 主内容区、卡片默认同级
//! - **L2 surface**: `--surface` / `--group-box` / `--list-hover` (#2a2b30) — 分组框、嵌套面板、列表悬停
//! - **L3 elevated**: `--list-active` / `--accordion-hover` (#33353a) — L2 上的 hover/选中
//! - **Border**: `--border` (#374151) —  subtle 分隔,非强对比
//! - **Primary**: `--primary` (#007acc) — 仅用于 focus/selection 点缀,不作容器大面积背景
//!
//! 合并顺序: 内置 light/dark 默认 → 主题 CSS 覆盖 → 派生补全(仅填充缺失键)。
//! 自定义主题包(如 `ocean.css`)只需覆盖基础变量,派生色自动计算;也可显式覆盖任意派生变量。
//!
//! 变量优先级（高 → 低）: 主题 CSS 文件 > 派生默认 > 框架内置默认 > `set_style` 基础变量。
//!
//! 启用 `gpui-component` feature 后,`set_theme` 还会同步 gpui-component 原生主题
//! （Button/Input/DescriptionList 等组件的配色），无需开发者手动调用。
//!
//! 主题文件格式(`assets/themes/dark.css`):
//! ```css
//! :root {
//!     --primary: #007bff;
//!     --background: #222427;
//!     --button-secondary: #2a2b30;  /* 可选:显式覆盖派生变量 */
//!     --spacing: 8px;
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

    /// 当前生效的合并颜色表（base + 主题覆盖）
    pub fn colors(&self) -> &HashMap<String, Rgba> {
        &self.colors
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

/// 批量写入内置 CSS 颜色变量
fn insert_builtin(m: &mut HashMap<String, Rgba>, vars: &[(&str, u32)]) {
    for (name, hex) in vars {
        m.insert(name.to_string(), rgba_from_hex(*hex));
    }
}

const RGBA_WHITE: Rgba = Rgba {
    r: 1.0,
    g: 1.0,
    b: 1.0,
    a: 1.0,
};
const RGBA_BLACK: Rgba = Rgba {
    r: 0.0,
    g: 0.0,
    b: 0.0,
    a: 1.0,
};

/// 线性混合两色
fn mix_rgba(a: Rgba, b: Rgba, t: f32) -> Rgba {
    let t = t.clamp(0.0, 1.0);
    Rgba {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}

fn darken_rgba(c: Rgba, amount: f32) -> Rgba {
    mix_rgba(c, RGBA_BLACK, amount)
}

fn lighten_rgba(c: Rgba, amount: f32) -> Rgba {
    mix_rgba(c, RGBA_WHITE, amount)
}

/// 在背景上叠加半透明前景
fn tint_rgba(bg: Rgba, fg: Rgba, fg_opacity: f32) -> Rgba {
    mix_rgba(bg, fg, fg_opacity.clamp(0.0, 1.0))
}

/// sRGB 相对亮度 (0.0–1.0)
fn relative_luminance(c: Rgba) -> f32 {
    fn channel(v: f32) -> f32 {
        if v <= 0.03928 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    }
    0.2126 * channel(c.r) + 0.7152 * channel(c.g) + 0.0722 * channel(c.b)
}

fn color_from_map<'a>(colors: &'a HashMap<String, Rgba>, key: &str, fallback: Rgba) -> Rgba {
    colors.get(key).copied().unwrap_or(fallback)
}

fn insert_derived(colors: &mut HashMap<String, Rgba>, key: &str, value: Rgba) {
    colors.entry(key.to_string()).or_insert(value);
}

/// 判断当前主题是否为暗色: 优先主题名,其次 `--background` 亮度
pub fn theme_is_dark(theme: &str, colors: &HashMap<String, Rgba>) -> bool {
    match theme {
        "dark" => true,
        "light" => false,
        _ => colors
            .get("--background")
            .map(|c| relative_luminance(*c) < 0.45)
            .unwrap_or(false),
    }
}

/// 从基础 CSS 变量派生 gpui-component 常用 token,仅填充缺失键
pub fn derive_theme_colors(base: &HashMap<String, Rgba>, is_dark: bool) -> HashMap<String, Rgba> {
    let mut colors = base.clone();

    let background = color_from_map(&colors, "--background", if is_dark {
        rgba_from_hex(0x222427)
    } else {
        rgba_from_hex(0xffffff)
    });
    let foreground = color_from_map(&colors, "--foreground", if is_dark {
        rgba_from_hex(0xd4d4d8)
    } else {
        rgba_from_hex(0x111827)
    });
    let primary = color_from_map(&colors, "--primary", rgba_from_hex(0x007acc));
    let primary_fg = color_from_map(&colors, "--primary-foreground", RGBA_WHITE);
    let secondary = color_from_map(
        &colors,
        "--secondary",
        if is_dark {
            rgba_from_hex(0x1a1b1d)
        } else {
            rgba_from_hex(0xf3f4f6)
        },
    );
    let secondary_fg = color_from_map(&colors, "--secondary-foreground", foreground);
    let muted_fg = color_from_map(
        &colors,
        "--muted-foreground",
        if is_dark {
            rgba_from_hex(0x8e8e93)
        } else {
            rgba_from_hex(0x6b7280)
        },
    );
    let border = color_from_map(
        &colors,
        "--border",
        if is_dark {
            rgba_from_hex(0x374151)
        } else {
            rgba_from_hex(0xe5e7eb)
        },
    );
    let input = color_from_map(
        &colors,
        "--input",
        if is_dark {
            rgba_from_hex(0x2e3035)
        } else {
            rgba_from_hex(0xffffff)
        },
    );
    let danger = color_from_map(
        &colors,
        "--danger",
        if is_dark {
            rgba_from_hex(0xf44747)
        } else {
            rgba_from_hex(0xdc2626)
        },
    );
    let success = color_from_map(
        &colors,
        "--success",
        if is_dark {
            rgba_from_hex(0x4ec9b0)
        } else {
            rgba_from_hex(0x059669)
        },
    );
    let warning = color_from_map(
        &colors,
        "--warning",
        if is_dark {
            rgba_from_hex(0xcca700)
        } else {
            rgba_from_hex(0xd97706)
        },
    );
    let info = color_from_map(
        &colors,
        "--info",
        if is_dark {
            rgba_from_hex(0x3794ff)
        } else {
            rgba_from_hex(0x2563eb)
        },
    );
    let link = color_from_map(
        &colors,
        "--link",
        if is_dark {
            rgba_from_hex(0x3794ff)
        } else {
            rgba_from_hex(0x2563eb)
        },
    );

    let active_darken = if is_dark { 0.2 } else { 0.1 };
    let primary_hover = tint_rgba(background, primary, 0.9);
    let primary_active = darken_rgba(primary, active_darken);
    // Lift secondary_hover above title_bar/secondary surfaces so title-bar controls
    // and Secondary buttons show a visible hover (gpui default ≈ #292929 dark / #d1d5db light).
    let secondary_hover = if is_dark {
        lighten_rgba(secondary, 0.3)
    } else {
        darken_rgba(secondary, 0.1)
    };
    let secondary_active = darken_rgba(secondary, active_darken);

    insert_derived(&mut colors, "--primary-hover", primary_hover);
    insert_derived(&mut colors, "--primary-active", primary_active);
    insert_derived(&mut colors, "--secondary-hover", secondary_hover);
    insert_derived(&mut colors, "--secondary-active", secondary_active);

    insert_derived(
        &mut colors,
        "--danger-hover",
        tint_rgba(background, danger, 0.9),
    );
    insert_derived(&mut colors, "--danger-active", darken_rgba(danger, active_darken));
    insert_derived(
        &mut colors,
        "--success-hover",
        tint_rgba(background, success, 0.9),
    );
    insert_derived(
        &mut colors,
        "--success-active",
        darken_rgba(success, active_darken),
    );
    insert_derived(
        &mut colors,
        "--warning-hover",
        tint_rgba(background, warning, 0.9),
    );
    insert_derived(
        &mut colors,
        "--warning-active",
        darken_rgba(warning, active_darken),
    );
    insert_derived(&mut colors, "--info-hover", tint_rgba(background, info, 0.9));
    insert_derived(&mut colors, "--info-active", darken_rgba(info, active_darken));

    insert_derived(
        &mut colors,
        "--link-hover",
        if is_dark {
            lighten_rgba(link, 0.15)
        } else {
            darken_rgba(link, 0.15)
        },
    );
    insert_derived(
        &mut colors,
        "--link-active",
        if is_dark {
            darken_rgba(link, 0.1)
        } else {
            darken_rgba(link, 0.25)
        },
    );

    // Default / Secondary button surfaces
    let button = if is_dark {
        tint_rgba(background, input, 0.3)
    } else {
        background
    };
    let button_hover = if is_dark {
        tint_rgba(background, input, 0.5)
    } else {
        tint_rgba(background, input, 0.5)
    };
    let button_active = if is_dark {
        tint_rgba(background, input, 0.7)
    } else {
        tint_rgba(background, input, 0.7)
    };

    insert_derived(&mut colors, "--button", button);
    insert_derived(&mut colors, "--button-foreground", foreground);
    insert_derived(&mut colors, "--button-hover", button_hover);
    insert_derived(&mut colors, "--button-active", button_active);

    insert_derived(&mut colors, "--button-primary", primary);
    insert_derived(&mut colors, "--button-primary-foreground", primary_fg);
    insert_derived(&mut colors, "--button-primary-hover", primary_hover);
    insert_derived(&mut colors, "--button-primary-active", primary_active);

    insert_derived(&mut colors, "--button-secondary", secondary);
    insert_derived(&mut colors, "--button-secondary-foreground", secondary_fg);
    insert_derived(&mut colors, "--button-secondary-hover", secondary_hover);
    insert_derived(&mut colors, "--button-secondary-active", secondary_active);

    insert_derived(&mut colors, "--button-danger", tint_rgba(background, danger, 0.2));
    insert_derived(&mut colors, "--button-danger-foreground", danger);
    insert_derived(
        &mut colors,
        "--button-danger-hover",
        tint_rgba(background, danger, 0.3),
    );
    insert_derived(
        &mut colors,
        "--button-danger-active",
        tint_rgba(background, danger, 0.4),
    );

    insert_derived(&mut colors, "--button-success", tint_rgba(background, success, 0.2));
    insert_derived(&mut colors, "--button-success-foreground", success);
    insert_derived(
        &mut colors,
        "--button-success-hover",
        tint_rgba(background, success, 0.3),
    );
    insert_derived(
        &mut colors,
        "--button-success-active",
        tint_rgba(background, success, 0.4),
    );

    insert_derived(&mut colors, "--button-warning", tint_rgba(background, warning, 0.2));
    insert_derived(&mut colors, "--button-warning-foreground", warning);
    insert_derived(
        &mut colors,
        "--button-warning-hover",
        tint_rgba(background, warning, 0.3),
    );
    insert_derived(
        &mut colors,
        "--button-warning-active",
        tint_rgba(background, warning, 0.4),
    );

    insert_derived(&mut colors, "--button-info", tint_rgba(background, info, 0.2));
    insert_derived(&mut colors, "--button-info-foreground", info);
    insert_derived(
        &mut colors,
        "--button-info-hover",
        tint_rgba(background, info, 0.3),
    );
    insert_derived(
        &mut colors,
        "--button-info-active",
        tint_rgba(background, info, 0.4),
    );

    insert_derived(
        &mut colors,
        "--description-list-label",
        tint_rgba(background, border, 0.2),
    );
    insert_derived(
        &mut colors,
        "--description-list-label-foreground",
        muted_fg,
    );

    // Surface hierarchy (L2 / L3) — group box, list, card, accordion
    let surface = color_from_map(
        &colors,
        "--surface",
        if is_dark {
            rgba_from_hex(0x2a2b30)
        } else {
            rgba_from_hex(0xf3f4f6)
        },
    );
    let surface_elevated = color_from_map(
        &colors,
        "--surface-elevated",
        if is_dark {
            rgba_from_hex(0x33353a)
        } else {
            rgba_from_hex(0xe5e7eb)
        },
    );
    let list_hover = color_from_map(
        &colors,
        "--list-hover",
        if is_dark {
            rgba_from_hex(0x2a2b30)
        } else {
            rgba_from_hex(0xf3f4f6)
        },
    );
    let list_active = color_from_map(
        &colors,
        "--list-active",
        if is_dark {
            rgba_from_hex(0x33353a)
        } else {
            rgba_from_hex(0xe5e7eb)
        },
    );
    let list_active_border = color_from_map(
        &colors,
        "--list-active-border",
        if is_dark {
            tint_rgba(border, primary, 0.35)
        } else {
            tint_rgba(border, primary, 0.25)
        },
    );

    insert_derived(&mut colors, "--list-hover", list_hover);
    insert_derived(&mut colors, "--list-active", list_active);
    insert_derived(&mut colors, "--list-active-border", list_active_border);
    // Tree / Select dropdown list reuse list surface tokens (L2 hover / L3 active)
    insert_derived(&mut colors, "--tree-hover", list_hover);
    insert_derived(&mut colors, "--tree-active", list_active);

    // GroupBox (gpui: group_box.background, group_box.foreground)
    insert_derived(&mut colors, "--group-box", surface);
    insert_derived(&mut colors, "--group-box-foreground", foreground);
    insert_derived(&mut colors, "--group-box-border", border);

    // Card — L2 elevation on dark, L1 + border on light
    insert_derived(
        &mut colors,
        "--card-bg",
        if is_dark { surface } else { background },
    );
    insert_derived(&mut colors, "--card-border", border);

    // Accordion / Collapse — L2 base, L3 hover (~5–10% luminance shift, never primary fill)
    insert_derived(
        &mut colors,
        "--accordion",
        if is_dark { surface } else { background },
    );
    insert_derived(&mut colors, "--accordion-hover", surface_elevated);

    // Slider — neutral track (never primary fill), thumb contrasts track
    let slider_bar = color_from_map(
        &colors,
        "--slider-bar",
        if is_dark {
            tint_rgba(border, foreground, 0.45)
        } else {
            tint_rgba(border, foreground, 0.25)
        },
    );
    let slider_thumb = color_from_map(
        &colors,
        "--slider-thumb",
        if is_dark {
            foreground
        } else {
            background
        },
    );
    insert_derived(&mut colors, "--slider-bar", slider_bar);
    insert_derived(&mut colors, "--slider-thumb", slider_thumb);

    // Switch — L2 track off-state, L1 thumb (checked fill uses primary in component)
    let switch_track = color_from_map(
        &colors,
        "--switch",
        if is_dark { surface } else { surface_elevated },
    );
    let switch_thumb = color_from_map(
        &colors,
        "--switch-thumb",
        if is_dark { background } else { background },
    );
    insert_derived(&mut colors, "--switch", switch_track);
    insert_derived(&mut colors, "--switch-thumb", switch_thumb);

    // Progress bar / circular progress (gpui: progress.bar.background → primary)
    insert_derived(&mut colors, "--progress-bar", primary);

    // Title-bar icon buttons (min/max/close, chrome toggle) — hover lift on title_bar bg
    insert_derived(
        &mut colors,
        "--title-bar-button-hover",
        secondary_hover,
    );
    insert_derived(
        &mut colors,
        "--title-bar-button-active",
        secondary_active,
    );
    insert_derived(
        &mut colors,
        "--title-bar-button-foreground",
        secondary_fg,
    );

    colors
}

/// 内置 light 主题默认 CSS 变量颜色（与 `apply_light_theme_config` 语义 token 对齐）
fn builtin_light_colors() -> HashMap<String, Rgba> {
    let mut m = HashMap::new();
    insert_builtin(
        &mut m,
        &[
            // 基础
            ("--primary", 0x007acc),
            ("--primary-foreground", 0xffffff),
            ("--background", 0xffffff),
            ("--foreground", 0x111827),
            ("--text", 0x111827),
            ("--text-muted", 0x6b7280),
            ("--muted-foreground", 0x6b7280),
            ("--border", 0xe5e7eb),
            ("--ring", 0x007acc),
            // 表面层级
            ("--surface", 0xf3f4f6),
            ("--surface-variant", 0xf9fafb),
            ("--muted", 0xf9fafb),
            ("--secondary", 0xf3f4f6),
            ("--secondary-foreground", 0x374151),
            ("--accent", 0xdbeafe),
            ("--accent-foreground", 0x374151),
            ("--code-bg", 0xf1f3f5),
            ("--chrome-surface", 0xf3f4f6),
            ("--editor-surface", 0xffffff),
            // 壳层
            ("--title-bar", 0xf3f4f6),
            ("--title-bar-border", 0xe5e7eb),
            ("--status-bar", 0xf3f4f6),
            ("--status-bar-border", 0xe5e7eb),
            ("--sidebar", 0xf3f4f6),
            ("--sidebar-foreground", 0x374151),
            // 标签页
            ("--tab-bar", 0xf3f4f6),
            ("--tab-foreground", 0x374151),
            ("--tab-active", 0xffffff),
            ("--tab-active-foreground", 0x111827),
            // 列表 / 表格 (L1 base → L2 hover → L3 active)
            ("--list", 0xffffff),
            ("--list-hover", 0xf3f4f6),
            ("--list-active", 0xe5e7eb),
            ("--list-active-border", 0x007acc),
            // 容器表面
            ("--group-box", 0xf9fafb),
            ("--group-box-foreground", 0x111827),
            ("--card-bg", 0xffffff),
            ("--card-border", 0xe5e7eb),
            ("--accordion", 0xffffff),
            ("--accordion-hover", 0xf3f4f6),
            ("--slider-bar", 0xd1d5db),
            ("--slider-thumb", 0xffffff),
            ("--switch", 0xe5e7eb),
            ("--switch-thumb", 0xffffff),
            ("--surface-elevated", 0xe5e7eb),
            ("--table-head", 0xf9fafb),
            ("--table-head-foreground", 0x6b7280),
            ("--table-even", 0xf9fafb),
            // 浮层 / 输入
            ("--popover", 0xffffff),
            ("--popover-foreground", 0x111827),
            ("--input", 0xffffff),
            ("--selection", 0xadd6ff),
            ("--link", 0x2563eb),
            // 语义色
            ("--success", 0x059669),
            ("--success-foreground", 0xffffff),
            ("--warning", 0xd97706),
            ("--warning-foreground", 0xffffff),
            ("--error", 0xdc2626),
            ("--danger", 0xdc2626),
            ("--danger-foreground", 0xffffff),
            ("--info", 0x2563eb),
            ("--info-foreground", 0xffffff),
            // 滚动条
            ("--scrollbar", 0xf3f4f6),
            ("--scrollbar-thumb", 0x9ca3af),
        ],
    );
    m
}

/// 内置 dark 主题默认 CSS 变量颜色（与 `apply_dark_theme_config` 语义 token 对齐）
fn builtin_dark_colors() -> HashMap<String, Rgba> {
    let mut m = HashMap::new();
    insert_builtin(
        &mut m,
        &[
            // 基础
            ("--primary", 0x007acc),
            ("--primary-foreground", 0xffffff),
            ("--background", 0x222427),
            ("--foreground", 0xd4d4d8),
            ("--text", 0xd4d4d8),
            ("--text-muted", 0x8e8e93),
            ("--muted-foreground", 0x8e8e93),
            ("--border", 0x374151),
            ("--ring", 0x007acc),
            // 表面层级
            ("--surface", 0x2a2b30),
            ("--surface-variant", 0x1a1b1d),
            ("--muted", 0x2a2b30),
            ("--secondary", 0x1a1b1d),
            ("--secondary-foreground", 0xd4d4d8),
            ("--accent", 0x094771),
            ("--accent-foreground", 0xd4d4d8),
            ("--code-bg", 0x1a1b1d),
            ("--chrome-surface", 0x1a1b1d),
            ("--editor-surface", 0x222427),
            // 壳层
            ("--title-bar", 0x1a1b1d),
            ("--title-bar-border", 0x0f1012),
            ("--status-bar", 0x1a1b1d),
            ("--status-bar-border", 0x0f1012),
            ("--sidebar", 0x1a1b1d),
            ("--sidebar-foreground", 0xd4d4d8),
            // 标签页
            ("--tab-bar", 0x1a1b1d),
            ("--tab-foreground", 0xd4d4d8),
            ("--tab-active", 0x222427),
            ("--tab-active-foreground", 0xffffff),
            // 列表 / 表格 (L1 base → L2 hover → L3 active)
            ("--list", 0x222427),
            ("--list-hover", 0x2a2b30),
            ("--list-active", 0x33353a),
            ("--list-active-border", 0x264f78),
            // 容器表面
            ("--group-box", 0x2a2b30),
            ("--group-box-foreground", 0xd4d4d8),
            ("--card-bg", 0x2a2b30),
            ("--card-border", 0x374151),
            ("--accordion", 0x2a2b30),
            ("--accordion-hover", 0x33353a),
            ("--slider-bar", 0x4b5563),
            ("--slider-thumb", 0xd4d4d8),
            ("--switch", 0x2a2b30),
            ("--switch-thumb", 0x222427),
            ("--surface-elevated", 0x33353a),
            ("--table-head", 0x25262a),
            ("--table-head-foreground", 0x8e8e93),
            ("--table-even", 0x25262a),
            // 浮层 / 输入
            ("--popover", 0x2a2b32),
            ("--popover-foreground", 0xd4d4d8),
            ("--input", 0x2e3035),
            ("--selection", 0x264f78),
            ("--link", 0x3794ff),
            // 语义色
            ("--success", 0x4ec9b0),
            ("--success-foreground", 0xffffff),
            ("--warning", 0xcca700),
            ("--warning-foreground", 0xffffff),
            ("--error", 0xf44747),
            ("--danger", 0xf44747),
            ("--danger-foreground", 0xffffff),
            ("--info", 0x3794ff),
            ("--info-foreground", 0xffffff),
            // 滚动条
            ("--scrollbar", 0x1a1b1d),
            ("--scrollbar-thumb", 0x555555),
        ],
    );
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

/// 以内置默认为底,叠加 CSS 文件提供的颜色与变量,派生缺失 token,返回合并结果
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
        vars = css_vars;
    }

    // 自定义主题包: 用 CSS 覆盖 + 按亮度选 light/dark 内置补全缺失基础变量
    if theme != "light" && theme != "dark" {
        let is_dark = theme_is_dark(theme, &colors);
        let fallback = if is_dark {
            builtin_dark_colors()
        } else {
            builtin_light_colors()
        };
        for (k, v) in fallback {
            colors.entry(k).or_insert(v);
        }
    }

    let is_dark = theme_is_dark(theme, &colors);
    colors = derive_theme_colors(&colors, is_dark);
    (colors, vars)
}

// ─── gpui-component 原生主题同步（feature-gated） ───

#[cfg(feature = "gpui-component")]
fn css_color(colors: &HashMap<String, Rgba>, key: &str, fallback: u32) -> gpui::Hsla {
    colors
        .get(key)
        .copied()
        .unwrap_or_else(|| rgba_from_hex(fallback))
        .into()
}

/// 将合并后的 CSS 颜色变量同步到 gpui-component `Theme` 字段
#[cfg(feature = "gpui-component")]
fn apply_gpui_theme_from_colors(theme: &str, colors: &HashMap<String, Rgba>, cx: &mut App) {
    use gpui::px;
    use gpui_component::theme::ThemeMode;

    let is_dark = theme_is_dark(theme, colors);

    gpui_component::theme::Theme::sync_scrollbar_appearance(cx);
    let t = gpui_component::theme::Theme::global_mut(cx);

    t.mode = if is_dark {
        ThemeMode::Dark
    } else {
        ThemeMode::Light
    };

    t.highlight_theme = if is_dark {
        gpui_component::highlighter::HighlightTheme::default_dark()
    } else {
        gpui_component::highlighter::HighlightTheme::default_light()
    };

    let c = |key: &str, fallback: u32| css_color(colors, key, fallback);

    t.background = c("--background", if is_dark { 0x222427 } else { 0xffffff });
    t.foreground = c("--foreground", if is_dark { 0xd4d4d8 } else { 0x111827 });
    t.secondary = c("--secondary", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.secondary_hover = c("--secondary-hover", if is_dark { 0x292929 } else { 0xd1d5db });
    t.secondary_active = c(
        "--secondary-active",
        if is_dark { 0x212121 } else { 0x9ca3af },
    );
    t.secondary_foreground = c(
        "--secondary-foreground",
        if is_dark { 0xd4d4d8 } else { 0x374151 },
    );
    t.muted = c("--muted", if is_dark { 0x2a2b30 } else { 0xf9fafb });
    t.muted_foreground = c(
        "--muted-foreground",
        if is_dark { 0x8e8e93 } else { 0x6b7280 },
    );

    t.title_bar = c("--title-bar", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.title_bar_border = c(
        "--title-bar-border",
        if is_dark { 0x0f1012 } else { 0xe5e7eb },
    );
    t.status_bar = c("--status-bar", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.status_bar_border = c(
        "--status-bar-border",
        if is_dark { 0x0f1012 } else { 0xe5e7eb },
    );

    t.sidebar = c("--sidebar", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.sidebar_accent = c(
        "--list-hover",
        if is_dark { 0x2a2b30 } else { 0xf3f4f6 },
    );
    t.sidebar_foreground = c(
        "--sidebar-foreground",
        if is_dark { 0xd4d4d8 } else { 0x374151 },
    );
    t.sidebar_border = c("--border", if is_dark { 0x374151 } else { 0xe5e7eb });

    let tab_bar = c("--tab-bar", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.tab_bar = if is_dark {
        gpui::transparent_black()
    } else {
        tab_bar
    };
    t.tab_bar_segmented = tab_bar;
    t.tab_foreground = c(
        "--tab-foreground",
        if is_dark { 0xd4d4d8 } else { 0x374151 },
    );
    t.tab_active = c("--tab-active", if is_dark { 0x222427 } else { 0xffffff });
    t.tab_active_foreground = c(
        "--tab-active-foreground",
        if is_dark { 0xffffff } else { 0x111827 },
    );

    let list = c("--list", if is_dark { 0x222427 } else { 0xffffff });
    t.colors.list = list;
    t.list_hover = c("--list-hover", if is_dark { 0x2a2b30 } else { 0xf3f4f6 });
    t.list_active = c("--list-active", if is_dark { 0x33353a } else { 0xe5e7eb });
    t.list_active_border = c(
        "--list-active-border",
        if is_dark { 0x264f78 } else { 0x007acc },
    );
    t.list.active_highlight = true;
    t.list_even = c("--table-even", if is_dark { 0x222427 } else { 0xffffff });
    t.list_head = c("--table-head", if is_dark { 0x222427 } else { 0xffffff });

    t.table = c(
        "--editor-surface",
        if is_dark { 0x222427 } else { 0xffffff },
    );
    t.table_head = c("--table-head", if is_dark { 0x25262a } else { 0xf9fafb });
    t.table_head_foreground = c(
        "--table-head-foreground",
        if is_dark { 0x8e8e93 } else { 0x6b7280 },
    );
    t.table_even = c("--table-even", if is_dark { 0x25262a } else { 0xf9fafb });
    t.table_hover = c("--list-hover", if is_dark { 0x2a2b30 } else { 0xf3f4f6 });
    t.table_row_border = c("--border", if is_dark { 0x374151 } else { 0xe5e7eb });

    t.input = c("--input", if is_dark { 0x2e3035 } else { 0xffffff });
    t.selection = c("--selection", if is_dark { 0x264f78 } else { 0xadd6ff });
    t.caret = if is_dark {
        gpui::rgb(0xffffff).into()
    } else {
        gpui::rgb(0x000000).into()
    };

    t.primary = c("--primary", 0x007acc);
    t.primary_hover = c("--primary-hover", 0x1a8ad4);
    t.primary_active = c("--primary-active", 0x006bb3);
    t.primary_foreground = c("--primary-foreground", 0xffffff);

    t.success = c("--success", if is_dark { 0x4ec9b0 } else { 0x059669 });
    t.success_foreground = c("--success-foreground", 0xffffff);
    t.success_hover = c("--success-hover", if is_dark { 0x5ed4bc } else { 0x047857 });
    t.success_active = c("--success-active", if is_dark { 0x3eb8a0 } else { 0x065f46 });

    t.warning = c("--warning", if is_dark { 0xcca700 } else { 0xd97706 });
    t.warning_foreground = c("--warning-foreground", 0xffffff);
    t.warning_hover = c("--warning-hover", if is_dark { 0xd4b000 } else { 0xb45309 });
    t.warning_active = c("--warning-active", if is_dark { 0xb38f00 } else { 0x92400e });

    t.danger = c("--danger", if is_dark { 0xf44747 } else { 0xdc2626 });
    t.danger_hover = c("--danger-hover", if is_dark { 0xff5555 } else { 0xef4444 });
    t.danger_active = c("--danger-active", if is_dark { 0xcc3333 } else { 0xb91c1c });
    t.danger_foreground = c("--danger-foreground", 0xffffff);

    t.info = c("--info", if is_dark { 0x3794ff } else { 0x2563eb });
    t.info_foreground = c("--info-foreground", 0xffffff);
    t.info_hover = c("--info-hover", if is_dark { 0x5aa8ff } else { 0x1d4ed8 });
    t.info_active = c("--info-active", if is_dark { 0x0066cc } else { 0x1e40af });

    t.link = c("--link", if is_dark { 0x3794ff } else { 0x2563eb });
    t.link_hover = c("--link-hover", if is_dark { 0x5aa8ff } else { 0x1d4ed8 });
    t.link_active = c("--link-active", if is_dark { 0x0066cc } else { 0x1e40af });

    t.accent = c("--accent", if is_dark { 0x094771 } else { 0xdbeafe });
    t.accent_foreground = c(
        "--accent-foreground",
        if is_dark { 0xd4d4d8 } else { 0x374151 },
    );

    t.popover = c("--popover", if is_dark { 0x2a2b32 } else { 0xffffff });
    t.popover_foreground = c(
        "--popover-foreground",
        if is_dark { 0xd4d4d8 } else { 0x111827 },
    );

    // Button tokens (Default / Secondary / semantic variants)
    t.button = c("--button", if is_dark { 0x2e3035 } else { 0xffffff });
    t.button_foreground = c(
        "--button-foreground",
        if is_dark { 0xd4d4d8 } else { 0x111827 },
    );
    t.button_hover = c("--button-hover", if is_dark { 0x33353a } else { 0xf3f4f6 });
    t.button_active = c("--button-active", if is_dark { 0x3a3b40 } else { 0xe5e7eb });

    t.button_primary = c("--button-primary", 0x007acc);
    t.button_primary_foreground = c("--button-primary-foreground", 0xffffff);
    t.button_primary_hover = c("--button-primary-hover", 0x1a8ad4);
    t.button_primary_active = c("--button-primary-active", 0x006bb3);

    t.button_secondary = c("--button-secondary", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.button_secondary_foreground = c(
        "--button-secondary-foreground",
        if is_dark { 0xd4d4d8 } else { 0x374151 },
    );
    t.button_secondary_hover = c(
        "--button-secondary-hover",
        if is_dark { 0x292929 } else { 0xd1d5db },
    );
    t.button_secondary_active = c(
        "--button-secondary-active",
        if is_dark { 0x212121 } else { 0x9ca3af },
    );

    t.button_danger = c(
        "--button-danger",
        if is_dark { 0x3a2020 } else { 0xfee2e2 },
    );
    t.button_danger_foreground = c(
        "--button-danger-foreground",
        if is_dark { 0xf44747 } else { 0xdc2626 },
    );
    t.button_danger_hover = c(
        "--button-danger-hover",
        if is_dark { 0x4a2828 } else { 0xfecaca },
    );
    t.button_danger_active = c(
        "--button-danger-active",
        if is_dark { 0x5a3030 } else { 0xfca5a5 },
    );

    t.button_success = c(
        "--button-success",
        if is_dark { 0x1a3330 } else { 0xd1fae5 },
    );
    t.button_success_foreground = c(
        "--button-success-foreground",
        if is_dark { 0x4ec9b0 } else { 0x059669 },
    );
    t.button_success_hover = c(
        "--button-success-hover",
        if is_dark { 0x224440 } else { 0xa7f3d0 },
    );
    t.button_success_active = c(
        "--button-success-active",
        if is_dark { 0x2a5550 } else { 0x6ee7b7 },
    );

    t.button_warning = c(
        "--button-warning",
        if is_dark { 0x33301a } else { 0xfef3c7 },
    );
    t.button_warning_foreground = c(
        "--button-warning-foreground",
        if is_dark { 0xcca700 } else { 0xd97706 },
    );
    t.button_warning_hover = c(
        "--button-warning-hover",
        if is_dark { 0x444028 } else { 0xfde68a },
    );
    t.button_warning_active = c(
        "--button-warning-active",
        if is_dark { 0x555030 } else { 0xfcd34d },
    );

    t.button_info = c(
        "--button-info",
        if is_dark { 0x1a2833 } else { 0xdbeafe },
    );
    t.button_info_foreground = c(
        "--button-info-foreground",
        if is_dark { 0x3794ff } else { 0x2563eb },
    );
    t.button_info_hover = c(
        "--button-info-hover",
        if is_dark { 0x223444 } else { 0xbfdbfe },
    );
    t.button_info_active = c(
        "--button-info-active",
        if is_dark { 0x2a4055 } else { 0x93c5fd },
    );

    t.description_list_label = c(
        "--description-list-label",
        if is_dark { 0x25262a } else { 0xf9fafb },
    );
    t.description_list_label_foreground = c(
        "--description-list-label-foreground",
        if is_dark { 0x8e8e93 } else { 0x6b7280 },
    );

    t.group_box = c("--group-box", if is_dark { 0x2a2b30 } else { 0xf9fafb });
    t.group_box_foreground = c(
        "--group-box-foreground",
        if is_dark { 0xd4d4d8 } else { 0x111827 },
    );

    t.accordion = c("--accordion", if is_dark { 0x2a2b30 } else { 0xffffff });
    t.accordion_hover = c(
        "--accordion-hover",
        if is_dark { 0x33353a } else { 0xf3f4f6 },
    );
    t.progress_bar = c("--progress-bar", 0x007acc);

    t.slider_bar = c(
        "--slider-bar",
        if is_dark { 0x4b5563 } else { 0xd1d5db },
    );
    t.slider_thumb = c(
        "--slider-thumb",
        if is_dark { 0xd4d4d8 } else { 0xffffff },
    );
    t.switch = c("--switch", if is_dark { 0x2a2b30 } else { 0xe5e7eb });
    t.switch_thumb = c(
        "--switch-thumb",
        if is_dark { 0x222427 } else { 0xffffff },
    );

    let scrollbar = c("--scrollbar", if is_dark { 0x1a1b1d } else { 0xf3f4f6 });
    t.scrollbar = if is_dark {
        gpui::transparent_black()
    } else {
        scrollbar
    };
    t.scrollbar_thumb = c(
        "--scrollbar-thumb",
        if is_dark { 0x555555 } else { 0x9ca3af },
    );
    t.scrollbar_thumb_hover = if is_dark {
        gpui::rgb(0x666666).into()
    } else {
        gpui::rgb(0x6b7280).into()
    };

    let border = c("--border", if is_dark { 0x374151 } else { 0xe5e7eb });
    t.border = border;
    t.drag_border = c("--ring", 0x007acc);
    t.ring = c("--ring", 0x007acc);

    t.transparent = gpui::transparent_black();
    t.window_border = if is_dark {
        gpui::transparent_black()
    } else {
        border
    };
    t.font_size = px(14.);
    t.scrollbar_show = gpui_component::scroll::ScrollbarShow::Scrolling;

    t.tokens = gpui_component::theme::ThemeTokens::from(&t.colors);
}

/// 从 `ThemeState` 读取当前合并颜色并同步 gpui-component 原生主题
#[cfg(feature = "gpui-component")]
fn sync_gpui_theme(cx: &mut App) {
    if !cx.has_global::<ThemeState>() {
        return;
    }
    let (theme, colors) = cx.read_global(|state: &ThemeState, _| {
        (state.theme().to_string(), state.colors().clone())
    });
    if theme.is_empty() {
        return;
    }
    apply_gpui_theme_from_colors(&theme, &colors, cx);
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
        sync_gpui_theme(self);
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
            sync_gpui_theme(self);
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
        #[cfg(feature = "gpui-component")]
        sync_gpui_theme(self);
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

    #[test]
    fn builtin_light_colors_cover_essential_vars() {
        let colors = builtin_light_colors();
        for key in [
            "--primary",
            "--background",
            "--surface",
            "--text",
            "--border",
            "--title-bar",
            "--status-bar",
            "--sidebar-foreground",
            "--table-head",
            "--table-even",
            "--success-foreground",
            "--danger-foreground",
        ] {
            assert!(colors.contains_key(key), "missing light builtin: {key}");
        }
        assert!(colors.len() >= 40);
    }

    #[test]
    fn builtin_dark_colors_cover_essential_vars() {
        let colors = builtin_dark_colors();
        for key in [
            "--primary",
            "--background",
            "--surface",
            "--text",
            "--border",
            "--title-bar",
            "--status-bar",
            "--sidebar-foreground",
            "--table-head",
            "--table-even",
            "--success-foreground",
            "--danger-foreground",
        ] {
            assert!(colors.contains_key(key), "missing dark builtin: {key}");
        }
        assert!(colors.len() >= 40);
    }

    #[test]
    fn merge_theme_with_builtin_uses_builtins_when_css_empty() {
        let (colors, vars) = merge_theme_with_builtin("light", DEFAULT_THEMES_DIR);
        assert!(colors.contains_key("--primary"));
        assert!(colors.contains_key("--editor-surface"));
        assert!(colors.contains_key("--button-secondary"));
        assert!(colors.contains_key("--accordion"));
        assert!(colors.contains_key("--group-box"));
        assert!(colors.contains_key("--card-bg"));
        assert!(colors.contains_key("--list-active-border"));
        assert!(colors.contains_key("--progress-bar"));
        assert!(colors.contains_key("--slider-bar"));
        assert!(colors.contains_key("--switch"));
        assert!(colors.contains_key("--description-list-label"));
        assert!(vars.is_empty());
    }

    #[test]
    fn theme_is_dark_by_name_and_luminance() {
        let dark = builtin_dark_colors();
        assert!(theme_is_dark("dark", &dark));
        assert!(!theme_is_dark("light", &dark));

        let mut ocean = HashMap::new();
        ocean.insert("--background".to_string(), rgba_from_hex(0x03045e));
        assert!(theme_is_dark("ocean", &ocean));

        let mut lightish = HashMap::new();
        lightish.insert("--background".to_string(), rgba_from_hex(0xf0f0f0));
        assert!(!theme_is_dark("ocean", &lightish));
    }

    #[test]
    fn derive_theme_colors_fills_accordion_and_progress_bar() {
        let base = builtin_dark_colors();
        let derived = derive_theme_colors(&base, true);

        assert!(derived.contains_key("--accordion"));
        assert!(derived.contains_key("--accordion-hover"));
        assert!(derived.contains_key("--progress-bar"));
        assert!(derived.contains_key("--title-bar-button-hover"));

        let accordion = derived.get("--accordion").unwrap();
        let bg = derived.get("--background").unwrap();
        assert!(
            relative_luminance(*accordion) >= relative_luminance(*bg) - 0.01,
            "accordion should sit at or above background luminance (L2 surface)"
        );

        let accordion_hover = derived.get("--accordion-hover").unwrap();
        let accent = derived.get("--accent").unwrap();
        assert!(
            (accordion_hover.r - accent.r).abs() > 0.05
                || (accordion_hover.g - accent.g).abs() > 0.05,
            "accordion-hover must not reuse accent (primary-tinted) fill"
        );
        assert!(
            relative_luminance(*accordion_hover) > relative_luminance(*accordion),
            "accordion-hover should lift above accordion base"
        );

        let list_hover = derived.get("--list-hover").unwrap();
        let list_active = derived.get("--list-active").unwrap();
        assert!(
            relative_luminance(*list_active) > relative_luminance(*list_hover),
            "list-active (L3) should be brighter than list-hover (L2) in dark theme"
        );

        let group_box = derived.get("--group-box").unwrap();
        assert!(
            relative_luminance(*group_box) > relative_luminance(*bg),
            "group-box should be elevated above page background"
        );

        let progress = derived.get("--progress-bar").unwrap();
        let primary = derived.get("--primary").unwrap();
        assert!((progress.r - primary.r).abs() < 1e-6);

        let hover = derived.get("--secondary-hover").unwrap();
        let secondary = derived.get("--secondary").unwrap();
        assert!(
            relative_luminance(*hover) > relative_luminance(*secondary),
            "secondary-hover should lift above secondary in dark theme"
        );

        let slider_bar = derived.get("--slider-bar").unwrap();
        assert!(
            (slider_bar.r - primary.r).abs() > 0.05
                || (slider_bar.g - primary.g).abs() > 0.05,
            "slider-bar must not reuse primary fill"
        );

        let switch_track = derived.get("--switch").unwrap();
        assert!(
            relative_luminance(*switch_track) >= relative_luminance(*bg) - 0.01,
            "switch track should sit at L2 surface, not below background"
        );
    }

    #[test]
    fn derive_theme_colors_fills_button_and_description_list() {
        let base = builtin_dark_colors();
        let derived = derive_theme_colors(&base, true);

        assert!(derived.contains_key("--button"));
        assert!(derived.contains_key("--button-secondary"));
        assert!(derived.contains_key("--button-secondary-foreground"));
        assert!(derived.contains_key("--description-list-label"));
        assert!(derived.contains_key("--description-list-label-foreground"));

        let btn_sec = derived.get("--button-secondary").unwrap();
        let bg = derived.get("--background").unwrap();
        assert!(
            relative_luminance(*btn_sec) < relative_luminance(*bg) + 0.3,
            "button-secondary should be dark-ish in dark theme"
        );
    }

    #[test]
    fn derive_respects_explicit_css_override() {
        let mut base = builtin_dark_colors();
        base.insert(
            "--button-secondary".to_string(),
            rgba_from_hex(0xff0000),
        );
        let derived = derive_theme_colors(&base, true);
        let btn = derived.get("--button-secondary").unwrap();
        assert!((btn.r - 1.0).abs() < 1e-6);
    }

    #[test]
    fn relative_luminance_black_vs_white() {
        assert!(relative_luminance(RGBA_BLACK) < relative_luminance(RGBA_WHITE));
    }
}
