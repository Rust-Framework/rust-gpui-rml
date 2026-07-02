# RML 贡献点架构重构计划

## 摘要

本计划对 RML 框架的贡献点机制进行架构级重构，并同步重组 demo 项目，使其真正体现「声明式 + MVVM 数据驱动」的设计初衷。

核心改动：
1. **框架核心**：新增 `IVisualContribution` 独立 trait（简化签名，不暴露 `ComponentEntityCacheImpl`），形成 `IContribution` / `IVisualContribution` / `IContributionHost` 三接口清晰分工；`ContributionRenderContext` 重命名为 `RenderContext`
2. **contributionhost 宏**：进一步精简，只生成注册函数 + `pub const ID`，不再自动 impl `IContributionHost`，移除 `bindings` 参数
3. **contribute 宏**：参数 `host` 改名为 `host_id`，只接受字符串字面量（`host_id = "demo.shell"`），彻底移除 `host = Type` 强依赖路径
4. **Demo**：所有 `#[contribute(host = MainWindow)]` 改为 `#[contribute(host_id = "demo.shell")]`；**删除 `case_host.rml` / `case_host.rml.rs`**——IVisualContribution 共享用于构造 TreeItems，选中后直接渲染到 tab body；**移除 `refresh_bindings`**——UI 更新由框架响应式机制自动处理

---

## 一、现状分析（回答用户的 4 个理解性问题）

### 问题 1：`case_host.rml` / `case_host.rml.rs` 是干什么用的？

**职责**：聚合 10 个案例组件的渲染宿主，但**不是** `#[contributehost]` 标注的贡献宿主，只是一个普通 `#[component]`。

- [case_host.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_host.rml)：根节点 `<component class="case-host">`，内部是 **10 个 `<div if={active_case_id == "..."}>`** 硬编码 if 链，按 `active_case_id` 字符串路由到对应 case 组件
- [case_host.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_host.rml.rs)：持有 10 个 `Option<Entity<XxxCase>>` 私有字段，`on_loaded` 中**一次性预创建全部 10 个 Entity**（非懒加载）

**问题所在**：IVisualContribution 本应共享——用于构造 TreeItems（案例树），选中后直接渲染到 tab body。case_host.rml 的 if 链完全是多余的中间层：新增 case 需同时改 `.rml`（加 if 分支）和 `.rs`（加字段 + on_loaded 初始化），**违背了贡献点「注册即生效」的初衷**。这正是用户感受到的「代码怪异、难以理解」的根源。

**处理**：**删除 `case_host.rml` 和 `case_host.rml.rs`**。MainWindow 的 tab body 直接渲染选中的 IVisualContribution。

### 问题 2：`menu_shell_contribs.rs` 应该放在 cases 中吗？如何定义菜单？

**职责**：定义 Shell 主菜单（File/View/Help 三级菜单）的 13 个 `#[contribute]` 数据贡献结构体，`host = MainWindow`，`kind = "menu"`，叶子命令在 `MainWindow.menu_commands` 绑定 `ICommand`。

