# Avatar 组件 RML 实现计划

## 概述

在 RML 框架中实现 gpui-component 的 `Avatar` 与 `AvatarGroup` 组件支持，严格遵循现有组件注册模式（tags.rs 路由表 + props\_registry + compiler 模块化 setter + 文档 + demo），职责划分清晰。

## 现状分析

### gpui-component Avatar API（已确认）

**Avatar**（`crates/ui/src/components/avatar/avatar.rs`）：

* `Avatar::new()` — 无参构造（`RenderOnce`，无 `ElementId`）

* `.src(impl Into<ImageSource>)` — 图片源

* `.name(impl Into<SharedString>)` — 用户名（无图片时显示首字母回退）

* `.placeholder(impl Into<Icon>)` — 占位图标（默认 `IconName::User`）

* 实现 `Sizable`（`.small()`/`.xsmall()`/`.large()`/`.with_size(px(N))`）

* 实现 `Styled`（border/shadow/rounded 等）

* 实现 `InteractiveElement`（`.on_click(...)`）

* **未实现** `ParentElement` — 不接受 `.child()` 子元素

**AvatarGroup**（`crates/ui/src/components/avatar/avatar_group.rs`）：

* `AvatarGroup::new()` — 无参构造

* `.child(Avatar)` — 添加头像（接受 `Avatar` 类型，非 `AnyElement`）

* `.children(impl IntoIterator<Item = Avatar>)` — 批量添加

* `.limit(usize)` — 最大显示数量（默认 3）

* `.ellipsis()` — 超限时显示省略标记

* 实现 `Sizable`（尺寸作用于所有子头像）

### RML 框架现有模式（已确认）

1. **路由表** `crates/engine/src/tags.rs::component_lookup()` — PascalCase 标签到 `ComponentTag { ctor_path, kind }` 映射
2. **属性注册** `crates/engine/src/compiler/props_registry.rs` — `COMMON_STATIC_PROPS`（含 `placeholder`）+ `COMPONENT_PROPS`（组件专用）
3. **Setter 分发** `crates/engine/src/compiler/component.rs`：

   * `component_static_setter` — 通用静态属性映射（`placeholder` → `.placeholder("...")` 字符串版本）

   * `component_bind_setter` — 通用绑定属性映射

   * 组件专用 setter 通过委托（如 `super::menu::bind_setter`、`super::accordion::static_setter`）
4. **容器判定** `gen_component` 中 `is_container = StatelessNoId && tag != menu/MenuBar/status_bar` — 容器组件用 `.child()` 注入子节点
5. **文本子节点** 非容器组件的文本子节点映射到 `.label("text")`
6. **模块化编译器** `crates/engine/src/compiler/{accordion,menu,input,tree}/` — 每个复杂组件族有独立模块

### 关键约束

* `Avatar::new()` 无 `ElementId` 参数 → 必须注册为 `StatelessNoId`

* `Avatar` 未实现 `ParentElement` → 不能作为容器处理（否则生成 `.child()` 编译失败）

* `Avatar` 无 `.label()` 方法 → 文本子节点不能走默认 `.label()` 路径

* `placeholder` 在通用 setter 中映射为字符串 `.placeholder("...")`，但 Avatar 需要 `Icon` 类型 → 必须用专用 setter 委托优先处理

* `AvatarGroup::child()` 接受 `Avatar` 类型 → 现有容器 codegen 生成的 `.child(rml_ui::Avatar::new()...)` 可直接工作

* `src` 在通用 setter 的跳过列表中（`"src" => None`）→ 必须用专用 setter 委托优先处理

## 设计决策

### 1. ComponentKind 选择

| 组件            | kind            | 理由                                  |
| ------------- | --------------- | ----------------------------------- |
| `Avatar`      | `StatelessNoId` | `Avatar::new()` 无 ElementId 参数      |
| `AvatarGroup` | `StatelessNoId` | `AvatarGroup::new()` 无 ElementId 参数 |

### 2. 容器判定调整

`Avatar` 是 `StatelessNoId` 但是叶子组件（无 `ParentElement`），必须加入 `is_container` 排除列表。
`AvatarGroup` 是 `StatelessNoId` 且是容器（有 `.child(Avatar)`），保持默认容器行为。

### 3. 文本子节点映射

`Avatar` 无 `.label()` 方法。决策：**文本子节点映射到** **`.name()`**（与 Button 文本→`.label()` 模式一致，符合人体工程学）。

