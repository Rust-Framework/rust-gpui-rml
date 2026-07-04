# RML 组件支持规范领域技能 + 代码修复计划

## Summary

本计划基于对 RML 框架 7 个维度的深度审查，产出两部分：
1. **Skill 规范文档**（`.trae/skills/rml-component/`）：统一后续组件开发规范的领域技能
2. **关键代码修复**（B1/B3/B4/B5/B8）：消除审查中发现的范式不一致与功能缺口

**核心设计原则**（用户明确要求）：
- 声明式强制 kebab-case，内部 snake_case，双层层命名模型
- 框架全新开发，**不保留任何兼容性设计**，拒绝补丁式代码
- `size=medium` 表示中等大小（使用 `medium`，不用 `middle`）
- `vertical=true` 表示纵向，**不提供 `horizontal`**（默认横向）
- 单一信源（props_registry.rs），三处同步协议（tags.rs + props_registry.rs + setters.rs）
- 最佳实践优先，架构师视角

## Current State Analysis（基于磁盘实际状态验证）

### 已完成（磁盘验证）
- ✅ **B2 canonical_tag 统一**：`menu/setters.rs`、`description_list/setters.rs`、`component.rs` 均使用 `canonical_tag()` 统一规范化
- ✅ **B3 部分**：`codegen/shell.rs`、`menu/setters.rs`、`description_list/setters.rs`、`component.rs` 已迁移到 kebab-case
- ✅ **B1 部分**：`tokenizer.rs::read_attr_name`（line 279）已拒绝 `_`
- ✅ **tags.rs::canonical_tag**（lines 154-169）：完整实现 kebab-case/小写别名 → PascalCase 映射
- ✅ **props_registry.rs**：已使用 `medium`（不用 `middle`），Tree 已注册 `items`/`on_activate`/`on_select`
- ✅ **B7 vertical**：仅 DescriptionList 实现 `vertical`，无组件误用 `horizontal`（范式正确）

### 未完成（磁盘验证）
- ❌ **Phase 1 Skill 文档**：`.trae/skills/rml-component/` 目录不存在（前轮会话产出丢失）
- ❌ **B1**：`tokenizer.rs::read_tag_name`（line 222）仍接受 `_`
- ❌ **B3 剩余**：14 个文件仍有 `tab_window`/`modern_window`/`tab_bar`/`status_bar` snake_case 引用（doc 注释、测试代码、warning 消息）
- ❌ **B4**：`tree/setters.rs` 仅有 `on_activate`，缺 `items`/`on_select` setter（props_registry 已注册）
- ❌ **B5**：`css/matcher.rs` lines 47-48 对 `Descendant`/`Child` 选择器只匹配末端，`ElementContext` 缺 `parents` 字段
- ❌ **B8**：`table/template.rs` 闭包参数全部前缀 `_`（不可访问），注释明示"需要参数访问时用 TableDelegate trait"
- ⏳ **B6 验证**：items 绑定语义基本统一（menu/StatusBar/DescriptionList 均用 `.items()` 或 `.children()`），仅需 Skill 文档固化
- ⏳ **B7 验证**：vertical 范式正确，仅需 Skill 文档固化

## Proposed Changes

### Phase 1: 创建 Skill 规范文档（9 个文件）

**位置**：`.trae/skills/rml-component/`

#### 1.1 `SKILL.md`（主入口）
- YAML frontmatter：`name: rml-component`、`description`、`when_to_apply`
- Quick Reference：7 维度速查表
- 索引到 01-08 reference 文件
- 核心设计原则（kebab-case 声明式、snake_case 内部、单一信源、不保留兼容性、medium 不用 middle）

#### 1.2 `01-naming-conventions.md`
- 双层命名模型（声明式 kebab-case / 内部 snake_case）
- `normalize_attr_name`、`normalize_component_tag`、`canonical_tag` 桥接机制
- 事件命名双层模型：`on-{event}` → `on_{event}`
- 反模式列表（`<tab_window>`、`size=middle`、`horizontal={true}` 等）
- tag 名等价形式完整表（`tab-bar`/`TabBar`/`tab_bar`[已废弃]）