**不应该移到 cases/**：这些是 **Shell 自身的主菜单**（File>New/Open/Exit、View>Theme、Help>About），不属于任何具体案例。cases/ 中的 `menu_context_case`、`menu_dropdown_case`、`menu_editor_case` 等才是「如何定义菜单」的案例示例。两者职责不同：
- `menu_shell_contribs.rs` = 应用框架自身的菜单（Shell 级）
- `cases/menu_*.rs` = 教学案例，演示菜单 API 用法（Case 级）

**保留在 shell/，但补充文档说明**其定位，避免与 cases/ 的菜单案例混淆。

### 问题 3：`shell_meta.rs` 干什么用？

**职责**：定义**案例树分类节点**和**状态栏项**的元数据贡献（[shell_meta.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_meta.rs)，24 行）：

- 4 个分类节点：`CatBinding`(order=0) / `CatComponents`(order=10) / `CatMenu`(order=15) / `CatI18n`(order=20)，`kind = "case"`，作为案例树父节点（各 case 用 `parent_id = "cat.xxx"` 挂载）
- 1 个状态栏项：`StatusReady`(order=0)，`kind = "status"`

**定位**：Shell 容器的元数据（分类骨架 + 状态栏），非 case 内容。保留在 shell/。

### 问题 4：`shell_chrome.rs` 是干什么的？为什么在 shell/？

**职责**：**贡献 registry → Shell 控件数据的应用层映射层**（[shell_chrome.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_chrome.rs)，145 行）：

- `entries_in_slot(host_id, slot)`：从 registry 过滤 `effective_slot() == slot` 的条目
- `map_status_items`：slot="status" → `StatusBarItems`，按 order 排序，读 `properties["align"]` 决定 Left/Right
- `map_menu_items`：slot="menu" → `MenuItems`，按 `parent_id` 分组递归建树，绑定 `commands: HashMap<String, Arc<dyn ICommand>>`
- `map_case_tree_items`：slot="case" → `Vec<TreeItem>`，按 parent_id 建树
- `map_shell_chrome`：聚合上述三者 + activity 面板，返回 `ShellChromeBindings`

**应该在 shell/**：这是连接 contribution registry 与 gpui-component UI 控件的**桥梁层**，是 Shell 框架代码的核心。移到 cases/ 会破坏分层——cases 是被贡献的内容，shell_chrome 是消费贡献的框架逻辑。

**但需调整调用方式**：当前被 `MainWindow.refresh_bindings` 命令式调用。`refresh_bindings` 移除后，shell_chrome 的映射函数改为响应式调用（见 Phase 4.2）。

---

## 二、架构问题总结

| 问题 | 根因 | 本计划处理 |
|------|------|-----------|
| case_host.rml 硬编码 if 链 | IVisualContribution 未共享用于 TreeItems + tab body 渲染 | 删除 case_host，tab body 直接渲染选中 IVisualContribution |
| `host = MainWindow` 强依赖 | contribute 宏接受 `host = Type` 路径，要求导入宿主类型 | 参数改名 `host_id`，只接受字符串 |
| contributionhost 宏职责模糊 | 自动 impl IContributionHost + bindings 机制半成品 | 精简为只注册 + 生成 ID 常量 |
| 无 IVisualContribution 接口 | 视觉贡献走 Registerable + VisualRenderer，无显式 trait | 新增独立 trait（简化签名） |
| `refresh_bindings` 命令式刷新 | 宿主手动调 refresh_bindings 同步 UI，违背响应式 | 彻底移除，改为框架响应式自动刷新 |
| `ComponentEntityCacheImpl` 暴露 | 开发者需手动管理 Entity 缓存 | 框架内部处理，不暴露给开发者 |
| `ContributionRenderContext` 命名冗长 | 带 Contribution 前缀，不够简洁 | 重命名为 `RenderContext` |

---

## 三、提议的改动

### Phase 1：框架核心 — 新增 IVisualContribution trait + 重命名 RenderContext

**文件**：[crates/core/src/contribution.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs)

**改动 1**：`ContributionRenderContext`（第 95-99 行）重命名为 `RenderContext`：

```rust
/// 渲染上下文（视觉贡献渲染时使用）
pub struct RenderContext<'a> {
    pub window: &'a mut gpui::Window,
    pub cx: &'a mut gpui::App,
    pub active: bool,
}
```

**改动 2**：在 `IContribution`（第 71-77 行）之后、`IContributionHost`（第 83-85 行）之前，新增 `IVisualContribution` trait。**签名简化——不暴露 `ComponentEntityCacheImpl`**，Entity 缓存由框架内部处理：

```rust
/// 视觉贡献契约：具备渲染能力的贡献点。
///
/// 由 `#[contribute]` + `#[component]` 叠加时自动实现。
/// IVisualContribution 共享用于：
/// 1. 构造 TreeItems（案例树/活动面板树）—— 从元数据（id/name/icon/parent_id/order）构建
/// 2. 渲染到 tab body —— 选中后调用 render() 直接渲染
///
/// 开发者无需关心 Entity 缓存，框架内部处理。
pub trait IVisualContribution: IContribution {
    /// 渲染贡献视图。框架内部负责 Entity 缓存与复用。
    fn render(&self, ctx: &mut RenderContext) -> gpui::AnyElement;
}
```

**设计决策**：
- `IVisualContribution: IContribution`（继承），视觉贡献**是**贡献，额外具备渲染能力
- `render` 签名只有 `&self` + `&mut RenderContext`，**无 `ComponentEntityCacheImpl` 参数**——Entity 缓存由框架在调用 `render` 前后内部处理，对开发者透明
- `ComponentEntityCache` / `ComponentEntityCacheImpl` 降级为框架内部实现细节，移出公共 API（或标记 `#[doc(hidden)]`）
- `VisualRenderer` 类型别名保留为 registry 内部存储格式，但其签名同步更新为使用 `RenderContext`（无 cache 参数）
- `ContributedEntry.visual: Option<VisualRenderer>` 字段类型不变

**三接口分工**：
| 接口 | 职责 | 实现方 |
|------|------|--------|
| `IContribution` | 数据贡献元数据（id/name/description/icon） | `#[contribute]` 自动 impl |
| `IVisualContribution` | 视觉贡献渲染契约（继承 IContribution + render） | `#[contribute]` + `#[component]` 自动 impl |
| `IContributionHost` | 宿主标识（const ID） | 用户手动 impl（宏不再自动生成） |

**IVisualContribution 共享语义**（用户澄清点 1）：
- 同一个 IVisualContribution 实例同时服务于 TreeItems 构造（读元数据）和 tab body 渲染（调 render）
- 选中树节点 → 按 id 从 registry 取 IVisualContribution → 调 render() → 渲染到 tab body
- 无需 case_host.rml 中间层

---

### Phase 2：框架 App — Registerable 桥接 + 缓存内部化

**文件**：[crates/app/src/contribution/registerable.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/registerable.rs) + [crates/app/src/contribution/entry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/entry.rs)

**改动 1**：`component_registerable` 的 trait bound 收紧为 `IVisualContribution`：

```rust
pub fn component_registerable<T>(
    contribution: Arc<T>,
    options: ContributionOptions,
) -> ContributedEntry
where
    T: IVisualContribution + IComponent + Render + Default + Send + Sync + 'static,
{
    component_entry(contribution, options)
}
```

**改动 2**：`entry.rs` 的 `component_entry` 构造 `VisualRenderer` 闭包时，**框架内部处理 Entity 缓存**，调用 `contribution.render(ctx)` 而非让开发者传 cache：

```rust
pub fn component_entry<T>(contribution: Arc<T>, options: ContributionOptions) -> ContributedEntry
where
    T: IVisualContribution + IComponent + Render + Default + Send + Sync + 'static,
{
    let id = options... ; // 或 contribution.id()
    let renderer: VisualRenderer = Arc::new(move |ctx: &mut RenderContext| {
        // 框架内部：从 ContributionRegistryGlobal 取 Entity 缓存
        // 查找或创建 Entity，调用 contribution.render(ctx)
        // 开发者不感知缓存
        render_visual_contribution(&contribution, ctx)
    });
    ContributedEntry { contribution, visual: Some(renderer), options }
}
```

> 注：`render_visual_contribution` 是框架内部函数，负责从 global registry 取 `ComponentEntityCacheImpl`、查找/创建 Entity、委托 `IVisualContribution::render`。具体实现需读取 [entry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/entry.rs) + [global.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/global.rs) 全文对齐现有缓存逻辑。

**改动 3**：`VisualRenderer` 类型别名同步更新（移除 cache 参数）：

```rust
// crates/core/src/contribution.rs
pub type VisualRenderer = Arc<
    dyn Fn(&mut RenderContext) -> gpui::AnyElement + Send + Sync,
>;
```

---

### Phase 3：宏重构

#### 3.1 contributehost 宏精简

**文件**：[crates/macros/src/contributehost.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/contributehost.rs)

**移除**：
- `bindings` 参数解析（第 33-35 行）+ `__rml_contribution_bindings_attached` 字段注入（第 73-90 行）+ `__rml_attach_contribution_bindings` 方法生成（第 115-134 行）
- `impl IContributionHost for #struct_name` 自动生成（第 146-148 行）
- `ContributeHostArgs.bindings` 字段

**保留**：
- `pub const ID: &'static str = #id`（第 142-144 行）——供注册函数和用户手动 impl 引用
- 隐藏模块 `__rml_host_<lower>` 内的 `register(cx)` 函数（第 152-161 行）
- `pub fn __rml_register_<lower>(cx)`（第 163-166 行）

**新增**：**编译期检测——目标对象必须实现 `IContributionHost` 接口**。宏不再自动 impl 该 trait，但必须验证用户已手动 impl。使用 `const _: () = { ... }` 断言模式，零运行时开销：

**展开结果（精简后）**：
```rust
// 原样输出 struct
#(#items)*

impl #struct_name {
    pub const ID: &'static str = #id;
}

// 编译期检测：目标对象必须实现 IContributionHost 接口
// 宏不再自动 impl，用户须手动声明；此处断言确保不遗漏
const _: () = {
    fn assert_contribution_host<T: rml_core::contribution::IContributionHost>() {}
    fn check() { assert_contribution_host::<#struct_name>(); }
};

#[doc(hidden)]
mod __rml_host_<lower> {
    use super::#struct_name;
    pub(super) fn register(cx: &mut gpui::App) {
        use rml_app::contribution::{ContributionExt, ensure_contribution_registry};
        ensure_contribution_registry(cx);
        cx.add(#struct_name::ID);
    }
}

#[doc(hidden)]
pub fn __rml_register_<lower>(cx: &mut gpui::App) {
    __rml_host_<lower>::register(cx);
}
```

> 若用户忘记 `impl IContributionHost for MainWindow`，编译报错：`the trait bound MainWindow: IContributionHost is not satisfied`，定位精确。

**用户侧**：宿主类型需手动 impl `IContributionHost`（一行）：
```rust
impl IContributionHost for MainWindow {
    const ID: &'static str = Self::ID;
}
```

#### 3.2 contribute 宏：`host_id` 参数 + 解耦宿主类型

**文件**：[crates/macros/src/contribute.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs)

**改动 1**：参数名 `host` → `host_id`。在 `ContributeArgs`（第 21-42 行）中，将 `host: Expr` 字段改名为 `host_id`，解析时匹配 `host_id` 关键字。

**改动 2**：`host_id_tokens` 函数（原 `host_id_tokens`，第 225-247 行）移除 `Expr::Path` 分支（`host = Type` → `Type::ID`），**只接受字符串字面量**：

```rust
fn host_id_tokens(host_id: &Expr) -> TokenStream {
    match host_id {
        Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) => quote! { #s },
        _ => quote! {
            compile_error!("host_id must be a string literal (e.g. host_id = \"demo.shell\"). \
                            The host = Type form is removed to decouple contributions from host types.")
        },
    }
}
```

**改动 3**：为 `#[contribute]` + `#[component]` 叠加场景自动生成 `impl IVisualContribution`。在 `expand` 函数（第 423 行 `use_component_visual` 判定后），**简化签名——无 cache 参数**：

```rust
let visual_impl = if use_component_visual {
    quote! {
        impl rml_core::contribution::IVisualContribution for #struct_name {
            fn render(&self, ctx: &mut rml_core::contribution::RenderContext) -> gpui::AnyElement {
                // 框架内部处理 Entity 缓存，委托给 rml_app 的渲染辅助函数
                rml_app::contribution::render_component_view::<Self>(self, ctx)
            }
        }
    }
} else {
    quote! {}
};
```

> 注：`render_component_view::<T>` 是 Phase 2 新增的框架内部函数，负责 Entity 缓存查找/创建 + 委托渲染。开发者不直接调用此函数，由宏生成的 impl 自动调用。

**改动 4**：注册函数中的 `host_id` 引用同步更新参数名。

---

### Phase 4：Demo 重组

#### 4.1 解耦宿主类型 + 参数改名（全局替换）

所有 `#[contribute(host = MainWindow, ...)]` → `#[contribute(host_id = "demo.shell", ...)]`

涉及文件（10 个 case + shell 内 3 个贡献文件）：
- [demo/src/cases/counter_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml.rs)
- [demo/src/cases/two_way_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml.rs)
- [demo/src/cases/button_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml.rs)
- [demo/src/cases/i18n_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/i18n_case.rml.rs)
- [demo/src/cases/menu_context_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_context_case.rml.rs)
- [demo/src/cases/menu_dropdown_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_dropdown_case.rml.rs)
- [demo/src/cases/menu_editor_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_editor_case.rml.rs)
- [demo/src/cases/menu_features_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml.rs)
- [demo/src/cases/menu_custom_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_custom_case.rml.rs)
- [demo/src/shell/menu_shell_contribs.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs)
- [demo/src/shell/shell_meta.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/shell_meta.rs)
- [demo/src/shell/activity_panel.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/activity_panel.rml.rs)

**移除各 case 文件中的 `use crate::shell::MainWindow` 导入**。

#### 4.2 MainWindow：手动声明宿主身份 + 移除 refresh_bindings

**文件**：[demo/src/shell/main_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

**改动 1**：宏标注改为 `#[contributehost(id = "demo.shell")]`（移除 `bindings = "refresh_bindings"`），手动添加：

```rust
impl rml::prelude::IContributionHost for MainWindow {
    const ID: &'static str = Self::ID;
}
```

**改动 2**：**彻底移除 `refresh_bindings` 方法**。当前 MainWindow 的 `refresh_bindings`（手动调用 shell_chrome 映射函数同步 menu_items / status_items / case_tree 等 UI 字段）全部删除。

**改动 3**：UI 更新改为响应式——MainWindow 在 `on_loaded` 中订阅 registry 变化，registry 变更时 `cx.notify()` 触发重算：

```rust
impl ILifecycle for MainWindow {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        // ... 现有 menu_commands 注册等逻辑保留 ...

        // 订阅 registry 变化，自动触发 UI 重算（替代 refresh_bindings）
        subscribe_host_changes(Self::ID, cx, |this, cx| {
            cx.notify();
        });

        // ... 其他 on_loaded 逻辑 ...
    }
}
```

**改动 4**：原 `refresh_bindings` 中的 shell_chrome 映射调用，改为 `#[computed]` 方法或 RML 模板内联绑定，依赖 registry 状态自动重算。具体方式：
- `menu_items` / `status_items` / `case_tree_items` 等字段改为 `#[computed]` 方法，内部调用 `shell_chrome::map_*` 函数
- `#[computed]` 方法在 `cx.notify()` 后自动重算（RML computed 追踪机制）
- 若 RML computed 不追踪 global 状态变化，则回退为在 `subscribe_host_changes` 回调中手动更新字段 + `cx.notify()`

> 注：shell_chrome.rs 的映射函数（`map_menu_items` / `map_status_items` / `map_case_tree_items`）本身逻辑不变，只是调用方式从「refresh_bindings 命令式调用」改为「响应式自动调用」。

#### 4.3 删除 case_host.rml / case_host.rml.rs，tab body 直接渲染 IVisualContribution

**删除文件**：
- [demo/src/shell/case_host.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_host.rml)
- [demo/src/shell/case_host.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/case_host.rml.rs)

**删除 [demo/src/shell/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/mod.rs) 中对 case_host 模块的声明**。

**MainWindow tab body 直接渲染选中的 IVisualContribution**：

[main_window.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) 中原 `<CaseHost active_case_id={active_case_id} />` 替换为直接渲染 active case view：

```rml
<component class="main-window">
    <!-- ... slot_left / slot_menu / slot_title / slot_bottom / slot_footer ... -->
    <div class="tab-body">
        {active_case_view}
    </div>
</component>
```

MainWindow 新增方法，按 `active_case_id` 从 registry 取 IVisualContribution 并渲染：

```rust
impl MainWindow {
    /// 渲染当前激活案例的视图（通过 IVisualContribution 动态渲染）
    pub fn active_case_view(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<gpui::AnyElement> {
        let entries = contribution_entries(Self::ID, cx);
        let entry = entries
            .iter()
            .find(|e| e.contribution.id() == self.active_case_id)?;
        let renderer = entry.visual.as_ref()?;
        let mut ctx = RenderContext { window, cx, active: true };
        Some(renderer(&mut ctx))
    }
}
```

> **IVisualContribution 共享语义**（用户澄清点 1）：同一个 IVisualContribution 实例既用于 ActivityPanel 构造 TreeItems（读 id/name/icon/parent_id/order 元数据），又用于 MainWindow tab body 渲染（调 render）。选中树节点 → 按 id 从 registry 取 IVisualContribution → 调 render() → 渲染到 tab body。无 case_host 中间层。

> **实现时需验证**：RML 模板能否嵌入 `Option<AnyElement>` / `AnyElement` 字段并渲染。若 RML 不支持 AnyElement 字段嵌入，回退方案：MainWindow 手写 `impl Render`，在 render 中直接调用 `active_case_view`。此为实现期验证项。

> **Entity 缓存**：`renderer(&mut ctx)` 内部由框架从 `ContributionRegistryGlobal` 取共享 `ComponentEntityCacheImpl`，查找/创建 Entity 后委托渲染。开发者不感知缓存。切换案例 → 前一案例 Entity 缓存保留，再次激活不重建。

#### 4.4 shell/ 文件文档化

为 shell/ 每个文件补充模块级 doc comment，明确职责分层：

| 文件 | 文档要点 |
|------|---------|
| `main_window.rml.rs` | 应用主窗口 ViewModel，`IContributionHost` 宿主，聚合菜单/状态/活动栏，tab body 直接渲染选中 IVisualContribution |
| `activity_panel.rml(.rs)` | 左侧活动面板，`#[contribute]` 视觉贡献，展示案例树（TreeItems 从 IVisualContribution 元数据构建） |
| `menu_shell_contribs.rs` | **Shell 自身主菜单**贡献（File/View/Help），非案例代码 |
| `shell_meta.rs` | **Shell 容器元数据**：案例树分类节点 + 状态栏项 |
| `shell_chrome.rs` | **Registry → UI 控件映射层**：消费贡献数据构建 MenuItems/TreeItem/StatusBarItems（响应式调用） |
| `login_dialog.rml(.rs)` | 登录对话框（独立功能） |

**不移文件**：经分析，shell/ 的文件归属正确（shell 基础设施 + shell 自身贡献），问题在于缺乏文档说明导致混淆。补充文档即可。

---

## 四、假设与决策

1. **反转先前决策**：[contribution-centric-demo-refactor-plan.md:543](file:///d:/GitCode/RF/rust-gpui-rml/.trae/documents/contribution-centric-demo-refactor-plan.md) 曾决定「案例渲染保持硬编码，不走 IVisualContribution」。本计划**反转此决策**——新增 `IVisualContribution` 并删除 case_host，这是用户明确要求的。

2. **IVisualContribution 签名简化**：`render(&self, ctx: &mut RenderContext) -> AnyElement`，**不暴露 `ComponentEntityCacheImpl`**。Entity 缓存由框架内部处理（从 `ContributionRegistryGlobal` 取共享缓存）。开发者面向干净的 `IVisualContribution` trait，不感知缓存细节。

3. **`ContributionRenderContext` → `RenderContext`**：重命名，去掉冗余 `Contribution` 前缀。

4. **`host` → `host_id`**：contribute 宏参数改名，只接受字符串字面量。`host = Type` 形式彻底移除，不保留向后兼容。框架尚无第三方使用者，无兼容包袱。

5. **`refresh_bindings` 彻底移除**：不是「手动调用」，而是**完全没有这个概念**。UI 更新通过 `subscribe_host_changes` + `cx.notify()` 响应式触发，shell_chrome 映射函数改为 `#[computed]` 或回调内调用。

6. **contributionhost 宏不自动 impl IContributionHost，但编译期检测**：宏只生成 `pub const ID` + 注册函数，**不**自动 impl `IContributionHost`。用户手动 `impl IContributionHost`，强制区分「宏的注册职责」与「trait 的契约职责」。宏生成 `const _: () = { assert_contribution_host::<T>(); }` 编译期断言，确保用户不遗漏手动 impl——若忘记，编译报错 `trait bound ... is not satisfied`，定位精确。`IContributionHost` 仅 `const ID`，手动 impl 成本一行。

7. **bindings 机制移除**：`__rml_attach_contribution_bindings` 半成品且 demo 未正确使用。移除后，宿主刷新改为响应式 `subscribe_host_changes`，与 ActivityPanel 现有模式一致。

8. **case_host 完全删除**：IVisualContribution 共享用于 TreeItems 构造 + tab body 渲染。选中树节点 → 取 IVisualContribution → 调 render() → 渲染到 tab body。无 case_host 中间层。MainWindow 直接持有 `active_case_id` + `active_case_view` 方法。

9. **VisualRenderer 保留为内部存储**：不改 `ContributedEntry.visual: Option<VisualRenderer>` 字段类型，避免 registry 大改。`VisualRenderer` 签名同步简化（移除 cache 参数）。`IVisualContribution::render` 在 `component_entry` 内被包装成 `VisualRenderer` 闭包，框架内部处理缓存。

10. **RML AnyElement 嵌入风险**：若 RML 模板不支持 `AnyElement` 字段嵌入，MainWindow 回退为手写 `impl Render`。此为实现期验证项，不影响架构决策。

---

## 五、验证步骤

### 5.1 框架编译验证
1. `cargo build -p rml-core` —— `IVisualContribution` trait + `RenderContext` 重命名编译通过
2. `cargo build -p rml-app` —— `Registerable` / `component_registerable` 新 bound + `render_component_view` 内部函数编译通过
3. `cargo build -p rust-rml-macros` —— 两个宏精简后编译通过

### 5.2 宏展开验证
1. `cargo expand -p rml-demo --lib shell::main_window` —— 确认 `contributehost` 不再生成 `impl IContributionHost` + 不再生成 `bindings` 相关代码 + **生成 `const _: () = { assert_contribution_host::<MainWindow>(); }` 编译期断言**
2. `cargo expand -p rml-demo --lib cases::counter_case` —— 确认 `contribute` 生成 `impl IVisualContribution`（因叠加 `#[component]`），且 `render` 签名无 cache 参数
3. `cargo expand -p rml-demo --lib shell::menu_shell_contribs` —— 确认 `contribute` 对纯数据贡献（无 `#[component]`）**不**生成 `IVisualContribution` impl
4. 确认所有 `#[contribute]` 的参数名为 `host_id`（非 `host`）
5. **编译期断言验证**：临时注释掉 MainWindow 的 `impl IContributionHost` → `cargo build -p rml-demo` 报错 `trait bound MainWindow: IContributionHost is not satisfied` → 恢复后编译通过

### 5.3 Demo 编译验证
1. `cargo build --workspace` 通过
2. 无 `unused import` 告警（所有 `use crate::shell::MainWindow` 已从 case 文件移除）
3. `case_host.rml` / `case_host.rml.rs` 已删除，无残留引用
4. `cargo clippy --workspace` 无新增 warning

### 5.4 运行时验证
1. 窗口启动 → 案例树显示 4 分类 + 10 案例节点（TreeItems 从 IVisualContribution 元数据构建）
2. 点击案例叶子节点 → **tab body 动态渲染对应案例内容**（验证 IVisualContribution 共享渲染路径）
3. 切换案例 → 前一案例 Entity 缓存保留，再次激活不重建（验证框架内部缓存复用）
4. 菜单栏 File/View/Help 正常显示与点击
5. 语言切换 → 菜单/状态/树标签自动刷新（验证 `refresh_bindings` 移除后响应式更新生效）
6. 主题切换 → 明暗切换

### 5.5 架构回归验证
1. 新增一个 case：只需新建 `.rml` + `.rml.rs` 文件加 `#[contribute(host_id = "demo.shell", ...)]` + `#[component]`，**无需修改任何 shell 文件**（验证 case_host 已消除、注册即生效）
2. case 文件无 `use crate::shell::MainWindow` 导入（验证宿主解耦）
3. MainWindow 无 `refresh_bindings` 方法（验证命令式刷新已移除）
4. 开发者无需接触 `ComponentEntityCacheImpl`（验证缓存内部化）
