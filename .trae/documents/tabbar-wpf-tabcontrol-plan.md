# TabBar WPF TabControl/TabItem 模式重构计划

## 摘要

参考 WPF TabControl/TabItem 模式，让 TabItem 同时承载 title (header) 与 body (内容)，
支持声明式 RML 语法：

```xml
<!-- 直接嵌套模式 -->
<tab-bar selected_index={active_tab}>
  <tab-item title="Tab1">
    <div>Tab1 body</div>
  </tab-item>
  <tab-item title="Tab2">
    <div>Tab2 body</div>
  </tab-item>
</tab-bar>

<!-- each 循环模式（数据驱动） -->
<tab-bar selected_index={active_tab}>
  <tab-item each={tab in tabs} title={tab.title}>
    <component content={tab.content} />
  </tab-item>
</tab-bar>
```

TabWindow 采用 `Vec<Arc<dyn IContribution>>` 承载 tab 业务数据，
利用 `IContribution::name()` 提供 title、`IVisualContribution::render()` 提供 body，
消除 `tab_item_template` 闭包，统一于项目贡献系统架构。

***

## 当前状态分析

### 已完成（Phase 1，前序会话产物）

| 文件                                         | 状态    | 说明                                                                                                        |
| ------------------------------------------ | ----- | --------------------------------------------------------------------------------------------------------- |
| `crates/ui/src/components/tab/tab_item.rs` | ✅ 已创建 | TabItem 结构体：title (label/icon/children) + body (Arc 闭包) + disabled + on\_click                            |
| `crates/ui/src/components/tab/tab.rs`      | ✅ 已修改 | ix/children/on\_click 字段改为 `pub(super)`                                                                   |
| `crates/ui/src/components/tab/tab_bar.rs`  | ✅ 已修改 | children 为 `SmallVec<[TabItem; 2]>`，child/children 接受 `impl Into<TabItem>`，render 提取 body 闭包 + v\_flex 堆叠 |
| `crates/ui/src/components/tab/mod.rs`      | ✅ 已修改 | `mod tab_item; pub use tab_item::*;`                                                                      |
| `crates/ui/src/window/tab_window.rs`       | ⚠️ 部分修改 | tabs 为 `Vec<Box<dyn Any>>`，新增 tab\_item\_template 字段（**需重构为 IContribution**）                              |
| `crates/ui/src/components/mod.rs`          | ✅ 已修改 | TabItem 重导出                                                                                              |
| `crates/ui/src/lib.rs`                     | ✅ 已修改 | TabItem 归入 components 重导出                                                                                 |

### Phase 1 警告清理（已完成）

* `tab_bar.rs:1`：移除未使用的 `sync::Arc`
* `tab_bar.rs:11`：移除未使用的 `Tab`
* `tab_window.rs:310`：移除 `fn render(mut self, ...)` 中的 `mut`

***

## 设计决策

### D1: TabBar 子节点 — TabItem 与 Tab 共存

* `<tab-item>` 作为新的 item builder 子标签，与 `<tab>` 并列
* `<tab>` 保持向后兼容（无 body，仅 header）
* `<tab-item>` 支持 body（子节点编译为闭包模板）
* `is_item_builder_tag()` 同时识别 Tab 和 TabItem

### D2: 循环模式 — each 指令置于 `<tab-item>`

```xml
<tab-bar selected_index={active_tab}>
  <tab-item each={tab in tabs} title={tab.title}>
    <component content={tab.content} />
  </tab-item>
</tab-bar>
```

* 显式循环变量 `tab`，`title={tab.title}` 明确是字段访问
* 复用现有 `each` 指令解析（`parser/mod.rs:169-173, 242-272`）
* codegen 生成 `.children(self.tabs.iter().map(|tab| { TabItem::new()... }))`

### D3: TabWindow — 采用 `Vec<Arc<dyn IContribution>>` 统一贡献架构

**核心架构决策**：TabWindow 的 `tabs` 字段从 `Vec<Box<dyn Any>>` 改为 `Vec<Arc<dyn IContribution>>`，
消除 `tab_item_template` 闭包，统一于项目贡献系统。

**架构理由**：

1. **`#[computed]` 兼容**：`Arc<dyn IContribution>: Clone`（Arc 标准行为），
   满足 `ComputedCache::get_or_compute<T: Clone + 'static>` 约束。
   `Box<dyn Any>` 不实现 `Clone`，是 `#[computed]` 编译失败（E0277）的根本原因。

