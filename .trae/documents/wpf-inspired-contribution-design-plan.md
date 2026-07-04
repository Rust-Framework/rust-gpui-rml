# WPF 启发的贡献系统设计计划

## 摘要

基于用户两轮纠正,重新审视设计:

1. **`IStatusBarItem` 在框架中定义** —— 仅额外提供 `align()`,`order` 由 `ContributionOptions` 提供,命令由 `render()` 自行处理
2. **参考 WPF 层级组件 XAML 设计精髓** —— 框架提供机制(traits + 通用注入点 + 容器),业务提供数据 + 渲染,不在框架层限定死扩展方式

**核心决策**: 取消前次计划中的 `<status-bar>` RML 元素、`submenu` 属性绑定、`<component>` IVisual 类型推断。这些都属于"在框架上限定死组件扩展能力"。WPF 启发的更好方案是: 框架提供最小 trait + 通用 `<component content={expr}/>` 注入点,业务在 `render()` 中自行决定结构(含递归)。

---

## 当前状态分析

### Phase 1 IVisual 抽象重构(已完成 5/6 文件)

- [crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs) —— `IVisual: IValue` 已提取,`IVisualContribution` 改为 marker trait + blanket impl,`as_visual()` 返回 `&dyn IVisual`,`register_visual_ability<T: IVisual>()` 已就位
- [crates/core/src/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/workbench.rs) —— `IWorkbench: IContribution + IVisual` 已就位
- [crates/core/src/prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs) —— `IVisual` 已导出
- [crates/macros/src/contribute.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs) —— 宏生成 `impl IVisual` + 注册 `dyn IVisual` 能力
- [crates/ui/src/components/activity_bar/visual_panel.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar/visual_panel.rs) —— `VisualActivityPanel` 直接 impl `IVisual`
- **待修改**: [demo/src/shell/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) 第 65 行 `impl IVisualContribution for CaseWorkbench` 和第 115 行 `impl IVisualContribution for LspWorkbench` 仍是旧的 marker trait impl,需改为 `impl IVisual`(因为 `IVisualContribution` 现在是 marker trait,无方法,业务必须 impl `IVisual` 才能获得 `render`)

### 现有 WPF 风格架构(已就位,无需改动)

- **`<component content={expr}/>` + `each`** —— 通用注入点 + 迭代(WPF ItemsControl 雏形),见 [crates/engine/src/compiler/codegen/node.rs:93-144](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs)
- **`<menu-bar>` ParentElement** —— 容器,接受 `.child()`/`.children()`,见 [crates/ui/src/components/menu.rs:69-127](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs)
- **`<menu-bar>` codegen** —— 声明式静态 + `each` 迭代 + 递归 submenu + separator + command 绑定,见 [crates/engine/src/compiler/menu/menu_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/menu_bar.rs) 和 [item.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/menu/item.rs)
- **`NativeStatusBar`** —— 命令式容器,`.left()`/`.right()`/`.child()`,见 [crates/ui/src/components/status_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs)
- **`MenuViewModel::build_popup_menu`** —— 递归构建 PopupMenu,见 [demo/src/shell/menu_view_model.rs:103-138](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs) —— **这就是 WPF HierarchicalDataTemplate 模式的 Rust 等价物**(模板自身递归,不经框架特殊属性)

### 待新增

- **`IStatusBarItem` trait** —— 框架缺失,用户明确要求

---

## WPF 设计原则映射

| WPF 概念 | RML 等价物 | 状态 |
|---------|-----------|------|
| `ItemsControl` 机制 | `<component each={x in items} content={x.render(_window, cx)} />` | ✅ 已有 |
| `HierarchicalDataTemplate` 递归 | `MenuViewModel::build_popup_menu` 内部递归 | ✅ 已有 |
| `StatusBarItem` 容器 + `DockPanel.Dock` | `IStatusBarItem: IVisualContribution + align()` | ❌ 待新增 |
| `MenuItem` 异构 + `Separator` | `MenuViewModel` 树 + `build_popup_menu` 分支 | ✅ 已有 |
| Container vs Content 分离 | `NativeStatusBar` 容器 + `IVisual::render` 内容 | ✅ 已有 |
| 机制 vs 策略分离 | 框架提供 trait + 容器,业务提供 ViewModel + render | ✅ 已有 |

**关键洞察**: 现有架构已高度符合 WPF 设计精髓。前次计划的 `<status-bar>` RML 元素和 `submenu` 属性绑定属于"在框架层硬编码扩展点",违反"机制 vs 策略分离"原则。WPF 的 `StatusBar` 也是 `ItemsControl` 子类,但扩展性来自 `ItemTemplate`/`ItemContainerStyle`/`ItemsPanel` 可替换,而非框架枚举item 类型。