#### 1.3 `02-component-registration.md`
- 三处同步协议（tags.rs::component_lookup + props_registry.rs + 各组件 setters.rs）
- ComponentKind 枚举（Stateless / StatelessNoId / StatelessWithItems / Stateful / EntityRef）
- 子标签识别（`is_item_builder_tag`）
- 窗口外壳注册（`is_root_tag` / `root_tag_lookup`）
- StatusBar 命名冲突解决（`StatusBar` → rml_ui::StatusBar，`NativeStatusBar` → rml_ui::NativeStatusBar）

#### 1.4 `03-property-classification.md`
- 三大类属性（static / bind / event）
- 三级分类（组件专用 → 通用 COMMON_*_PROPS → 警告丢弃）
- 通用属性表、原生 HTML 事件表
- EventHandler 三种形式（Ident / MethodName / WithArgs）
- 职责边界规则与分类决策树

#### 1.5 `04-data-binding.md`
- 四种绑定形式（children / items={expr} / {each} / model={field}）
- 不支持 v-model（设计选择）
- each 不扩展到扩展容器（用 items={expr} 代替）
- 绑定表达式规范化规则（`component_bind_rust_expr`）

#### 1.6 `05-slot-template.md`
- 基础插槽、Table 专用模板、Scoped slot 协议
- TabWindow 插槽表（left/right/bottom/tabs/menu/title/footer）
- SlotContext enum 定义（B8 实现后回填）
- SlotRenderer trait 签名
- TabItem 模板（WPF TabControl 模式）
- Slot 名称校验

#### 1.7 `06-css-customization.md`
- 支持的选择器（Class/Id/Tag/Universal/Compound/Descendant/Child）
- 父链匹配语义（B5 实现后回填）：ElementContext 扩展 parents 字段
- 支持的 CSS 属性（颜色/长度/简写/字体/布局）
- 主题变量 `var(--name)`
- 限制（不实现完整 Cascade / !important / 伪类 / 媒体查询 / 动画）

#### 1.8 `07-size-layout-conventions.md`
- `size=xsmall|small|medium|large`（**使用 medium，不使用 middle**）
- `vertical=true` 表示纵向，**不提供 horizontal**
- variant 快捷方法（underline/pill/flat/outline/segmented）
- 字体权重快捷方法、布局快捷方法、状态属性
- 综合示例

#### 1.9 `08-new-component-checklist.md`
- 12 项检查清单（命名/注册/属性/setter/接入/绑定/插槽/CSS/size/测试/demo/文档）
- 验证命令汇总
- 5 个常见陷阱（COMPONENT_PROPS key 规范、SHELL_PROPS key 规范、tag 字面量比对、items 绑定散落、vertical 重复实现）

### Phase 2: 代码修复

#### B1: 严格执行 tag 名称 kebab-case（tokenizer.rs）
**文件**：`crates/engine/src/parser/tokenizer.rs`

**变更**：line 222，`read_tag_name` 移除 `_` 接受

```rust
// Before
if c.is_alphanumeric() || c == '-' || c == '_' || c == ':' {

// After
if c.is_alphanumeric() || c == '-' || c == ':' {
```

**理由**：与 `read_attr_name`（line 279）保持一致，强制声明式 tag 名 kebab-case。框架全新开发，不保留 snake_case tag 兼容。

**影响**：所有 `<tab_window>`、`<modern_window>`、`<status_bar>`、`<tab_bar>` 写法将在解析期报错。demo 与测试需同步迁移（B3 覆盖）。

**测试**：在 `tokenizer.rs` 测试模块新增 `read_tag_name_rejects_underscore` 测试。

#### B3: 完成剩余 snake_case tag 迁移（14 个文件）

**剩余文件清单**（基于 Grep 验证）：