```html
<Avatar>John Doe</Avatar>   →  Avatar::new().name("John Doe")
<Avatar name="John Doe" />  →  Avatar::new().name("John Doe")
<avatar>John Doe</avatar>   →  Avatar::new().name("John Doe")
<avatar name="John Doe" />  →  Avatar::new().name("John Doe")
```

### 4. 专用 setter 模块

创建 `crates/engine/src/compiler/avatar/` 模块（与 `accordion/`、`input/` 模式一致），包含：

* `mod.rs` — 模块入口与导出

* `setters.rs` — Avatar 与 AvatarGroup 的 static/bind setter

委托优先级：`avatar::static_setter` > 通用 `component_static_setter`（确保 `placeholder`/`src` 走 Avatar 专用路径）

### 5. 属性注册

| 组件            | 专用属性                | 说明                                     |
| ------------- | ------------------- | -------------------------------------- |
| `Avatar`      | `src`, `name`       | `placeholder` 已在 `COMMON_STATIC_PROPS` |
| `AvatarGroup` | `limit`, `ellipsis` | —                                      |

### 6. UI 封装

创建 `crates/ui/src/components/avatar.rs`（与 `alert_dialog.rs` 模式一致）—— 带 RML 映射文档注释的 re-export，非自定义包装。

## 变更清单

### A. UI 层（crates/ui）

#### A1. 新建 `crates/ui/src/components/avatar.rs`

带文档的 re-export（参照 `alert_dialog.rs`）：

```rust
//! Avatar 封装 —— 基于 gpui-component 的 Avatar / AvatarGroup
//!
//! RML `<Avatar>` 编译为 `rml_ui::Avatar::new().<setters>...`：
//! - `src` 属性 → `Avatar::src`（图片源）
//! - `name` 属性 / 文本子节点 → `Avatar::name`（用户名，无图片时回退为首字母）
//! - `placeholder` 属性 → `Avatar::placeholder(IconName::...)`（占位图标）
//! - `small`/`xsmall`/`large` → Sizable 尺寸
//!
//! RML `<AvatarGroup>` 编译为 `rml_ui::AvatarGroup::new().<setters>.child(Avatar)...`：
//! - `limit` 属性 → `AvatarGroup::limit`（最大显示数量）
//! - `ellipsis` 标志 → `AvatarGroup::ellipsis()`（超限省略标记）
//! - 子节点必须是 `<Avatar>` 元素

pub use gpui_component::avatar::{Avatar, AvatarGroup};
```

#### A2. 修改 `crates/ui/src/components/mod.rs`

* 添加 `pub mod avatar;`

* 添加 `pub use avatar::{Avatar, AvatarGroup};`

#### A3. 修改 `crates/ui/src/lib.rs`

在 `pub use components::{...}` 块中添加 `Avatar, AvatarGroup`。

#### A4. 修改 `crates/ui/src/prelude.rs`

在 `pub use crate::{...}` 块中添加 `Avatar, AvatarGroup`。

### B. Engine 路由层（crates/engine）

#### B1. 修改 `crates/engine/src/tags.rs`

在 `component_lookup()` 添加：

```rust
"Avatar" => Some(ComponentTag {
    ctor_path: "rml_ui::Avatar",
    kind: ComponentKind::StatelessNoId,
}),
"AvatarGroup" => Some(ComponentTag {
    ctor_path: "rml_ui::AvatarGroup",
    kind: ComponentKind::StatelessNoId,
}),
```

#### B2. 修改 `crates/engine/src/compiler/props_registry.rs`

在 `COMPONENT_PROPS` 添加：

```rust
("Avatar", &["src", "name"]),
("AvatarGroup", &["limit", "ellipsis"]),
```

#### B3. 新建 `crates/engine/src/compiler/avatar/mod.rs`

```rust
//! Avatar / AvatarGroup 组件 codegen 模块入口。
//!
//! 构造器由 `component::gen_component` 的 `StatelessNoId` 分支统一处理，
//! 本模块仅提供专用 setter（src/name/placeholder/limit/ellipsis）。

pub mod setters;

pub use setters::{bind_setter, static_setter};
```

#### B4. 新建 `crates/engine/src/compiler/avatar/setters.rs`

