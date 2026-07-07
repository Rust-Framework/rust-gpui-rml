# 08 新组件开发检查清单

新增组件时，按以下 13 项检查清单逐项确认，确保范式一致性与架构功能完整性。

## 检查清单

### 1. 命名规范

- [ ] tag 名使用 kebab-case（如 `<my-component>`）
- [ ] 属性名使用 kebab-case（如 `on-click` / `selected-index`）
- [ ] 事件名使用 `on-{event}` 形式（如 `on-click` / `on-change`）
- [ ] Rust 模块名使用 snake_case（如 `mod my_component`）
- [ ] Rust builder 方法使用 snake_case（如 `.on_click()` / `.selected_index()`）

### 2. 组件注册

- [ ] `tags.rs::component_lookup` 注册 PascalCase + kebab-case 形式
- [ ] 选择正确的 `ComponentKind`：
  - `Stateless`：`Component::new(id)`
  - `StatelessNoId`：`Component::new()`（无 ElementId）
  - `StatelessWithItems`：`Component::new(id)` + 子节点闭包/直接 .child()
  - `Stateful`：`Component::new(&self.field)`
  - `EntityRef`：`self.field.as_ref().expect(...).clone()`
- [ ] item builder 子标签在 `is_item_builder_tag` 注册（不注册到 component_lookup）

### 3. 属性注册

- [ ] `props_registry.rs::COMPONENT_PROPS` 注册组件专用属性
- [ ] key 使用 `canonical_tag` 规范化后的 PascalCase 名（如 `"StatusBar"`）
- [ ] 通用属性不重复注册（已在 `COMMON_*_PROPS` 中）

### 4. Setter 实现

- [ ] 创建 `setters.rs` 实现 `static_setter` / `bind_setter` / `event_setter`
- [ ] 内部使用 `canonical_tag(tag)` 比对，不用裸 tag 字符串
- [ ] 在 `component.rs::component_static_setter` / `component_bind_setter` / `component_event_setter` 中委托调用

### 5. 接入 codegen

- [ ] `StatelessWithItems` 组件在 `component.rs::gen_component` 的 `StatelessWithItems` 分支添加委托
- [ ] 特殊构造器组件（如 Tree/CodeEditor）在 `Stateful` 分支添加 `canonical_tag` 判断委托

### 6. 数据绑定

- [ ] 集合数据用 `items={expr}` 绑定，不用 `each` 扩展到容器本身
- [ ] `each` 仅用于原生 HTML 元素或 item builder 子标签
- [ ] `model={field}` 仅限 Stateful 组件

### 7. 插槽

- [ ] 容器组件支持 `<template slot="name">` 时，在 `partition_slot_children` 添加 slot 名
- [ ] validator 校验 slot 名合法性
- [ ] Table 类组件用 `gen_template` 生成闭包

### 8. CSS

- [ ] 组件支持 `class` 属性匹配
- [ ] 父链匹配通过 `ElementContext.parents` 传递
- [ ] 主题变量用 `var(--name)`

### 9. 尺寸布局

- [ ] `size` 用 `medium` 不用 `middle`
- [ ] `vertical=true` 表示纵向，不提供 `horizontal`
- [ ] variant 快捷方法省略值（`primary` 等价于 `primary="true"`）

### 10. 测试

- [ ] `setters.rs` 内联单元测试（static/bind/event 三类）
- [ ] `gen.rs` 端到端测试（通过 `gen_component` 入口）
- [ ] 小写/kebab-case tag 形式测试

### 11. Demo

- [ ] 在 `demo/src/cases/` 添加 demo case
- [ ] demo `.rml` 文件使用 kebab-case tag/attr
- [ ] demo case 注册到 WorkbenchManager

### 12. 文档

- [ ] 更新 `props_registry.rs` 注释
- [ ] 更新本 Skill 文档（如新增维度）
- [ ] 更新 `tags.rs` 注释

### 13. Sourcemap 与调试支持

**参考**：[10-sourcemap-and-debug-support.md](10-sourcemap-and-debug-support.md)

- [ ] 组件 codegen 通过 `gen_node`/`gen_node_impl` 递归处理子节点（不绕过，否则子元素无 sourcemap 标记）
- [ ] codegen 报错路径透传 `elem.span` 到 `CodegenError.span`（不丢失源码位置）
- [ ] 新增 AST 节点类型时，携带 `span: Span` 字段并在 `gen_node_impl` 添加标记注入分支
- [ ] `compile()` 返回的 `source_map.entries` 包含新组件对应 AST 节点的 span
- [ ] 直接调用 `gen_component` 等子函数的单元测试，使用 `strip_sourcemap_markers` 清理 code 后再断言
- [ ] 生成的 `.rml.rs` 文件不包含 `__rml_sm:` 字符串（标记已被 postprocess_sourcemap 删除）

