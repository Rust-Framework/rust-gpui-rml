# RML Accordion 组件支持实施计划

## 概述

为 RML 框架添加对 [gpui-component Accordion](https://longbridge.github.io/gpui-component/zh-CN/docs/components/accordion) 的声明式支持，使用户可在 `.rml` 模板中以 `<Accordion>` / `<AccordionItem>` 标签声明可折叠内容组件，并通过 RML 的属性绑定、事件绑定、i18n 等机制进行配置。

## 当前状态分析

### gpui-component Accordion 实际 API（已确认源码）

源码位置：`c:\Users\lusid\.cargo\git\checkouts\gpui-component-f2cfc37a601d48ab\063e55b\crates\ui\src\accordion.rs`

**Accordion（`RenderOnce`，`#[derive(IntoElement)]`）**
- 构造：`Accordion::new(id: impl Into<ElementId>) -> Self`
- Builder 方法：
  - `.multiple(bool)` — 是否允许多项同时展开（默认 false）
  - `.bordered(bool)` — 是否带边框（默认 true）
  - `.disabled(bool)` — 是否禁用（默认 false）
  - `.item(F: FnOnce(AccordionItem) -> AccordionItem) -> Self` — 添加项（**闭包式 builder**）
  - `.on_toggle_click(impl Fn(&[usize], &mut Window, &mut App) + Send + Sync + 'static)` — 切换回调
- 实现 `Sizable`（支持 `.small()` / `.medium()` / `.large()` / `.xsmall()`）

**AccordionItem（`RenderOnce`，`#[derive(IntoElement)]`，`impl ParentElement`）**
- 构造：`AccordionItem::new() -> Self`（无 id 参数）
- Builder 方法：
  - `.title(impl IntoElement)` — 标题（支持任意元素，不只是字符串）
  - `.icon(impl Into<Icon>)` — 标题图标
  - `.open(bool)` — 初始是否展开（默认 false）
  - `.bordered(bool)` / `.disabled(bool)`
- 实现 `Sizable`、`ParentElement`（用 `.child(...)` 添加内容）

### RML 现有扩展组件集成模式（已确认）

- 路由表：[crates/engine/src/tags.rs:269-365](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L269-L365) 的 `component_lookup()` 函数
- ComponentKind 四种变体：`Stateless` / `StatelessNoId` / `Stateful { state_field }` / `EntityRef`
- codegen 入口：[crates/engine/src/compiler/component.rs:29-171](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L29-L171) 的 `gen_component()`
- 属性注册：[crates/engine/src/compiler/props_registry.rs:66-76](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L66-L76) 的 `COMPONENT_PROPS`
- 现有 `gen_component` 子节点处理：
  - `StatelessNoId` 容器（TitleBar/StatusBar）：子节点 → `.child(...)` / `.children(...)`
  - 其他 Stateless：仅支持文本子节点作为 `.label(...)`
- re-export 轨：[crates/ui/src/lib.rs:42-67](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs#L42-L67)

### 与现有模式的差异（关键挑战）

Accordion 的 `.item(|item| item.title(...).child(...))` 是**闭包式 builder**，与现有的两种子节点模式都不匹配：
- 不是简单 `.child(...)` 容器（需要闭包包装）
- 不是文本 `.label(...)`（子节点是结构化的 `AccordionItem`）

需要引入新的 `ComponentKind` 变体以表达这种模式。

### 用户偏好约束（来自 memory）

- 偏好"添加变体到现有枚举"而非暴露新接口（支持新增 `ComponentKind::StatelessWithItems`）
- 命名简洁清晰，避免冗余前缀
- 数据管理统一，不硬编码
- Demo 是教学项目，需清晰展示用法

## 提议变更

### 变更 1：在 `crates/ui/src/lib.rs` re-export Accordion

**文件**：[crates/ui/src/lib.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/ui/src/lib.rs)

**What**：在 `pub use gpui_component::{...}` 块中新增 accordion 模块导出。

**Why**：与现有 Button/Input/Tree 等 re-export 模式一致，使 codegen 可用 `rml_ui::Accordion` / `rml_ui::AccordionItem` 路径引用。

**How**：在第 42-67 行的 re-export 块中添加：
```rust
pub use gpui_component::{
    // ...existing...
    accordion::{Accordion, AccordionItem},
    // ...existing...
};
```

### 变更 2：新增 `ComponentKind::StatelessWithItems` 变体

**文件**：[crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)

**What**：在 `ComponentKind` 枚举（240-255 行）添加新变体 `StatelessWithItems`。

**Why**：Accordion 的 `.item(|item| ...)` 闭包式 builder 模式无法用现有四种变体表达。新变体语义为"无状态组件，接受 `.item(|x| ...)` 闭包式子项"。未来类似组件（如 List、Carousel）可复用。

**How**：
```rust
pub enum ComponentKind {
    Stateless,
    StatelessNoId,
    Stateful { state_field: &'static str },
    EntityRef,
    /// 无状态组件，子节点通过 `.item(|item| ...)` 闭包式 builder 注入。
    ///
    /// 构造调用形如 `Accordion::new(id)`，与 `Stateless` 一致；
    /// 但子节点处理不同：每个 `<AccordionItem>` 子节点生成
    /// `.item(|__rml_item: rml_ui::AccordionItem| __rml_item.<setters>.child(...))`。
    /// 子节点 tag 名（如 `AccordionItem`）由 codegen 在 `StatelessWithItems` 分支硬编码识别。
    StatelessWithItems,
}
```

### 变更 3：在 `component_lookup` 注册 `Accordion`

**文件**：[crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs) 第 269-365 行

**What**：在 `component_lookup()` 函数中添加 `"Accordion"` 分支。

**Why**：让 RML 识别 `<Accordion>` 标签并路由到 `StatelessWithItems` codegen 路径。

**How**：
```rust
"Accordion" => Some(ComponentTag {
    ctor_path: "rml_ui::Accordion",
    kind: ComponentKind::StatelessWithItems,
}),
```

**注意**：不注册 `AccordionItem` 为独立扩展组件——它仅在 `<Accordion>` 内部有意义，由 `StatelessWithItems` codegen 分支专门处理。

### 变更 4：新增 `is_item_builder_tag` 辅助函数

**文件**：[crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs)

**What**：新增公共函数 `is_item_builder_tag(tag: &str) -> bool`，返回 `true` 当 `tag == "AccordionItem"`。

**Why**：
- `AccordionItem` 不在 `component_lookup` 中，`is_extension_component` 返回 false，导致 `validate_unknown_props` 不会校验其属性
- 通过此函数让 validator 和 codegen 共同识别"item builder 子标签"
- 未来可扩展支持其他闭包式 builder 子项

**How**：
```rust
/// 判断标签是否为 `StatelessWithItems` 组件的子项 builder
///
/// 如 `AccordionItem` 是 `Accordion` 的子项，仅在 `<Accordion>` 内合法。
/// 不在 `component_lookup` 中注册（避免被误用为顶层扩展组件），
/// 但在 validator 和 codegen 中通过此函数识别。
pub fn is_item_builder_tag(tag: &str) -> bool {
    matches!(tag, "AccordionItem")
}
```

### 变更 5：扩展 `gen_component` 处理 `StatelessWithItems`

**文件**：[crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)

**What**：在 `gen_component()` 中添加 `StatelessWithItems` 分支，处理 `<Accordion>` 及其 `<AccordionItem>` 子节点。

**Why**：现有 `gen_component` 仅支持 `.child(...)` / `.label(...)` 两种子节点模式，无法生成 `.item(|item| ...)` 闭包。

**How**：

1. **构造器**（65-100 行 match 块）：`StatelessWithItems` 与 `Stateless` 完全一致：
   ```rust
   tags::ComponentKind::StatelessWithItems => {
       if let Some(name) = ref_name {
           format!("{}::new({:?})", component.ctor_path, format!("rml_ref:{}", name))
       } else {
           format!("{}::new((\"rml_el\", {}usize))", component.ctor_path, id_val)
       }
   }
   ```

2. **子节点处理**（140-168 行）：新增 `StatelessWithItems` 分支：
   ```rust
   let is_items_container = matches!(component.kind, tags::ComponentKind::StatelessWithItems);

   if is_items_container {
       // 闭包式 builder：每个 <AccordionItem> 子节点生成 .item(|__rml_item| ...)
       for child in &elem.children {
           match child {
               Node::Element(child_elem) if tags::is_item_builder_tag(&child_elem.tag) => {
                   let item_code = gen_item_builder(child_elem, ctx, id_counter, loop_vars, &resolved)?;
                   code.push_str(&format!("\n            .item({})", item_code));
               }
               Node::Text(text) => {
                   // 容错：纯文本子节点被忽略（Accordion 不接受文本 label）
                   eprintln!("[rml warning] <{}> 不支持文本子节点 {:?}，已忽略", resolved, text);
               }
               _ => {
                   return Err(CodegenError {
                       message: format!(
                           "<{}> 仅支持 <AccordionItem> 子节点，得到 <{}>",
                           resolved,
                           match child {
                               Node::Element(e) => e.tag.as_str(),
                               _ => "<non-element>",
                           }
                       ),
                   });
               }
           }
       }
   } else if is_container {
       // ...existing StatelessNoId container logic...
   } else if !label_set_by_attr {
       // ...existing text-as-label logic...
   }
   ```

3. **新增 `gen_item_builder` 辅助函数**：
   ```rust
   /// 为 `<AccordionItem>` 子节点生成闭包式 builder 代码
   ///
   /// 生成形如：
   /// ```text
   /// |__rml_item: rml_ui::AccordionItem| __rml_item.title("Section 1").open(true).child("Content")
   /// ```
   fn gen_item_builder(
       elem: &Element,
       ctx: &CodegenCtx,
       id_counter: &mut usize,
       loop_vars: &[String],
       parent_tag: &str,
   ) -> Result<String, CodegenError> {
       let lv: Vec<&str> = loop_vars.iter().map(|s| s.as_str()).collect();
       let computed: Vec<&str> = ctx.computed_methods.iter().map(|s| s.as_str()).collect();

       let mut code = String::from("|__rml_item: rml_ui::AccordionItem| __rml_item");

       // 静态/绑定/事件属性 → AccordionItem setter
       for attr in &elem.attributes {
           match attr {
               Attribute::Static { name, value } => {
                   if let Some(setter) = component_static_setter(name, value, "AccordionItem") {
                       code.push_str(&setter);
                   }
               }
               Attribute::Bind { name, expr } => {
                   if let Some(setter) = component_bind_setter(name, expr, &lv, &computed, "AccordionItem") {
                       code.push_str(&setter);
                   }
               }
               // AccordionItem 当前无事件属性，跳过
               _ => {}
           }
       }

       // 子节点 → .child(...)
       for child in &elem.children {
           let (child_code, is_iter) = gen_node(child, ctx, 0, id_counter, loop_vars)?;
           if is_iter {
               code.push_str(&format!(".children({})", child_code));
           } else {
               code.push_str(&format!(".child({})", child_code));
           }
       }

       Ok(code)
   }
   ```

### 变更 6：在 setter 映射函数中添加 Accordion 专用分支

**文件**：[crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs)

**What**：在 `component_static_setter` / `component_bind_setter` / `component_event_setter` 三个函数中添加 Accordion 与 AccordionItem 的专用属性映射。

**Why**：Accordion 的 `multiple` / `bordered` / `on_toggle_click`，AccordionItem 的 `title` / `open` / `icon` 不在通用属性中。

**How**：

1. **`component_static_setter`**（328-394 行）添加：
   ```rust
   // Accordion 专用：multiple / bordered 接受 bool 字面量
   "multiple" | "bordered" | "open" => Some(format!(".{}({})", name, parse_bool(value))),
   // AccordionItem 专用：icon="Settings" → .icon(rml_ui::IconName::Settings)
   "icon" => Some(format!(".icon(rml_ui::IconName::{})", value)),
   ```

2. **`component_bind_setter`**（438-487 行）添加：
   ```rust
   // Accordion：multiple={expr} / bordered={expr} → .multiple(expr) / .bordered(expr)
   "multiple" | "bordered" | "open" => {
       let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
       Some(format!(".{}({})", name, rust_expr))
   }
   // AccordionItem：title={expr} → .title(expr)（title 接受 impl IntoElement，不需 clone）
   "title" => {
       let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
       Some(format!(".title({})", rust_expr))
   }
   // AccordionItem：icon={expr} → .icon(expr)
   "icon" => {
       let rust_expr = component_bind_rust_expr(expr_str, loop_vars, computed);
       Some(format!(".icon({})", rust_expr))
   }
   ```

3. **`component_event_setter`**（497-561 行）添加：
   ```rust
   "on_toggle_click" if tag == "Accordion" => {
       let method = match handler {
           EventHandler::Ident(m) | EventHandler::MethodName(m) => m,
           EventHandler::WithArgs(m, _) => m,
       };
       // on_toggle_click 回调签名：Fn(&[usize], &mut Window, &mut App)
       // 用户方法签名建议：fn on_toggle(&mut self, open_indices: &[usize], cx: &mut Context<Self>)
       Some(format!(
           ".on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| {{\n                    \
            this.{}(open_ixs, cx);\n                }}))",
           method
       ))
   }
   ```

### 变更 7：在 `props_registry.rs` 注册 Accordion 属性

**文件**：[crates/engine/src/compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs)

**What**：在 `COMPONENT_PROPS`（66-76 行）添加 Accordion 与 AccordionItem 的专用属性。

**Why**：保证 `validate_unknown_props` 能识别 `<Accordion>` / `<AccordionItem>` 上的合法 bind/event 属性，避免误报未知属性错误；同时让 `is_prop_registered` 在 setter 未命中时输出 warning。

**How**：
```rust
pub static COMPONENT_PROPS: &[(&str, &[&str])] = &[
    // ...existing...
    ("Input", &["onchange"]),
    ("TextInput", &["onchange"]),
    ("Tree", &["items", "on_activate", "on_select"]),
    ("MenuBar", &["items"]),
    ("menu", &["items"]),
    ("status_bar", &["items"]),
    // 新增
    ("Accordion", &["multiple", "bordered", "on_toggle_click"]),
    ("AccordionItem", &["title", "open", "icon"]),
];
```

**注意**：`AccordionItem` 不在 `component_lookup` 中，但 `COMPONENT_PROPS` 中的注册仍有效——通过变更 4 的 `is_item_builder_tag` 在 validator 中识别。需在 `component_props_tags_align_with_routing_table` 测试中排除 `AccordionItem`（或调整测试逻辑），避免 "COMPONENT_PROPS contains 'AccordionItem' but component_lookup returns None" 断言失败。

调整测试：
```rust
#[test]
fn component_props_tags_align_with_routing_table() {
    use crate::tags;
    for (tag, _) in COMPONENT_PROPS {
        // AccordionItem 是 item builder 子标签，不在 component_lookup 中
        if *tag == "AccordionItem" {
            assert!(tags::is_item_builder_tag(tag));
            continue;
        }
        assert!(
            tags::component_lookup(tag).is_some(),
            "COMPONENT_PROPS contains tag '{}' but tags::component_lookup returns None",
            tag
        );
    }
}
```

### 变更 8：在 `validator.rs` 中校验 AccordionItem 属性

**文件**：[crates/engine/src/compiler/validator.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/validator.rs)

**What**：在 `validate_unknown_props`（125-161 行）中扩展，使 `is_item_builder_tag` 返回 true 的标签也走属性校验。

**Why**：`AccordionItem` 不在 `component_lookup`，默认不会被 `is_extension_component` 识别，导致属性拼写错误（如 `<AccordionItem titel="...">`）无法在编译期捕获。

**How**：
```rust
fn validate_unknown_props(elem: &Element) -> Result<(), ValidationError> {
    let tag = &elem.tag;

    // Shell 根标签
    if tags::root_tag_lookup(tag).is_some() {
        // ...existing...
    }

    // 扩展组件 OR item builder 子标签
    if tags::is_extension_component(tag) || tags::is_item_builder_tag(tag) {
        for attr in &elem.attributes {
            if let Attribute::Bind { name, .. } | Attribute::Event { name, .. } = attr {
                if !crate::compiler::props_registry::is_prop_registered(tag, name) {
                    return Err(ValidationError {
                        message: format!(
                            "unknown property `{}` on <{}>: not in component property registry",
                            name, tag
                        ),
                    });
                }
            }
        }
    }

    Ok(())
}
```

### 变更 9：添加 Accordion demo 案例

**文件**（新增 4 个 + 修改 3 个）：

1. **新增** `demo/src/cases/accordion_case.rml.rs`：
   ```rust
   use rml::prelude::*;

   #[contribute(
       host_id = "demo.activity",
       id = "components.accordion",
       name = "case.accordion.title",
       kind = "case",
       group = "components",
       order = 13,
   )]
   #[component]
   #[derive(Default)]
   pub struct AccordionCase {
       pub last_open: String,
   }

   impl ILifecycle for AccordionCase {}

   impl AccordionCase {
       #[computed]
       pub fn status_text(&self) -> String {
           if self.last_open.is_empty() {
               "尚未切换任何项".to_string()
           } else {
               format!("上次展开项索引：{}", self.last_open)
           }
       }

       #[command]
       pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
           self.last_open = format!("{:?}", open_ixs);
           cx.notify();
       }
   }
   ```

2. **新增** `demo/src/cases/accordion_case.rml`：
   ```xml
   <component>
       <div v_flex="" class="case-pane">
           <h2>{t("case.accordion.title")}</h2>
           <p>{status_text}</p>

           <h3>{t("case.accordion.basic")}</h3>
           <Accordion bordered="">
               <AccordionItem title={t("case.accordion.section1")} open="">
                   <p>{t("case.accordion.content1")}</p>
               </AccordionItem>
               <AccordionItem title={t("case.accordion.section2")}>
                   <p>{t("case.accordion.content2")}</p>
               </AccordionItem>
               <AccordionItem title={t("case.accordion.section3")}>
                   <p>{t("case.accordion.content3")}</p>
               </AccordionItem>
           </Accordion>

           <h3>{t("case.accordion.multiple")}</h3>
           <Accordion multiple="" bordered="">
               <AccordionItem title={t("case.accordion.section1")} open="">
                   <p>{t("case.accordion.content1")}</p>
               </AccordionItem>
               <AccordionItem title={t("case.accordion.section2")} open="">
                   <p>{t("case.accordion.content2")}</p>
               </AccordionItem>
           </Accordion>

           <h3>{t("case.accordion.sizes")}</h3>
           <Accordion small="" bordered="" on_toggle_click={on_toggle}>
               <AccordionItem title={t("case.accordion.small")}>
                   <p>{t("case.accordion.content1")}</p>
               </AccordionItem>
           </Accordion>
           <Accordion large="" bordered="" on_toggle_click={on_toggle}>
               <AccordionItem title={t("case.accordion.large")}>
                   <p>{t("case.accordion.content1")}</p>
               </AccordionItem>
           </Accordion>

           <h3>{t("case.accordion.with_icon")}</h3>
           <Accordion bordered="" on_toggle_click={on_toggle}>
               <AccordionItem title={t("case.accordion.settings")} icon="Settings">
                   <p>{t("case.accordion.content1")}</p>
               </AccordionItem>
               <AccordionItem title={t("case.accordion.disabled")} icon="Lock" disabled="true">
                   <p>{t("case.accordion.content2")}</p>
               </AccordionItem>
           </Accordion>

           <h3>{t("case.accordion.nested")}</h3>
           <Accordion bordered="">
               <AccordionItem title={t("case.accordion.parent")}>
                   <Accordion bordered="" multiple="">
                       <AccordionItem title={t("case.accordion.child1")}>
                           <p>{t("case.accordion.content1")}</p>
                       </AccordionItem>
                       <AccordionItem title={t("case.accordion.child2")}>
                           <p>{t("case.accordion.content2")}</p>
                       </AccordionItem>
                   </Accordion>
               </AccordionItem>
           </Accordion>
       </div>
   </component>
   ```

3. **修改** `demo/src/cases/mod.rs`：添加模块声明
   ```rust
   #[path = "accordion_case.rml.rs"]
   pub mod accordion_case;
   ```

4. **修改** `demo/src/cases/catalog.rs`：添加 case id 映射
   ```rust
   "components.accordion" => "case.accordion.title",
   ```

5. **修改** `demo/assets/i18n/zh-CN.json`：添加中文翻译键
   ```json
   "case.accordion.title": "折叠面板",
   "case.accordion.basic": "基础用法",
   "case.accordion.multiple": "允许多项展开",
   "case.accordion.sizes": "不同尺寸",
   "case.accordion.with_icon": "带图标与禁用",
   "case.accordion.nested": "嵌套折叠面板",
   "case.accordion.section1": "第一部分",
   "case.accordion.section2": "第二部分",
   "case.accordion.section3": "第三部分",
   "case.accordion.content1": "这是第一部分的内容。",
   "case.accordion.content2": "这是第二部分的内容。",
   "case.accordion.content3": "这是第三部分的内容。",
   "case.accordion.small": "小尺寸项",
   "case.accordion.large": "大尺寸项",
   "case.accordion.settings": "设置",
   "case.accordion.disabled": "禁用项",
   "case.accordion.parent": "父级（含嵌套）",
   "case.accordion.child1": "子项 1",
   "case.accordion.child2": "子项 2"
   ```

6. **修改** `demo/assets/i18n/en-US.json`：添加英文翻译键（同 key，英文值）

### 变更 10：添加单元测试

**文件**：[crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs) 测试模块

**What**：为 `StatelessWithItems` codegen 添加单元测试，覆盖：
- 最小 `<Accordion>` 构造
- 带 `multiple` / `bordered` 静态属性
- 带 `<AccordionItem>` 子节点（含 title 静态属性、子节点）
- 带 `on_toggle_click` 事件
- ref 指令稳定 ID

**Why**：与现有 `gen_component_*` 测试系列保持一致，确保 codegen 行为可验证。

**How**：参考现有 `gen_component_button_minimal` / `gen_component_button_with_label_attr` 等测试风格，新增 `gen_component_accordion_*` 系列测试。

## 假设与决策

### 假设

1. **gpui-component 版本**：使用 workspace 中锁定的 git rev `f2cfc37a601d48ab` 中的 `Accordion` API（已通过源码验证）。若未来 gpui-component 升级 Accordion API，需同步更新本实现。
2. **AccordionItem 不作为顶层组件**：`<AccordionItem>` 仅在 `<Accordion>` 内合法，单独使用会触发 codegen 错误。这符合 AccordionItem 在 gpui-component 中的语义（无独立 `new(id)` 用途）。
3. **`on_toggle_click` 回调签名**：用户命令方法签名约定为 `fn(&mut self, &[usize], &mut Context<Self>)`，与 gpui-component 的 `Fn(&[usize], &mut Window, &mut App)` 对齐（去除 `_window` 参数，与现有 `onclick` → `fn(&ClickEvent, &mut Context<Self>)` 风格一致）。
4. **`title` 属性不调用 `.clone()`**：AccordionItem 的 `title(impl IntoElement)` 接受任意元素，与 Button 的 `label(impl Into<SharedString>)` 不同。`title={expr}` 直接传递表达式，不加 `.clone()`。
5. **`icon` 静态属性解析为 `IconName` 枚举**：`<AccordionItem icon="Settings">` 生成 `.icon(rml_ui::IconName::Settings)`。要求用户提供有效的 `IconName` 变体名（PascalCase）。

### 决策

| 决策点 | 选项 | 决策 | 理由 |
|---|---|---|---|
| 表达闭包式 builder 模式 | A. 新增 `ComponentKind::StatelessWithItems`<br>B. 在 `Stateless` 中硬编码 tag 检查 | **A** | 用户偏好"添加变体到现有枚举"；类型安全；未来可复用 |
| `AccordionItem` 注册方式 | A. 加入 `component_lookup`<br>B. 单独 `is_item_builder_tag` 函数 | **B** | 避免 `AccordionItem` 被误用为顶层组件；语义清晰 |
| `title` 绑定是否 `.clone()` | A. 加 `.clone()`<br>B. 不加 | **B** | `title(impl IntoElement)` 接受任意元素，`.clone()` 不通用 |
| `on_toggle_click` 用户方法签名 | A. `fn(&mut self, &[usize], &mut Context<Self>)`<br>B. `fn(&mut self, Vec<usize>, &mut Context<Self>)` | **A** | 与 gpui-component 原始签名 `&[usize]` 一致；避免不必要的分配 |
| Demo 案例位置 | A. 在现有 `cases/` 目录新增<br>B. 单独 `components/` 目录 | **A** | 与 button_case / slot_case 等组织一致 |

## 验证步骤

### 1. 编译验证

```powershell
# 在项目根目录执行
cargo build -p rust-rml-engine    # 验证编译器改动
cargo build -p rust-rml-ui        # 验证 re-export
cargo build -p rust-rml-demo      # 验证 demo 案例
```

### 2. 单元测试

```powershell
cargo test -p rust-rml-engine --lib
# 重点关注：
# - tags::tests 中 ComponentKind 新变体相关
# - component::tests 中新增的 gen_component_accordion_* 系列
# - props_registry::tests 中 component_props_tags_align_with_routing_table 调整后通过
```

### 3. 集成测试

```powershell
cargo test -p rust-rml-engine --test '*'
```

### 4. Demo 运行验证

```powershell
cargo run -p rust-rml-demo
```

手动验证：
- 案例树"组件"分组下出现"折叠面板"案例
- 点击案例在新 Tab 中打开 accordion_case 视图
- 各示例（基础/多展开/尺寸/图标/嵌套）渲染正确
- 切换折叠项时底部状态文本更新（验证 `on_toggle_click` 事件绑定）
- 嵌套 Accordion 内层独立展开/收起
- 禁用项不可点击
- 中英文切换后所有文本正确翻译

### 5. Codegen 输出验证（可选）

在 `target/debug/build/rust-rml-demo-*/out/rml_generated/accordion_case.rs` 中检查生成的 `impl Render` 代码，确认：
- `rml_ui::Accordion::new(("rml_el", N))` 构造
- `.multiple(true)` / `.bordered(true)` 等 setter
- `.item(|__rml_item: rml_ui::AccordionItem| __rml_item.title(...).child(...))` 闭包式调用
- `.on_toggle_click(cx.listener(...))` 事件包装

## 实施顺序

1. **变更 1**：re-export Accordion（最简单，无依赖）
2. **变更 2**：新增 `ComponentKind::StatelessWithItems`（其他变更依赖此变体）
3. **变更 3 + 4**：注册 `Accordion` + 新增 `is_item_builder_tag`（tags.rs 同文件改动）
4. **变更 7**：props_registry 注册属性（validator 依赖）
5. **变更 8**：validator 扩展（依赖变更 4 + 7）
6. **变更 5 + 6**：codegen 扩展 + setter 映射（component.rs 同文件改动，核心逻辑）
7. **变更 10**：单元测试（与变更 5+6 一起完成）
8. **变更 9**：demo 案例（最后，验证端到端）

## 风险与缓解

| 风险 | 缓解 |
|---|---|
| `StatelessWithItems` 子节点 codegen 逻辑复杂，可能引入 bug | 充分的单元测试覆盖（变更 10） |
| `AccordionItem` 未注册为扩展组件，可能被用户误用为顶层标签 | codegen 中显式报错："仅支持 `<AccordionItem>` 子节点" |
| `icon="Settings"` 静态属性要求用户知道 `IconName` 枚举变体名 | 文档说明 + demo 示例展示常用图标 |
| `on_toggle_click` 回调签名与现有事件不同（`&[usize]` 而非事件结构体） | demo 案例展示用法；setter 注释说明签名 |
| 嵌套 Accordion 的 codegen ID 计数器可能冲突 | 现有 id_counter 已递增，闭包内子节点使用 `gen_node` 复用同一计数器，无冲突 |

## 文件变更清单

| 文件 | 类型 | 行数估计 |
|---|---|---|
| `crates/ui/src/lib.rs` | 修改 | +1 |
| `crates/engine/src/tags.rs` | 修改 | +20 |
| `crates/engine/src/compiler/component.rs` | 修改 | +120（含测试） |
| `crates/engine/src/compiler/props_registry.rs` | 修改 | +5 |
| `crates/engine/src/compiler/validator.rs` | 修改 | +3 |
| `demo/src/cases/accordion_case.rml.rs` | 新增 | ~30 |
| `demo/src/cases/accordion_case.rml` | 新增 | ~60 |
| `demo/src/cases/mod.rs` | 修改 | +2 |
| `demo/src/cases/catalog.rs` | 修改 | +1 |
| `demo/assets/i18n/zh-CN.json` | 修改 | +18 |
| `demo/assets/i18n/en-US.json` | 修改 | +18 |

总计：~280 行变更（含测试与 demo）。