---

## 提议变更

### Phase 1: 完成 IVisual 抽象重构(收尾)

**文件**: [demo/src/shell/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs)

**变更**:
1. 第 17 行: 移除 `IVisualContribution` 导入,改为 `IVisual`
2. 第 65-69 行: `impl IVisualContribution for CaseWorkbench` → `impl IVisual for CaseWorkbench`(签名相同,仅 trait 名变化)
3. 第 115-137 行: `impl IVisualContribution for LspWorkbench` → `impl IVisual for LspWorkbench`
4. 更新第 3 行和第 25 行注释: `IVisualContribution` → `IVisual`

**为什么**: `IVisualContribution` 现在是 marker trait(blanket impl 自动获得),业务必须 impl `IVisual` 提供 `render`。`IWorkbench: IContribution + IVisual` 要求 CaseWorkbench/LspWorkbench 直接 impl `IVisual`。

**验证**: `cargo build -p rml-core && cargo test -p rml-core && cargo test -p rml-macros && cargo build`

---

### Phase 2: 定义 `IStatusBarItem` trait

**文件**: 
- [crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs) —— 新增 trait
- [crates/ui/src/components/status_bar.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/status_bar.rs) —— `StatusBarAlign` 移至 `rml_core::contribution`(或 re-export)
- [crates/core/src/prelude.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/prelude.rs) —— 导出 `IStatusBarItem` 和 `StatusBarAlign`

**变更**:

1. 在 `crates/core/src/contribution.rs` 末尾新增:
   ```rust
   /// 状态栏项 —— 状态栏容器的视觉贡献,额外提供对齐提示。
   ///
   /// WPF `StatusBarItem` + `DockPanel.Dock` 类比:容器按 `align()` 决定布局位置,
   /// 内容由 `IVisual::render` 提供。`order` 经 `ContributionOptions` 传入,
   /// 命令由 `render` 自行处理(返回的 `AnyElement` 可携带 `.on_click` 等)。
   pub trait IStatusBarItem: IVisualContribution {
       fn align(&self) -> StatusBarAlign;
   }
   ```

2. 将 `StatusBarAlign` 枚举从 `rml_ui::status_bar` 移至 `rml_core::contribution`(因为 trait 方法返回类型需要):
   ```rust
   #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
   pub enum StatusBarAlign {
       #[default]
       Left,
       Right,
       Center,
   }
   ```
   `rml_ui::status_bar` 改为 `pub use rml_core::contribution::StatusBarAlign;` 保持兼容。

3. `crates/core/src/prelude.rs` 导出 `IStatusBarItem` 和 `StatusBarAlign`。

**为什么**:
- `IStatusBarItem` 是 WPF `StatusBarItem` 容器的数据契约 —— 容器需要 `align()` 决定布局,这是框架必须知道的最小信息
- `order` 不在 trait 中,因为它属于注册元数据(`ContributionOptions`),由 host 排序时使用,不属于 item 自身能力
- 命令不在 trait 中,因为 `render()` 返回的 `AnyElement` 可携带任意交互能力(包括 `.on_click`),无需框架额外抽象
- `StatusBarAlign` 移至 `rml_core` 是因为 trait 方法返回类型必须在 core 定义

**验证**: `cargo build -p rml-core && cargo test -p rml-core`

---

### Phase 3: 重构 demo 使用 `IStatusBarItem`

**文件**:
- [demo/src/shell/status_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/status_view_model.rs) —— 删除 `StatusViewModel`,直接使用 `Arc<dyn IStatusBarItem>`
- [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) —— `status` 字段类型改为 `Vec<Arc<dyn IStatusBarItem>>`,`render_status_bar` 直接调用 `item.align()` + `item.render()`
- 业务侧的 status 贡献结构体需 impl `IStatusBarItem`(见下)

**变更**:

