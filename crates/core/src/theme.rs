//! 主题系统 —— `ThemeState` Global + `ThemeExt` 扩展
//!
//! 开发体验与 [`crate::i18n`] 完全对齐:
//! - `cx.set_style("styles.css")` 加载全局样式 CSS 的 `:root` 颜色变量作为基础颜色
//! - `cx.set_theme("dark")` 加载/切换主题(主题颜色覆盖基础颜色)
//! - `cx.theme_color("--primary")` 取当前生效颜色(基础 + 主题)
//! - `theme_color_static("--primary")` 供 `#[computed]` 等无 `App` 上下文场景使用
//!
//! 主题文件格式(`assets/themes/dark.css`):
//! ```css
//! :root {
//!     --primary-color: #007bff;
//!     --text-color: #333333;
//! }
//! ```
//! 仅解析 `:root` 块中的 `#hex` 颜色变量;非颜色变量被忽略。
//!
//! `set_style` 加载的基础颜色作为默认值,`set_theme` 的主题颜色优先级更高:
//! 主题中定义的变量覆盖基础颜色,主题中未定义的变量回退到基础颜色。

use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::RwLock;

use gpui::{App, AppContext, BorrowAppContext, Context, Global, Rgba, SharedString};

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
}

impl Default for ThemeState {
    fn default() -> Self {
        Self {
            theme: String::new(),
            dir: DEFAULT_THEMES_DIR.to_string(),
            base_colors: HashMap::new(),
            colors: HashMap::new(),
            themes: HashMap::new(),
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

    pub fn set_dir(&mut self, dir: impl Into<String>) {
        self.dir = dir.into();
    }

    /// 设置全局基础颜色(由 `set_style` 调用),并重新合并当前主题颜色
    pub fn set_base_colors(&mut self, base: HashMap<String, Rgba>) {
        self.base_colors = base;
        self.recompute_colors();
    }

    /// 加载主题颜色表;若当前未设置主题或与当前主题同名,则激活
    pub fn load_theme(&mut self, name: impl Into<String>, theme_colors: HashMap<String, Rgba>) {
        let name = name.into();
        self.themes.insert(name.clone(), theme_colors.clone());
        if self.theme.is_empty() || self.theme == name {
            self.theme = name;
            self.recompute_colors();
        }
    }

    /// 切换到已加载的主题;成功返回 true
    pub fn switch_theme(&mut self, name: impl Into<String>) -> bool {
        let name = name.into();
        if self.themes.contains_key(&name) {
            self.theme = name;
            self.recompute_colors();
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
}

/// 确保 `ThemeState` Global 已注册
pub fn ensure_theme(cx: &mut App) {
    if !cx.has_global::<ThemeState>() {
        cx.set_global(ThemeState::default());
    }
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
        if let Ok(colors) = load_theme_colors_embedded(&theme, &dir) {
            self.update_global::<ThemeState, _>(|state, _| {
                state.set_dir(&dir);
                state.load_theme(&theme, colors);
            });
        }
    }

    fn set_theme(&mut self, theme: impl AsRef<str>) {
        let theme = theme.as_ref().to_string();
        ensure_theme(self);
        let has_theme = self
            .read_global(|state: &ThemeState, _| state.themes.contains_key(&theme));
        if !has_theme {
            let dir = self.read_global(|state: &ThemeState, _| state.dir().to_string());
            if let Ok(colors) = load_theme_colors_embedded(&theme, &dir) {
                self.update_global::<ThemeState, _>(|state, _| {
                    state.load_theme(&theme, colors);
                });
            }
        }
        let mut switched = false;
        self.update_global::<ThemeState, _>(|state, _| {
            switched = state.switch_theme(&theme);
        });
        if switched {
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
        let base_colors = parse_theme_css(css).unwrap_or_default();
        ensure_theme(self);
        self.update_global::<ThemeState, _>(|state, _| {
            state.set_base_colors(base_colors);
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
    // 嵌入资源 key 是相对 assets/ 根的路径,去掉 "assets/" 前缀
    let sub_dir = dir.strip_prefix("assets/").unwrap_or(dir);
    let path = format!("{}/{}.css", sub_dir.trim_end_matches('/'), theme);
    let css = crate::assets::load_str(&path)
        .ok_or_else(|| format!("theme asset not embedded: {}", path))?;
    parse_theme_css(css)
}

/// 解析主题 CSS,提取 `:root` 块中的 `#hex` 颜色变量
///
/// 仅识别 `--name: #hexvalue;` 形式的声明;其他内容(规则、非颜色变量)忽略。
/// 支持 `#rgb`、`#rrggbb`、`#rrggbbaa` 三种 hex 格式。
pub fn parse_theme_css(css: &str) -> Result<HashMap<String, Rgba>, String> {
    let mut colors = HashMap::new();
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
            if let Some(rgba) = parse_hex_color(value) {
                colors.insert(name, rgba);
            }
            // 非颜色值忽略(主题变量仅支持颜色)
        }
    }
    Ok(colors)
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
}
