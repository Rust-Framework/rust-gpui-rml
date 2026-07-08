# RML Engine Translator 重构计划

## 1. 摘要

将 RML engine 中分散、硬编码的 AST → Rust / AST → RML 转译逻辑，收敛到统一的 `IRmlTranslator` 接口与 `TranslatorRegistry` 注册表。每个标签（原生 HTML 标签、扩展组件、用户自定义组件、根节点、`<component>` 透明容器）对应一个 translator 实现；`codegen/node.rs` 退化为注册表路由层，不再包含任何标签专属构造逻辑。为可视化设计器提供标准、可枚举、带元数据的组件注册体系。

## 2. 当前状态分析

已完成的基底：

- `crates/engine/src/compiler/translator/mod.rs` 已定义 `IRmlTranslator` 接口，含 `tag / matches / to_rust / to_rml / metadata`。
- `crates/engine/src/compiler/translator/registry.rs` 已存在 `TranslatorRegistry`，内部使用 `Arc<dyn IRmlTranslator>`，支持 `get / resolve / by_category / metadata`。
- `crates/engine/src/compiler/translator/builtin/` 下已为每个原生 HTML 标签（div、span、p、h1-h6、button、input、textarea、ul、ol、li、img、a、label、br、code）建立独立 translator，并通过 `BuiltinTranslator` 公共引擎复用通用逻辑。
- `crates/engine/src/compiler/printer.rs` 已完全通过 `registry.resolve(elem)` 路由到 `to_rml`。

仍存在的割裂：

- `crates/engine/src/compiler/codegen/node.rs` 的 `gen_element` 仍走旧硬编码分支：先判断 `component / slot / menu / user_components / is_extension_component / tags::lookup(tag)`，最后才用 `tags::lookup` 取 `BuiltinTag::codegen_ctor()` 生成构造器。注册表 `ctx.registry` 未被使用。
- `crates/engine/src/build/mod.rs:309` 构造 `CodegenCtx` 时传入 `TranslatorRegistry::empty()`，导致实际编译流程完全没有加载任何 translator。
- 扩展组件（Button、Input、Tabs 等）仍在 `crates/engine/src/tags.rs` 的 `component_lookup` 硬编码表中，由 `compiler/component.rs` 统一生成，未拆分为独立 translator。
- 用户自定义组件（`#[component]`）和根节点（`<window>` 等）尚未接入注册表。
- `<component>` 透明容器在 `codegen/node.rs` 中特殊处理，未抽象为 translator。

## 3. 目标

1. **标准化接口**：`IRmlTranslator` 是 AST 节点转译的唯一接口；`to_rust` / `to_rml` / `metadata` 覆盖全部标签。
2. **注册表唯一信源**：所有 translator 必须在 `TranslatorRegistry` 注册；`codegen/node.rs` 通过 `ctx.registry.resolve(elem)` 统一路由。
3. **通用逻辑复用**：原生标签共用 `BuiltinTranslator` + `builtin_engine`；扩展组件按类型共用 `StatelessTranslator` / `StatefulTranslator` / `ContainerTranslator` 等公共引擎，避免重复实现 id、CSS、if/show/each、子节点配对等流程。
4. **用户组件内置化**：用户自定义组件通过注册表中的通配 translator（`UserComponentTranslator`）处理，其 `matches` 方法检查 `ctx.user_components`；`<component>` 透明容器同样作为一个 translator 注册。
5. **可视化设计基础**：每个 translator 提供 `metadata`，注册表支持按分类枚举、查询允许子节点、默认属性，为设计器组件面板和属性面板提供数据。
6. **移除旧路径**：完成后删除 `tags.rs` 中的 `BuiltinTag::codegen_ctor` 等 codegen 专用字段、`component_lookup`  eventually 退化为仅用于设计时元数据校验的辅助表。

## 4. 架构设计

### 4.1 核心接口（保持不变，已存在）