2. **消除 `tab_item_template` 闭包**：
   * `IContribution::name() -> SharedString` 直接提供 tab title
   * `IVisualContribution::render(&self, &mut Window, &mut App) -> AnyElement` 直接提供 tab body
   * TabWindowShell.render() 内部完成 `IContribution → TabItem` 映射，无需外部闭包

3. **统一存储**：`MainWindow::entries` 已是 `Vec<Arc<dyn IContribution>>`，
   `open_tabs` 改为同类型后与贡献系统一致。`active_case_view` 已用 `c.as_visual()?.render()` 模式，
   tab 渲染复用同一机制。

4. **`Any` supertrait 兜底**：`IContribution: Send + Sync + Any`，
   需要具体类型时（如 LSP tab 的 `relative_path`）可 `downcast_ref::<T>()`。

**数据流对比**：

```text
当前：OpenTab → Box::new(OpenTab) as Box<dyn Any> → tab_item_template 闭包 downcast → TabItem
改后：OpenTab(impl IContribution) → Arc<dyn IContribution> → name() + as_visual()?.render() → TabItem
```

### D4: TabItem body 闭包 — 子节点编译为 `Fn(&mut Window, &mut App) -> AnyElement`

* 单子节点：直接 `.into_any_element()`
* 多子节点：包 `gpui::div().child(...).child(...).into_any_element()`
* 与 `SlotRenderer` 签名一致（`crates/core/src/slot.rs:23-25`）

### D5: selected\_index（索引）与 selected\_item（数据项）分离

参照 WPF TabControl 的 `SelectedIndex` / `SelectedItem` 双属性模式：

* **`selected_index: usize`**（重命名自 `selected_tab`）：选中标签的整数索引
  * RML bind 属性：`selected_index={active_tab}`（可双向，点击 tab 更新索引）
  * Rust setter：`pub fn selected_index(mut self, index: usize) -> Self`

* **`selected_item`**（新增）：选中标签对应的数据项
  * Rust 只读 getter：`pub fn selected_item(&self) -> Option<&Arc<dyn IContribution>>`
  * 内部实现：`self.tabs.get(self.selected_index)`
  * **不是 RML bind 属性**：`Arc<dyn IContribution>` 无法做 PartialEq 查找，不适合双向绑定
  * 用途：TabWindowShell render 时获取选中 tab 的业务数据，或在自定义 shell 逻辑中访问

***

## 实施方案

### Phase 1: 修复编译问题（前置）✅ 已完成

已在前序会话完成：TabItem 重导出 + 警告清理。

***

### Phase 2: TabWindow 重构为 IContribution 架构 + selected_index/selected_item

**目标**：将 `tabs: Vec<Box<dyn Any>>` 重构为 `Vec<Arc<dyn IContribution>>`，
消除 `tab_item_template` 闭包，让 `#[computed]` 可用于 `tab_bar_items()`；
同时完成 `selected_index` 重命名与 `selected_item` getter（D5）。

#### 2.1 `crates/ui/src/window/tab_window.rs` — tabs 类型重构 + selected_index/selected_item

**2.1.1** 字段类型重构（行 142）：

```diff
-     tabs: Vec<Box<dyn Any>>,
+     tabs: Vec<Arc<dyn IContribution>>,
```

**2.1.2** 移除 `tab_item_template` 字段（行 145-146）及其 setter（行 216-222）：

```diff
-     tab_item_template: Option<
-         Arc<dyn Fn(usize, &Box<dyn Any>, &mut Window, &mut App) -> TabItem + Send + Sync + 'static>,
-     >,
```

**2.1.3** 字段重命名 `selected_tab` → `selected_index`（行 147）：

```diff
-     selected_tab: usize,
+     selected_index: usize,
```

**2.1.4** 构造函数默认值（行 170）：

```diff
-         selected_tab: 0,
+         selected_index: 0,
```

移除 `tab_item_template: None,` 默认值。

**2.1.5** setter 重命名 + 移除 tab_item_template setter（行 216-227）：

```diff
-     pub fn tab_item_template<F>(mut self, f: F) -> Self
-     where
-         F: Fn(usize, &Box<dyn Any>, &mut Window, &mut App) -> TabItem
-             + Send
-             + Sync
-             + 'static,
-     { ... }
-
-     pub fn selected_tab(mut self, index: usize) -> Self {
-         self.selected_tab = index;
+     pub fn selected_index(mut self, index: usize) -> Self {
+         self.selected_index = index;
          self
      }
```

