# MVVM 视觉抽象与 WPF 风格组件扩展性重构 — 实施计划

> **目标**：提取 `IVisual` 通用视觉能力 trait；`<component>` 自动识别 `IVisual`；`<status-bar>` 与 `<menu-bar>` 采用 WPF ItemsControl 风格的 items source + item template 机制；删除 demo 中命令式 `render_menu_bar` / `render_status_bar` / `active_view` 方法,改为声明式模板绑定。
>
> **设计哲学**：参考 WPF `ItemsControl` + `HierarchicalDataTemplate` 精髓 —— **框架提供机制(items source 绑定 + item template 槽 + children 抽取),业务提供数据结构与模板**。框架不在数据契约层限定死组件扩展能力(`IMenuItem` 不入框架;`IStatusBarItem` 仅因 align 容器角色而入框架,与 WPF `StatusBarItem` 容器同义)。

---

## 一、当前状态分析

### 1.1 三处严重 MVVM 违规(demo/src/shell/main_window.rml)

| 行号 | 违规代码 | 问题 |
|------|---------|------|
| 23 | `<component content={self.render_menu_bar(_window, cx)} />` | 模板调用命令式方法,ViewModel 构造 `MenuBar` + `PopupMenu` 闭包 |
| 35 | `<component content={self.render_status_bar(_window, cx)} />` | 模板调用命令式方法,ViewModel 持有 `NativeStatusBar` 组装逻辑 |
| 38 | `<component content={self.active_view(_window, cx)} />` | 模板调用命令式方法,直接 `visual.render(window, cx)` 构造 `AnyElement` |

对比正确 MVVM:`<template slot="tabs" each={w in workbenches}><Tab label={w.name()} closable /></template>`(line 14-16)。

### 1.2 现有能力盘点(已验证,可复用)

