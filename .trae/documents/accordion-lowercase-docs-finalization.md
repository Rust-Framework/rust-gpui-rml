# Accordion 小写语法文档同步与最终验证

## Context

前序会话已完成 RML 小写标签匹配的全部引擎改造（`canonical_tag()` 别名解析、`component_lookup` 双注册、`is_item_builder_tag` 扩展、props_registry 统一查询、LSP `EXTENSION_TAGS` 补全、demo `accordion_case.rml` 全量改写为 `<accordion>`/`<item>` 小写语法），并验证 293 个测试通过（含 10 个新增 accordion 测试），仅 1 个与本次工作无关的预存 Avatar 测试失败。

本计划聚焦**剩余两项收尾工作**：
1. 同步 docs 文档（新建 accordion.md 详细教学 + 更新 4 处现有文档）
2. 运行 `cargo build -p rust-rml-demo` 验证小写标签 demo 编译通过

## 当前状态确认（Phase 1 探索结果）

### 引擎代码（已完成，无需改动）
- [crates/engine/src/tags.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/tags.rs#L148-L161) — `canonical_tag()` 函数就位，`component_lookup` 含 `"Accordion" | "accordion"` 双匹配臂，`is_item_builder_tag` 接受 `"item"`
- [crates/engine/src/compiler/props_registry.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L93-L116) — `props_for` / `is_prop_registered` 已用 `canonical_tag` 统一查询
- [crates/engine/src/compiler/accordion/gen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/accordion/gen.rs#L82-L85) — 错误消息已更新为 `"<accordion> 仅支持 <item> 或 <AccordionItem> 子节点"`
- [crates/lsp/src/features/source.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/lsp/src/features/source.rs) — `EXTENSION_TAGS` 已含 `"accordion"` / `"item"`

### Demo（已完成，无需改动）
- [demo/src/cases/accordion_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml) — 5 个示例（basic/multiple/sizes/icon/nested）全部使用 `<accordion>`/`<item>` 小写语法

### 文档现状（待补全）
- `docs/06-components/reference/accordion.md` — **不存在**，需新建
- `docs/06-components/reference/INDEX.md` — 数据/导航区无 accordion 行
- `docs/06-components/builtin-components.md` — 6.1.2 扩展轨组件一览无 accordion
- `docs/06-components/reference/props-mapping.md` — 组件专用属性表无 Accordion/AccordionItem，未说明 `canonical_tag`
- `docs/02-syntax/tags-mapping.md` §2.2.9 — 别名表无 `accordion`/`item`，未说明 `canonical_tag` 规则

## 变更清单

### 1. 新建 `docs/06-components/reference/accordion.md`（详细教学）

参照 [avatar.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/06-components/reference/avatar.md) 文档结构（概述 → 基本用法 → 属性表 → 事件 → 子节点 → 完整示例 → 常见错误 → 相关组件 → RML 未覆盖 API），内容覆盖：

**章节结构**：
1. **概述** — 路由到 `rml_ui::Accordion`，`StatelessWithItems` 闭包式 builder 组件；说明小写语法为推荐写法
2. **标签别名表** — 5 种写法 + 推荐度（`accordion` 推荐 / `Accordion` 兼容 / `item` 推荐 / `AccordionItem` 兼容 / `accordion-item` kebab 兼容）
3. **基本用法** — 最小示例 `<accordion bordered=""><item title="...">...</item></accordion>`
4. **容器属性表** — `multiple`（布尔，多选模式）/ `bordered`（布尔，边框）/ `on_toggle_click`（事件）+ 通用 `small`/`large`/`disabled`
5. **子项属性表** — `title`（字符串/绑定）/ `open`（布尔，初始展开）/ `icon`（`IconName` 枚举名，如 `Settings`/`Bell`）+ 通用 `disabled`
6. **事件签名** — `on_toggle_click` → `fn(&mut self, open_ixs: &[usize], &mut Window, &mut Context<Self>)`，说明 `open_ixs` 为当前展开项索引列表
7. **多选与单选** — `multiple=""` 启用多选；默认单选（展开一项自动收起其他）
8. **尺寸** — `small` / `large` 走 Sizable 通用属性
9. **图标** — `icon="Settings"` 生成 `.icon(rml_ui::IconName::Settings)`
10. **嵌套** — `<item>` 内可嵌套 `<accordion>`（demo nested 示例）
11. **Codegen 说明** — 展示生成的闭包 builder 代码：
    ```rust
    rml_ui::Accordion::new(("rml_el", 0usize))
        .bordered(true)
        .item(|__rml_item: rml_ui::AccordionItem| {
            __rml_item.title("Section 1").child("Content")
        })
    ```
12. **完整示例** — 引用 demo 的 5 个示例（basic/multiple/sizes/icon/nested），全部小写语法
13. **常见错误** —
    - `<item>` 必须在 `<accordion>` 内（顶层使用报 "unknown tag"）
    - `<accordion>` 不支持文本子节点（warning + 忽略）
    - `icon` 值必须为合法 `IconName` 枚举名（否则 Rust 编译失败）
    - `on_toggle_click` 仅在 `<accordion>` 上有效，`<item>` 上无效
14. **相关组件** — 链接到 INDEX、props-mapping、tags-mapping §2.2.9
15. **RML 未覆盖的 API** — gpui-component Accordion 的高级 API（自定义渲染、动态控制展开状态等）需 Rust code-behind 手写

### 2. 更新 `docs/06-components/reference/INDEX.md`

在「数据 / 导航（Data / Navigation）」表格末尾新增一行：

```markdown
| [accordion.md](./accordion.md) | `accordion` / `Accordion` | StatelessWithItems（闭包 builder） |
```

### 3. 更新 `docs/06-components/builtin-components.md`

在「数据 / 导航」表格（L65-72）末尾新增一行：

```markdown
| `accordion` / `Accordion` | [accordion.md](./reference/accordion.md) |
```

### 4. 更新 `docs/06-components/reference/props-mapping.md`

**4a. 组件专用属性表**（L69-76）新增两行：

```markdown
| `Accordion` | `multiple`, `bordered`, `on_toggle_click` | 多选/边框/切换事件 |
| `AccordionItem` | `title`, `open`, `icon` | 子项标题/初始展开/图标 |
```

**4b. Tag 规范化章节**（L13-15）追加 `canonical_tag` 说明：

```markdown
### Tag 规范化

`is_prop_registered(tag, attr)` / `is_shell_prop_registered(tag, attr)` 查询时通过 `canonical_tag()` 规范化标签：
- kebab-case → PascalCase（如 `menu-bar` → `MenuBar`、`status_bar` → `StatusBar`）
- 小写别名 → PascalCase（如 `accordion` → `Accordion`、`item` → `AccordionItem`）

因此在 `COMPONENT_PROPS` 中登记的 tag 用 PascalCase 即可，`<accordion>` / `<item>` / `<accordion-item>` / `<Accordion>` / `<AccordionItem>` 五种写法都能命中同一注册条目，无需重复登记。
```

### 5. 更新 `docs/02-syntax/tags-mapping.md` §2.2.9

**5a. 别名表**（L233-241）新增两行：

```markdown
| `accordion` | `Accordion`（小写别名，非 kebab） |
| `item` | `AccordionItem`（仅 `<accordion>` 内上下文敏感短标签） |
```

**5b. 规则列表**（L243-248）新增第 5 条：

```markdown
5. `canonical_tag()` 在 `normalize_component_tag` 基础上额外处理小写别名：`accordion` → `Accordion`、`item` → `AccordionItem`。供 `props_registry` 属性查询使用，避免在 `COMPONENT_PROPS` 中重复登记
6. `<item>` 短标签仅在 `<accordion>` / `<Accordion>` 父容器内被识别为 `AccordionItem`（由 `is_item_builder_tag` 判断）；顶层使用 `<item>` 报 "unknown tag" 错误
```

**5c. 章节标题**改为「扩展组件 kebab-case 与小写别名规范」以反映新增的小写模式。

### 6. 最终验证

运行 `cargo build -p rust-rml-demo` 确认 demo 用小写标签编译成功（PowerShell 环境需用 `cmd /c "set PATH=%USERPROFILE%\.cargo\bin;%PATH% && cargo build -p rust-rml-demo"`）。

## 验证步骤

1. 新建 `accordion.md` 后，检查所有内部 markdown 链接指向有效路径
2. 更新 4 处现有文档后，grep 确认无遗漏的 accordion 引用
3. `cargo build -p rust-rml-demo` 成功（无新增错误，已有的 counter_case warning 可忽略）
4. 文档教学完整性：用户读 `accordion.md` 能独立完成 accordion 组件开发（声明、属性、事件、嵌套、Codegen 理解）

## 假设与决策

- **假设**：引擎代码与 demo 改写已完成且测试通过（前序会话验证 293 测试通过），本计划不再改动引擎代码
- **假设**：`accordion_case.rml.rs` code-behind 文件无需改动（前序会话确认）
- **决策**：文档以小写 `<accordion>`/`<item>` 为推荐写法，PascalCase/kebab 写法标注为"兼容"
- **决策**：`accordion.md` 参照 `avatar.md` 结构（最接近的 StatelessNoId 参考文档），但增加 Codegen 章节以解释闭包 builder 模式（Accordion 独有）
- **决策**：不创建额外的新示例文件，完整示例直接引用 demo 的 `accordion_case.rml`