**2.1.6** 新增 `selected_item()` 只读 getter：

```rust
/// 获取当前选中 tab 对应的业务数据项。
///
/// 返回 `tabs[selected_index]` 的引用；若索引越界返回 None。
/// 与 `selected_index`（索引）对应，参照 WPF TabControl.SelectedItem。
pub fn selected_item(&self) -> Option<&Arc<dyn IContribution>> {
    self.tabs.get(self.selected_index)
}
```

**2.1.7** render 方法重构 — 从 IContribution 构建 TabItem（替换 tab_item_template 闭包调用）：

```rust
// 从 tabs 构建 TabItem 列表（替代 tab_item_template 闭包）
let tab_items: Vec<TabItem> = self.tabs.iter().map(|contribution| {
    let c = Arc::clone(contribution);
    let mut item = TabItem::new().title(c.name());
    // 若贡献实现 IVisualContribution，注入 body 闭包
    item = item.body(move |window, cx| {
        if let Some(visual) = c.as_visual() {
            visual.render(window, cx)
        } else {
            gpui::div().into_any_element()
        }
    });
    item
}).collect();
```

**关键编译可行性**：
* `Arc<dyn IContribution>: Send + Sync + 'static`（IContribution supertraits 含 Send + Sync + Any）
* 闭包捕获 `Arc` by move，满足 `'static + Send + Sync`
* `c.as_visual()` 返回 `Option<&dyn IVisualContribution>`（借用 moved `c`，生命周期正确）
* `c.name()` 返回 `SharedString`，`TabItem::title(impl Into<SharedString>)` 直接接受

**2.1.8** render 中 selected_index 引用更新：

```diff
-             .selected_index(self.selected_tab)
+             .selected_index(self.selected_index)
```

**2.1.9** import 调整：移除 `std::any::Any`，新增 `std::sync::Arc` + `rml_core::contribution::{IContribution, VisualAbilityExt}`

#### 2.2 `crates/engine/src/compiler/props_registry.rs`

SHELL\_PROPS tab\_window 条目：`selected_tab` → `selected_index`，**移除** `tab_item_template`：

```diff
  ("tab_window", &[
      "title", "width", "height", "startup", "icon",
-     "tabs", "selected_tab", "show_chrome",
+     "tabs", "selected_index", "show_chrome",
      "left_size", "right_size", "bottom_size",
-     "on_tab_click", "on_chrome_toggle", "tab_item_template",
+     "on_tab_click", "on_chrome_toggle",
  ]),
```

#### 2.3 `crates/engine/src/compiler/codegen/shell.rs`

**2.3.1** `selected_tab` codegen 重命名为 `selected_index`（行 291）：

```diff
-                     "selected_tab" => code.push_str(&format!(".selected_tab({})", rust_expr)),
+                     "selected_index" => code.push_str(&format!(".selected_index({})", rust_expr)),
```

**2.3.2** **移除** `tab_item_template` bind 分支（行 292-306）：

```diff
-                     "tab_item_template" => {
-                         let method = expr.trim();
-                         code.push_str(&format!(".tab_item_template({{ ... }})", method));
-                     }
```

**2.3.3** `tabs` bind codegen 保持走 `shell_bind_expr`（因 `tab_bar_items` 将标注 `#[computed]`，`shell_bind_expr` 会生成 `self.tab_bar_items()`）：

```rust
"tabs" => code.push_str(&format!(".tabs({})", rust_expr)),
// rust_expr 来自 shell_bind_expr，因 tab_bar_items 在 computed 列表中，
// 生成 "self.tab_bar_items()"（带括号方法调用）
```

#### 2.4 `demo/src/shell/main_window.rml`

`selected_tab` → `selected_index`，**移除** `tab_item_template` 属性：

```diff
  <tab_window
      title="RML Showcase"
      ...
      tabs={tab_bar_items}
-     tab_item_template={render_tab_item}
-     selected_tab={selected_tab}
+     selected_index={selected_tab}
      ...
```

#### 2.5 `demo/src/cases/catalog.rs` — OpenTab 实现 IContribution