1. **`crates/engine/src/compiler/tab_bar/gen.rs`**（4 处）
   - line 384: comment `<tab_bar>` → `<tab-bar>`
   - line 386: test fn `gen_tab_bar_lowercase_tag` → `gen_tab_bar_kebab_tag`
   - line 388: `make_element("tab_bar", ...)` → `make_element("tab-bar", ...)`
   - line 405: `make_element("tab_bar", ...)` → `make_element("tab-bar", ...)`

2. **`crates/engine/src/compiler/codegen/mod.rs`** - doc 注释和错误消息中 `<modern_window>`/`<tab_window>` → kebab-case

3. **`crates/engine/src/compiler/validator.rs`** - 注释中 `modern_window`/`tab_window`/`status_bar` → kebab-case

4. **`crates/engine/src/compiler/codegen/node.rs`** - 注释中 `status_bar` → `status-bar`

5. **`crates/engine/src/compiler/codegen/render.rs`** - 注释中 `modern_window/tab_window` → `modern-window/tab-window`

6. **`crates/engine/src/compiler/codegen/window.rs`** - doc 注释 `<modern_window>`/`<tab_window>` → kebab-case

7. **`crates/engine/src/tags.rs`** - doc 注释 `<modern_window>`/`<tab_window>`/`<tab_bar>` → kebab-case

8. **`crates/engine/src/compiler/props_registry.rs`** - 注释中 `status_bar` → `status-bar`（如有）

9. **`crates/engine/src/compiler/tab_bar/setters.rs`** - 模块 doc 中 `tab_bar` → `tab-bar`（仅注释，不动模块名）

10. **`crates/engine/src/compiler/tab_bar/mod.rs`** - 模块 doc

11. **`crates/engine/src/compiler/tab_bar/tab.rs`** - 模块 doc

12. **`crates/engine/src/compiler/menu/mod.rs`** - 模块 doc

13. **`crates/engine/src/compiler/mod.rs`** - 模块 doc

14. **`demo/src/shell/main_window.rml.rs`** - line 29 注释 `status_bar` → `status-bar`

**注意**：Rust 模块名 `tab_bar::` 保持 snake_case（idiomatic Rust），仅迁移字符串字面量、doc 注释、warning/error 消息。

#### B4: 实现 Tree items/on_select setter
**文件**：`crates/engine/src/compiler/tree/setters.rs`

**当前状态**：仅 `on_activate`。props_registry 已注册 `items`/`on_activate`/`on_select`。

**新增**：
1. `bind_setter` 函数（当前缺失）：
   - `"items"` → `.items(self.<expr>.clone())`（Tree 接收 `Vec<Arc<dyn IValue>>`）
2. `event_setter` 扩展：
   - `"on_select"` → `.on_select_rc(Rc::new(...))`（与 `on_activate` 同模式，但参数为 `Option<&str>` 或 `&TreeItem`）

**签名参考**（与 `on_activate` 一致）：
```rust
pub fn bind_setter(name: &str, expr_str: &str, loop_vars: &[&str], computed: &[&str], _tag: &str) -> Option<String> {
    match name {
        "items" => {
            let rust_expr = super::super::component::component_bind_rust_expr(expr_str, loop_vars, computed);
            Some(format!(".items({}.clone())", rust_expr))
        }
        _ => None,
    }
}
```

**测试**：新增 `bind_setter_items`、`event_setter_on_select` 测试。

#### B5: 实现 CSS 父链匹配
**文件**：
- `crates/engine/src/css/matcher.rs`
- `crates/engine/src/compiler/codegen/node.rs`（传递父链）

**当前缺陷**：`matcher.rs` lines 47-48 对 `Descendant`/`Child` 只匹配末端选择器；`ElementContext` 缺 `parents` 字段。