```rust
//! Avatar / AvatarGroup 专用属性 → builder 方法映射。
//!
//! 由 `component::component_static_setter` / `component_bind_setter`
//! 在 tag 为 "Avatar" 或 "AvatarGroup" 时委托调用。
//! 未命中返回 None，由公共 setter 回退到通用属性（Sizable、disabled 等）。

use crate::parser::ast::EventHandler;

/// 静态属性 → builder 方法
///
/// - Avatar: `src="url"` → `.src("url")`，`name="John"` → `.name("John")`，
///   `placeholder="UserCircle"` → `.placeholder(rml_ui::IconName::UserCircle)`
/// - AvatarGroup: `limit="3"` → `.limit(3)`，`ellipsis=""` → `.ellipsis()`
pub fn static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    match tag {
        "Avatar" => match name {
            "src" => Some(format!(".src({:?})", value)),
            "name" => Some(format!(".name({:?})", value)),
            "placeholder" => Some(format!(".placeholder(rml_ui::IconName::{})", value)),
            _ => None,
        },
        "AvatarGroup" => match name {
            "limit" => Some(format!(".limit({})", value)),
            "ellipsis" => {
                if value.is_empty() || value.eq_ignore_ascii_case("true") {
                    Some(".ellipsis()".to_string())
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    }
}

/// 绑定属性 → builder 方法
///
/// - Avatar: `src={url}` → `.src(self.url.clone())`，`name={user.name}` → `.name(self.user.name.clone())`，
///   `placeholder={icon}` → `.placeholder(self.icon)`
/// - AvatarGroup: `limit={count}` → `.limit(self.count)`
pub fn bind_setter(
    name: &str,
    expr_str: &str,
    loop_vars: &[&str],
    computed: &[&str],
    tag: &str,
) -> Option<String> {
    match tag {
        "Avatar" => match name {
            "src" | "name" => {
                let rust_expr = super::super::component::component_bind_rust_expr(
                    expr_str, loop_vars, computed,
                );
                Some(format!(".{}({}.clone())", name, rust_expr))
            }
            "placeholder" => {
                let rust_expr = super::super::component::component_bind_rust_expr(
                    expr_str, loop_vars, computed,
                );
                Some(format!(".placeholder({})", rust_expr))
            }
            _ => None,
        },
        "AvatarGroup" => match name {
            "limit" => {
                let rust_expr = super::super::component::component_bind_rust_expr(
                    expr_str, loop_vars, computed,
                );
                Some(format!(".limit({})", rust_expr))
            }
            _ => None,
        },
        _ => None,
    }
}
```

附单元测试（参照 `accordion/setters.rs` 测试模式），覆盖：

* `static_setter_avatar_src/name/placeholder`

* `static_setter_avatar_group_limit/ellipsis`

* `bind_setter_avatar_src/name`

* `bind_setter_avatar_group_limit`

* 未命中返回 None

#### B5. 修改 `crates/engine/src/compiler/mod.rs`

添加 `pub mod avatar;`。

#### B6. 修改 `crates/engine/src/compiler/component.rs`

**变更 1**：在 `component_static_setter` 顶部添加 avatar 委托（参照 menu 委托模式）：

```rust
pub fn component_static_setter(name: &str, value: &str, tag: &str) -> Option<String> {
    // 组件专用 static setter 委托（Avatar/AvatarGroup 的 src/name/placeholder/limit/ellipsis）
    if let Some(s) = super::avatar::static_setter(name, value, tag) {
        return Some(s);
    }
    match name {
        // ... 现有代码不变
    }
}
```

**变更 2**：在 `component_bind_setter` 顶部添加 avatar 委托（参照 menu 委托）：

```rust
pub fn component_bind_setter(...) -> Option<String> {
    // 组件专用 bind setter 委托（menu/MenuBar/status_bar 的 items 属性）
    if let Some(s) = super::menu::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    // Avatar/AvatarGroup 的 src/name/placeholder/limit 属性
    if let Some(s) = super::avatar::bind_setter(name, expr_str, loop_vars, computed, tag) {
        return Some(s);
    }
    match name { ... }
}
```

**变更 3**：在 `gen_component` 中将 Avatar 排除出容器判定，并将文本子节点映射到 `.name()`：

```rust
// Avatar 是 StatelessNoId 但叶子组件（无 ParentElement），不是容器
let is_container = matches!(component.kind, tags::ComponentKind::StatelessNoId)
    && resolved != "menu"
    && resolved != "MenuBar"
    && resolved != "status_bar"
    && resolved != "Avatar";
```

```rust
} else if !label_set_by_attr {
    for child in &elem.children {
        if let Node::Text(text) = child {
            // Avatar 用 .name() 接收文本子节点（无 .label() 方法）
            let method = if resolved == "Avatar" { "name" } else { "label" };
            code.push_str(&format!(".{}({:?})", method, text));
            break;
        }
    }
}
```

