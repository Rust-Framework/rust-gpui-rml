# 主窗口菜单栏第一性原理分析与实现方案

## 一、第一性原理分析

### 1.1 菜单栏的本质

菜单栏是一个**水平排列的顶层菜单入口集合**，每个入口点击/悬停后展示**垂直弹出的菜单项列表**。菜单项可以是：动作项、分隔符、分组标题、子菜单、链接、带勾选的项。菜单项可携带：图标、快捷键、禁用状态、勾选状态。

### 1.2 核心关注点分离

从第一性原理出发，菜单栏涉及三个**正交关注点**，必须分离：

| 关注点 | 归属 | 数据形态 |
|--------|------|----------|
| **菜单结构**（哪些项出现、层级关系） | View 层 | `MenuItems`（MVVM 数据绑定） |
| **菜单行为**（点击执行什么） | Command 层 | `Arc<dyn ICommand>` |
| **菜单渲染**（如何画出来） | 框架层 | gpui-component `PopupMenu` + `Button::dropdown_menu` |

**关键洞察**：菜单结构是 ViewModel 数据，不是 UI 元素树。ViewModel 提供 `MenuItems`（`Vec<Arc<dyn IMenuItem>>`），框架在 render 时将其翻译为 gpui-component 的 `Button` + `PopupMenu` 实体。这是 MVVM 的正确做法——ViewModel 不依赖 gpui-component 类型。

### 1.3 设计选项评估

**选项 A：MVVM 数据绑定**（`<menu items={self.menu_items} />`）
- ViewModel 持有 `MenuItems` 字段，`<menu>` 组件在 render 时翻译为 `Button::dropdown_menu` + `PopupMenu`
- 优点：MVVM 纯净、支持贡献点扩展、支持运行时动态重建（语言切换/上下文菜单）
- 这是 demo 当前使用的方式

**选项 B：RML 声明式**（`<menu-bar><menu-item label="File">...</menu-item></menu-bar>`）
- RML 标签直接编译为 `Button::dropdown_menu` 闭包，不经过 `IMenuItem` 抽象
- 优点：编译期确定、无运行时开销
- 缺点：不支持贡献点扩展、不支持运行时动态变化

**选项 C：gpui-component 原生 AppMenuBar**（`GlobalState::app_menus()` + `OwnedMenu`）
- 菜单注册到全局 `GlobalState`，由 `AppMenuBar` 渲染
- 优点：可同时注册为 macOS 原生菜单栏、快捷键自动绑定 GPUI Action
- 缺点：`OwnedMenu` 是 GPUI 原生类型，ViewModel 依赖 gpui 类型，违反 MVVM；与贡献点机制不兼容

### 1.4 结论：正确方案

**主窗口菜单栏应采用选项 A（MVVM 数据绑定）作为主路径**，理由：

1. **MVVM 纯净性**：`IMenuItem` trait 是框架自有抽象，ViewModel 不依赖 gpui-component 类型，符合 WPF-style 设计
2. **贡献点兼容**：`contributions.rs:build_menu_items` 产出 `MenuItems`，与数据绑定路径天然对接
3. **动态菜单**：语言切换时重建 `menu_items` 字段即可，无需改 RML
4. **gpui-component 封装**：`render_menu_bar_from_items` 已使用 `Button::dropdown_menu` + `PopupMenu`，是真正的 gpui-component 封装

**Shell 层使用 slot 扩展**：`<slot_menu>` 作为元素插槽传递菜单组件，菜单组件内部使用 `items={...}` 绑定。即：
```xml
<slot_menu>
  <menu items={menu_items} />
</slot_menu>
```
这满足"slot 扩展"和"MVVM 数据绑定"双重要求——Shell 不直接接收 `Vec<MenuItem>` 数据，而是接收一个 element；element 内部用 MVVM 绑定。

**选项 B（声明式）保留为次要路径**，用于静态菜单场景（如右键菜单、下拉菜单），但主窗口菜单栏不用它。