## 验证命令

```bash
# 编译
cargo build -p rust-rml-engine

# 测试
cargo test -p rust-rml-engine

# sourcemap 端到端测试
cargo test -p rust-rml-engine --test sourcemap_e2e_test

# 验证生成的代码无 sourcemap 标记残留
cargo build -p rust-rml-demo && grep -rn "__rml_sm:" target/debug/build/ --include="*.rml.rs"
# 应无结果

# 范式一致性
grep -rn "tab_window\|modern_window" crates/engine/src/ --include="*.rs"
# 仅应返回模块名/路径

grep -rn "middle" crates/engine/src/ --include="*.rs"
# 应无结果

grep -rn "\"horizontal\"" crates/engine/src/ --include="*.rs"
# 应无结果

# props_registry 完整性
cargo test -p rust-rml-engine --test props_registry_complete
```

## 常见陷阱

### 1. COMPONENT_PROPS key 规范

**陷阱**：key 用 `status_bar`（snake_case）而非 `StatusBar`（PascalCase）

**正确**：使用 `canonical_tag` 规范化后的 PascalCase 名

```rust
// ❌ 错误
("status_bar", &["items"]),

// ✅ 正确
("StatusBar", &["items"]),
```

### 2. SHELL_PROPS key 规范

**陷阱**：key 用 `modern_window`（snake_case）而非 `modern-window`（kebab-case）

**正确**：使用 kebab-case tag 名作为 key

```rust
// ❌ 错误
("modern_window", &["menu", "footer"]),

// ✅ 正确
("modern-window", &["menu", "footer"]),
```

### 3. tag 字面量比对

**陷阱**：用裸 tag 字符串比对，导致多形式漏洞

**正确**：用 `canonical_tag(tag)` 统一规范化后比对

```rust
// ❌ 错误
if tag == "StatusBar" { ... }

// ✅ 正确
if tags::canonical_tag(tag) == "StatusBar" { ... }
```

### 4. items 绑定散落

**陷阱**：items 绑定逻辑分散在多个文件，未通过 setters.rs 统一

**正确**：items 绑定在组件 `setters.rs::bind_setter` 中统一处理

### 5. vertical 重复实现

**陷阱**：同时提供 `vertical` 和 `horizontal` 属性

**正确**：仅提供 `vertical=true`，默认横向，不提供 `horizontal`

```rust
// ❌ 错误
"vertical" => ...,
"horizontal" => ...,

// ✅ 正确
"vertical" => ...,
// 不提供 horizontal
```

### 6. 绕过 gen_node_impl 处理子节点（破坏 sourcemap）

**陷阱**：组件 codegen 函数直接调用 `gen_element` 处理子节点，绕过 `gen_node_impl`

**后果**：子元素无 sourcemap 标记，调试器无法映射到该子元素，破坏 `.rml` 调试能力

**正确**：通过 `gen_node`（公共入口）处理子节点，让标记注入逻辑统一生效

```rust
// ❌ 错误：绕过 gen_node_impl
for child in &elem.children {
    if let Node::Element(child_elem) = child {
        let (code, _) = gen_element(child_elem, ctx, depth, id_counter, loop_vars, parents)?;
        // code 无 sourcemap 标记
    }
}

// ✅ 正确：通过 gen_node
for child in &elem.children {
    let (code, _) = gen_node(child, ctx, depth, id_counter, loop_vars)?;
    // code 携带 /*__rml_sm:S:E*/ 标记，postprocess_sourcemap 会记录并删除
}
```

### 7. CodegenError 丢失 span

**陷阱**：codegen 报错时构造 `CodegenError { message, span: None }`

**后果**：build.rs / LSP 无法定位错误到 `.rml` 具体行号

**正确**：透传 `elem.span`（或 `attr.span` / `directive.span`）

```rust
// ❌ 错误：丢失 span
return Err(CodegenError {
    message: format!("unknown tag: <{}>", tag),
    span: None,
});

// ✅ 正确：透传 elem.span
return Err(CodegenError {
    message: format!("unknown tag: <{}>", tag),
    span: Some(elem.span),
});
```
