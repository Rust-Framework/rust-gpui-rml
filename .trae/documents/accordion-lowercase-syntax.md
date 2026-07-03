# Accordion 小写标签 + 短子标签语法支持

## Context

用户要求 RML 语法支持小写模式匹配（`accordion` → `Accordion`）和上下文敏感的短子标签（`<item>` → `AccordionItem`），目标语法：

```rml
<accordion bordered="">
  <item title="Section 1" open="">
    <p>Content</p>
  </item>
</accordion>
```

当前 RML 仅支持 PascalCase `<Accordion>` / `<AccordionItem>` 和 kebab-case `<accordion-item>`。需要新增小写别名和短标签，同时保持全部已有写法向后兼容。同步更新 demo 案例和 docs 文档，提供详细教学。

## 设计方案

### 别名解析机制：`canonical_tag()` 函数

在 `tags.rs` 新增 `canonical_tag(tag: &str) -> String`，在 `normalize_component_tag`（kebab-case → PascalCase）基础上额外处理小写别名：`accordion` → `Accordion`、`item` → `AccordionItem`。供 `props_registry` 查询使用，**避免在 `COMPONENT_PROPS` 中重复登记**。

- `<accordion>` 注册到 `component_lookup`（复用 `"menu"` / `"MenuBar"` 双注册模式）→ 自动被 `is_special_lowercase_component` / `is_extension_component` 识别 → 走现有 `gen_component` → `gen_accordion` 调度链
- `<item>` 扩展 `is_item_builder_tag` 识别（不注册到 `component_lookup`，防止被误用为顶层组件）
- Codegen 路径无需改动：`gen_accordion` 不检查父标签，`gen_item_builder` 不检查子标签

## 变更清单

### 1. `crates/engine/src/tags.rs`

- **`component_lookup`**（~L372）：`"Accordion"` 改为 `"Accordion" | "accordion"` 双匹配臂
- **新增 `canonical_tag(tag)` 函数**（~L147，`normalize_component_tag` 之后）：
  ```rust
  pub fn canonical_tag(tag: &str) -> String {
      let normalized = normalize_component_tag(tag);
      match normalized.as_str() {
          "accordion" => "Accordion".to_string(),
          "item" => "AccordionItem".to_string(),
          _ => normalized,
      }
  }
  ```
- **`is_item_builder_tag`**（L385-387）：扩展为匹配 `"AccordionItem" | "item"` + `normalize_component_tag(tag) == "AccordionItem"`（kebab 回退）
- 更新 `is_special_lowercase_component` 文档注释加入 `accordion`

### 2. `crates/engine/src/compiler/props_registry.rs`

- **`is_prop_registered`**（L125-144）：用 `canonical_tag(tag)` 替换两步查找（raw + normalize），一行搞定
- **`props_for`**（L88-101）：同上替换
- `COMPONENT_PROPS` 数据不变（仍为 PascalCase 条目）
- 更新文档注释提及 `canonical_tag` 和小写别名

### 3. `crates/engine/src/compiler/accordion/gen.rs`

- **错误消息**（L82-85）：`"<accordion> 仅支持 <item> 或 <AccordionItem> 子节点，得到 <{}>"`
- **测试断言**（L268）：同步更新

### 4. `crates/lsp/src/features/source.rs`

- **`EXTENSION_TAGS`**（L78）：添加 `"accordion"` 和 `"item"`

### 5. 测试新增

**tags.rs**（`normalize_tests` 模块或新建 `canonical_tests`）：
- `canonical_tag` 映射小写别名、透传 PascalCase/kebab、保留 `menu`/`status_bar`
- `is_item_builder_tag` 匹配所有形式（`AccordionItem` / `item` / `accordion-item`），拒绝 `Accordion` / `div`
- `component_lookup("accordion")` 返回 `StatelessWithItems`

**props_registry.rs**：
- `is_prop_registered("accordion", "multiple"/"bordered"/"on_toggle_click")` = true
- `is_prop_registered("item", "title"/"open"/"icon")` = true
- `props_for("accordion")` 和 `props_for("item")` 返回正确属性列表

**accordion/gen.rs**：
- `gen_accordion_lowercase_tag`：`make_element("accordion", ...)` → 生成 `rml_ui::Accordion::new`
- `gen_accordion_with_item_short_form`：子元素 tag `"item"` + title 属性 → 生成 `.item(...)` + `.title(...)`
- 通过 `gen_component` 入口验证 `<accordion>` 调度路径

### 6. Demo 改写

**`demo/src/cases/accordion_case.rml`**：全部 5 个示例改用 `<accordion>` / `<item>` 小写语法（`accordion_case.rml.rs` 不变）

### 7. 文档

**新建 `docs/06-components/reference/accordion.md`**（详细教学）：
- 标签别名表（5 种形式 + 推荐度）
- 容器属性表（`multiple` / `bordered` / `on_toggle_click` + 通用 `small` / `large` / `disabled`）
- 子项属性表（`title` / `open` / `icon` + 通用 `disabled`）
- 事件签名（`on_toggle_click` → `fn(&mut self, open_ixs: &[usize], cx)`）
- 完整示例（basic / multiple / sizes / icon / nested，全部小写语法）
- Codegen 说明（`.item(|__rml_item: rml_ui::AccordionItem| ...)` 闭包 builder）
- 常见错误（`<item>` 必须在 `<accordion>` 内）

**更新**：
- `docs/06-components/reference/INDEX.md` — 数据/导航区加 accordion 行
- `docs/06-components/builtin-components.md` — 组件列表加 accordion
- `docs/06-components/reference/props-mapping.md` — 专用属性表 + `canonical_tag` 说明
- `docs/02-syntax/tags-mapping.md` §2.2.9 — 别名表 + `canonical_tag` 规则

## 验证

1. `cargo test -p rust-rml-engine --lib` — 全部已有测试 + 新增测试通过
2. `cargo build -p rust-rml-demo` — demo 用小写标签编译成功
3. 向后兼容：`<Accordion>` / `<AccordionItem>` / `<accordion-item>` 仍正常工作
4. 混合写法：`<accordion><AccordionItem/></accordion>` 正常工作
5. `<item>` 在 `<accordion>` 外 → codegen "unknown tag" 错误（与现有 `<AccordionItem>` 行为一致）