**变更 1**：`ElementContext` 扩展
```rust
#[derive(Debug, Clone)]
pub struct ElementContext<'a> {
    pub tag: &'a str,
    pub classes: Vec<&'a str>,
    pub id: Option<&'a str>,
    pub parents: Vec<ParentInfo<'a>>,  // 新增
}

#[derive(Debug, Clone)]
pub struct ParentInfo<'a> {
    pub tag: &'a str,
    pub classes: Vec<&'a str>,
    pub id: Option<&'a str>,
}
```

**变更 2**：`matches_selector` 完整实现
```rust
Selector::Descendant(ancestor, descendant) => {
    matches_selector(descendant, ctx)
        && ctx.parents.iter().any(|p| selector_matches_context(ancestor, p))
}
Selector::Child(parent, child) => {
    matches_selector(child, ctx)
        && ctx.parents.last().map(|p| selector_matches_context(parent, p)).unwrap_or(false)
}
```

**变更 3**：`codegen/node.rs` 在调用 `generate_styles` 时构建父链（从 codegen 上下文传递）。

**测试**：新增 `descendant_matches_with_ancestor`、`child_matches_direct_parent_only`、`descendant_no_ancestor_no_match` 测试。

#### B8: Scoped slot 参数支持
**文件**：`crates/engine/src/compiler/table/template.rs`

**当前缺陷**：闭包参数全部前缀 `_`（不可访问），注释明示"需要参数访问时用 TableDelegate trait"。

**设计**：引入 `SlotContext` enum，让模板内容能引用闭包参数：
```rust
enum SlotContext {
    ColIdx,      // col_idx: usize
    Column,      // column: &TableColumn
    RowIdx,      // row_idx: usize
    RowData,     // row_data: &TableRow
    Cx,          // cx: &mut gpui::App
}
```

**变更**：
1. 在 template 内容中识别 `{col_idx}`/`{column.field}`/`{row_idx}`/`{row_data.field}` 占位符
2. 转换为闭包参数引用（去掉 `_` 前缀）
3. 仅 `cell` slot 支持 row 参数；`header` slot 支持 col 参数；`footer` slot 无参数

**简化方案**（首选，避免过度工程）：
- 移除参数前缀 `_`（如 `_row_idx` → `row_idx`）
- 在模板内容中支持 `{row_data.field}` → `row_data.field` 直接引用
- 在模板内容中支持 `{col_idx}` → `col_idx` 直接引用
- 限制：模板内容中的绑定表达式需显式声明所用参数（通过 `slot-params="row_idx,col_idx"` 属性）

**测试**：新增 `gen_template_cell_with_row_data`、`gen_template_header_with_col_idx` 测试。

### Phase 3: Demo 迁移 + 测试补充

#### 3.1 Demo .rml 文件验证
**文件**：22 个 `.rml` 文件（demo/src/**/*.rml）

**操作**：
- Grep 所有 `.rml` 文件，确认无 `tab_window`/`modern_window`/`status_bar`/`tab_bar` snake_case tag
- 如有，迁移到 kebab-case
- 重点验证：`demo/src/shell/main_window.rml`（已迁移 ✅）、`demo/src/cases/status_bar_case.rml`、`demo/src/cases/tab_bar_case.rml`

#### 3.2 测试补充
- `tokenizer.rs`：`read_tag_name_rejects_underscore`
- `tree/setters.rs`：`bind_setter_items`、`event_setter_on_select`
- `css/matcher.rs`：`descendant_matches_with_ancestor`、`child_matches_direct_parent_only`
- `table/template.rs`：`gen_template_cell_with_row_data`、`gen_template_header_with_col_idx`

### Phase 4: 验证

#### 4.1 编译验证
```bash
cargo build -p rust-rml-engine
cargo build -p rml-derive
cargo build  # 全工作区
```

#### 4.2 测试验证
```bash
cargo test -p rust-rml-engine
cargo test  # 全工作区
```