| 能力 | 文件 | 用途 |
|------|------|------|
| `<component content={expr} />` 透明容器 | [crates/engine/src/compiler/codegen/node.rs#L97-L144](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs#L97-L144) | 表达式原样输出,无类型识别 |
| `each` 指令在 `<menu-item>` 上 | [crates/engine/src/compiler/menu/menu_bar.rs#L52-L84](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/menu_bar.rs#L52-L84) | 顶层菜单项 MVVM 迭代 |
| `field_types: HashMap<String, String>` | [crates/engine/src/compiler/mod.rs#L138](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L138) | 字段名→类型字符串,可做类型推断 |
| `cell_template: Arc<dyn Fn>` 闭包 | [crates/engine/src/compiler/table/template.rs#L56-L96](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/table/template.rs#L56-L96) | 闭包参数注入 loop_vars,模板可引用 |
| mopa 能力注册 | [crates/core/src/ability.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/ability.rs) | `register::<T, A>` / `query::<A>` 通用机制 |
| `MenuBar` 为 `ParentElement` 容器 | [crates/ui/src/components/menu.rs#L69-L116](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs#L69-L116) | 接受 `.children(...)` |
| `NativeStatusBar` re-export | [crates/ui/src/components/status_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs) | 仅 15 行,无 RML 封装 |

### 1.3 当前 trait 关系

```
IValue (Send + Sync + Any)
  └─ IContribution: IValue  (id/name/description/icon)
       └─ IVisualContribution: IContribution  (render)
            [#[contribute] + #[component] 自动 impl,注册 dyn IVisualContribution 能力]

IWorkbench: IContribution  (uri/close/activate/set)  ← 不要求 IVisual
IWorkbenchProvider: IContribution  (schema/render→IWorkbench)  ← 工厂方法,非视觉
```

`IWorkbench` 不强制 `IVisualContribution`,但 demo 的 `CaseWorkbench` / `LspWorkbench` 经 `#[contribute] + #[component]` 实现 IVisualContribution,使 `active_view` 能经 `as_visual()?.render()` 渲染。

### 1.4 限制

- **RML 不支持递归模板**:无法在模板内递归调用自身(无模板自引用机制)
- **`<menu-bar>` 现有 codegen 限制**:`each` 仅识别第一个 top-level `<menu-item>`,嵌套 `each`(子菜单迭代)未支持
- **`<status-bar>` 标签未注册**:目前只有 `<native-status-bar>`(gpui-component 原生)

---

## 二、设计方案(基于 WPF 精髓)

### 2.1 WPF 设计精髓对照

| WPF 概念 | RML 对应实现 |
|---------|------------|
| `ItemsControl.ItemsSource` | `items={self.field}` 属性绑定 |
| `ItemsControl.ItemTemplate` | `<template slot="item" each={x in items}>` |
| `HierarchicalDataTemplate.ItemsSource` | `submenu={m.children}` 子项数据绑定 |
| `ContentControl.Content` + `ContentTemplate` | `<component content={expr} />` + IVisual 自动识别 |
| `StatusBarItem` 容器角色(align) | `IStatusBarItem: IVisualContribution + align()`(框架定义容器契约) |
| `MenuItem` 容器(无 IMenuItem 接口) | **不**在框架定义 IMenuItem;业务自定 MenuViewModel |

### 2.2 核心 trait 重构

```rust
// crates/core/src/contribution.rs

/// 视觉能力 trait —— 任何可渲染为 UI 元素的值对象实现此 trait。
/// 与 IContribution 解耦:非贡献的视觉对象(如纯视图模型)也可实现 IVisual。
pub trait IVisual: IValue {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement;
}

/// 可视化贡献点 —— 标记 trait,纯组合语义(IContribution + IVisual)。
/// 业务视觉贡献(如 ActivityPanel)同时实现 IContribution + IVisual 即自动满足。
pub trait IVisualContribution: IContribution + IVisual {}

/// Blanket impl —— 任何 IContribution + IVisual 自动获得 IVisualContribution 标记
impl<T: IContribution + IVisual> IVisualContribution for T {}
```

**能力查询分离**:
```rust
/// IVisual 能力查询(新)
pub trait VisualExt {
    fn as_visual(&self) -> Option<&dyn IVisual>;
}
impl VisualExt for dyn IValue {
    fn as_visual(&self) -> Option<&dyn IVisual> {
        let erased = crate::ability::query::<dyn IVisual>(self)?;
        Some(unsafe { crate::ability::restore::<dyn IVisual>(erased) })
    }
}

/// IVisualContribution 能力查询(保留,改名)
pub trait VisualContributionExt {
    fn as_visual_contribution(&self) -> Option<&dyn IVisualContribution>;
}
```

> **破坏性变更说明**:旧 `VisualAbilityExt::as_visual()` 改名为 `as_visual_contribution()`,新 `VisualExt::as_visual()` 返回 `&dyn IVisual`。框架无历史包袱,允许破坏(`project_memory.md` 已记录此偏好)。

### 2.3 `<component>` IVisual 识别

**机制**:
1. **类型推断优先**:codegen 检查 `field_types`,若 `self.field` 类型字符串匹配 `dyn IVisual` / `dyn IStatusBarItem` / `dyn IWorkbench` 等 IVisual 子 trait → 自动生成 `expr.render(_window, cx).into_any_element()`
2. **显式 `visual` 属性回退**:`<component content={expr} visual />` 强制视觉模式(用于方法调用等无法推断的场景)

**Codegen 改动**([crates/engine/src/compiler/codegen/node.rs#L97-L144](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs#L97-L144)):
```rust
if tag == "component" {
    let content_expr = ...;
    let is_visual_attr = elem.attributes.iter().any(|a| matches!(a, 
        Attribute::Static { name, .. } if name == "visual"));
    
    // 类型推断:self.field → field_types 查询
    let inferred_visual = if let Some(field) = extract_self_field(&content_expr) {
        let ty = ctx.field_types.get(field).map(|s| s.as_str()).unwrap_or("");
        ty.contains("IVisual") || ty.contains("IStatusBarItem") || ty.contains("IWorkbench")
    } else {
        false
    };
    
    let code = if is_visual_attr || inferred_visual {
        format!("({}).render(_window, cx).into_any_element()", expr_code)
    } else {
        expr_code  // 当前行为
    };
    ...
}
```

### 2.4 `<status-bar>` WPF 风格设计

**框架契约**([crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs)):
```rust
/// 状态栏项贡献契约 —— 容器角色(类比 WPF StatusBarItem)。
/// 仅提供 align 容器属性;order 来自 ContributionOptions;
/// 命令/点击事件由 render() 自行处理(IVisual 已具备此能力)。
pub trait IStatusBarItem: IVisualContribution {
    fn align(&self) -> StatusBarAlign;
}
```

**`StatusBar` 组件**([crates/ui/src/components/status_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs)):
```rust
#[derive(IntoElement)]
pub struct StatusBar {
    id: ElementId,
    items: Vec<Arc<dyn IStatusBarItem>>,
}

impl StatusBar {
    pub fn new(id: impl Into<ElementId>) -> Self { ... }
    
    /// items source 绑定 —— 自动按 align() 路由到 left/right/center
    pub fn items(mut self, items: impl IntoIterator<Item = Arc<dyn IStatusBarItem>>) -> Self {
        self.items = items.into_iter().collect();
        self
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _window, _cx) -> impl IntoElement {
        let mut bar = NativeStatusBar::new();
        // 按 align 路由(自动机制,业务无需手写)
        for item in &self.items {
            let elem = item.render(_window, _cx);  // IVisual::render
            match item.align() {
                StatusBarAlign::Left => bar = bar.left(elem),
                StatusBarAlign::Right => bar = bar.right(elem),
                StatusBarAlign::Center => bar = bar.child(elem),
            }
        }
        bar
    }
}
```

**RML 用法**:
```xml
<status-bar items={self.status} />
```
Codegen:`rml_ui::StatusBar::new(("rml_status_bar", 0usize)).items(self.status.iter().cloned())`

> **关键**:无需 `<template slot="item">` —— `IStatusBarItem: IVisual` 已提供 render,框架自动调用。业务若想自定义渲染,可实现 `IVisual::render` 返回自定义元素。

### 2.5 `<menu-bar>` WPF 风格 submenu 递归

**问题**:RML 不支持递归模板,无法在模板内表达"对子菜单项应用同一模板"。

**方案**:`submenu` 属性绑定 —— 业务提供 `build_submenu` 方法,框架经 `.dropdown_menu` 闭包集成。这是 WPF `HierarchicalDataTemplate` 的 Rust 务实翻译:**框架提供集成点,业务控制递归**。

**RML 用法**:
```xml
<menu-bar>
    <menu-item each={m in menus} 
               label={m.label()} 
               command={m.command}
               submenu={m.build_submenu} />
</menu-bar>
```

**Codegen 改动**([crates/engine/src/compiler/menu/menu_bar.rs#L103-L175](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/menu_bar.rs#L103-L175)):
```rust
// 在 gen_menu_bar_button_for_item 中检测 submenu 属性
let submenu_expr = item.attributes.iter().find_map(|a| match a {
    Attribute::Bind { name, expr } if name == "submenu" => Some(expr.clone()),
    _ => None,
});

if let Some(submenu_expr) = submenu_expr {
    // 生成 dropdown_menu 闭包,业务方法控制递归
    let submenu_access = gen_expr_code(&submenu_expr, loop_vars, &computed);
    let button = format!(
        "rml_ui::menu_bar_button((\"rml_menu_bar\", 0usize), {label})\n
         .dropdown_menu(move |menu, window, cx| {{\n
             let menu = rml_ui::configure_menu_bar_popup(menu);\n
             {submenu_access}(menu, window, cx)\n
         }})"
    );
}
```

**业务侧 MenuViewModel**([demo/src/shell/menu_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs)):
```rust
impl MenuViewModel {
    /// 递归构建子菜单 —— 业务控制递归(WPF HierarchicalDataTemplate 等价)
    pub fn build_submenu(
        &self,
        mut menu: PopupMenu,
        window: &mut Window,
        cx: &mut Context<PopupMenu>,
    ) -> PopupMenu {
        MenuViewModel::build_popup_menu(menu, &self.children, window, cx)
    }
}
```

> **关键**:`build_submenu` 由业务定义,可包含任意递归逻辑。框架仅提供 `.dropdown_menu` 集成点。叶子节点用 `command={m.command}` 绑定(WPF `MenuItem.Command` 等价)。

### 2.6 `IWorkbench` 视觉化

**决策**:`IWorkbench: IContribution + IVisual`(workbench 本质是视觉的)

**理由**(WPF 类比):WPF `ContentControl` 强制有 `Content`。Workbench 是"已打开资源的会话句柄",必然有视图。无视图的"后台任务"不应实现 IWorkbench,而应实现 IContribution。

**变更**:[crates/core/src/workbench.rs#L31](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/workbench.rs#L31) 改为:
```rust
pub trait IWorkbench: IContribution + IVisual {
    fn uri(&self) -> &str;
    fn close(&self);
    fn activate(&self);
    fn set(&self, key: SharedString, value: Box<dyn Any + Send + Sync>);
}
```

**`IWorkbenchProvider: IContribution`(不变)**:`render(&self, uri) -> Arc<dyn IWorkbench>` 是工厂方法,不产出 UI 元素。

---

## 三、实施步骤(按依赖顺序)

### Phase 1:核心 trait 重构(crates/core + crates/macros)

**文件**:
- [crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs)
- [crates/core/src/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/workbench.rs)
- [crates/macros/src/contribute.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs)

**改动**:
1. 提取 `IVisual: IValue` trait(含 `render` 方法)
2. `IVisualContribution` 改为 `IContribution + IVisual` 标记 trait + blanket impl
3. 新增 `VisualExt::as_visual() -> Option<&dyn IVisual>` 能力查询
4. 旧 `VisualAbilityExt::as_visual()` 改名 `as_visual_contribution()`(返回 `&dyn IVisualContribution`)
5. `IWorkbench: IContribution + IVisual`
6. `#[contribute]` 宏:自动 impl `IVisual`(不再 impl `IVisualContribution`,blanket 自动覆盖);能力注册改为 `register::<T, dyn IVisual>`(替代 `dyn IVisualContribution`)

**验证**:
- `cargo build -p rml-core` 编译通过
- `cargo test -p rml-core` 全部测试通过
- `cargo test -p rml-macros` 全部测试通过

### Phase 2:`<component>` IVisual 识别(crates/engine)

**文件**:
- [crates/engine/src/compiler/codegen/node.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs#L97-L144)

**改动**:
1. 在 `<component content={expr} />` 处理逻辑中:
   - 检测 `visual` 静态属性 → 强制视觉模式
   - 类型推断:`self.field` 模式经 `field_types` 查询,类型字符串包含 `IVisual` / `IStatusBarItem` / `IWorkbench` → 视觉模式
   - 视觉模式:`({expr}).render(_window, cx).into_any_element()`
   - 默认:当前行为(原样输出,作 IntoElement)
2. 新增单元测试:覆盖类型推断 + visual 属性 + 默认行为

**验证**:
- `cargo test -p rml-engine` 全部通过(含新增测试)
- 现有 demo 模板编译不破坏(`<component content={self.active_view(...)} />` 仍按 IntoElement 处理,因方法调用不匹配类型推断;待 Phase 5 替换)

### Phase 3:`<status-bar>` 组件 + `IStatusBarItem`(crates/core + crates/ui + crates/engine)

**文件**:
- [crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs) — 定义 `IStatusBarItem: IVisualContribution + align()`
- [crates/ui/src/components/status_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs) — 新增 `StatusBar` 包装组件(items + auto-route)
- [crates/ui/src/components/mod.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/mod.rs) — 导出 `StatusBar`
- [crates/ui/src/lib.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs#L82-L89) — 导出 `StatusBar` + `IStatusBarItem`
- [crates/engine/src/tags.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L423-L433) — 注册 `<status-bar>` 标签
- [crates/engine/src/compiler/menu/setters.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/setters.rs) — 注册 `items` 属性为合法 bind setter

**改动**:
1. `StatusBar` 新组件:持有 `Vec<Arc<dyn IStatusBarItem>>`,RenderOnce 内部按 align 路由到 NativeStatusBar 的 left/right/child
2. `IStatusBarItem` trait + `StatusBarAlign` 移至 `rml_core::contribution`(或保持 rml_ui re-export,但 trait 定义在 core)
3. `<status-bar>` 标签注册 → `rml_ui::StatusBar`,支持 `items={...}` 属性
4. `items` 属性经现有 setter 机制生成 `.items(self.field.iter().cloned())`

**验证**:
- `cargo build -p rml-ui` 通过
- `cargo test -p rml-engine` 标签查找测试覆盖 `<status-bar>`

### Phase 4:`<menu-bar>` submenu 属性绑定(crates/engine)

**文件**:
- [crates/engine/src/compiler/menu/menu_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/menu_bar.rs#L103-L175)
- [crates/engine/src/compiler/menu/item.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/item.rs) — 检查是否需同步支持 `submenu` 属性(若 `gen_menu_item_stmt` 也处理子菜单项的 submenu)

**改动**:
1. `gen_menu_bar_button_for_item`:检测 `submenu={expr}` 属性,生成 `.dropdown_menu(move |menu, window, cx| { let menu = configure_menu_bar_popup(menu); {submenu_expr}(menu, window, cx) })`
2. 与现有 `command={...}` 叶子节点处理并存:`submenu` 优先于 `command`(有子菜单的项不直接执行命令)
3. 单元测试:`<menu-item each={m in menus} submenu={m.build_submenu} />` codegen 正确

**验证**:
- `cargo test -p rml-engine` 全部通过
- `<menu-item each={...} submenu={...}>` codegen 生成预期代码

### Phase 5:Demo 重构(demo)

**文件**:
- [demo/src/shell/main_window.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml) — 替换三处 `<component content={...} />`
- [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) — 删除 `render_menu_bar` / `render_status_bar` / `active_view`
- [demo/src/shell/menu_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs) — 添加 `build_submenu` 公开方法(从 `build_popup_menu` 提取)
- [demo/src/shell/status_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/status_view_model.rs) — `StatusViewModel` 改为实现 `IStatusBarItem`,或直接用 `Arc<dyn IStatusBarItem>` 替换
- [demo/src/cases/status_bar_case.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rs) — 状态栏贡献实现 `IStatusBarItem`(添加 `align()` 方法)

**RML 模板改动**:
```xml
<!-- Before -->
<template slot="menu">
    <component content={self.render_menu_bar(_window, cx)} />
</template>
<template slot="footer">
    <component content={self.render_status_bar(_window, cx)} />
</template>
<component content={self.active_view(_window, cx)} />

<!-- After -->
<template slot="menu">
    <menu-bar>
        <menu-item each={m in menus} 
                   label={m.label()}
                   command={m.command}
                   submenu={m.build_submenu} />
    </menu-bar>
</template>

<template slot="footer">
    <status-bar items={self.status} />
</template>

<component content={self.activated_view} visual />
```

**MainWindow 改动**:
1. 删除 `render_menu_bar` / `render_status_bar` / `active_view` 三个方法
2. 新增 `activated_view` 字段(`Option<Arc<dyn IWorkbench>>`),由 `on_activated_changed` 同步更新
   - **或**:保留 `activated` 字段,新增 `#[computed] activated_view() -> Option<Arc<dyn IWorkbench>>`,模板用 `<component content={self.activated_view()} visual />`(`visual` 属性强制视觉模式,绕过方法调用无法类型推断的限制)
3. `StatusViewModel` 重构:
   - **方案 A**(简化):`status: Vec<Arc<dyn IStatusBarItem>>` 直接持有;`build_status_view_models` 返回 `Vec<Arc<dyn IStatusBarItem>>`;`StatusViewModel` 类型删除
   - **方案 B**(渐进):`StatusViewModel` 实现 `IStatusBarItem`(委托 contribution),保持现有结构
   - **推荐方案 A** —— 更彻底,符合 WPF "无中间 ViewModel" 风格

**status_bar_case.rs 改动**:
- 现有 `StatusReadyCase` 等 contribution 实现 `IVisualContribution`(经 `#[contribute] + #[component]`)
- 改为实现 `IStatusBarItem`(添加 `align()` 方法)
- `#[contribute]` 宏需支持 `IStatusBarItem` 子 trait 注册(检查宏是否需扩展,或经 `register_status_bar_item_ability::<T>()` 手动注册)

**验证**:
- `cargo build --workspace` 全部编译通过
- `cargo test --workspace` 全部测试通过
- demo 运行无 panic,菜单/状态栏/活动视图正常显示
- locale 切换菜单标签刷新正常
- 状态栏 left/right/center 对齐正确

---

## 四、假设与决策

### 4.1 关键设计决策(已基于 WPF 最佳实践决定)

| # | 决策点 | 选择 | 理由 |
|---|--------|------|------|
| 1 | `IWorkbench` 是否 extends IVisual | **是** | WPF `ContentControl` 强制有 Content;workbench 本质是视觉的;无视图"后台任务"应实现 IContribution 而非 IWorkbench |
| 2 | `<component>` IVisual 识别机制 | 类型推断 + `visual` 属性回退 | WPF Binding 类型推断风格;`field_types` 已存在;方法调用无法推断时显式 `visual` |
| 3 | 子菜单递归方案 | `submenu={m.build_submenu}` 属性绑定 | RML 不支持递归模板;务实翻译 HierarchicalDataTemplate:框架提供集成点,业务控制递归 |
| 4 | `IMenuItem` 是否入框架 | **不入** | WPF 不定义 IMenuItem 接口;业务自定义 MenuViewModel;框架提供机制不限制数据结构 |
| 5 | `IStatusBarItem` 是否入框架 | **入**(仅 align) | WPF `StatusBarItem` 是容器角色;align 是容器属性;order 来自 ContributionOptions;命令由 render 处理 |
| 6 | `StatusViewModel` 是否保留 | **删除**(方案 A) | 直接用 `Arc<dyn IStatusBarItem>`;符合 WPF "无中间 ViewModel" 风格 |
| 7 | `IVisualContribution` 能力注册 | 改为注册 `dyn IVisual` | IVisualContribution 是 blanket impl 标记 trait;查 IVisual 更通用;查 IVisualContribution 经 as_contribution + as_visual 组合 |
| 8 | 旧 `as_visual()` 改名 | `as_visual_contribution()` | 避免与新 `VisualExt::as_visual()` 冲突;框架无历史包袱允许破坏 |

### 4.2 假设

- `field_types` 包含的字段类型字符串中,trait object 类型形如 `Arc<dyn IVisual>` / `Arc<dyn IStatusBarItem>` / `Option<Arc<dyn IWorkbench>>` 等(需在 Phase 2 实施时验证 scanner 输出格式)
- `#[contribute]` 宏的 `use_visual` 检测逻辑(`has_component` 检查)可扩展为生成 `impl IVisual` 而非 `impl IVisualContribution`
- `StatusBar` 新组件经 `Stateless` kind 注册,与 `MenuBar` 同类(`container: false`)
- `submenu` 属性经 `Attribute::Bind` 处理,与 `command` 同类

### 4.3 风险与缓解

| 风险 | 缓解 |
|------|------|
| `field_types` 类型字符串格式不确定 | Phase 2 实施时先 `println!` 打印实际格式,再调整匹配规则 |
| `#[contribute]` 宏改动影响范围广 | Phase 1 完成后跑全量 `cargo test --workspace`,任何破坏立即定位 |
| `IWorkbench: IVisual` 破坏现有非视觉 workbench | demo 中所有 workbench 均 `#[contribute] + #[component]`,已实现 IVisual;若存在非视觉 workbench,需提供"空 render" 默认实现 |
| `submenu` 闭包捕获 `m` 的生命周期 | 参考 `command={m.command}` 现有 codegen 模式,clone Arc 后 move 捕获 |

---

## 五、验证清单

### 5.1 单元测试

- [ ] `cargo test -p rml-core` —— IVisual / IVisualContribution / VisualExt / IStatusBarItem trait 测试
- [ ] `cargo test -p rml-macros` —— `#[contribute]` + `#[component]` 生成 `impl IVisual` 而非 `impl IVisualContribution`
- [ ] `cargo test -p rml-engine` —— `<component visual>` / `<component content={self.visual_field}>` codegen;`<status-bar items={...}>` 标签查找;`<menu-item submenu={...}>` codegen
- [ ] `cargo test -p rml-ui` —— StatusBar 组件
- [ ] `cargo test --workspace` —— 全量回归

### 5.2 集成验证(demo)

- [ ] demo 启动无 panic
- [ ] 菜单栏显示完整层级(File/Edit/View/Help),子菜单可展开
- [ ] 菜单叶子项点击触发命令(如切换主题、切换语言)
- [ ] 状态栏 left/right 对齐正确(状态项可见)
- [ ] 活动视图(tab 内容)正常显示
- [ ] locale 切换 → 菜单标签刷新
- [ ] 主题切换 → 全局 UI 刷新

### 5.3 架构验证

- [ ] `main_window.rml` 中无 `<component content={self.render_xxx(...)} />` 模式
- [ ] `main_window.rml.rs` 中无 `render_menu_bar` / `render_status_bar` / `active_view` 方法
- [ ] `MenuViewModel` 有公开 `build_submenu` 方法
- [ ] `status_bar_case.rs` 实现 `IStatusBarItem`(含 `align()`)
- [ ] `IWorkbench` trait 定义中 `: IContribution + IVisual`

---

## 六、实施顺序与里程碑

```
Phase 1 (core trait)        ─┐
                              ├─ cargo build -p rml-core ✓
                              └─ cargo test -p rml-core ✓

Phase 2 (component IVisual) ─┐
                              ├─ cargo test -p rml-engine ✓
                              └─ 现有 demo 仍可编译(向后兼容)

Phase 3 (status-bar)        ─┐
                              ├─ cargo build -p rml-ui ✓
                              └─ cargo test -p rml-engine ✓ (标签查找)

Phase 4 (menu-bar submenu)  ─┐
                              ├─ cargo test -p rml-engine ✓ (codegen)
                              └─ 现有 demo 仍可编译

Phase 5 (demo 重构)          ─┐
                              ├─ cargo build --workspace ✓
                              ├─ cargo test --workspace ✓
                              └─ demo 运行验证 ✓
```

**预计文件改动数**:11 个(2 core + 1 macros + 2 engine + 4 ui + 1 engine tags + 5 demo)
**预计代码量**:约 800 行新增/修改(含测试)