```rust
use gpui::SharedString;
use rml_core::contribution::IContribution;

#[derive(Clone)]
pub struct OpenTab {
    pub id: String,
    pub title: String,
}

impl IContribution for OpenTab {
    fn id(&self) -> &str {
        &self.id
    }
    fn name(&self) -> SharedString {
        self.title.clone().into()
    }
}
```

#### 2.6 `demo/src/shell/main_window.rml.rs` — ViewModel 重构

**2.6.1** 字段类型重构（行 36）：

```diff
-     open_tabs: Vec<OpenTab>,
+     open_tabs: Vec<Arc<dyn IContribution>>,
```

**2.6.2** import 调整（行 1）：

```diff
- use std::any::Any;
+ use std::sync::Arc;
+ use rml_core::contribution::{IContribution, VisualAbilityExt};
```

**2.6.3** `tab_bar_items()` 改用 `#[computed]`（Arc 是 Clone，编译通过）：

```rust
#[computed]
pub fn tab_bar_items(&self) -> Vec<Arc<dyn IContribution>> {
    self.open_tabs.clone()  // Vec<Arc<dyn IContribution>>: Clone ✓
}
```

**2.6.4** **移除** `render_tab_item` 方法（行 214-228）— TabWindowShell 内部通过 IContribution 直接渲染。

**2.6.5** `on_loaded` 中 welcome tab 构造调整（行 84-91）：

```rust
if self.open_tabs.is_empty() {
    self.open_tabs.push(Arc::new(OpenTab {
        id: "welcome".to_string(),
        title: cx.t("shell.welcome").to_string(),
    }) as Arc<dyn IContribution>);
    self.selected_tab = 0;
    self.active_case_id = "welcome".to_string();
}
```

**2.6.6** `open_case` / `open_lsp_file` 方法中 `open_tabs.push` 调整为 `Arc::new(OpenTab { ... }) as Arc<dyn IContribution>`。

**2.6.7** `on_tab_click` 中访问 tab id 调整（行 282-288）：

```rust
#[command]
pub fn on_tab_click(&mut self, index: usize, cx: &mut Context<Self>) {
    if let Some(tab) = self.open_tabs.get(index) {
        self.selected_tab = index;
        self.active_case_id = tab.id().to_string();  // IContribution::id()
        cx.notify();
    }
}
```

**2.6.8** `active_case_view` 中 LSP tab 分流保持 `active_case_id.starts_with("lsp://")` 判断不变；
case 分支从 `open_tabs` 直接取贡献（可选优化，当前 `entries` 查找仍可用）。

***

### Phase 3: RML Codegen — `<tab-item>` 直接嵌套模式

**目标**：支持 `<tab-bar><tab-item title="...">body</tab-item></tab-bar>`

> 注：此 Phase 针对 **TabBar 组件**（非 TabWindowShell），与 Phase 2 的 IContribution 重构独立。

#### 3.1 `crates/engine/src/tags.rs`

**3.1.1** `is_item_builder_tag()` 行 438-445：新增 TabItem

```diff
  pub fn is_item_builder_tag(tag: &str) -> bool {
      matches!(
          tag,
-         "AccordionItem" | "item" | "Tab" | "tab" | "Column" | "column"
+         "AccordionItem" | "item" | "Tab" | "tab" | "TabItem" | "tab_item" | "Column" | "column"
      ) || normalize_component_tag(tag) == "AccordionItem"
          || normalize_component_tag(tag) == "Tab"
+         || normalize_component_tag(tag) == "TabItem"
          || normalize_component_tag(tag) == "Column"
  }
```

**3.1.2** `canonical_tag()` 行 154-165：新增 tab\_item 映射

```diff
      match normalized.as_str() {
          "accordion" => "Accordion".to_string(),
          "item" => "AccordionItem".to_string(),
          "tab_bar" => "TabBar".to_string(),
          "tab" => "Tab".to_string(),
+         "tab_item" => "TabItem".to_string(),
          "table" => "Table".to_string(),
          "column" => "Column".to_string(),
          _ => normalized,
      }
```

#### 3.2 `crates/engine/src/compiler/props_registry.rs`

COMPONENT\_PROPS 新增 TabItem 条目：

```rust
// TabItem 专用（item builder 子标签，WPF TabItem 模式：title + body）
("TabItem", &["title", "title_icon", "disabled", "on_click"]),
```

#### 3.3 `crates/engine/src/compiler/tab_bar/gen.rs`