```rust
pub trait IRmlTranslator: Send + Sync + Debug {
    fn tag(&self) -> &'static str;
    fn matches(&self, elem: &Element) -> bool { elem.tag == self.tag() }
    fn to_rust(&self, elem: &Element, ctx: &CodegenCtx, id_counter: &mut usize,
               loop_vars: &[String], parents: &[ParentInfo]) -> Result<(String, bool), CodegenError>;
    fn to_rml(&self, elem: &Element, ctx: &PrinterCtx) -> Result<String, PrintError>;
    fn metadata(&self) -> TranslatorMetadata;
}
```

### 4.2 注册表组成

`TranslatorRegistry::builtin()` 注册以下 translator（按职责分组）：

- **原生 HTML**：`div / span / p / h1..h6 / button / input / textarea / ul / ol / li / img / a / label / br / code`
- **扩展组件（无状态）**：`Button / Badge / Label / Separator / Tag / Progress / Spinner / Skeleton / Link / Collapsible / GroupBox / Pagination / Radio / ...`
- **扩展组件（有状态）**：`Input / TextInput / Slider / Switch / Tree / CodeEditor / ...`
- **扩展组件（容器/项 builder）**：`Accordion / AccordionItem / Tabs / Tab / TabBar / Table / Column / DescriptionList / DescriptionItem / Popover / RadioGroup / ...`
- **菜单**：`MenuBar / menu / ContextMenu / DropdownMenu / AppMenuBar / MenuItem / ...`
- **用户组件通配**：`UserComponentTranslator`（匹配 `ctx.user_components.contains_key(&elem.tag)`）
- **透明容器**：`ComponentTranslator`（匹配 `<component>`）
- **插槽**：`SlotTranslator`（匹配 `<slot>`）
- **根节点**：`WindowTranslator / ModernWindowTranslator / TabWindowTranslator / DialogTranslator / ComponentRootTranslator`

### 4.3 通用引擎

已存在 `builtin_engine` 处理原生标签。新增/复用以下引擎：

- `stateless_engine`：处理 `ComponentKind::Stateless / StatelessNoId`，统一生成 `.id(...)`、属性 setter、子节点 `.child/.children`、if/show/each 包装。
- `stateful_engine`：处理 `ComponentKind::Stateful / EntityRef`，统一处理 `ref` → `__rml_state.get_or_init_ref` 或字段 clone、属性 setter、子节点。
- `items_engine`：处理 `StatelessWithItems`，统一生成 `.item(|__rml_item| ...)` 闭包。
- `menu_engine`：将现有 `compiler/menu.rs` 的生成逻辑抽取为引擎。
- `user_component_engine`：将现有 `compiler/component.rs` 中用户组件生成逻辑抽取为引擎，供 `UserComponentTranslator` 调用。
- `root_engine`：将 `compiler/codegen/window.rs`、`shell.rs`、`render.rs` 中的根节点逻辑抽取为引擎。

### 4.4 路由流程

`codegen/node.rs::gen_element` 新流程：

1. 保留全局预处理：`Directive::Once` → `super::once::gen_once_element`；`Directive::Html` → Label 降级。
2. 通过 `ctx.registry.resolve(elem)` 查找 translator。
3. 若命中，调用 `translator.to_rust(elem, ctx, id_counter, loop_vars, parents)` 并返回。
4. 未命中则报错 `unknown tag: <tag>`。
5. 删除原有的 `component / slot / menu / user_components / is_extension_component / tags::lookup` 等硬编码分支。

`PrinterCtx` 已经携带 `registry`，printer 流程保持不变。

## 5. 实施阶段

### Phase 1：原生 HTML 标签走注册表（当前立即实施）

目标：让 `codegen/node.rs` 对原生 HTML 标签使用 `BuiltinTranslator`，并确保实际编译加载注册表。

修改文件：

