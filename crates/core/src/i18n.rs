//! 国际化（i18n）——`I18nState` Global + `I18nExt` 扩展
//!
//! 业务代码通过 `cx.use_i18n` / `cx.set_i18n` / `cx.t` 访问，不暴露独立 App 单例类型。

use std::borrow::{Borrow, BorrowMut};
use std::collections::HashMap;
use std::sync::RwLock;

use gpui::{App, AppContext, BorrowAppContext, Context, Global, SharedString};

/// 线程内同步翻译表快照，供 `#[computed]` 等无 `App` 上下文场景使用
static ACTIVE_CATALOG: RwLock<Option<HashMap<String, String>>> = RwLock::new(None);

fn sync_active_catalog(catalog: &HashMap<String, String>) {
    if let Ok(mut guard) = ACTIVE_CATALOG.write() {
        *guard = Some(catalog.clone());
    }
}

/// 无 `App` 上下文时取翻译（依赖 `sync_active_catalog` 维护的快照）
pub fn t_static(key: &str) -> SharedString {
    ACTIVE_CATALOG
        .read()
        .ok()
        .and_then(|guard| guard.as_ref()?.get(key).cloned())
        .map(|s| SharedString::from(s.as_str()))
        .unwrap_or_else(|| key.into())
}

/// 默认 i18n 资源目录（相对工作目录）
pub const DEFAULT_I18N_DIR: &str = "assets/i18n";

/// 内部 Global：当前 locale 与翻译表
#[derive(Debug, Clone)]
pub struct I18nState {
    locale: String,
    dir: String,
    catalog: HashMap<String, String>,
    catalogs: HashMap<String, HashMap<String, String>>,
}

impl Default for I18nState {
    fn default() -> Self {
        Self {
            locale: String::new(),
            dir: DEFAULT_I18N_DIR.to_string(),
            catalog: HashMap::new(),
            catalogs: HashMap::new(),
        }
    }
}

impl Global for I18nState {}

impl I18nState {
    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn dir(&self) -> &str {
        &self.dir
    }

    pub fn t(&self, key: &str) -> SharedString {
        self.catalog
            .get(key)
            .map(|s| SharedString::from(s.as_str()))
            .unwrap_or_else(|| key.into())
    }

    pub fn set_dir(&mut self, dir: impl Into<String>) {
        self.dir = dir.into();
    }

    pub fn load_catalog(&mut self, locale: impl Into<String>, catalog: HashMap<String, String>) {
        let locale = locale.into();
        self.catalogs.insert(locale.clone(), catalog.clone());
        if self.locale.is_empty() || self.locale == locale {
            self.locale = locale;
            self.catalog = catalog.clone();
            sync_active_catalog(&self.catalog);
        }
    }

    pub fn switch_locale(&mut self, locale: impl Into<String>) -> bool {
        let locale = locale.into();
        if let Some(catalog) = self.catalogs.get(&locale).cloned() {
            self.locale = locale;
            self.catalog = catalog.clone();
            sync_active_catalog(&self.catalog);
            true
        } else {
            false
        }
    }
}

/// 将嵌套 JSON 对象扁平化为点路径 key（`menu.file`）
pub fn flatten_json_value(
    value: &serde_json::Value,
    prefix: &str,
    out: &mut HashMap<String, String>,
) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_json_value(v, &key, out);
            }
        }
        serde_json::Value::String(s) => {
            out.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Number(n) => {
            out.insert(prefix.to_string(), n.to_string());
        }
        serde_json::Value::Bool(b) => {
            out.insert(prefix.to_string(), b.to_string());
        }
        _ => {}
    }
}

/// 从 JSON 字符串解析 catalog
pub fn catalog_from_json(json: &str) -> Result<HashMap<String, String>, String> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|e| format!("invalid i18n JSON: {e}"))?;
    let mut catalog = HashMap::new();
    flatten_json_value(&value, "", &mut catalog);
    Ok(catalog)
}

/// 从嵌入资源加载 catalog
///
/// `dir` 为相对 cwd 的资源目录(如 `"assets/i18n"`),内部去掉 `"assets/"` 前缀
/// 得到嵌入资源 key(如 `"i18n/{locale}.json"`)。
/// 若资源未嵌入或 `assets` 模块未初始化,返回 Err(调用方可 fallback 到磁盘)。
pub fn load_catalog_embedded(
    locale: &str,
    dir: &str,
) -> Result<HashMap<String, String>, String> {
    // 嵌入资源 key 是相对 assets/ 根的路径,去掉 "assets/" 前缀
    let sub_dir = dir.strip_prefix("assets/").unwrap_or(dir);
    let path = format!("{}/{}.json", sub_dir.trim_end_matches('/'), locale);
    let json = crate::assets::load_str(&path)
        .ok_or_else(|| format!("i18n asset not embedded: {}", path))?;
    catalog_from_json(json)
}

/// 确保 `I18nState` Global 已注册
pub fn ensure_i18n(cx: &mut App) {
    if !cx.has_global::<I18nState>() {
        cx.set_global(I18nState::default());
    }
}