修改子节点处理逻辑（行 70-92），根据标签分派到 `gen_tab_child` 或 `gen_tab_item_child`：

```rust
for child in &elem.children {
    match child {
        Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
            let canonical = tags::canonical_tag(&child_elem.tag);
            let (child_code, is_iter) = if canonical == "TabItem" {
                super::tab_item::gen_tab_item_child(child_elem, ctx, id_counter, loop_vars)?
            } else {
                (super::tab::gen_tab_child(child_elem, ctx, id_counter, loop_vars)?, false)
            };
            if is_iter {
                code.push_str(&format!("\n            .children({})", child_code));
            } else {
                code.push_str(&format!("\n            .child({})", child_code));
            }
        }
        // ... 其余分支不变
    }
}
```

#### 3.4 新建 `crates/engine/src/compiler/tab_bar/tab_item.rs`

`gen_tab_item_child()` 生成 `rml_ui::TabItem::new().title(...).body(closure)` 表达式：

````rust
//! <tab-item> 子节点 codegen — 生成 TabItem::new().title(...).body(closure)

use crate::compiler::codegen::gen_node;
use crate::compiler::{CodegenCtx, CodegenError};
use crate::parser::ast::{Attribute, Element, Node};
use crate::tags;

/// 为 <tab-item> 子节点生成 TabItem 构造表达式
///
/// 返回 (代码, 是否迭代器)：
/// - 无 each 指令：(构造表达式, false) → 父用 .child(...)
/// - 有 each 指令：(iter().map(...), true) → 父用 .children(...)
pub fn gen_tab_item_child(
    elem: &Element,
    ctx: &CodegenCtx,
    id_counter: &mut usize,
    loop_vars: &[String],
) -> Result<(String, bool), CodegenError> {
    let each_clause = elem.directives.iter().find_map(|d| match d {
        crate::parser::ast::Directive::Each(c) => Some(c.clone()),
        _ => None,
    });

    let mut child_loop_vars: Vec<String> = loop_vars.to_vec();
    if let Some(clause) = &each_clause {
        child_loop_vars.push(clause.item.clone());
        if let Some(idx) = &clause.index {
            child_loop_vars.push(idx.clone());
        }
    }

    let lv: Vec<&str> = child_loop_vars.iter().map(|s| s.as_str()).collect();
    let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

    let mut code = String::from("rml_ui::TabItem::new()");

    let mut title_set_by_attr = false;
    for attr in &elem.attributes {
        match attr {
            Attribute::Static { name, value } => {
                if let Some(s) = super::setters::static_setter(name, value, "TabItem") {
                    code.push_str(&s);
                    if name == "title" { title_set_by_attr = true; }
                } else if let Some(s) = super::super::component::component_static_setter(name, value, "TabItem") {
                    code.push_str(&s);
                    if name == "title" { title_set_by_attr = true; }
                }
            }
            Attribute::Bind { name, expr } => {
                if let Some(s) = super::setters::bind_setter(name, expr, &lv, &computed, "TabItem") {
                    code.push_str(&s);
                    if name == "title" { title_set_by_attr = true; }
                } else if let Some(s) = super::super::component::component_bind_setter(name, expr, &lv, &computed, "TabItem") {
                    code.push_str(&s);
                    if name == "title" { title_set_by_attr = true; }
                }
            }
            Attribute::Event { name, handler } => {
                if let Some(s) = super::setters::event_setter(name, handler, "TabItem") {
                    code.push_str(&s);
                } else if let Some(s) = super::super::component::component_event_setter(name, handler, "TabItem") {
                    code.push_str(&s);
                }
            }
        }
    }

    if !title_set_by_attr {
        for child in &elem.children {
            if let Node::Text(text) = child {
                code.push_str(&format!(".title({:?})", text));
                break;
            }
        }
    }

    let body_children: Vec<&Node> = elem.children.iter()
        .filter(|c| !matches!(c, Node::Text(_)))
        .collect();

    if !body_children.is_empty() {
        let body_code = if body_children.len() == 1 {
            let (child_code, _) = gen_node(body_children[0], ctx, 0, id_counter, &child_loop_vars)?;
            format!("({}).into_any_element()", child_code)
        } else {
            let mut div_code = String::from("gpui::div()");
            for child in &body_children {
                let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, &child_loop_vars)?;
                if is_iter {
                    div_code.push_str(&format!(".children({})", child_code));
                } else {
                    div_code.push_str(&format!(".child({})", child_code));
                }
            }
            format!("({}).into_any_element()", div_code)
        };

        code.push_str(&format!(
            ".body(move |_window: &mut gpui::Window, _cx: &mut gpui::App| -> gpui::AnyElement {{\n                \
             {}\n            }})",
            body_code
        ));
    }

    if let Some(clause) = each_clause {
        let iter_code = format!(
            "self.{}.iter().map(|{}| {{\n                {}\n            }})",
            clause.iterable, clause.item, code
        );
        return Ok((iter_code, true));
    }

    Ok((code, false))
}
````

