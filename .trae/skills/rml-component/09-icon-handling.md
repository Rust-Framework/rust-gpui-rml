# 09 图标处理规范（IconSpec）

> RML 框架所有贡献点图标的统一规格类型是 `IconSpec`，定义在 `rml_core::contribution`。本规范涵盖何时用哪个 variant、嵌入资源路径写法、与 `resolve_icon` 的协作。

## 核心规则

**所有 `IContribution::icon()` 实现必须返回 `Option<IconSpec>`**，不再返回 `Option<SharedString>`。`IconSpec` 的 variant tag 显式声明图标种类，框架无需字符串推断。

```rust
pub enum IconSpec {
    Named(SharedString),  // 内置命名图标
    Path(SharedString),   // SVG 资产路径（含嵌入资源）
    Url(SharedString),    // 外部 URL
}
```

## 三种 Variant 的选择决策

| 场景 | 选择 | 理由 |
|------|------|------|
| 使用 gpui-component 内置图标（`BookOpen`、`Settings` 等） | `IconSpec::named("BookOpen")` | 类型已知，直接查 `IconName` 表 |
| 自定义 SVG 图标（用户嵌入资源） | `IconSpec::path("brand/logo.svg")` | 经 `CompositeAssets` 路由到 `rml_core::assets::load` |
| 加载外部 HTTP/HTTPS 图片 | `IconSpec::url("https://...")` | `Icon::path()` 不接受 URL，必须走 `gpui::img` |
| 不需要图标 | `None` | 框架 fallback 到 `IconName::PanelLeft` |

## 构造器约定

`IconSpec` 提供三个 `impl Into<SharedString>` 构造器，**优先使用构造器**而非直接构造 enum：

```rust
// 推荐
IconSpec::named("BookOpen")
IconSpec::path("logo.svg")
IconSpec::url("https://example.com/icon.png")

// 不推荐（绕过构造器，但语义等价）
IconSpec::Named("BookOpen".into())
```

## 嵌入资源路径写法

RML 框架的 `CompositeAssets`（`rml_app::assets`）已将 gpui-component-assets 与 `rml_core::assets::load` 桥接为统一 `AssetSource`。`IconSpec::Path` 透明支持两种来源：

```rust
// 1. gpui-component 内置图标路径
IconSpec::path("icons/foo.svg")

// 2. 用户嵌入资源（在项目 assets/ 目录下）
IconSpec::path("brand/logo.svg")
IconSpec::path("icons/custom.svg")
```

**路径规范**：
- 使用正斜杠 `/`，不使用反斜杠
- 路径相对资产根（`assets/`），不含前导 `/`
- 嵌入模式由 `build.rs` 的 `.assets(path, embed)` 配置决定（Embedded 编译期 `include_bytes!` / Filesystem 运行期读取）

**资源不存在时**：`Icon::default().path(s)` 渲染空白，不崩溃。可通过 `rml_core::assets::load(path)` 预校验。

## IContribution 实现模板

```rust
use rml_core::contribution::{IContribution, IconSpec};
use gpui::SharedString;

impl IContribution for MyPanel {
    fn id(&self) -> &str { "my-panel" }
    fn name(&self) -> SharedString { "My Panel".into() }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::path("icons/my-panel.svg"))  // 嵌入资源
    }
}
```

## 框架组件构造器签名约定

当组件接受图标参数时（如 `ActivityAct::new`、`ActivityPanel::new`），**参数类型用 `impl Into<IconSpec>`**：

```rust
pub fn new(
    id: impl Into<String>,
    icon: impl Into<IconSpec>,           // 不再是 impl Into<SharedString>
    title: impl Into<SharedString>,
) -> Self
```

调用方既可传 `IconSpec::named("X")`，也可传 `IconSpec::path("y.svg")`。

## 渲染入口

业务代码**不直接调用** `rml_ui::resolve_icon`。它由 `ActivityBar`、`TabWindow` 等宿主组件在渲染时调用：

```rust
// 框架内部代码（业务无需关心）
use rml_ui::activity_bar::icon::resolve_icon;

let icon_element = resolve_icon(panel.icon(), window);
```

`resolve_icon` 按 variant tag 直接分派：
- `Named` → `Icon::new(name).small()`（查 `parse_icon_name` 表，未命中 fallback `PanelLeft`）
- `Path` → `Icon::default().path(s).small()`（经 `CompositeAssets` 解析）
- `Url` → `gpui::img(s).size_4()`
- `None` → `Icon::new(IconName::PanelLeft).small()`

## 反模式

以下写法**已废弃/禁止**：

```rust
// ❌ 返回 SharedString（旧 API，已删除）
fn icon(&self) -> Option<SharedString> {
    Some("BookOpen".into())
}

// ❌ 返回 Any（破坏 trait 契约，框架需 downcast 猜测）
fn icon(&self) -> Option<Box<dyn Any>>;

// ❌ 字符串硬编码 URL/Path 歧义（无 variant tag）
fn icon(&self) -> Option<SharedString> {
    Some("https://...".into())  // 框架要靠 is_url() 推断
}

// ❌ 业务代码自己渲染图标（应交给 resolve_icon）
fn icon(&self) -> Option<IconSpec> { ... }
fn render(&self, ...) -> AnyElement {
    // ❌ 不要在这里手动 Icon::new(...)
}
```

## 设计决策（背景知识）

### 为何 `Named` 载荷是 `SharedString` 而非 `IconName`？

`rml_core` 保持框架中立，不依赖 `gpui-component`。引入 `IconName` 会让 core 所有消费者（CLI、测试、非 UI 上下文）被迫拉入 gpui-component 编译负担。字符串→`IconName` 的映射集中在 `rml_ui::resolve_icon` 的 `parse_icon_name` 表中（单一位置），未来可通过给上游 `icon_named!` 宏加 `FromStr` impl 一次性消除。

### 为何不返回 `Any`？

`Any` 在 Rust 中是反模式逃生口，仅适用于类型集合真正开放的场景（插件系统）。图标种类是封闭集合，`enum` + variant tag 提供编译期穷举检查，比 `Any` + downcast 猜测更安全。

### 为何 `Path` 能透明支持嵌入资源？

`CompositeAssets`（`rml_app::assets`）实现 `gpui::AssetSource`，在 `RmlApplication::run` 时自动注册。它按顺序解析路径：
1. `gpui_component_assets::Assets.load(path)` —— 内置图标
2. `rml_core::assets::load(path)` —— 用户嵌入资源

`Icon::default().path(s)` 走 GPUI 的 `AssetSource` 接口，因此自动获得嵌入资源支持，无需新 variant 或新 API。

## 完整图标清单参考

- 内置 `IconName` 完整变体：[gpui-component Icon 文档](https://longbridge.github.io/gpui-component/zh-CN/docs/components/icon)
- `parse_icon_name` 匹配表：`crates/ui/src/components/activity_bar/icon.rs`
- 嵌入资源 API：`rml_core::assets::{load, load_str, list}`

## 相关规范

- [01-naming-conventions.md](01-naming-conventions.md) —— 命名规范
- [03-property-classification.md](03-property-classification.md) —— 属性分类
- [08-new-component-checklist.md](08-new-component-checklist.md) —— 新组件检查清单
