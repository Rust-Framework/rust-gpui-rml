# Tabs / TabBar 文档与示范迁移计划

## 背景

前置会话已完成 **阶段 1（源码改造）** 与 **阶段 2（编译器改造）**：

- `crates/ui/src/components/tab/` 已拆分为 `tabs.rs`（WPF TabControl）、`tab_bar.rs`（原生 header-only）、`tab.rs`、`tab_item.rs`
- `crates/engine/src/compiler/` 已拆分为 `tabs/`（TabControl codegen）与 `tab_bar/`（原生 codegen）
- 标签路由、props_registry、component.rs 已注册 `Tabs` 与 `TabBar` 两个独立组件
- `rust-rml-ui` 与 `rust-rml-engine` 两个 crate 均编译通过

本计划覆盖剩余三个阶段：**文档（阶段 3）**、**示范迁移（阶段 4）**、**全量验证（阶段 5）**。

## 现状分析

### 文档现状

| 文件 | 状态 | 问题 |
|------|------|------|
| `docs/06-components/reference/tab-bar.md` | **过期** | 描述旧架构（`<tab-item>` + header/body 混合），引用 `tab_bar.rs:634-642` 等已不存在的行号，未提及 `<Tabs>` |
| `docs/06-components/reference/tabs.md` | **缺失** | 新 Tabs 组件无文档 |
| `docs/06-components/reference/INDEX.md` 第 39 行 | **过期** | 单行条目 `tab-bar.md \| TabBar / Tab / tab-item`，未拆分 Tabs/TabBar，仍引用已废弃的 `tab-item` |

### Demo 现状

| 文件 | 使用模式 | 迁移需求 |
|------|----------|----------|
| `demo/src/cases/tab_bar_case.rml` | 混合：前 7 节 header-only（`<TabBar>`），后 2 节有 body（WPF TabControl 模式） | body 节改 `<Tabs>`，header-only 节保持 `<TabBar>` |
| `demo/src/cases/tab_preview_case.rml` | 全部使用 `on-close`/`on-close-all`/`on-close-others`/`on-promote`（Tabs 专属） + body | 全部改 `<Tabs>` |
| 其余 41 个 demo 文件 | "示例代码"模式：`<TabBar>` + `<Tab>` 包裹 CodeEditor（有 body） | 暂不迁移（codegen 允许 `<TabBar>` 接受 body 子节点，通过内部 Tabs 委托渲染，不会编译报错） |

**关键设计决策**：`tab_bar/gen.rs` 复用 `tabs::tab::gen_tab_child` 生成 `TabItem::new()...`，不拒绝 body 子节点。这提供了平滑迁移路径——旧 `<TabBar>` 含 body 的代码仍可编译运行，语义上应逐步迁移到 `<Tabs>`。

## 阶段 3：文档

### 3.1 重写 `docs/06-components/reference/tab-bar.md`

**目标**：反映新原生 TabBar（header-only，委托内部 Tabs 渲染）。

**结构**：
1. 概述：原生 gpui-component 形态标签栏，纯 header，无 body/close/promote/bordered
2. 与 Tabs 的关系：TabBar 委托 Tabs 渲染，当所有 TabItem 的 body=None 时 Tabs 自动退化为 header-only
3. 基本用法：`<TabBar selected-index={...} on-click={...}><Tab label="..." /></TabBar>`
4. 5 种 variant：underline/pill/flat/outline/segmented
5. 尺寸：size 属性（xsmall/small/large）
6. 图标、禁用、menu 模式、prefix/suffix
7. TabBar 属性表（仅 header 相关：selected_index/on_click/variant/size/menu/prefix/suffix/last_empty_space/track_scroll）
8. Tab 子项属性表（label/icon/disabled/selected/closable/on_click + header 插槽）
9. 与 Tabs 的选择指南

**删除内容**：旧的 `<tab-item>` 章节、WPF TabControl body 模式、`tab_bar.rs:634-642` 等过期行号引用、溢出压缩实现细节（移到 tabs.md）

### 3.2 创建 `docs/06-components/reference/tabs.md`

**目标**：WPF TabControl 风格文档，header + body 一体化切换。

**结构**：
1. 概述：WPF TabControl 风格，header + body 切换，bordered 包裹整体
2. 与 TabBar 的关系：Tabs 是完整 TabControl，TabBar 是其 header-only 子集
3. 基本用法（body 模式）：`<Tabs selected-index={...} on-click={...}><Tab label="..."><div>body</div></Tab></Tabs>`
4. bordered 属性：1px 边框包裹 header + body 整体（v_flex 外层）
5. 5 种 variant
6. 关闭按钮与事件：closable + on-close/on-close-all/on-close-others
7. 预览模式与 promote：preview + on-promote（双击触发）
8. 右键菜单：框架内置 Close/Close All/Close Others（i18n 文本 rml.tab.close/close_all/close_others）
9. 溢出压缩（自适应紧凑模式）：滚动模式 ↔ 压缩模式
10. Tabs 属性表（全量：含 bordered/on_close*/on_promote）
11. Tab 子项属性表（同 TabBar 的 Tab，增加 body 子节点）
12. 运行时渲染结构（v_flex > [header, body]）
13. 完整示例