#### 3.5 `crates/engine/src/compiler/tab_bar/setters.rs`

新增 TabItem 专用 setter：

```rust
// static_setter 新增（在现有 match 中添加）：
"title_icon" if tag == "TabItem" => {
    Some(format!(".title_icon(rml_ui::IconName::{})", value))
}

// bind_setter 新增：
"title" | "title_icon" if tag == "TabItem" => {
    let rust_expr = super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
    Some(format!(".{}({})", name, rust_expr))
}
```

#### 3.6 `crates/engine/src/compiler/tab_bar/mod.rs`

新增模块声明：

```rust
mod tab_item;
```

***

### Phase 4: each 指令支持（已含于 Phase 3）

`gen_tab_item_child()` 已内置 each 指令处理（步骤 5）。
无需修改 `node.rs` 的 each 逻辑 — each 在 tab\_item.rs 内部处理，不走 node.rs 的原生 each 路径。

***

### Phase 5: Demo + 回归测试

#### 5.1 `demo/src/cases/tab_bar_case.rml`

新增 TabItem 演示 section：

```xml
<div class="demo-section">
    <h3>TabItem (WPF TabControl 模式)</h3>
    <p>TabItem 同时包含 title 和 body，选中时显示对应 body 内容：</p>
    <TabBar selected_index={active_tab} on_click={on_tab_select}>
        <tab-item title="Account">
            <div class="tab-body">
                <p>Account settings panel</p>
            </div>
        </tab-item>
        <tab-item title="Profile">
            <div class="tab-body">
                <p>User profile panel</p>
            </div>
        </tab-item>
        <tab-item title="Settings">
            <div class="tab-body">
                <p>System settings panel</p>
            </div>
        </tab-item>
    </TabBar>
</div>
```

#### 5.2 单元测试

在 `crates/engine/src/compiler/tab_bar/tab_item.rs` 添加测试：

* `gen_tab_item_minimal`：`<tab-item title="A" />` → `TabItem::new().title("A")`
* `gen_tab_item_with_body`：`<tab-item title="A"><div>body</div></tab-item>` → 含 `.body(closure)`
* `gen_tab_item_with_each`：`<tab-item each={t in tabs} title={t.name}>` → 生成迭代器
* `gen_tab_item_with_title_icon`：`<tab-item title_icon="User" />` → `.title_icon(IconName::User)`
* `gen_tab_item_text_body`：`<tab-item title="A">text body</tab-item>` → 文本作为 body

在 `crates/engine/src/compiler/tab_bar/gen.rs` 添加测试：

* `gen_tab_bar_with_tab_item_child`：混合 `<Tab>` 和 `<tab-item>` 子节点
* `gen_tab_bar_with_tab_item_each`：each 循环模式

***

## 验证步骤

1. **编译验证**：

   ```powershell
   cargo check -p rust-rml-ui
   cargo check -p rust-rml-engine
   cargo check -p rml-demo
   cargo check  # 全 workspace
   ```

2. **单元测试**：

   ```powershell
   cargo test -p rust-rml-engine -- tab_bar
   cargo test -p rust-rml-engine -- tab_item
   cargo test -p rust-rml-engine --test props_registry_complete
   ```

3. **运行时验证**：

   * 启动 demo 应用
   * 验证 main\_window 的 tab\_window 标题栏 tabs 正常显示（IContribution 路径）
   * 打开 TabBar case 页面，验证 TabItem demo section（声明式路径）
   * 验证选中 tab 时 body 内容正确切换
   * 验证 welcome tab / case tab / LSP tab 三种类型均正常

***

## 文件变更清单