**变更 4**：添加 `gen_component` 测试用例（参照现有 Button 测试）：

* `gen_component_avatar_minimal` — `<Avatar />` → `rml_ui::Avatar::new()`

* `gen_component_avatar_with_src_and_name` — `<Avatar src="..." name="John" />`

* `gen_component_avatar_with_placeholder` — `<Avatar placeholder="UserCircle" />`

* `gen_component_avatar_text_child_maps_to_name` — `<Avatar>John</Avatar>` → `.name("John")`

* `gen_component_avatar_group_with_children` — `<AvatarGroup limit="3"><Avatar /></AvatarGroup>`

* `gen_component_avatar_group_ellipsis` — `<AvatarGroup ellipsis="" />`

### C. 文档层（docs）

#### C1. 新建 `docs/06-components/reference/avatar.md`

参照 `badge.md` 结构，包含：

* 概述（路由到 `rml_ui::Avatar` / `rml_ui::AvatarGroup`，StatelessNoId）

* 基本用法（图片头像、首字母回退、占位图标）

* AvatarGroup 用法（基础分组、限制数量、省略标记、批量添加）

* 属性表（Avatar: src/name/placeholder + 通用 Sizable/Styled；AvatarGroup: limit/ellipsis + Sizable）

* 子节点规则（Avatar: 文本子节点→`.name()`；AvatarGroup: 子节点必须是 `<Avatar>`）

* 完整示例

* 常见错误（AvatarGroup 子节点非 Avatar、placeholder 值非 IconName 枚举名）

* 相关组件

* RML 未覆盖的 API（`.with_size(px(N))`、`.children(Vec<Avatar>)` 需 Rust 手写）

#### C2. 修改 `docs/06-components/reference/INDEX.md`

在「表单（Form）」表添加：

```markdown
| [avatar.md](./avatar.md) | `Avatar` / `AvatarGroup` | StatelessNoId |
```

### D. Demo 层（demo）

#### D1. 新建 `demo/src/cases/avatar_case.rml`

```html
<component>
    <div v_flex="" class="case-pane">
        <h2>{t("case.avatar.title")}</h2>

        <h3>{t("case.avatar.image")}</h3>
        <div h_flex="" gap_4="">
            <Avatar src="https://avatars.githubusercontent.com/u/5518?v=4" large="" />
            <Avatar src="https://avatars.githubusercontent.com/u/28998859?v=4" />
            <Avatar src="https://avatars.githubusercontent.com/u/20092316?v=4" small="" />
        </div>

        <h3>{t("case.avatar.initials")}</h3>
        <div h_flex="" gap_4="">
            <Avatar name="Jason Lee" large="" />
            <Avatar name="Floyd Wang" />
            <Avatar name="Alice" small="" />
        </div>

        <h3>{t("case.avatar.placeholder")}</h3>
        <div h_flex="" gap_4="">
            <Avatar large="" />
            <Avatar placeholder="Building2" />
        </div>

        <h3>{t("case.avatar.group")}</h3>
        <AvatarGroup limit="3" ellipsis="">
            <Avatar src="https://avatars.githubusercontent.com/u/5518?v=4" />
            <Avatar src="https://avatars.githubusercontent.com/u/28998859?v=4" />
            <Avatar src="https://avatars.githubusercontent.com/u/20092316?v=4" />
            <Avatar src="https://avatars.githubusercontent.com/u/22312482?v=4" />
            <Avatar name="John Doe" />
        </AvatarGroup>
    </div>
</component>
```

#### D2. 新建 `demo/src/cases/avatar_case.rml.rs`

参照 `button_case.rml.rs`，实现 `AvatarCase` 结构体 + `IContribution` + `ILifecycle`：

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.avatar",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct AvatarCase;

impl IContribution for AvatarCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.avatar.title").into()
    }
}

