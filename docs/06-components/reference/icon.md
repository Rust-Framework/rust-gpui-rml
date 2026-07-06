# IconSpec —— 贡献点图标规格

## 概述

`IconSpec` 是 RML 框架中**所有贡献点图标的统一规格类型**，定义在 `rml_core::contribution`。它是 `IContribution::icon()` 的返回类型，替代了早期未类型化的 `Option<SharedString>`。

设计目标：
- **消除字符串歧义**：variant tag 显式声明图标种类，框架无需 `is_url`/`is_asset_path` 等字符串推断
- **统一封装**：所有图标渲染走同一入口 `rml_ui::resolve_icon`
- **原生支持嵌入资源**：`Path` variant 经 `CompositeAssets` 自动路由到 RML 嵌入资源系统
- **保持 trait 中立**：`rml_core` 不依赖 `gpui-component`，故 `Named` 载荷为 `SharedString` 而非 `IconName`

## 三种 Variant

```rust
pub enum IconSpec {
    Named(SharedString),  // 内置命名图标
    Path(SharedString),   // SVG 资产路径（含嵌入资源）
    Url(SharedString),    // 外部 URL
}
```

### `Named(SharedString)` —— 内置命名图标

字符串对应 gpui-component `IconName` 枚举的变体名（PascalCase，如 `"BookOpen"`、`"Settings"`）。

```rust
fn icon(&self) -> Option<IconSpec> {
    Some(IconSpec::named("BookOpen"))
}
```

**渲染路径**：`Icon::new(name).small()`，未匹配时 fallback `IconName::PanelLeft`。

**完整图标清单**：见 [gpui-component Icon 文档](https://longbridge.github.io/gpui-component/zh-CN/docs/components/icon) 与 `crates/ui/src/components/activity_bar/icon.rs` 中 `parse_icon_name` 的匹配表。

### `Path(SharedString)` —— SVG 资产路径（推荐用于自定义图标）

SVG 资产路径，相对资产根。**这是嵌入资源的入口**。

```rust
fn icon(&self) -> Option<IconSpec> {
    Some(IconSpec::path("logo.svg"))  // 用户嵌入资源
    // 或
    // Some(IconSpec::path("icons/foo.svg"))  // gpui-component 内置图标
}
```

**渲染路径**：`Icon::default().path(s).small()`，经 `CompositeAssets`（`rml_app::assets`）路由解析。

**自动支持两种来源**：
1. **gpui-component 内置图标**：`icons/**/*.svg`（由 `gpui_component_assets::Assets` 提供）
2. **RML 用户嵌入资源**：`assets/<path>`（由 `rml_core::assets::load` 提供，编译期 `include_bytes!` 嵌入或运行期文件系统读取）

无需为嵌入资源引入新 variant 或新 API——`CompositeAssets` 在 `RmlApplication::run` 时自动注册为 GPUI 的 `AssetSource`，`Icon::default().path(s)` 透明地解析到嵌入资源。

### `Url(SharedString)` —— 外部 URL

`http:`/`https:`/`file:` 等协议前缀的外部图片 URL。

```rust
fn icon(&self) -> Option<IconSpec> {
    Some(IconSpec::url("https://example.com/logo.png"))
}
```

**渲染路径**：`gpui::img(s).size_4()`，不经过 `Icon` 组件（`Icon::path()` 不接受 URL）。

**注意**：URL 图片通常自带颜色，不应用 `text_color` 着色。

## Fallback 行为

| 返回值 | 渲染结果 |
|--------|----------|
| `Some(IconSpec::Named(s))` 且 `s` 匹配 | 对应 `IconName` |
| `Some(IconSpec::Named(s))` 且 `s` 未匹配 | `IconName::PanelLeft` |
| `Some(IconSpec::Path(s))` | `Icon::default().path(s)`（路径不存在则渲染空白） |
| `Some(IconSpec::Url(s))` | `gpui::img(s)` |
| `None` | `IconName::PanelLeft` |

## 渲染入口

`rml_ui::resolve_icon` 是唯一的图标渲染函数：

```rust
pub fn resolve_icon(spec: Option<IconSpec>, window: &Window) -> AnyElement
```

业务代码不直接调用 `resolve_icon`——它由 `ActivityBar`、`TabWindow` 等宿主组件在渲染时调用。贡献点只需返回 `IconSpec`，渲染交给框架。

## 嵌入资源集成详解

### 资源注册

RML 的 `build.rs` 在编译期扫描 `assets/` 目录，按 `.assets(path, embed)` 配置生成注册代码：

- **Embedded 模式**：`include_bytes!` 编译期嵌入二进制（推荐用于发布）
- **Filesystem 模式**：运行期从磁盘读取，首次读取后 `Box::leak` 缓存为 `&'static [u8]`

### 资源加载

```rust
// rml_core::assets
pub fn load(path: &str) -> Option<&'static [u8]>;
pub fn load_str(path: &str) -> Option<&'static str>;
pub fn list() -> Vec<&'static str>;
```

### 与图标的桥接

`CompositeAssets`（`rml_app::assets`）实现 `gpui::AssetSource`，按以下顺序解析路径：

1. `gpui_component_assets::Assets.load(path)` —— 内置图标
2. `rml_core::assets::load(path)` —— 用户嵌入资源

`RmlApplication::run` 自动注册 `CompositeAssets`，无需手动配置。`IconSpec::Path("logo.svg")` 因此能直接解析到用户嵌入的 `assets/logo.svg`。

## 完整示例

```rust
use rml_core::contribution::{IContribution, IconSpec};
use gpui::SharedString;

pub struct MyPanel;

impl IContribution for MyPanel {
    fn id(&self) -> &str { "my-panel" }
    fn name(&self) -> SharedString { "My Panel".into() }
    fn icon(&self) -> Option<IconSpec> {
        // 优先使用嵌入资源中的品牌图标
        Some(IconSpec::path("brand/icon.svg"))
    }
}
```

## 设计决策记录

### 为何不返回 `Any`？

`Any` 在 Rust 中是反模式逃生口，仅适用于类型集合真正开放的场景（插件系统）。图标种类是封闭集合，`enum` + variant tag 提供编译期穷举检查，比 `Any` + downcast 猜测更安全。

### 为何 `Named` 载荷是 `SharedString` 而非 `IconName`？

`rml_core` 保持框架中立，不依赖 `gpui-component`。引入 `IconName` 会让 core 所有消费者（CLI、测试、非 UI 上下文）被迫拉入 gpui-component 编译负担。字符串→`IconName` 的映射集中在 `rml_ui::resolve_icon` 的 `parse_icon_name` 表中（单一位置），未来可通过给上游 `icon_named!` 宏加 `FromStr` impl 一次性消除。

### 为何不引入 `IconSpec::Svg(SharedString)` 内联 SVG 字符串？

当前无业务需求。未来如有运行期生成 SVG 的场景，可增加此 variant，编译器会强制 `resolve_icon` 更新匹配臂——这正是封闭 enum 的安全保证。

## 相关文档

- [activity-bar.md](./activity-bar.md) —— `IActivityPanel` 使用 `IconSpec`
- [贡献点架构](../../09-architecture/contribution-system.md) —— `IContribution` trait 详解
- [gpui-component Icon](https://longbridge.github.io/gpui-component/zh-CN/docs/components/icon) —— 上游 `IconName` 完整清单