| 文件                                               | Phase | 操作                                                                                               |
| ------------------------------------------------ | ----- | ------------------------------------------------------------------------------------------------ |
| `crates/ui/src/components/mod.rs`                | 1     | ✅ 已修改：TabItem 重导出                                                                              |
| `crates/ui/src/components/tab/tab_bar.rs`        | 1     | ✅ 已修改：清理警告                                                                                     |
| `crates/ui/src/window/tab_window.rs`             | 1, 2  | 修改：tabs 类型 `Vec<Box<dyn Any>>` → `Vec<Arc<dyn IContribution>>`，移除 tab\_item\_template，selected\_tab→selected\_index + selected\_item getter，render 内部构建 TabItem |
| `crates/engine/src/compiler/props_registry.rs`   | 2, 3  | 修改：SHELL\_PROPS(selected\_tab→selected\_index，移除 tab\_item\_template) + COMPONENT\_PROPS(TabItem) |
| `crates/engine/src/compiler/codegen/shell.rs`    | 2     | 修改：selected\_tab→selected\_index codegen，移除 tab\_item\_template bind 代码生成                      |
| `demo/src/shell/main_window.rml`                 | 2     | 修改：selected\_tab→selected\_index，移除 tab\_item\_template 属性                                     |
| `demo/src/shell/main_window.rml.rs`              | 2     | 修改：open\_tabs 类型 → `Vec<Arc<dyn IContribution>>`，tab\_bar\_items 加 `#[computed]`，移除 render\_tab\_item，调整 push/id 访问 |
| `demo/src/cases/catalog.rs`                      | 2     | 修改：OpenTab impl IContribution                                                                   |
| `crates/engine/src/tags.rs`                      | 3     | 修改：is\_item\_builder\_tag + canonical\_tag 新增 TabItem                                           |
| `crates/engine/src/compiler/tab_bar/mod.rs`      | 3     | 修改：新增 mod tab\_item                                                                              |
| `crates/engine/src/compiler/tab_bar/gen.rs`      | 3     | 修改：子节点分派 Tab vs TabItem                                                                          |
| `crates/engine/src/compiler/tab_bar/tab_item.rs` | 3     | 新建：gen\_tab\_item\_child()                                                                       |
| `crates/engine/src/compiler/tab_bar/setters.rs`  | 3     | 修改：TabItem 专用 setter                                                                             |
| `demo/src/cases/tab_bar_case.rml`                | 5     | 修改：新增 TabItem demo                                                                               |

***

## 假设与约束

1. **TabItem 不实现 IntoElement**：TabItem 是纯数据载体，由 TabBar::render 内部消费
2. **TabItem body 闭包签名**：`Fn(&mut Window, &mut App) -> AnyElement + Send + Sync + 'static`，与 SlotRenderer 一致
3. **each 指令在 tab\_item.rs 内部处理**：不走 node.rs 的原生 each 路径（因为 TabBar 是 StatelessWithItems 组件，node.rs 对扩展组件提前 return）
4. **TabItem 与 Tab 向后兼容**：`From<Tab> for TabItem` 已实现（body=None），现有 `<Tab>` 标签不受影响
5. **`IContribution` trait 签名不修改**：复用现有 `id()` / `name()` / `icon()` / `as_visual()` 方法，不新增 trait 方法
6. **`Arc<dyn IContribution>: Clone + Send + Sync + 'static`**：满足 `ComputedCache::get_or_compute<T: Clone + 'static>` 约束，`#[computed]` 可用
7. **TabWindowShell 依赖 `IContribution`**：`crates/ui` 已依赖 `crates/core`（theme/i18n），新增 `IContribution` 依赖不引入新 crate
8. **OpenTab 实现 IContribution**：`id() → &self.id`，`name() → self.title.clone().into()`，无需 `IVisualContribution`（body 由 case 贡献的 `as_visual()?.render()` 提供）
9. **LSP Tab 保持特殊路径**：`active_case_view` 中 `if id.starts_with("lsp://")` 分流不变；`OpenTab` 存储元数据（id/title），`CodeEditorTab` Entity 懒加载机制不变
10. **selected\_index 是唯一可绑定的选中属性**：`selected_item` 仅为 Rust 只读 getter，不作为 RML bind 属性
11. **ViewModel 字段名不变**：MainWindow 的 `selected_tab: usize` 字段名保持不变，仅 RML 属性名从 `selected_tab` 改为 `selected_index`
