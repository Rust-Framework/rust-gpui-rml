# Assets / 主题 / 资源管理方案

## Summary

为 RML 框架新增三项能力,对标 i18n 的开发体验:

1. **资源嵌入**:将 `assets/` 下所有文件编译进二进制,运行时通过路径访问,避免软件资源泄露(类似 Avalonia `AssetLoader`)。
2. **主题系统**:`assets/themes/{dark,light}.css` 仅含 `:root` 颜色变量;运行时通过 `cx.use_theme("dark")` / `cx.set_theme("light")` 切换,开发体验与 i18n 完全对齐。
3. **build.rs 集成**:构建期自动扫描 `assets/` 嵌入资源、扫描 `assets/themes/` 注册主题,开发者只需 `Builder::assets_dir("assets")` 一行配置。

同步改造:i18n JSON 资源也从磁盘加载改为嵌入资源加载(统一资源管理,彻底避免泄露)。

## Current State Analysis

### i18n 现状(主题系统的参考模板)

| 层 | 文件 | 职责 |
|---|---|---|
| 运行时状态 | [crates/core/src/i18n.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/i18n.rs) | `I18nState` Global + `I18nExt` trait(`use_i18n`/`set_i18n`/`t`)+ `t_static` 无 App 上下文快照 |
| 资源加载 | [crates/app/src/resources.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/resources.rs) | `load_i18n_catalog(locale, dir)` 从磁盘 `assets/i18n/{locale}.json` 读取 |
| 构建期提取 | [crates/engine/src/build/i18n_extractor.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/i18n_extractor.rs) | `I18nExtractor` 扫描 `.rml` 中 `t("key")` 调用,合并写入 JSON |
| 构建入口 | [crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs) | `Builder::extract_i18n(path)` / `Builder::plugin(extractor)` 链式 API |
| 使用方式 | [demo/src/app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs#L16) | `cx.use_i18n_with_dir("zh-CN", "demo/assets/i18n")` |

i18n 的关键设计模式:
- `Global` 状态 + `Ext` trait 扩展 `App`/`Context`
- `ensure_xxx(cx)` 懒注册 Global
- `set_xxx` 切换后调用 `cx.refresh_windows()` 触发重渲染
- `_static` 全局快照供 `#[computed]` 等无 App 上下文场景使用

### CSS / 样式现状

| 文件 | 职责 |
|---|---|
| [crates/engine/src/css/parser.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/parser.rs) | 递归下降解析器,`:root { --name: value }` 解析到 `StyleSheet.variables: HashMap<String, Value>` |
| [crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) | `map_declarations` 将 CSS 声明映射为 GPUI 方法调用字符串;`resolve_var` **构建期内联**解析 `var(--name)` |
| [crates/engine/src/css/matcher.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/matcher.rs) | `generate_styles(sheet, ctx)` 匹配选择器,收集声明 |
| [crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs#L122) | `Builder::with_style(path)` 构建期加载 CSS,合并为 `StyleSheet` 传给 `CodegenCtx` |
| [crates/engine/src/compiler/codegen.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs#L704) | codegen 时调用 `css::styles_for_class(sheet, ...)` **构建期内联**样式到生成代码 |

**关键问题**:CSS 变量(`var(--name)`)在构建期被 `resolve_var` 内联为具体值,运行时无法切换主题。

### 资源管理现状

- i18n JSON:运行时从磁盘读取(`std::fs::read_to_string`),依赖 cwd
- `demo/assets/logo.svg`:存在但代码中未引用
- 无统一资源嵌入方案,所有 `assets/` 文件以明文形式随发布包暴露

### build.rs 当前 API

```rust
// demo/build.rs
rml::build()
    .scan_dir("src")
    .with_style("src/styles.css")
    .output_dir(std::env::var("OUT_DIR").unwrap())
    .build()
```

## Proposed Changes

### Phase 1: 资源嵌入基础设施

**目标**:build.rs 扫描 `assets/` 目录,生成资源注册表,通过 `include_bytes!` 嵌入二进制。

#### 1.1 新增构建期资源处理器

**新文件**:[crates/engine/src/build/assets_processor.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/assets_processor.rs)

```rust
pub struct AssetsProcessor {
    root_dir: PathBuf,
    exclude_patterns: Vec<String>,  // 可选:排除规则(如 *.tmp)
}

impl AssetsProcessor {
    pub fn new(root_dir: impl Into<PathBuf>) -> Self { ... }

    /// 递归扫描 root_dir,生成资源注册表 Rust 代码
    /// 输出到 OUT_DIR/rml_generated/rml_assets.rs
    pub fn generate(&self, output_dir: &Path) -> Result<(), BuildError> {
        // 1. 递归收集 assets/ 下所有文件,相对路径作为 key
        // 2. 生成:
        //    pub static RML_ASSETS: &[(&str, &[u8])] = &[
        //        ("i18n/zh-CN.json", include_bytes!("/abs/path/zh-CN.json")),
        //        ("themes/dark.css", include_bytes!("...")),
        //        ...
        //    ];
        //    pub fn load(path: &str) -> Option<&'static [u8]> { ... }
        //    pub fn load_str(path: &str) -> Option<&'static str> { ... }
        // 3. 对每个文件 println!("cargo:rerun-if-changed=...")
    }
}
```

**修改**:[crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)
- 新增字段 `assets_dir: Option<PathBuf>`
- 新增链式 API `Builder::assets_dir(dir)` 
- 在 `Builder::build()` 末尾调用 `AssetsProcessor::generate`
- 模块声明 `pub mod assets_processor;` + `pub use assets_processor::AssetsProcessor;`

#### 1.2 新增运行时资源查询 API

**新文件**:[crates/core/src/assets.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/assets.rs)

```rust
//! 资源嵌入运行时查询
//!
//! 资源注册表由 build.rs 生成到 OUT_DIR/rml_generated/rml_assets.rs,
//! 通过 include! 注入本模块。

pub fn load(path: &str) -> Option<&'static [u8]> {
    RML_ASSETS.iter().find(|(p, _)| *p == path).map(|(_, b)| *b)
}

pub fn load_str(path: &str) -> Option<&'static str> {
    load(path).and_then(|b| std::str::from_utf8(b).ok())
}

/// 路径归一化:去掉前导 `/`、统一正斜杠
fn normalize(path: &str) -> String { ... }
```

**修改**:[crates/core/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs)
- 新增 `pub mod assets;`
- 在模块内 `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"));`

**注意**:core crate 需要新增 `build.rs`(目前没有),用于触发资源注册表生成。但资源扫描在用户 crate 的 build.rs 中完成,生成的 `rml_assets.rs` 在用户 crate 的 `OUT_DIR`。

**架构决策**:资源注册表生成在**用户 crate**(demo)的 `OUT_DIR`,而非 core crate。因此:
- core crate 提供 `assets` 模块的**接口**(trait 或自由函数声明)
- 用户 crate 通过宏或 `include!` 注入注册表
- 或:core 提供 `AssetsRegistry` trait,用户 crate 实现

**采用方案**(最简):core crate 的 `assets.rs` 通过 `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))` 注入。由于 core crate 本身没有 assets,需要在用户 crate 的 build.rs 中生成到用户 OUT_DIR,然后 core crate 的 `include!` 会读取用户 crate 的 OUT_DIR(因为 core 是依赖,其 OUT_DIR 与用户不同)。

**修正方案**:资源查询 API 放在 **app crate**(应用层),而非 core crate。app crate 的 build.rs 生成注册表,app crate 的代码 `include!` 注入。但 app crate 当前也没有 build.rs。

**最终方案**(符合现有架构):
- 在 **engine crate** 中提供 `runtime::assets` 模块,声明注册表接口
- 用户 crate 的 build.rs 生成 `rml_assets.rs` 到用户 OUT_DIR
- 用户 crate 通过 `rml::assets::load(path)` 调用,但实际注册表在用户 crate 中
- 通过宏 `#[rml::assets]` 或在 `app.rs` 中 `include!` 注入

**最简洁方案**(推荐):借鉴 i18n 模式,资源注册表生成到用户 OUT_DIR,用户 crate 在 `main.rs` 或 `lib.rs` 中通过宏注入:

```rust
// demo/src/main.rs
rml::include_assets!();  // 展开为 include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))
```

或更自动化:engine crate 的 `runtime::assets` 模块直接 `include!`,因为 engine crate 的 `OUT_DIR` 在用户 crate 编译时可见(实际上不可见,每个 crate 有独立 OUT_DIR)。

**确定方案**:在 app crate 新增 build.rs,生成资源注册表到 app crate 的 OUT_DIR;app crate 的 `resources.rs` 通过 `include!` 注入。但 app crate 是库,其 OUT_DIR 在依赖它的 demo crate 编译时可用。

**最终确定**(与 i18n 现状一致):i18n 的 `load_catalog_from_dir` 在 core crate,运行时从磁盘读。改造后:
- 资源注册表生成在**用户 crate**(demo)的 OUT_DIR
- 用户 crate 通过 `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))` 注入到自己的模块
- 通过宏简化:`#[rml::assets]` 注入到 `main.rs`
- engine crate 提供 `assets::AssetLoader` trait,用户 crate 实现 trait 并注册到 Global

**简化最终方案**(Microsoft 风格"极其易用"):
1. engine 的 build 模块新增 `AssetsProcessor`,生成注册表代码到用户 OUT_DIR
2. engine 的 runtime 模块新增 `assets` 子模块,提供 `load(path)` / `load_str(path)` 函数,内部通过 `include!` 注入注册表
3. 由于 engine crate 编译时其 OUT_DIR 不同于用户 crate,采用**用户 crate 注入**方案:
   - 用户 crate 的 `main.rs` 调用 `rml::init_assets!()` 宏
   - 宏展开为 `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))`
   - 注入的代码实现 `rml::runtime::assets::ASSETS` 常量

**实现细节**:engine `runtime::assets` 模块声明:
```rust
pub static ASSETS: &[(&str, &[u8])] = &[];  // 默认空,被用户 crate 覆盖
pub fn load(path: &str) -> Option<&'static [u8]> { ... }
```

用户 crate 通过宏覆盖:
```rust
// demo/src/main.rs
rml::embed_assets!();  // 宏生成: pub static ASSETS ... = include!(...)
```

**最简洁实现**(采用):engine `runtime::assets` 模块**不**提供默认 ASSETS,而是要求用户 crate 调用 `rml::embed_assets!()` 宏在 crate 根注入。`load` 函数通过宏注入的 ASSETS 查询。

#### 1.3 build.rs API 扩展

**修改**:[crates/engine/src/build/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs)

```rust
impl Builder {
    /// 注册资源根目录,构建期扫描并嵌入所有文件
    /// 自动包含 {assets_dir}/themes/ 主题文件
    pub fn assets_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.assets_dir = Some(dir.into());
        self
    }
}
```

### Phase 2: 主题运行时系统

**目标**:对标 i18n,实现 `cx.use_theme("dark")` / `cx.set_theme("light")` / `theme_color_static("--primary")`。

#### 2.1 主题文件格式

**新文件**:[demo/assets/themes/dark.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/dark.css) 和 [demo/assets/themes/light.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/light.css)

```css
/* dark.css */
:root {
    --primary-color: #0d6efd;
    --text-color: #f8f9fa;
    --bg-color: #1a1a1a;
    --border-color: #333333;
}

/* light.css */
:root {
    --primary-color: #007bff;
    --text-color: #333333;
    --bg-color: #f8f9fa;
    --border-color: #e5e7eb;
}
```

**约束**:主题文件仅含 `:root` 颜色变量定义。非颜色变量(如 `--spacing: 8px`)不参与主题切换,应放在全局 CSS 中(构建期内联)。

#### 2.2 主题运行时状态

**新文件**:[crates/core/src/theme.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)

```rust
//! 主题系统 —— ThemeState Global + ThemeExt 扩展
//! 开发体验与 i18n 完全对齐

use std::collections::HashMap;
use std::sync::RwLock;
use gpui::{App, AppContext, BorrowAppContext, BorrowMut, Context, Global, Hsla};

/// 线程内同步主题快照(供 #[computed] 等无 App 上下文场景)
static ACTIVE_THEME_COLORS: RwLock<Option<HashMap<String, Hsla>>> = RwLock::new(None);

fn sync_active_theme(colors: &HashMap<String, Hsla>) { ... }

/// 无 App 上下文时取主题颜色
pub fn theme_color_static(name: &str) -> Hsla { ... }

pub const DEFAULT_THEMES_DIR: &str = "assets/themes";

#[derive(Debug, Clone)]
pub struct ThemeState {
    theme: String,
    dir: String,
    colors: HashMap<String, Hsla>,           // 当前主题的颜色表
    themes: HashMap<String, HashMap<String, Hsla>>,  // 所有主题: theme_name → color_table
}

impl Global for ThemeState {}

impl ThemeState {
    pub fn theme(&self) -> &str { ... }
    pub fn color(&self, name: &str) -> Option<Hsla> { ... }
    pub fn load_theme(&mut self, name: &str, colors: HashMap<String, Hsla>) { ... }
    pub fn switch_theme(&mut self, name: &str) -> bool { ... }
}

pub fn ensure_theme(cx: &mut App) { ... }

pub trait ThemeExt {
    fn use_theme(&mut self, theme: impl AsRef<str>);
    fn use_theme_with_dir(&mut self, theme: impl AsRef<str>, dir: impl AsRef<str>);
    fn set_theme(&mut self, theme: impl AsRef<str>);
    fn theme_color(&self, name: &str) -> Hsla;
    fn current_theme(&self) -> SharedString;
}

impl ThemeExt for App { ... }
impl<T> ThemeExt for Context<'_, T> { ... }
```

**关键设计**:
- 主题颜色表 `HashMap<String, Hsla>`:变量名(含 `--` 前缀)→ GPUI 颜色
- `use_theme(name)`:从嵌入资源加载 `{themes_dir}/{name}.css`,解析 `:root` 变量,只保留 `Value::Color` 类型(非颜色变量忽略并 warning)
- `set_theme(name)`:切换 + `cx.refresh_windows()`(与 i18n 一致)
- `theme_color_static(name)`:`#[computed]` 中使用,从全局快照取

**修改**:[crates/core/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/lib.rs)
- 新增 `pub mod theme;`

**修改**:[crates/app/src/application.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/application.rs)
- `run` 方法中新增 `ensure_theme(cx);`(与 `ensure_i18n` 并列)

#### 2.3 主题加载(从嵌入资源)

**修改**:[crates/app/src/resources.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/resources.rs)

新增:
```rust
use rml_core::assets;

/// 从嵌入资源加载主题 CSS 文本
pub fn load_theme_css(theme: &str, themes_dir: &str) -> Result<String, String> {
    let path = format!("{}/{}.css", themes_dir.trim_end_matches('/'), theme);
    assets::load_str(&path)
        .ok_or_else(|| format!("theme asset not found: {}", path))
}

/// 从嵌入资源加载 i18n catalog
pub fn load_i18n_catalog_embedded(locale: &str, i18n_dir: &str) -> Result<HashMap<String, String>, String> {
    let path = format!("{}/{}.json", i18n_dir.trim_end_matches('/'), locale);
    let json = assets::load_str(&path)
        .ok_or_else(|| format!("i18n asset not found: {}", path))?;
    catalog_from_json(json)
}
```

### Phase 3: i18n 改造为嵌入资源

**目标**:i18n 优先从嵌入资源加载,保留磁盘 fallback。

**修改**:[crates/core/src/i18n.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/i18n.rs)

```rust
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
    // ... 其他方法类似改造
}

/// 从嵌入资源加载 catalog
pub fn load_catalog_embedded(locale: &str, dir: &str) -> Result<HashMap<String, String>, String> {
    let path = format!("{}/{}.json", dir.trim_end_matches('/'), locale);
    let json = crate::assets::load_str(&path)
        .ok_or_else(|| format!("i18n asset not embedded: {}", path))?;
    catalog_from_json(json)
}
```

**兼容性**:保留 `load_catalog_from_dir` 作为 fallback,确保开发期资源未嵌入时仍可运行。

### Phase 4: codegen 改造(var() 运行时查询)

**目标**:CSS `var(--name)` 不再构建期内联,改为生成运行时主题查询代码。

#### 4.1 mapper 改造

**修改**:[crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs)

核心变化:`resolve_var` 不再递归解析主题变量,而是保留 `Value::Var` 并由 `color_method` 等生成运行时查询代码。

```rust
fn map_declaration(decl: &Declaration, vars: &HashMap<String, Value>) -> Option<String> {
    let prop = decl.property.as_str();
    let value = &decl.value;  // 不再调用 resolve_var
    
    match prop {
        "background" | "background-color" => color_method("bg", value, vars),
        "color" => color_method("text_color", value, vars),
        // ... 其他属性
    }
}

/// 颜色值 → GPUI 调用
/// 遇到 var() 时生成运行时主题查询
fn color_method(method: &str, value: &Value, vars: &HashMap<String, Value>) -> Option<String> {
    match value {
        Value::Color(c) => {
            // 直接颜色字面量:构建期内联
            let rgba = ((c.r as u32) << 24) | ((c.g as u32) << 16) | ((c.b as u32) << 8) | (c.a as u32);
            Some(format!("{}(gpui::rgb(0x{:08x}))", method, rgba))
        }
        Value::Var(name, _fallback) => {
            // 主题变量:生成运行时查询代码
            Some(format!("{}(rml::runtime::theme::color(\"{}\"))", method, name))
        }
        _ => None,
    }
}
```

**关键决策**:
- `Value::Color` 字面量仍构建期内联(性能)
- `Value::Var` 生成运行时查询 `rml::runtime::theme::color("--name")`
- 非颜色属性(如 `padding: var(--spacing)`)中的 var() 暂不支持运行时查询(返回 None,该声明跳过)。用户应在全局 CSS 中直接写尺寸值,仅颜色用 var()

#### 4.2 runtime::theme 模块

**修改**:[crates/engine/src/runtime/styling.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/runtime/styling.rs)(当前是 stub)

```rust
//! 主题运行时查询(codegen 生成代码调用)

use gpui::Hsla;
use rml_core::theme::theme_color_static;

/// codegen 生成的 var() 查询入口
/// 返回当前主题中变量的颜色值;未找到时返回透明黑作为 fallback
pub fn color(name: &str) -> Hsla {
    theme_color_static(name)
}
```

**或直接在 core crate 的 theme.rs 中提供**:`pub fn color(name: &str) -> Hsla`。codegen 生成 `rml_core::theme::color("--name")`。选择此方案,无需 engine runtime 中介。

**最终**:codegen 生成 `rml::theme::color("--name")`(engine 重导出 core 的 theme 模块)。

#### 4.3 engine 重导出 theme 模块

**修改**:[crates/engine/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/lib.rs)
- 新增 `pub use rml_core::theme;`(与 `rml_core::i18n` 同级重导出)

### Phase 5: CSS 加载方式改造(全局运行时 + 组件级)

**目标**:按用户澄清,支持两种 CSS 加载方式。

#### 5.1 全局 CSS 运行时加载

**新文件**:[crates/core/src/style.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/style.rs)

```rust
//! 全局样式系统 —— StyleState Global + StyleExt 扩展
//! 在 app.rs on_launch 中加载全局 CSS

use gpui::{App, AppContext, BorrowAppContext, Global};
use rml_core::assets;

#[derive(Default)]
pub struct StyleState {
    /// 全局 CSS 文本(从嵌入资源加载)
    global_css: String,
}

impl Global for StyleState {}

pub trait StyleExt {
    /// 从嵌入资源加载全局 CSS
    fn use_style(&mut self, asset_path: impl AsRef<str>);
    /// 从 CSS 文本加载
    fn use_style_text(&mut self, css: impl Into<String>);
}

impl StyleExt for App {
    fn use_style(&mut self, asset_path: impl AsRef<str>) {
        let path = asset_path.as_ref();
        if let Some(css) = assets::load_str(path) {
            self.use_style_text(css);
        }
    }
    fn use_style_text(&mut self, css: impl Into<String>) {
        // 注册/更新 StyleState
    }
}
```

**注意**:全局 CSS 运行时加载后,codegen 无法在构建期知道全局样式。需要 codegen 改造:对元素生成"运行时样式查询"代码。这是较大改动,Phase 5 标记为**可选/后续**,Phase 1-4 已能交付主题切换 + 资源嵌入 + i18n 嵌入的核心价值。

**Phase 5 暂缓理由**:当前 `with_style` 构建期内联模式性能最优,且 `var()` 改造后主题变量已可运行时切换。全局 CSS 运行时加载需要重写 codegen 样式生成逻辑(从内联改为查询),影响面大。建议 Phase 1-4 稳定后再评估。

#### 5.2 组件级 CSS(.rml 中引入)

**暂缓**:需要扩展 `.rml` 解析器支持 `<style>` 块或 `style_src` 属性,改动较大。当前 `with_style` 全局模式 + 主题变量已能满足主要需求。

### Phase 6: build.rs 集成

**修改**:[demo/build.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/build.rs)

```rust
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")
        .with_style("src/styles.css")
        .assets_dir("assets")  // 新增:嵌入 assets/ 下所有资源
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

**自动化**:`assets_dir("assets")` 后,Builder 自动:
1. 扫描 `assets/` 嵌入所有文件(含 `i18n/*.json`、`themes/*.css`、`logo.svg`)
2. 扫描 `assets/themes/` 注册主题(无需单独 `themes_dir` 配置)
3. 生成 `OUT_DIR/rml_generated/rml_assets.rs`

### Phase 7: demo 改造

#### 7.1 拆分样式文件

**新文件**:[demo/assets/themes/light.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/light.css) 和 [demo/assets/themes/dark.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/dark.css)

从 [demo/src/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/styles.css) 提取 `:root` 变量到主题文件;`src/styles.css` 保留规则(颜色用 `var(--xxx)` 引用)。

#### 7.2 应用启动初始化主题

**修改**:[demo/src/app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs)

```rust
impl IAppLifecycle for AppBootstrap {
    fn on_launch(&mut self, cx: &mut App) {
        cx.use_i18n_with_dir("zh-CN", "demo/assets/i18n");  // 改造后从嵌入资源加载
        cx.use_theme("light");  // 新增:初始化主题
        LoginWindow::default().open(cx);
    }
}
```

#### 7.3 演示主题切换

**修改**:[demo/src/shell/main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) 和 [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

新增"切换主题"按钮,调用 `cx.set_theme("dark")` / `cx.set_theme("light")`。

#### 7.4 资源嵌入注入

**修改**:[demo/src/main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs)(或 lib.rs)

```rust
rml::embed_assets!();  // 宏展开为 include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))
```

## Assumptions & Decisions

### 关键决策

1. **主题切换粒度**:仅 CSS 变量(`:root` 颜色变量)切换。主题文件只含颜色变量定义,规则文件中颜色用 `var(--xxx)` 引用,其他属性(尺寸/布局)直接写值。覆盖 90% 主题切换场景,实现最简。

2. **i18n 嵌入**:同步改造为嵌入资源,统一资源管理,彻底避免泄露。保留磁盘 fallback 确保开发期灵活性。

3. **主题变量类型**:仅支持 `Color`。非颜色变量(如 `--spacing: 8px`)不参与主题切换,应在全局 CSS 中定义并构建期内联。codegen 对 `var(--xxx)` 一律生成 `rml::theme::color("--xxx")`。

4. **with_style 兼容性**:保留 `with_style` 用于全局 CSS 规则(构建期内联),与主题系统协同工作。全局 CSS 中的 `var()` 走运行时主题查询。

5. **资源注册表注入**:通过 `rml::embed_assets!()` 宏在用户 crate 根注入,符合 cargo `OUT_DIR` 模型。

6. **Phase 5 暂缓**:全局 CSS 运行时加载和组件级 CSS 语法需要重写 codegen 样式生成逻辑,影响面大。当前 Phase 1-4 已能交付核心价值。

### 假设

- GPUI 的 `Hsla` 颜色类型满足主题切换需求
- 主题文件中 `:root` 变量均为颜色值(非颜色值忽略并 warning)
- 用户 crate 的 `OUT_DIR` 在编译时可用(cargo 标准行为)
- `include_bytes!` 路径为绝对路径,避免相对路径问题

## Verification Steps

### Phase 1 验证
1. `cargo build -p rust-rml-demo` 成功,`OUT_DIR/rml_generated/rml_assets.rs` 生成
2. 检查生成文件包含 `assets/i18n/zh-CN.json`、`assets/themes/light.css` 等条目
3. 运行 demo,`rml::assets::load("i18n/zh-CN.json")` 返回非 None

### Phase 2-4 验证
1. `cargo build -p rust-rml-demo` 成功
2. 运行 demo,初始主题为 light,界面颜色与 light 主题一致
3. 点击"切换主题"按钮,界面颜色切换为 dark 主题
4. `theme_color_static("--primary-color")` 在 `#[computed]` 中返回正确颜色
5. 检查 codegen 生成的代码,`var(--xxx)` 变为 `rml::theme::color("--xxx")` 调用

### Phase 3 验证
1. 删除 `demo/assets/i18n/` 目录(模拟资源未在磁盘)
2. 运行 demo,i18n 翻译正常显示(从嵌入资源加载)
3. 切换 locale,翻译正常切换

### Phase 6-7 验证
1. `cargo build --release -p rust-rml-demo` 生成单一可执行文件
2. 将可执行文件复制到其他目录运行,无需附带 `assets/` 目录
3. 主题切换、i18n 切换、资源访问均正常

### 回归验证
1. 现有 `with_style("src/styles.css")` 功能不受影响
2. 现有 `extract_i18n` 功能不受影响
3. demo 现有案例(计数器、双向绑定、按钮、i18n)正常工作

## 实现顺序建议

1. Phase 1:资源嵌入基础设施(独立可交付)
2. Phase 4:codegen var() 改造(独立可交付,主题变量先支持但无切换 UI)
3. Phase 2:主题运行时系统(依赖 Phase 1 + 4)
4. Phase 3:i18n 嵌入改造(依赖 Phase 1)
5. Phase 6:build.rs 集成(串联所有)
6. Phase 7:demo 改造(端到端验证)
7. Phase 5:暂缓,后续评估