1. `status_view_model.rs` 重写为:
   ```rust
   use std::sync::Arc;
   use rml_core::contribution::{ContributionOptions, IContribution, IStatusBarItem, StatusBarAlign, VisualAbilityExt};
   
   pub type ContribEntry = (Arc<dyn IContribution>, ContributionOptions);
   
   /// 从贡献条目列表构建 `IStatusBarItem` 列表(按 order 排序)。
   pub fn build_status_items(entries: &[ContribEntry]) -> Vec<Arc<dyn IStatusBarItem>> {
       let mut items: Vec<Arc<dyn IStatusBarItem>> = entries
           .iter()
           .filter_map(|(c, o)| {
               if o.effective_slot() != Some("status") { return None; }
               // 经 as_visual 校验是 IVisualContribution;进一步 downcast 为 IStatusBarItem
               // (依赖 ability registry 已注册 IStatusBarItem cast)
               ...
           })
           .collect();
       items.sort_by_key(|s| /* order from options */);
       items
   }
   ```

   **注意**: 此处需要解决一个技术问题 —— `IStatusBarItem` 能力查询。`as_visual()` 返回 `&dyn IVisual`,但需要 `&dyn IStatusBarItem`。两种方案:
   
   - **方案 A**: 为 `IStatusBarItem` 注册独立 ability cast(`register_status_bar_item_ability::<T>()`),新增 `StatusBarItemAbilityExt::as_status_bar_item()`
   - **方案 B**: 让 `IStatusBarItem` 不再是 trait,改为在 `ContributionOptions.properties["align"]` 中携带 align 信息,业务 ViewModel 仍需存在
   
   **推荐方案 A**(与现有 `as_visual`/`as_contribution` 模式一致),但需在 `contribution.rs` 中新增 `StatusBarItemAbilityExt` + `register_status_bar_item_ability::<T>()`,并在 `#[contribute]` 宏中识别 `kind="status"` 自动注册(或业务手动注册)。

2. 业务侧 status 贡献结构体(如 `CaseCountItem`、`LspStatusItem` 等)impl `IStatusBarItem`:
   ```rust
   impl IStatusBarItem for CaseCountItem {
       fn align(&self) -> StatusBarAlign { StatusBarAlign::Left }
   }
   ```
   `render` 方法已由 `#[contribute] + #[component]` 宏自动生成 impl `IVisual`(委托给 `RenderOnce::render`)。

3. `main_window.rml.rs` 中:
   - `status: Vec<StatusViewModel>` → `status: Vec<Arc<dyn IStatusBarItem>>`
   - `render_status_bar` 简化:
     ```rust
     for s in &self.status {
         let content = s.render(window, _cx);
         match s.align() {
             StatusBarAlign::Left => bar = bar.left(content),
             StatusBarAlign::Right => bar = bar.right(content),
             StatusBarAlign::Center => bar = bar.child(content),
         }
     }
     ```

**为什么**:
- 删除 `StatusViewModel` 中间层 —— `IStatusBarItem` 已提供 `align()`,`IVisual` 已提供 `render`,无需再包装
- WPF `StatusBarItem` 也是直接容器,不强制业务再包一层 ViewModel
- `order` 仍由 host 排序时使用(从 `ContributionOptions` 读取),不进入 trait

**验证**: `cargo build -p demo && cargo test -p demo`(若有测试);运行 demo 验证状态栏显示

---

### Phase 4: 文档与 skill 更新