/// `Context` / `App` 国际化扩展
pub trait I18nExt {
    /// 加载并绑定指定 locale 的 catalog
    fn use_i18n(&mut self, locale: impl AsRef<str>);
    /// 指定资源目录并加载 locale
    fn use_i18n_with_dir(&mut self, locale: impl AsRef<str>, dir: impl AsRef<str>);
    /// 运行时切换已加载的 locale（未缓存则尝试从磁盘加载）
    fn set_i18n(&mut self, locale: impl AsRef<str>);
    /// 取当前 locale 下的翻译
    fn t(&self, key: &str) -> SharedString;
    /// 当前 locale
    fn current_locale(&self) -> SharedString;
}

impl I18nExt for App {
    fn use_i18n(&mut self, locale: impl AsRef<str>) {
        let locale = locale.as_ref().to_string();
        ensure_i18n(self);
        // 优先从嵌入资源加载,失败则 fallback 到磁盘
        let catalog = load_catalog_embedded(&locale, DEFAULT_I18N_DIR)
            .or_else(|_| load_catalog_from_dir(&locale, DEFAULT_I18N_DIR));
        if let Ok(catalog) = catalog {
            self.update_global::<I18nState, _>(|state, _| {
                state.load_catalog(&locale, catalog);
            });
        }
    }

    fn use_i18n_with_dir(&mut self, locale: impl AsRef<str>, dir: impl AsRef<str>) {
        let locale = locale.as_ref().to_string();
        let dir = dir.as_ref().to_string();
        ensure_i18n(self);
        // 优先从嵌入资源加载,失败则 fallback 到磁盘
        let catalog = load_catalog_embedded(&locale, &dir)
            .or_else(|_| load_catalog_from_dir(&locale, &dir));
        if let Ok(catalog) = catalog {
            self.update_global::<I18nState, _>(|state, _| {
                state.set_dir(&dir);
                state.load_catalog(&locale, catalog);
            });
        }
    }

    fn set_i18n(&mut self, locale: impl AsRef<str>) {
        let locale = locale.as_ref().to_string();
        ensure_i18n(self);
        let has_catalog = self.read_global(|state: &I18nState, _| state.catalogs.contains_key(&locale));
        if !has_catalog {
            let dir = self.read_global(|state: &I18nState, _| state.dir().to_string());
            // 优先从嵌入资源加载,失败则 fallback 到磁盘
            let catalog = load_catalog_embedded(&locale, &dir)
                .or_else(|_| load_catalog_from_dir(&locale, &dir));
            if let Ok(catalog) = catalog {
                self.update_global::<I18nState, _>(|state, _| {
                    state.load_catalog(&locale, catalog);
                });
            }
        }
        let mut switched = false;
        self.update_global::<I18nState, _>(|state, _| {
            switched = state.switch_locale(&locale);
        });
        if switched {
            self.refresh_windows();
        }
    }

    fn t(&self, key: &str) -> SharedString {
        if self.has_global::<I18nState>() {
            self.global::<I18nState>().t(key)
        } else {
            key.into()
        }
    }

    fn current_locale(&self) -> SharedString {
        if self.has_global::<I18nState>() {
            self.global::<I18nState>().locale().into()
        } else {
            SharedString::default()
        }
    }
}

impl<T> I18nExt for Context<'_, T> {
    fn use_i18n(&mut self, locale: impl AsRef<str>) {
        I18nExt::use_i18n(BorrowMut::<App>::borrow_mut(self), locale);
    }

    fn use_i18n_with_dir(&mut self, locale: impl AsRef<str>, dir: impl AsRef<str>) {
        I18nExt::use_i18n_with_dir(BorrowMut::<App>::borrow_mut(self), locale, dir);
    }

    fn set_i18n(&mut self, locale: impl AsRef<str>) {
        I18nExt::set_i18n(BorrowMut::<App>::borrow_mut(self), locale);
    }

    fn t(&self, key: &str) -> SharedString {
        I18nExt::t(Borrow::<App>::borrow(self), key)
    }

    fn current_locale(&self) -> SharedString {
        I18nExt::current_locale(Borrow::<App>::borrow(self))
    }
}

/// 自由函数：取翻译
pub fn t(cx: &App, key: &str) -> SharedString {
    cx.t(key)
}

/// 从目录加载 `{dir}/{locale}.json`
pub fn load_catalog_from_dir(
    locale: &str,
    dir: &str,
) -> Result<HashMap<String, String>, String> {
    let path = std::path::Path::new(dir).join(format!("{locale}.json"));
    let json = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    catalog_from_json(&json)
}

/// 从内存 JSON 加载并应用到 `App`
pub fn apply_catalog(cx: &mut App, locale: impl AsRef<str>, json: &str) {
    let locale = locale.as_ref().to_string();
    ensure_i18n(cx);
    if let Ok(catalog) = catalog_from_json(json) {
        cx.update_global::<I18nState, _>(|state, _| {
            state.load_catalog(&locale, catalog);
        });
    }
}