### 3.3 更新 `docs/06-components/reference/INDEX.md`

**第 39 行替换为两行**：

```markdown
| [tabs.md](./tabs.md) | `Tabs` / `Tab` | Stateless（WPF TabControl：header + body） |
| [tab-bar.md](./tab-bar.md) | `TabBar` / `Tab` | Stateless（原生 header-only 标签栏） |
```

**删除** `tab-item` 引用（已废弃，统一用 `<Tab>`）。

## 阶段 4：示范迁移

### 4.1 迁移 `demo/src/cases/tab_bar_case.rml`

**策略**：拆分为 TabBar（header-only）与 Tabs（body）两种用法演示。

| 节 | 当前 | 迁移后 | 说明 |
|----|------|--------|------|
| 基础用法（L11-15） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| 5 种 variant（L20-39） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| 尺寸（L44-55） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| 图标（L60-64） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| 禁用（L69-72） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| menu 模式（L77-84） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| header 插槽（L90-102） | `<TabBar>` | `<TabBar>` | header-only，保持 |
| **内容面板 body（L108-124）** | `<TabBar>` | **`<Tabs>`** | WPF TabControl 模式，改 Tabs |
| **示例代码切换（L130-137）** | `<TabBar>` | **`<Tabs>`** | CodeEditor body，改 Tabs |

**新增一节**：在 body 节之前插入 `<Tabs bordered>` 演示，展示 bordered 包裹 header + body 效果。

### 4.2 迁移 `demo/src/cases/tab_bar_case.rml.rs`

- 更新 `rml_sample` 字符串（示例代码）：`<TabBar>` → `<Tabs>` 对应 body 节
- API 表格 `tab_bar_api_rows`：保持（TabBar 属性子集）
- **新增** `tabs_api_columns`/`tabs_api_rows` 字段：Tabs 全量属性（含 bordered/on_close*/on_promote）
- `.rml` 中 API 卡片增加 `<h3>Tabs</h3>` + Tabs API 表格
- 顶部描述文字更新：说明 TabBar（header-only）与 Tabs（WPF TabControl）的分工

### 4.3 迁移 `demo/src/cases/tab_preview_case.rml`

**全部 `<TabBar>` → `<Tabs>`**（使用 on-close/on-close-all/on-close-others/on-promote，Tabs 专属）：
- L15 `<TabBar>` → `<Tabs>`（演示区，含关闭/promote 事件）
- L31 `<TabBar>` → `<Tabs>`（代码示例区，CodeEditor body）

### 4.4 迁移 `demo/src/cases/tab_preview_case.rml.rs`

- `rml_sample` 字符串中 `<TabBar>` → `<Tabs>`
- API 表格标签从 "TabBar" 改为 "Tabs"
- 字段名 `tab_bar_api_columns`/`tab_bar_api_rows` → `tabs_api_columns`/`tabs_api_rows`（保持一致性）

## 阶段 5：全量验证

### 5.1 编译验证

```bash
cargo build -p rust-rml-engine    # 引擎编译
cargo build -p rust-rml-ui        # UI 编译
cargo build                       # 全工作区编译（含 demo）
```

### 5.2 测试验证

```bash
cargo test -p rust-rml-engine     # 引擎单测（含 tabs/tab_bar codegen 测试）
cargo test -p rust-rml-ui         # UI 单测
cargo test                        # 全量测试
```

### 5.3 验证点

- 引擎 codegen 测试：`tabs/setters.rs`、`tab_bar/setters.rs`、`tab_bar/gen.rs`、`tags.rs`、`props_registry.rs` 全部通过
- UI 编译：`Tabs`/`TabBar`/`Tab`/`TabItem` 类型导出正确
- Demo 编译：迁移后的 `tab_bar_case` 与 `tab_preview_case` 编译通过
- 无回归：原有测试不因迁移失败

## 假设与决策

1. **不迁移剩余 41 个 demo 文件**：用户明确要求"框架 + 示范迁移"，非全量迁移。codegen 允许 `<TabBar>` 接受 body 子节点（通过内部 Tabs 委托），旧代码不会编译报错。
2. **不重命名 `tab_bar_case.rml`**：文件名保持，仅更新内容。重命名涉及 contribute 注册（host_id/order/id）变更，侵入性过大。
3. **`<tab-item>` 标签废弃**：统一用 `<Tab>`。文档不再描述 `<tab-item>`，但 codegen 可能仍兼容（不在本计划范围内清理）。
4. **文档语言**：中文为主，技术术语保留英文（与现有文档风格一致）。
5. **bordered 仅 Tabs 支持**：TabBar 不暴露 bordered（header-only 无需边框包裹）。codegen 中 `bordered` setter 检查 `tag == "Tabs"`。

## 实施顺序

1. 阶段 3.1：重写 `tab-bar.md`
2. 阶段 3.2：创建 `tabs.md`
3. 阶段 3.3：更新 `INDEX.md`
4. 阶段 4.1-4.2：迁移 `tab_bar_case`（.rml + .rml.rs）
5. 阶段 4.3-4.4：迁移 `tab_preview_case`（.rml + .rml.rs）
6. 阶段 5：全量编译 + 测试验证
