# 主题/资源管理 — Demo 集成与端到端验证计划

## 概述

本计划承接此前已完成的 Phase 1-4(框架基础),聚焦剩余的 Phase 6(build.rs 集成)+ Phase 7(demo 端到端验证)。

**已完成(Phase 1-4,不在本计划范围):**
- `crates/engine/src/build/assets_processor.rs` — 构建期扫描 `assets/` 生成 `RML_ASSETS` 注册表
- `crates/core/src/assets.rs` — 运行时 `OnceLock` 资源查询 API
- `crates/core/src/theme.rs` — `ThemeState` Global + `ThemeExt`(`use_theme`/`set_theme`/`theme_color`)
- `crates/core/src/i18n.rs` — `load_catalog_embedded`(嵌入优先 + 磁盘 fallback)
- `crates/engine/src/lib.rs` — `embed_assets!` 宏 + `pub use {assets, i18n, theme}`
- `crates/app/src/application.rs` — `ensure_theme(cx)` 在 `run()` 中调用

**本计划目标:** 将 demo 接入嵌入资源 + 主题系统,修复一处路径前缀 bug,验证主题切换端到端闭环。

---

## 当前状态分析

### 资源 key 约定(关键)

`AssetsProcessor` 扫描 `assets_dir`(如 `"assets"`)时,**key 是相对 `assets/` 根的路径**(正斜杠):
- `assets/i18n/zh-CN.json` → key = `"i18n/zh-CN.json"`
- `assets/themes/dark.css` → key = `"themes/dark.css"`

### 已发现的 Bug:`load_theme_colors_embedded` 路径前缀不匹配

