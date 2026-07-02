# 删除 shell_meta.rs，用 group 实现案例树分组

## Context

`demo/src/shell/shell_meta.rs` 当前定义了两类纯元数据贡献：

1. **案例树分类节点**（`CatBinding`/`CatComponents`/`CatMenu`/`CatI18n`，`kind = "case"`）：作为父节点，各 case 通过 `parent_id = "cat.xxx"` 挂载，`shell_chrome::map_case_tree_items` 按 `parent_id` 构建父子树。
2. **状态栏项**（`StatusReady`，`kind = "status"`）。

但 `ContributionOptions` 已提供 `group` 字段（`crates/core/src/contribution.rs:21`），`#[contribute]` 宏已支持 `group = "..."` 参数（`crates/macros/src/contribute.rs:78`）。`group` 完全可以替代 `parent_id` 实现分类分组效果，无需单独定义分类节点结构体。

**目标**：删除 `shell_meta.rs`，各 case 改用 `group` 分组，`map_case_tree_items` 按 `group` 构建分类树（group 作为虚拟 folder 父节点）。`StatusReady` 迁移到 `cases/` 下新增的 status-bar 演示案例中。

## 决策（已与用户确认）

- group 父节点 id 用 `"group.{group}"`，i18n key 用 `"tree.group.{group}"`（新增 i18n 资源，`open_case` 保留 `starts_with("group.")` 保护）。
- `StatusReady` 放到 `cases/status_bar_case`（新建的 status-bar 演示案例）中，与 `StatusBarCase` 同文件。

## 改动清单

### 1. 删除 `demo/src/shell/shell_meta.rs`

### 2. `demo/src/shell/mod.rs`
- 移除 `pub mod shell_meta;`（其余模块声明不变）。

### 3. 新建 `demo/src/cases/status_bar_case.rml.rs`
```rust
use rml::prelude::*;

#[contribute(
    host_id = "demo.shell",
    id = "components.status_bar",
    name = "case.status_bar.title",
    kind = "case",
    group = "components",
    order = 12,
)]
#[component]
#[derive(Default)]
pub struct StatusBarCase {}

impl ILifecycle for StatusBarCase {}

/// 状态栏贡献：演示 status slot（从 shell_meta.rs 迁入）
#[contribute(host_id = "demo.shell", id = "status.ready", name = "shell.status_ready", kind = "status", order = 0)]
#[derive(Default)]
pub struct StatusReady;
```

### 4. 新建 `demo/src/cases/status_bar_case.rml`
```xml
<component>
    <div v_flex="" class="case-pane">
        <h2>{t("case.status_bar.title")}</h2>
        <p>{t("case.status_bar.hint")}</p>
    </div>
</component>
```

### 5. `demo/src/cases/mod.rs`
- 新增 `#[path = "status_bar_case.rml.rs"] pub mod status_bar_case;`
- 新增 `pub use status_bar_case::StatusBarCase;`

### 6. `demo/src/cases/catalog.rs`
- `case_title_key` 新增 `"components.status_bar" => "case.status_bar.title"`。

### 7. 9 个 case 文件：`parent_id = "cat.xxx"` → `group = "..."`
| 文件 | id | 旧 parent_id | 新 group | order |
|------|-----|-----------|---------|-------|
| counter_case | binding.counter | cat.binding | binding | 1 |
| two_way_case | binding.two-way | cat.binding | binding | 2 |
| button_case | components.button | cat.components | components | 11 |
| menu_context_case | components.menu.context | cat.menu | menu | 16 |
| menu_dropdown_case | components.menu.dropdown | cat.menu | menu | 17 |
| menu_editor_case | components.menu.editor | cat.menu | menu | 18 |
| menu_features_case | components.menu.features | cat.menu | menu | 19 |
| menu_custom_case | components.menu.custom | cat.menu | menu | 20 |
| i18n_case | i18n.basic | cat.i18n | i18n | 21 |

### 8. 重写 `demo/src/shell/shell_chrome.rs` 的 `map_case_tree_items`
按 `group` 分组构建树（替换原 `parent_id` 父子建树逻辑）：
- 读取 `slot = "case"` 的条目，按 `options.group` 分组。
- 每个 `Some(group)` 组作为一个 folder 父节点：id = `format!("group.{}", group)`，name = `rml_core::i18n::t_static(&format!("tree.group.{}", group))`，`expanded(true)`，子节点为该组 case（按 order 排序）。
- group 父节点的顶层排序：按组内最小 case `order` 推导（binding=1 < components=11 < menu=16 < i18n=21），自动得到正确分类顺序。
- `None` group 的零散 case 作为顶层节点。
- 复用现有 `TreeItem::new(id, name).expanded(true).child(...)` API（`crates/ui` re-export 的 gpui-component `TreeItem`）。
- `map_menu_items` / `map_status_items` / `map_shell_chrome` 不变（菜单仍用 `parent_id` 构建多级菜单，状态栏仍按 `slot="status"` + order）。

### 9. `demo/src/shell/main_window.rml.rs`
- `open_case` 的保护：`if case_id.starts_with("cat.")` → `if case_id.starts_with("group.")`（folder 节点不触发 `on_activate`，此为防御性保护）。

### 10. i18n 资源 `demo/assets/i18n/zh-CN.json` + `en-US.json`
- **新增**：
  - `tree.group.binding` / `tree.group.components` / `tree.group.menu` / `tree.group.i18n`（group folder 显示名，复用原 `tree.cat.*` 的译文）
  - `case.status_bar.title`（tab 标题）
  - `case.status_bar.hint`（案例说明文本）
- **删除**：`tree.cat.binding` / `tree.cat.components` / `tree.cat.menu` / `tree.cat.i18n`（不再引用）。
- **保留**：`shell.status_ready`（`StatusReady` 的 name key，继续使用）。

### 11. 文档更新（仅描述性调整）
- `docs/01-overview/developer-guide.md:174`：`map_case_tree_items` 描述补充"按 group 分组"。
- `docs/06-components/reference/tree.md:59-64`：同上。
- 无需大改，仅措辞从"按 parent_id 建树"调整为"按 group 分组"。

## 不变项

- `crates/core/src/contribution.rs`：`ContributionOptions.group` 字段已存在，无需改。
- `crates/macros/src/contribute.rs`：`group` 参数解析已存在，无需改。
- `map_menu_items`：菜单仍用 `parent_id` 构建多级菜单（菜单的父子层级是真实贡献节点，非分类分组，保持不变）。
- `map_activity_panels`（框架 `crates/app`）：与 case 树无关，不变。
- `menu_shell_contribs.rs`：Shell 菜单贡献，不变。

## 验证

1. `cargo build -p rust-rml-demo` —— 编译通过。
2. `cargo run -p rust-rml-demo` —— 启动后验证：
   - 左侧案例树显示 4 个分类 folder（绑定/组件/菜单/国际化），folder 名取自 `tree.group.*` i18n。
   - 各 folder 展开后显示对应 case，顺序正确（binding: 计数器/双向绑定；components: 按钮样式/状态栏；menu: 5 个菜单案例；i18n: t() 插值）。
   - 点击 case 在 tab 打开正确内容；点击 folder 不开 tab。
   - 状态栏显示"就绪 — 请从左侧案例树选择示例"（来自 `StatusReady`）。
   - 点击"状态栏"案例 tab 显示演示内容。
   - 切换语言（en-US）后 folder/case 名正确切换。
3. `cargo test -p rust-rml-core` —— 确保核心 crate 测试不受影响。