**文件**:
- [docs/01-overview/developer-guide.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/01-overview/developer-guide.md) 第 62、81、170 行 —— 删除过时的 `<status_bar items={...}/>` 和 `Vec<Arc<dyn IStatusBarItem>>` 引用,替换为新模式
- [docs/06-components/slots.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/06-components/slots.md) 第 69 行
- [docs/06-components/builtin-components.md](file:///e:/GitCode/RF/rust-gpui-rml/docs/06-components/builtin-components.md) 第 106、115 行
- [.trae/skills/rml-component/04-data-binding.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/04-data-binding.md) 第 22、32、44 行
- [.trae/skills/rml-component/05-slot-template.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/05-slot-template.md) 第 13 行
- [.trae/skills/rml-component/02-component-registration.md](file:///e:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/02-component-registration.md) 第 96-114 行

**变更**: 统一替换为"WPF 启发模式":
- 框架提供 `IStatusBarItem` trait(仅 `align()`)
- 业务 impl `IStatusBarItem + IContribution + IVisual`(后两者经 `#[contribute] + #[component]` 自动)
- host 经 `Vec<Arc<dyn IStatusBarItem>>` 持有,`render_status_bar` 命令式组装
- 不存在 `<status-bar>` RML 元素

**为什么**: 消除前次"WPF style —— 不定义 IStatusBarItem"决策留下的过时文档,与新决策对齐。

**验证**: 全文搜索 `IStatusBarContribution`、`<status_bar` 应无残留;搜索 `IStatusBarItem` 应仅出现在新文档。

---

## 假设与决策

### 决策

1. **`IStatusBarItem` 在框架中定义** —— 仅 `align()`,WPF `StatusBarItem` + `DockPanel.Dock` 类比。容器需要 layout hint,这是框架必须知道的最小信息。
2. **不添加 `<status-bar>` RML 元素** —— 使用 `NativeStatusBar` 命令式容器 + `<component content={self.render_status_bar(...)}/>` 注入。WPF `StatusBar` 也是命令式容器,扩展性来自 `ItemTemplate`/`ItemsPanel` 可替换,不来自框架枚举 item 类型。
3. **不添加 `submenu` 属性绑定** —— `MenuViewModel::build_popup_menu` 已实现递归(WPF HierarchicalDataTemplate 模式的 Rust 等价物)。框架不限制递归方式。
4. **不添加 `<component>` IVisual 类型推断** —— 显式 `x.render(_window, cx)` 调用比类型推断更清晰、更灵活(可链式调用、条件渲染等)。
5. **不添加 `IMenuItem` trait** —— WPF style,菜单项数据契约由业务定义。`MenuViewModel` 已在 demo 层,框架不强制。
6. **`StatusBarAlign` 从 `rml_ui` 移至 `rml_core`** —— `IStatusBarItem::align()` 返回类型需要在 core 定义。`rml_ui` re-export 保持兼容。
7. **`IStatusBarItem` 能力查询走 ability registry** —— 与 `as_visual`/`as_contribution` 模式一致,新增 `StatusBarItemAbilityExt::as_status_bar_item()` + `register_status_bar_item_ability::<T>()`。
8. **删除 `StatusViewModel` 中间层** —— `IStatusBarItem` 已提供所需能力,无需业务再包一层 ViewModel。`order` 在 host 排序时从 `ContributionOptions` 读取,不进入 trait。

### 假设

1. Phase 1 的 5 个文件修改已正确完成(系统提醒中已展示当前状态)
2. `#[contribute] + #[component]` 宏已正确生成 `impl IVisual`(委托给 `RenderOnce::render`)
3. `NativeStatusBar` 的 `.left()`/`.right()`/`.child()` API 稳定,无需改动
4. demo 中存在或可新增 impl `IStatusBarItem` 的业务结构体(需在实施时确认)

### 取消的前次计划项

- ❌ Phase 2: `<component>` IVisual 识别(类型推断 + `visual` 属性) —— 显式调用更清晰
- ❌ Phase 3: `<status-bar>` RML 组件 + codegen —— 命令式容器已足够
- ❌ Phase 4: `<menu-bar>` `submenu` 属性绑定 —— `build_popup_menu` 递归已实现

---

## 验证步骤

### 编译验证

1. `cargo build -p rml-core` —— Phase 1 + Phase 2 编译通过
2. `cargo test -p rml-core` —— 单元测试通过
3. `cargo test -p rml-macros` —— 宏测试通过
4. `cargo build` —— 全工作区编译通过(含 demo)
5. `cargo test` —— 全工作区测试通过

### 功能验证(手动)

6. `cargo run -p demo` 启动应用
7. 状态栏显示正常,左右对齐正确(`StatusBarAlign::Left`/`Right` 项分别位于左右)
8. 菜单栏显示正常,点击顶级菜单弹出 PopupMenu
9. 子菜单展开正常(验证 `build_popup_menu` 递归未受影响)
10. 菜单项点击触发命令(验证 `as_command()` 路径未受影响)
11. 案例切换正常(验证 `IWorkbench: IContribution + IVisual` + `as_visual()` 路径未受影响)
12. LSP 工作台打开文件正常(验证 `LspWorkbench: IVisual` impl 正确)

### 文档验证

13. `grep -r "IStatusBarContribution" docs/ .trae/skills/` 无结果
14. `grep -r "<status_bar" docs/ .trae/skills/` 无结果
15. `grep -r "IStatusBarItem" docs/ .trae/skills/` 仅出现在新文档中

---

## 实施顺序

1. **Phase 1**(收尾,~5 分钟): 修改 `demo/src/shell/workbench.rs`,运行编译验证
2. **Phase 2**(核心,~15 分钟): 在 `crates/core/src/contribution.rs` 新增 `IStatusBarItem` + `StatusBarAlign` + ability 查询,移动 `StatusBarAlign` 至 core,更新 prelude
3. **Phase 3**(业务适配,~20 分钟): 重构 `status_view_model.rs` 删除 `StatusViewModel`,修改 `main_window.rml.rs` 字段类型和 `render_status_bar`,业务侧 status 贡献结构体 impl `IStatusBarItem`
4. **Phase 4**(文档,~10 分钟): 更新 docs 和 skills 中过时的 status-bar 引用