#### 4.3 范式一致性 Grep 验证
```bash
# 确认无 snake_case tag 字面量（模块名除外）
grep -rn "tab_window\|modern_window" crates/engine/src/ --include="*.rs"
grep -rn "tab_bar\|status_bar" crates/engine/src/ --include="*.rs"
# 仅应出现在：Rust 模块名（mod tab_bar）、路径 use crate::compiler::tab_bar::

# 确认 size 不用 middle
grep -rn "middle" crates/engine/src/ --include="*.rs"
# 应无结果

# 确认无 horizontal 属性
grep -rn "\"horizontal\"" crates/engine/src/ --include="*.rs"
# 应无结果
```

#### 4.4 Skill 文档验证
- 确认 `.trae/skills/rml-component/` 下 9 个文件存在
- SKILL.md frontmatter 格式正确
- 所有 reference 文件可被 SKILL.md 索引

## Assumptions & Decisions

### 假设
1. `tree/setters.rs` 的 `on_select` setter 签名与 `on_activate` 一致（`Rc<dyn Fn(TreeItem, ...)>`）
2. `rml_ui::Tree` 组件已有 `.items()` 和 `.on_select_rc()` builder 方法（需在实施时验证）
3. CSS 父链信息可从 codegen 上下文获取（`CodegenCtx` 或 `gen_node` 调用栈）
4. Table 闭包参数去 `_` 前缀后，模板内容中的 `{row_data.field}` 可直接转为 `row_data.field` 引用

### 决策
1. **B1 不保留兼容**：移除 `read_tag_name` 的 `_` 接受，强制 kebab-case。所有 snake_case tag 写法将在解析期报错。
2. **B3 模块名不动**：Rust 模块名 `tab_bar::` 保持 snake_case（idiomatic Rust），仅迁移字符串字面量、doc 注释、warning/error 消息。
3. **B5 父链传递方式**：通过 `ElementContext.parents` 字段传递，由 codegen 在调用 `generate_styles` 时构建。不引入全局 DOM 树。
4. **B8 简化方案**：移除闭包参数 `_` 前缀 + 支持 `{param.field}` 占位符，不引入复杂 SlotContext enum。保持 TableDelegate trait 作为高级场景的逃生通道。
5. **B6/B7 不需代码修改**：仅 Skill 文档固化范式。
6. **Skill 文档优先级**：Phase 1 先于 Phase 2，确保规范先行，代码修复对照规范执行。

## Implementation Order

```
Phase 1 (Skill 文档) → B3 (剩余迁移) → B1 (tag 严格) → B4 (Tree setter) → B5 (CSS 父链) → B8 (Scoped slot) → Phase 3 (Demo+测试) → Phase 4 (验证)
```

**理由**：
- Phase 1 先行：规范文档作为后续修复的对照标准
- B3 先于 B1：B1 会触发解析期报错，需先迁移所有 snake_case tag 写法
- B4/B5/B8 独立：可按任意顺序，但 B5 涉及 codegen 改动较大，放后
- Phase 3 收尾：Demo + 测试补充在代码修复后
- Phase 4 验证：全局编译 + 测试 + Grep 一致性检查

## Verification Steps

1. **Phase 1 完成标志**：`.trae/skills/rml-component/` 下 9 个文件存在，SKILL.md frontmatter 解析正确
2. **B3 完成标志**：`grep -rn "tab_window\|modern_window" crates/engine/src/ --include="*.rs"` 仅返回模块名/路径
3. **B1 完成标志**：`read_tag_name` 拒绝 `_`，新增测试通过
4. **B4 完成标志**：`tree/setters.rs` 实现 `bind_setter` 和 `on_select` event setter，测试通过
5. **B5 完成标志**：`matcher.rs` 父链匹配测试通过，`Descendant`/`Child` 选择器完整匹配
6. **B8 完成标志**：`table/template.rs` 闭包参数可访问，`{row_data.field}` 占位符解析正确
7. **Phase 4 完成标志**：`cargo build` + `cargo test` 全工作区通过，Grep 一致性检查无 snake_case tag 字面量