impl ILifecycle for AvatarCase {}
```

#### D3. 修改 `demo/src/cases/mod.rs`

添加：

```rust
#[path = "avatar_case.rml.rs"]
pub mod avatar_case;
```

#### D4. 修改 `demo/src/cases/catalog.rs`

在 `case_title_key` 添加：

```rust
"components.avatar" => "case.avatar.title",
```

#### D5. 修改 `demo/assets/i18n/zh-CN.json`

添加：

```json
"case.avatar.title": "头像",
"case.avatar.image": "图片头像",
"case.avatar.initials": "首字母回退",
"case.avatar.placeholder": "占位图标",
"case.avatar.group": "头像分组",
```

#### D6. 修改 `demo/assets/i18n/en-US.json`

添加：

```json
"case.avatar.title": "Avatar",
"case.avatar.image": "Image avatar",
"case.avatar.initials": "Initials fallback",
"case.avatar.placeholder": "Placeholder icon",
"case.avatar.group": "Avatar group",
```

## 假设与决策

1. **Avatar 不需要自定义包装**：gpui-component 的 `Avatar` 直接 re-export 即可，无需在 ui crate 中包装（与 Badge/Button 模式一致）
2. **文本子节点映射到** **`.name()`**：与 Button 文本→`.label()` 模式一致，提供人体工程学的简写 `<Avatar>John</Avatar>`
3. **`placeholder`** **属性接受 IconName 枚举名**：如 `placeholder="Building2"` → `.placeholder(rml_ui::IconName::Building2)`（与 Accordion 的 `icon` 属性模式一致）
4. **`with_size(px(N))`** **不在 RML 中支持**：自定义像素尺寸需 Rust 手写；RML 仅支持 `small`/`xsmall`/`large`/默认 medium 尺寸
5. **AvatarGroup 子节点必须是** **`<Avatar>`**：容器 codegen 生成的 `.child(rml_ui::Avatar::new()...)` 直接匹配 `AvatarGroup::child(Avatar)` 签名
6. **order = 12**：紧随 accordion（order = 11 未确认，按 demo mod.rs 顺序推断）

## 验证步骤

1. **编译验证**：

   ```bash
   cargo build -p rust-rml-ui
   cargo build -p rust-rml-engine
   cargo build -p rust-rml-demo
   ```

2. **单元测试**：

   ```bash
   cargo test -p rust-rml-engine --lib
   ```

   验证新增的 avatar setter 测试 + props\_registry 一致性测试通过

3. **属性注册一致性**：

   ```bash
   cargo test -p rust-rml-engine --test props_registry_complete
   ```

   验证 `COMPONENT_PROPS` 中的 Avatar/AvatarGroup 在 `component_lookup` 中已注册

4. **Demo 运行**：

   ```bash
   cargo run -p rust-rml-demo
   ```

   在案例树中打开「头像」案例，验证：

   * 图片头像显示

   * 首字母回退（彩色背景）

   * 占位图标

   * AvatarGroup 限制数量 + 省略标记

5. **Clippy 检查**：

   ```bash
   cargo clippy -p rust-rml-ui -p rust-rml-engine -p rust-rml-demo -- -D warnings
   ```

## 文件影响清单

| 文件                                             | 操作 | 说明                           |
| ---------------------------------------------- | -- | ---------------------------- |
| `crates/ui/src/components/avatar.rs`           | 新建 | 文档化 re-export                |
| `crates/ui/src/components/mod.rs`              | 修改 | 注册 avatar 模块                 |
| `crates/ui/src/lib.rs`                         | 修改 | re-export Avatar/AvatarGroup |
| `crates/ui/src/prelude.rs`                     | 修改 | 加入 prelude                   |
| `crates/engine/src/tags.rs`                    | 修改 | 路由表注册                        |
| `crates/engine/src/compiler/mod.rs`            | 修改 | 注册 avatar 编译器模块              |
| `crates/engine/src/compiler/avatar/mod.rs`     | 新建 | 编译器模块入口                      |
| `crates/engine/src/compiler/avatar/setters.rs` | 新建 | 专用 setter + 测试               |
| `crates/engine/src/compiler/props_registry.rs` | 修改 | 注册组件属性                       |
| `crates/engine/src/compiler/component.rs`      | 修改 | 委托 + 容器排除 + 文本映射 + 测试        |
| `docs/06-components/reference/avatar.md`       | 新建 | 组件文档                         |
| `docs/06-components/reference/INDEX.md`        | 修改 | 索引注册                         |
| `demo/src/cases/avatar_case.rml`               | 新建 | demo 模板                      |
| `demo/src/cases/avatar_case.rml.rs`            | 新建 | demo code-behind             |
| `demo/src/cases/mod.rs`                        | 修改 | 注册 demo 模块                   |
| `demo/src/cases/catalog.rs`                    | 修改 | 案例标题 key                     |
| `demo/assets/i18n/zh-CN.json`                  | 修改 | 中文翻译                         |
| `demo/assets/i18n/en-US.json`                  | 修改 | 英文翻译                         |