[theme.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs#L232-L240) 中:

```rust
pub fn load_theme_colors_embedded(theme: &str, dir: &str) -> Result<...> {
    let path = format!("{}/{}.css", dir.trim_end_matches('/'), theme);
    let css = crate::assets::load_str(&path)...
}
```

`DEFAULT_THEMES_DIR = "assets/themes"` → `path = "assets/themes/light.css"`,但注册表 key 是 `"themes/light.css"`(无 `assets/` 前缀)。**主题加载必然失败。**

对比 [i18n.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/i18n.rs#L142-L152) 已正确处理:

```rust
let sub_dir = dir.strip_prefix("assets/").unwrap_or(dir);
let path = format!("{}/{}.json", sub_dir.trim_end_matches('/'), locale);
```

### Demo 现状

- [demo/build.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/build.rs):仅 `scan_dir("src").with_style("src/styles.css")`,**无 `assets_dir`**
- [demo/src/main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs):**无 `embed_assets!` / `assets::init`**
- [demo/src/app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs):`cx.use_i18n_with_dir("zh-CN", "demo/assets/i18n")` — 路径前缀 `"demo/"` 与嵌入约定不匹配;**无 `use_theme`**
- [demo/src/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/styles.css):`:root` 块含 4 个颜色变量(`--primary-color`/`--text-color`/`--bg-color`/`--border-color`),需拆分到主题文件
- `demo/assets/themes/` 目录:**不存在**
- [demo/src/shell/main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml):已有 i18n 切换按钮(`on_switch_en`),可仿照添加主题切换

### `var()` 在 mapper 中的行为(已实现)

[mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L29-L33):颜色属性(`background`/`color`)的 `var(--x)` 生成运行时查询 `rml::theme::color("--x")`;非颜色属性仍构建期内联。因此将 `:root` 从 `styles.css` 移除后,所有颜色 `var()` 自动变为运行时主题查询,无需改 mapper。

---

## 实施步骤

### Step 1:修复 `load_theme_colors_embedded` 路径前缀(关键 bug)

**文件:** [crates/core/src/theme.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs#L232-L240)

**改动:** 仿照 i18n 的 `load_catalog_embedded`,在构建嵌入资源 path 前剥离 `"assets/"` 前缀。

```rust
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
```

**理由:** 注册表 key 无 `assets/` 前缀,查询路径必须一致。不修此 bug,主题永远加载失败。

---

### Step 2:创建主题文件

**新建文件:**
- `demo/assets/themes/light.css`
- `demo/assets/themes/dark.css`

**light.css**(沿用 styles.css 现有亮色值):

```css
:root {
    --primary-color: #007bff;
    --text-color: #333333;
    --bg-color: #f8f9fa;
    --border-color: #e5e7eb;
}
```

**dark.css**(典型暗色配色):

```css
:root {
    --primary-color: #3b82f6;
    --text-color: #e5e7eb;
    --bg-color: #1f2937;
    --border-color: #374151;
}
```

**理由:** 仅 `:root` 颜色变量;`parse_theme_css` 只解析 `:root` 块中的 `#hex` 声明。

---

### Step 3:从 `styles.css` 移除 `:root` 块

**文件:** [demo/src/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/styles.css#L5-L15)

**改动:** 删除 `:root { ... }` 块(第 5-15 行)。其余规则(`.login`、`.case-pane` 等)保留不变。

**理由:**
- 颜色 `var()` 引用(`var(--text-color)`、`var(--bg-color)`)在 mapper 中已生成运行时 `rml::theme::color(...)` 查询,不依赖构建期 `vars`。
- 移除 `:root` 后,颜色变量唯一来源是主题文件,确保主题切换生效。
- `--primary-color`/`--border-color` 虽在 styles.css 中未实际引用,但保留在主题文件中供未来组件使用。

---

### Step 4:demo/build.rs 注册 assets_dir

**文件:** [demo/build.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/build.rs)

**改动:** 在 `.with_style("src/styles.css")` 后追加 `.assets_dir("assets")`。

```rust
fn main() {
    rml::build()
        .scan_dir("src")
        .with_style("src/styles.css")
        .assets_dir("assets")
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

**理由:** `assets_dir("assets")` 相对 `CARGO_MANIFEST_DIR`(即 `demo/`),扫描 `demo/assets/`,生成 `OUT_DIR/rml_generated/rml_assets.rs`。

---

### Step 5:demo/src/main.rs 注入嵌入资源

**文件:** [demo/src/main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs)

**改动:** 在 `fn main()` 中、`app::run()` 前调用 `embed_assets!` 宏并初始化资源注册表。

```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

mod app;
mod cases;
mod login;
mod shell;

// 嵌入 assets/ 资源到二进制(由 build.rs 生成 RML_ASSETS 注册表)
rml::embed_assets!();

fn main() {
    rml::assets::init(RML_ASSETS);
    app::run();
}
```

**理由:** `embed_assets!()` 展开 `include!(concat!(env!("OUT_DIR"), "/rml_generated/rml_assets.rs"))`,定义 `RML_ASSETS` 静态切片;`assets::init` 写入 `OnceLock` 供 `load_str`/`load` 查询。

---

### Step 6:demo/src/app.rs 初始化 i18n + 主题

**文件:** [demo/src/app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs)

**改动:**
1. 将 `use_i18n_with_dir("zh-CN", "demo/assets/i18n")` 改为 `use_i18n("zh-CN")` — 匹配嵌入约定(`DEFAULT_I18N_DIR = "assets/i18n"`,`load_catalog_embedded` 剥离前缀后查询 `"i18n/zh-CN.json"`,与注册表 key 一致)。
2. 追加 `cx.use_theme("light")` — 初始化亮色主题。
3. 导入 `ThemeExt`。

```rust
use gpui::App;
use rml_app::{IAppLifecycle, RmlApplication};
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;
use rml_core::window::IWindow;

use crate::login::LoginWindow;

#[derive(Default)]
pub struct AppBootstrap;

impl IAppLifecycle for AppBootstrap {
    fn on_launch(&mut self, cx: &mut App) {
        cx.use_i18n("zh-CN");
        cx.use_theme("light");
        LoginWindow::default().open(cx);
    }
}

pub fn run() {
    RmlApplication::new().run::<AppBootstrap>();
}
```

**理由:**
- `use_i18n("zh-CN")` 使用 `DEFAULT_I18N_DIR`,嵌入资源 key 匹配;磁盘 fallback 路径 `"assets/i18n/..."` 在 workspace 根目录运行时不存在,但嵌入优先成功所以不影响。
- `use_theme("light")` 触发 `load_theme_colors_embedded("light", "assets/themes")` → (Step 1 修复后)查询 `"themes/light.css"` → 命中注册表 → 解析颜色 → 激活。
- 主题在 `on_launch` 初始化,登录窗与主窗均能通过 `theme_color`/`theme_color_static` 取色。

---

### Step 7:主窗口添加主题切换按钮

**文件:** [demo/src/shell/main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml)

**改动:** 在 i18n 案例区(`active_case_id == "i18n.basic"`)追加主题切换按钮,仿照 `on_switch_en` 模式。

在第 48-52 行的 i18n 案例块内,追加:

```xml
<div if={active_case_id == "i18n.basic"} class="case-pane">
    <h2 class="case-title">{t("case.i18n.title")}</h2>
    <p>{t("demo.hello")}</p>
    <Button label={t("menu.lang_en")} onclick={on_switch_en} />
    <Button label={t("menu.theme_toggle")} onclick={on_toggle_theme} />
</div>
```

**文件:** [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

**改动:** 添加 `on_toggle_theme` 命令,在 `light`/`dark` 间切换。

```rust
use rml_core::theme::ThemeExt;

// 在 impl MainWindow 块中追加:
#[command]
pub fn on_toggle_theme(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    let next = if cx.current_theme() == "dark" { "light" } else { "dark" };
    cx.set_theme(next);
    self.i18n_version = self.i18n_version.wrapping_add(1);
    cx.notify();
}
```

**理由:**
- `cx.current_theme()` 读当前主题名;`cx.set_theme(next)` 切换并 `refresh_windows()`(已在 ThemeExt 实现中)。
- bump `i18n_version` 触发 `#[computed]` 重算(菜单/状态栏等依赖 `t_static` 的计算属性)。
- `set_theme` 内部从嵌入资源加载 `dark.css`(若未缓存),无需手动 preload。

---

### Step 8:补充 i18n 翻译 key

**文件:** [demo/assets/i18n/zh-CN.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json) 和 [demo/assets/i18n/en-US.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/en-US.json)

**改动:** 追加 `"menu.theme_toggle"` 键。

zh-CN.json:
```json
"menu.theme_toggle": "切换主题"
```

en-US.json(对应追加):
```json
"menu.theme_toggle": "Toggle Theme"
```

---

## 假设与决策

| 决策点 | 选择 | 理由 |
|---|---|---|
| 主题切换粒度 | 仅 CSS 颜色变量(`:root` 中的 `#hex`) | 用户已确认;覆盖 90% 场景 |
| i18n 嵌入 | 统一嵌入 | 用户已确认;`load_catalog_embedded` 已实现 |
| 全局 CSS 加载 | 保留 `with_style("src/styles.css")` 构建期加载 | 用户已确认;全局样式在 build.rs 中加载 |
| demo i18n dir | 改用 `use_i18n("zh-CN")`(默认 dir) | 嵌入资源 key 无 `demo/` 前缀,必须用 `DEFAULT_I18N_DIR` |
| 主题切换 UI | 在 i18n 案例区添加按钮 | 仿照现有 `on_switch_en` 模式,最小改动 |
| `--primary-color`/`--border-color` | 保留在主题文件中 | styles.css 当前未引用,但主题文件应完整定义调色板 |

---

## 验证步骤

### 1. 编译验证

```powershell
cargo build -p rust-rml-core
cargo build -p rust-rml-demo
```

预期:全量编译通过,无 warning(除已知的非相关 warning)。

### 2. 单元测试

```powershell
cargo test -p rust-rml-core -- theme
cargo test -p rust-rml-core -- assets
cargo test -p rust-rml-engine -- assets_processor
cargo test -p rust-rml-engine -- mapper
```

预期:主题解析、资源加载、mapper 运行时查询生成相关测试全部通过。

### 3. 运行时验证

```powershell
cargo run -p rust-rml-demo
```

验证清单:
- [ ] 登录窗正常显示(亮色主题)
- [ ] 登录后主窗口显示,背景为 `#f8f9fa`(亮色 `--bg-color`)
- [ ] 文字颜色为 `#333333`(亮色 `--text-color`)
- [ ] 进入 "国际化 t()" 案例,点击 "切换主题" 按钮
- [ ] 窗口立即刷新,背景变为 `#1f2937`(暗色),文字变为 `#e5e7eb`
- [ ] 再次点击切换回亮色
- [ ] 切换 i18n 到 English 后,主题切换按钮文字为 "Toggle Theme",主题切换功能仍正常

### 4. 资源嵌入验证(可选)

确认 `OUT_DIR/rml_generated/rml_assets.rs` 包含主题文件:
```powershell
cargo build -p rust-rml-demo
# 检查 OUT_DIR(可通过 cargo metadata 或在 target/ 下搜索)
# rml_assets.rs 应包含 ("themes/light.css", include_bytes!(...)) 和 ("themes/dark.css", ...)
```

---

## 文件变更清单

| 文件 | 操作 | 说明 |
|---|---|---|
| `crates/core/src/theme.rs` | 修改 | Step 1:修复 `load_theme_colors_embedded` 路径前缀 |
| `demo/assets/themes/light.css` | 新建 | Step 2:亮色主题 |
| `demo/assets/themes/dark.css` | 新建 | Step 2:暗色主题 |
| `demo/src/styles.css` | 修改 | Step 3:移除 `:root` 块 |
| `demo/build.rs` | 修改 | Step 4:追加 `.assets_dir("assets")` |
| `demo/src/main.rs` | 修改 | Step 5:追加 `embed_assets!` + `assets::init` |
| `demo/src/app.rs` | 修改 | Step 6:`use_i18n` + `use_theme("light")` |
| `demo/src/shell/main_window.rml` | 修改 | Step 7:主题切换按钮 |
| `demo/src/shell/main_window.rml.rs` | 修改 | Step 7:`on_toggle_theme` 命令 |
| `demo/assets/i18n/zh-CN.json` | 修改 | Step 8:追加 `menu.theme_toggle` |
| `demo/assets/i18n/en-US.json` | 修改 | Step 8:追加 `menu.theme_toggle` |

**实施顺序:** Step 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8(严格顺序,Step 1 是前置依赖)。