**选项 C（AppMenuBar）不采用**，因为 `OwnedMenu` 依赖 GPUI 原生类型，违反 MVVM。如需 macOS 原生菜单栏，可未来在框架层从 `IMenuItem` 翻译到 `OwnedMenu`，但不在本次范围。

---

## 二、现状评估

### 2.1 架构方向正确

当前实现的核心架构是正确的：
- `IMenuItem` trait（[menu.rs:21-48](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs#L21-L48)）：MVVM 数据契约，object-safe，`Send + Sync + 'static`
- `Menu` 组件（[menu.rs:168-193](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs#L168-L193)）：`RenderOnce`，render 时调用 `render_menu_bar_from_items`
- `render_menu_bar_from_items`（[menu.rs:196-243](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs#L196-L243)）：顶层有 children 用 `Button::dropdown_menu`，无 children 用 `Button::on_click`
- `build_popup_menu_from_items`（[menu.rs:246-312](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs#L246-L312)）：递归构建 `PopupMenu`，支持 submenu/separator/header/link/check/icon/disabled

### 2.2 实现缺陷清单

#### BUG-1（严重）：`<modern_window>` 丢弃 `slot_menu`

- [codegen/mod.rs:212-218](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L212-L218)：调用 `gen_modern_window_wrapper` 时只传 `slot_title_code` 和 `slot_footer_code`，**未传 `slot_menu_code`**
- [codegen/shell.rs:15-21](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L15-L21)：`gen_modern_window_wrapper` 签名无 `slot_menu` 参数
- `slot_menu_code` 已在 [mod.rs:186-189](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L186-L189) 生成，但从未使用
- **后果**：`<modern_window><slot_menu>...</slot_menu></modern_window>` 的菜单内容被静默丢弃

#### BUG-2（严重）：`gen_modern_window_wrapper` 生成 `.footer_slot()` 但 `ModernWindowShell` 无此方法

- [shell.rs:51](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L51)：`footer={...}` bind 属性生成 `.footer_slot(...)`
- [shell.rs:77](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L77)：`<slot_footer>` 子节点生成 `.footer_slot(...)`
- [modern_window.rs:76](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/modern_window.rs#L76)：`ModernWindowShell` 只有 `status_slot()` 方法，无 `footer_slot()`
- **后果**：使用 `<slot_footer>` 或 `footer={...}` 时编译失败

#### BUG-3（重要）：贡献点菜单构建不支持子菜单

- [contributions.rs:96-117](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/contributions.rs#L96-L117)：`build_menu_items` 将每个菜单贡献 flat 映射为 `MenuItem::new(name).command(cmd).into_arc()`，**从不构建 `children`**
- **后果**：菜单栏永远只渲染顶层按钮，不会出现下拉菜单。`render_menu_bar_from_items` 的 dropdown 分支永远不会触发

#### BUG-4（次要）：`<menu>` 与 `<menu-bar>` 绑定路径重复

- `<menu items={...}>`（lowercase）→ [tags.rs:346-349](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L346-L349) 路由到 `rml_ui::Menu` 组件 → `render_menu_bar_from_items()`
- `<menu-bar items={...}>`（kebab）→ [menu_bar.rs:16-24](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/menu_bar.rs#L16-L24) 直接调用 `render_menu_bar_from_items()`
- 两条路径语义重复，应明确分工或合并

#### BUG-5（次要）：`AppMenuBar` codegen 为占位

- [app_menu_bar.rs:6-8](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/app_menu_bar.rs#L6-L8)：仅生成 `rml_ui::AppMenuBar::new(cx)`，不处理子节点或属性
- 既然方案不采用 AppMenuBar，应移除此占位或明确标注未实现

---

## 三、修改方案

### 修改 1：修复 `gen_modern_window_wrapper` 传递 `slot_menu`（BUG-1）

**文件**：`crates/engine/src/compiler/codegen/shell.rs` + `crates/engine/src/compiler/codegen/mod.rs`

**shell.rs 修改**：
- `gen_modern_window_wrapper` 签名增加 `slot_menu: Option<&str>` 参数
- 在 `title_ext_slot` 之前插入 `slot_menu` 的 `.menu_slot(...)` 调用

```rust
// shell.rs 修改后的函数签名
pub(super) fn gen_modern_window_wrapper(
    elem: &Element,
    ctx: &CodegenCtx,
    children_body: &str,
    slot_menu: Option<&str>,      // 新增
    slot_title: Option<&str>,
    slot_footer: Option<&str>,   // 注意：此参数名保持 slot_footer，但生成的调用改为 .status_slot()
) -> Result<String, CodegenError> {
    // ...
    if let Some(menu) = slot_menu {
        code.push_str(&format!(".menu_slot({menu})"));
    }
    if let Some(title) = slot_title {
        code.push_str(&format!(".title_ext_slot({title})"));
    }
    // ...
}
```

**mod.rs 修改**：
- [mod.rs:212-218](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L212-L218)：调用时传入 `slot_menu_code.as_deref()`

### 修改 2：修复 `.footer_slot()` → `.status_slot()`（BUG-2）

**文件**：`crates/engine/src/compiler/codegen/shell.rs`

- [shell.rs:51](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L51)：`footer={...}` bind 生成的 `.footer_slot(...)` 改为 `.status_slot(...)`
- [shell.rs:77](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L77)：`<slot_footer>` 生成的 `.footer_slot(...)` 改为 `.status_slot(...)`

**注意**：`TabWindowShell` 的 codegen（[shell.rs:206](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L206)）也有 `.footer_slot()` 调用，需检查 `TabWindowShell` 是否有 `footer_slot` 方法或同样需要改为 `status_slot`。需先读取 `tab_window.rs` 确认。

### 修改 3：贡献点菜单支持子菜单层级（BUG-3）

**文件**：`demo/src/shell/contributions.rs`

当前 `build_menu_items` 只处理 flat 列表。需要支持 `parent_id` 层级组装（参照同文件 `build_case_tree_items` 的树构建模式）。

**方案**：利用 `ContributionOptions::parent_id`（已存在于 case 树构建）对菜单项进行层级组装：

```rust
pub fn build_menu_items<C>(
    cx: &gpui::Context<C>,
    commands: &HashMap<String, Arc<dyn ICommand>>,
) -> MenuItems {
    let mut entries: Vec<&ContributedEntry> = host_entries(cx, SHELL_HOST)
        .into_iter()
        .filter(|e| kind_of(e) == Some(KIND_MENU))
        .collect();
    entries.sort_by_key(|e| e.options.order);

    // 按 parent_id 分组（参照 build_case_tree_items 模式）
    let mut by_parent: HashMap<Option<String>, Vec<&ContributedEntry>> = HashMap::new();
    for e in &entries {
        by_parent
            .entry(e.options.parent_id.as_ref().map(|s| s.to_string()))
            .or_default()
            .push(e);
    }

    fn build_children(
        parent_id: Option<&str>,
        by_parent: &HashMap<Option<String>, Vec<&ContributedEntry>>,
        commands: &HashMap<String, Arc<dyn ICommand>>,
    ) -> MenuItems {
        let key = parent_id.map(|s| s.to_string());
        let mut siblings = by_parent.get(&key).cloned().unwrap_or_default();
        siblings.sort_by_key(|e| e.options.order);

        siblings
            .into_iter()
            .map(|e| {
                let id = e.contribution.id();
                let mut item = MenuItem::new(e.contribution.name());
                if let Some(cmd) = commands.get(id) {
                    item = item.command(cmd.clone());
                }
                let children = build_children(Some(id), by_parent, commands);
                if !children.is_empty() {
                    item = item.children(children);
                }
                item.into_arc()
            })
            .collect()
    }

    build_children(None, &by_parent, commands)
}
```

**配套**：需要在注册菜单贡献的地方支持 `parent_id`。当前 `register_menu_entry` 不接受 `parent_id`，需增加一个变体或参数：

```rust
pub fn register_menu_entry_with_parent(
    cx: &mut App,
    id: &'static str,
    name_key: &'static str,
    parent_id: Option<&'static str>,
    order: i32,
) {
    let contribution = Arc::new(TextContribution { id, name_key });
    let mut options = ContributionOptions::new().property("kind", KIND_MENU).order(order);
    if let Some(p) = parent_id {
        options = options.parent_id(p);
    }
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        global.0.register(SHELL_HOST, contribution, options, cx);
    });
}
```

### 修改 4：明确 `<menu>` 与 `<menu-bar>` 分工（BUG-4）

**文件**：`crates/engine/src/compiler/menu/menu_bar.rs` + 文档注释

**决策**：保留两条路径但明确分工：
- `<menu items={...}>`：**MVVM 数据绑定**，主窗口菜单栏用此。ViewModel 提供 `MenuItems`，框架运行时翻译。
- `<menu-bar>`：**声明式**，静态菜单场景用此。RML 子节点直接编译为 `Button::dropdown_menu`。

在 `menu_bar.rs` 顶部添加文档注释说明分工，不做代码合并（合并会破坏声明式路径的灵活性）。

### 修改 5：清理 `AppMenuBar` 占位（BUG-5）

**文件**：`crates/engine/src/compiler/menu/app_menu_bar.rs`

**决策**：保留占位但添加明确注释说明未实现原因（`OwnedMenu` 违反 MVVM），避免后续开发者误解。

```rust
/// `<app-menu-bar>` codegen
///
/// 注意：当前为占位实现。AppMenuBar 基于 gpui-component 的 `GlobalState::app_menus()`
/// + `OwnedMenu`（GPUI 原生类型），与 MVVM 数据绑定设计冲突。
/// 主窗口菜单栏应使用 `<menu items={...}>` MVVM 路径。
/// 如未来需要 macOS 原生菜单栏集成，需从 `IMenuItem` 翻译到 `OwnedMenu`。
pub fn gen_app_menu_bar(...) -> Result<String, CodegenError> {
    Ok("rml_ui::AppMenuBar::new(cx)".to_string())
}
```

---

## 四、假设与决策

| # | 决策 | 理由 |
|---|------|------|
| 1 | 主窗口菜单栏采用 MVVM 数据绑定（`<menu items={...}>`） | MVVM 纯净、贡献点兼容、动态菜单支持 |
| 2 | Shell 层使用 `<slot_menu>` 元素插槽 | 满足"slot 扩展"要求，Shell 不直接接收数据 |
| 3 | 不采用 `AppMenuBar` + `OwnedMenu` | 违反 MVVM，ViewModel 会依赖 GPUI 原生类型 |
| 4 | 保留 `<menu-bar>` 声明式路径 | 静态菜单场景（右键菜单等）仍有价值 |
| 5 | 贡献点菜单支持 `parent_id` 层级 | 参照 case 树构建模式，复用已有 `ContributionOptions::parent_id` |
| 6 | `.footer_slot()` 改为 `.status_slot()` | `ModernWindowShell` API 只有 `status_slot`，需对齐 |

## 五、验证步骤

1. **编译验证**：`cargo build -p rust-rml-engine` + `cargo build -p rust-rml-demo` 确认无编译错误
2. **slot_menu 验证**：demo 中 `<modern_window><slot_menu><menu items={menu_items} /></slot_menu></modern_window>` 应正确渲染菜单栏
3. **子菜单验证**：注册带 `parent_id` 的菜单贡献，确认菜单栏显示为 "File > New/Open/Save" 层级下拉结构
4. **status_bar 验证**：`<slot_footer>` 内容应正确渲染到底部状态栏区域
5. **集成测试**：`cargo test -p rust-rml-engine` 确认 codegen 测试通过

## 六、实施顺序

1. **修改 2**（`.footer_slot()` → `.status_slot()`）— 最简单，先修复编译错误
2. **修改 1**（传递 `slot_menu`）— 修复核心功能缺失
3. **修改 3**（贡献点子菜单）— 功能增强
4. **修改 4 + 5**（文档清理）— 非阻塞性改进