- `crates/engine/src/compiler/codegen/node.rs`
  - 在 `gen_element` 中，删除 `let builtin = tags::lookup(tag)?` 及后续构造器生成、属性、子节点、if/show/each 处理代码（这些逻辑已迁移到 `builtin_engine::translate`）。
  - 在全局 `Once`/`Html` 预处理之后，直接调用 `ctx.registry.resolve(elem)`，命中则返回 `translator.to_rust(...)`。
  - 保留 `build_parent_info` 函数（CSS 父链仍由 `gen_node_impl` 使用）。
  - 保留 `gen_node` / `gen_node_impl` 的 sourcemap 注入逻辑。
- `crates/engine/src/build/mod.rs`
  - 将 `registry: TranslatorRegistry::empty()` 改为 `registry: crate::compiler::translator::TranslatorRegistry::builtin()`。
- `crates/engine/src/compiler/codegen/mod.rs` 与单元测试
  - 所有单元测试中的 `minimal_ctx()` / `ctx()` 需设置 `registry: TranslatorRegistry::builtin()`，否则 `resolve` 会找不到 translator。

验证：

- `cargo test -p rust-rml-engine --lib codegen::node::tests` 通过。
- `cargo test -p rust-rml-engine --lib translator::builtin` 通过。
- 编译示例项目成功生成 `.rml.rs`。

### Phase 2：扩展组件拆分为 Translator

目标：将 `tags.rs::component_lookup` 中的每个组件转为独立 translator，复用通用引擎。

修改文件：

- 新建 `crates/engine/src/compiler/translator/component/stateless.rs`：定义 `StatelessTranslator` 与 `stateless_engine`，处理 `Stateless / StatelessNoId`。
- 新建 `crates/engine/src/compiler/translator/component/stateful.rs`：定义 `StatefulTranslator` 与 `stateful_engine`，处理 `Stateful / EntityRef`。
- 新建 `crates/engine/src/compiler/translator/component/items.rs`：定义 `ItemsTranslator` 与 `items_engine`，处理 `StatelessWithItems` 及其子项 builder。
- 新建 `crates/engine/src/compiler/translator/component/mod.rs`：注册所有扩展组件 translator。
- `crates/engine/src/compiler/component.rs`：将可复用逻辑迁移到上述 engine；保留对旧路径的最小兼容直到 Phase 6。
- `crates/engine/src/compiler/translator/registry.rs`：`builtin()` 调用 `super::component::register_all(&mut reg)`。

验证：

- 现有扩展组件示例（Button、Input、Tabs、Accordion 等）编译与运行测试通过。
- `cargo test -p rust-rml-engine --lib component` 通过。

### Phase 3：菜单组件 Translator 化

目标：将 `compiler/menu.rs` 接入注册表。

修改文件：

- 新建 `crates/engine/src/compiler/translator/menu/` 目录，按菜单标签拆分 translator（`MenuBarTranslator`、`MenuItemTranslator`、`ContextMenuTranslator` 等）。
- 提取公共 `menu_engine`。
- `crates/engine/src/compiler/translator/registry.rs`：注册菜单 translators。
- `codegen/node.rs`：删除 `if menu::is_menu_container(tag)` 分支。

验证：

- 菜单相关示例运行正常。
- `cargo test -p rust-rml-engine --lib menu` 通过。

### Phase 4：用户组件与 `<component>` 透明容器

目标：让用户组件和 `<component>` 也走注册表。

修改文件：

- 新建 `crates/engine/src/compiler/translator/user.rs`：
  - `UserComponentTranslator`：`matches` 检查 `ctx.user_components.contains_key(&elem.tag)`；`to_rust` 调用 `user_component_engine`。
  - `ComponentTranslator`：处理 `<component content={...} />` 透明容器，复用现有逻辑。
  - `SlotTranslator`：处理 `<slot>`。
- 新建 `crates/engine/src/compiler/translator/user/engine.rs`：将 `compiler/component.rs` / `compiler/user_component.rs` 中用户组件生成逻辑迁移至此。
- `crates/engine/src/compiler/translator/registry.rs`：注册 `UserComponentTranslator`、`ComponentTranslator`、`SlotTranslator`。
- `codegen/node.rs`：删除 `tag == "component"`、`tag == "slot"`、`ctx.user_components.contains_key(tag)`、`tags::is_extension_component(tag)` 等硬编码分支。

验证：

- `cargo test -p rust-rml-engine --lib user_component` 通过。
- 用户组件 slot、once、content 等示例正常。

### Phase 5：根节点 Translator 化

目标：`<window>` / `<modern-window>` / `<tab-window>` / `<dialog>` / `<component>` 根节点由 translator 处理。

修改文件：

- 新建 `crates/engine/src/compiler/translator/root/` 目录，包含 `WindowTranslator`、`ModernWindowTranslator`、`TabWindowTranslator`、`DialogTranslator`、`ComponentRootTranslator`。
- 提取 `root_engine`：将 `compiler/codegen/window.rs`、`shell.rs`、`render.rs` 中的根节点逻辑逐步迁移。
- `crates/engine/src/compiler/codegen/mod.rs`：`codegen()` 函数简化，通过注册表解析根节点 translator 生成 impl。
- `crates/engine/src/compiler/translator/registry.rs`：注册根节点 translators。

验证：

- 窗口、对话框、组件根节点示例编译与运行通过。
- `cargo test -p rust-rml-engine --lib codegen` 通过。

### Phase 6：清理旧路径并启用新路径

目标：删除硬编码残余，使注册表成为唯一路由。

修改文件：

- `crates/engine/src/tags.rs`：
  - 删除 `BuiltinTag::codegen_ctor`、`is_self_closing` 等仅用于 codegen 的字段/方法。
  - `component_lookup` 保留为设计时元数据辅助（供 validator / 设计器），但 codegen 不再查询。
- `crates/engine/src/compiler/component.rs`、`compiler/menu.rs`、`compiler/user_component.rs`：删除已被 engine 取代的旧函数，或整文件删除后合并到 translator 目录。
- `crates/engine/src/compiler/codegen/node.rs`：确认只剩全局预处理 + 注册表路由；删除对 `tags` 模块的 import（`build_parent_info` 需要的 `implicit_class_for` 可移到 css 模块或 translator utils）。
- `crates/engine/src/compiler/validator.rs`：改为从 `TranslatorRegistry` 查询允许子节点与属性，而非 `tags.rs`。

验证：

- `cargo test -p rust-rml-engine --lib` 全绿。
- 框架内所有 demo 案例编译通过。

## 6. 关键设计决策

1. **注册表是否静态**：`TranslatorRegistry::builtin()` 在编译期构建完整静态 translator 集合；用户组件通过通配 translator 在运行时匹配 `ctx.user_components`，兼顾性能与灵活性。
2. **公共引擎 vs. 独立文件**：每个标签独占一个 rs 文件，但通用逻辑集中到 engine；engine 不是 translator，不直接实现 `IRmlTranslator`。
3. **`matches` 默认精确匹配**：需要模糊匹配（用户组件通配、`<component>`、`<slot>`）的实现重载 `matches`；其余按 `tag` 精确匹配，保证路由确定性。
4. **metadata 来源**：原生标签 metadata 来自 `BuiltinMeta`；扩展组件 metadata 来自 `ComponentTag` 的扩展描述；用户组件 metadata 在扫描期生成并注入 `ctx.user_components`。
5. **不维护向下兼容**：旧硬编码分支在 Phase 6 直接删除，不需要 shim 层。

## 7. 验证步骤

每个 Phase 完成后执行：

1. `cargo check -p rust-rml-engine`
2. `cargo test -p rust-rml-engine --lib`
3. 运行框架 demo 案例，确认界面渲染与交互正常
4. 确认 `TranslatorRegistry::builtin().all_tags()` 包含该 Phase 新增的标签

最终验收：

- `codegen/node.rs` 中不存在 `tags::lookup`、`is_extension_component`、`user_components.contains_key` 等硬编码判断。
- `build/mod.rs` 使用 `TranslatorRegistry::builtin()`。
- 新增 translator 均实现 `IRmlTranslator` 并通过 `registry.resolve` 路由。
- `cargo test -p rust-rml-engine --lib` 全绿。
