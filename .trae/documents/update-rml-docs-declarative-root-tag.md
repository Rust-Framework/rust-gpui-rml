# 计划：更新 RML 文档以匹配声明式根节点新 API

## 摘要

RML 框架近期重构：`#[component]` 与 `#[window]` 宏不再接受任何参数，窗口/组件配置改为在 `.rml` 根节点声明（`<window title="..." width="N" height="N">` / `<component>`）。这两个宏现在自动生成 `impl IModel`，因此当它们与 `#[derive(IModel)]` 相邻时，`#[derive(IModel)]` 是冗余的，应当移除。

本任务需更新 **12 个指定文档文件**，使其代码片段与当前 API 一致。前序已完成 `ViewContext<Self>` → `Context<Self>` 与 `derive(Model` → `derive(IModel` 的全量替换（已通过 grep 验证：12 个文件中已无 `ViewContext`/`derive(Model` 残留，仅 viewmodel-structure.md 第 52 行有一处 `**ViewContext**` 概念性散文提及，保留不动）。

剩余工作：①剥离 `#[component(template = "...")]` 参数；②移除与 `#[component]`/`#[window]` 相邻的冗余 `#[derive(IModel)]`；③处理 3 个特殊用例。

## API 事实确认（来自源码）

| 事实 | 证据 |
| --- | --- |
| `#[component]` 不接受参数 | `crates/macros/src/component.rs:112` 报错 "#[component] takes no arguments; template path is fixed as <snake_case>.rml" |
| `#[window]` 不接受参数 | `crates/macros/src/window.rs:29` 报错 "#[window] takes no arguments; configure window properties in .rml root element (`<window title=\"...\" width=\"N\" height=\"N\">`)" |
| 二者自动生成 `impl IModel` | `component.rs` 的 `expand_component_impls` 调用 `gen_impl_i_model`，`#[component]` 与 `#[window]` 共用此函数 |
| 模板路径固定为 `<snake_case>.rml` | `component.rs` 中 `template_path = format!("{}.rml", snake)` |

**推论**：当 `#[derive(IModel)]` 紧邻 `#[component]` 或 `#[window]`（下一行）时，移除该 `#[derive(IModel)]`；当 `#[derive(IModel)]` 紧邻 `pub struct`（独立数据结构）时，保留。

## 当前状态分析（grep 验证结果）

### 已完成（无需改动）
- `docs/07-styling/style-reuse.md` ✅ 全部完成
- `docs/09-architecture/solid-principles.md` ✅ 仅有独立数据结构的 `#[derive(IModel)]`，全部保留，无需移除

### 待处理：剥离 `#[component(template = "...")]` → `#[component]`
| 文件 | 行 | 路径 |
| --- | --- | --- |
| `docs/06-components/custom-components.md` | 127 | `views/user/profile.rml` |
| | 332 | `components/data_loader.rml` |
| | 407 | `components/search_box.rml` |
| `docs/06-components/composition.md` | 277 | `components/loading_wrapper.rml` |
| | 377 | `components/user_list_presentation.rml` |
| | 390 | `components/tabs.rml` |
| | 398 | `components/tab_item.rml` |
| | 406 | `components/tab_panel.rml` |
| | 471, 478 | `components/user_card.rml`（出现 2 次，可用 replace_all） |
| `docs/06-components/component-props.md` | 38 | `components/avatar.rml` |
| | 56, 374 | `components/user_card.rml`（出现 2 次，可用 replace_all） |
| | 209 | `components/data_view.rml` |
| | 241 | `components/counter.rml` |
| | 420 | `components/progress_bar.rml` |

### 待处理：移除相邻 `#[derive(IModel)]`（用 replace_all 模式匹配）
| 文件 | 模式 | 说明 |
| --- | --- | --- |
| `docs/05-events/custom-events.md` | `#[derive(IModel)]\n#[component]` → `#[component]` | 4 处（行 18, 118, 262, 361） |
| `docs/08-lifecycle/on-loaded.md` | 同上 | 1 处（行 30） |
| `docs/08-lifecycle/lifecycle-overview.md` | 同上 | 4 处（行 96-97, 214-215, 366-367, 399-400） |
| `docs/03-binding/two-way-binding.md` | 同上 | grep 确认存在 |
| `docs/06-components/slots.md` | 同上 | grep 确认存在 |
| `docs/06-components/custom-components.md` | 同上 | 路径剥离后处理 |
| `docs/06-components/component-props.md` | 同上 | 路径剥离后处理 |
| `docs/06-components/composition.md` | 同上 | 路径剥离后处理 |
| `docs/04-code-behind/macros.md` | `#[derive(IModel)]\n#[window]` → `#[window]` + `#[derive(IModel)]\n#[component]` → `#[component]` | 行 22-23（window）、63、254、280（component） |
| `docs/04-code-behind/viewmodel-structure.md` | `#[derive(IModel)]\n#[component]` → `#[component]`（replace_all） | 混合：独立数据结构保留，相邻 component 的移除 |

## 执行顺序（关键）

顺序很重要，因为剥离参数会生成裸 `#[component]`，随后的 derive 移除模式才能匹配：

1. **先剥离路径参数**（3 个文件：custom-components、composition、component-props）
2. **再移除相邻 derive**（按文件逐个 replace_all）
3. **最后处理特殊用例**

## 具体变更

### 步骤 1：剥离 `#[component(template = "...")]` → `#[component]`

对每个唯一路径用独立 Edit（同一文件内相同字符串可用 replace_all）：

- **custom-components.md**：4 处编辑（含散文行 668）+ 3 处代码（行 127/332/407）
- **composition.md**：6 个唯一路径 + `components/user_card.rml`（replace_all，2 次）
- **component-props.md**：4 个唯一路径 + `components/user_card.rml`（replace_all，2 次）

### 步骤 2：移除相邻 `#[derive(IModel)]`

对每个文件用 replace_all：
- `#[derive(IModel)]\n#[component]` → `#[component]`
- `#[derive(IModel)]\n#[window]` → `#[window]`（仅 macros.md）

此模式天然保留独立数据结构的 `#[derive(IModel)]\npub struct ...`（不匹配）。

### 步骤 3：特殊用例

#### 3a. `docs/04-code-behind/macros.md`（Rule 7）
当前第 35-46 行的 `### 参数` 小节展示了旧 API（`#[window(template=...)]`、`#[window(generated_path=...)]`）。将其替换为 `### 声明式根节点配置` 小节，说明新设计：`#[window]` 不带参数，窗口属性在 `.rml` 根节点声明。

替换内容（第 35-46 行）：
```
### 声明式根节点配置

`#[window]` 不接受任何参数。窗口属性（标题、尺寸等）在 `.rml` 根节点声明：

```rust
// counter.rml.rs
#[window]
pub struct Counter {
    pub count: i32,
}
```

```html
<!-- counter.rml -->
<window title="计数器" width="400" height="300">
    <p>{count}</p>
</window>
```

若需现代化窗口样式，使用 `<modern_window>` 根节点；可复用组件使用 `<component>` 根节点。
```

注意：保留 `### 与 #[component] 的区别` 表格不变。

#### 3b. `docs/04-code-behind/viewmodel-structure.md` 第 18 行（带尾注释的特殊情况）
第 18 行 `#[derive(IModel)]    // 1. 成为 GPUI Entity` 后跟第 19 行 `#[component]`。因有尾注释，replace_all 模式不匹配，需单独 Edit：
```
#[component]             // 1. 标记为 RML 视图（自动成为 GPUI Entity）
```
（移除 derive 行，将注释合并到 component 行，说明 component 自动生成 IModel）

注意：第 408-409 行的散文 `1. **#[derive(IModel)]**：成为 GPUI Entity` / `2. **#[component]**：...` 属于解释性文字，按用户指示"保留解释性文字"不动；但若与步骤 2 的代码示例冲突需酌情。优先保留散文。

#### 3c. `docs/06-components/custom-components.md` 第 668 行（散文中的宏引用）
```
- **创建**：`#[component(template = "...")]` + `.rml` 模板
```
改为：
```
- **创建**：`#[component]` + `.rml` 模板
```

## 假设与决策

1. **范围限定为 12 个文件**：用户原始任务明确列出 12 个文件。grep 发现另有约 20+ 个文件存在相同的陈旧模式（`derive(Model`、`ViewContext`），但这些不在本任务范围内（如 `01-overview/`、`02-syntax/`、`04-code-behind/state-management.md`、`08-lifecycle/async-tasks.md`、`09-architecture/responsibility.md` 等）。本计划不改动这些文件。如需扩展范围，需用户另行确认。

2. **散文中的概念性提及保留**：viewmodel-structure.md 第 52 行 `**ViewContext**：在命令方法中接收 cx: &mut Context<Self>` —— 此处 "ViewContext" 为概念术语（已正确显示 `Context<Self>` 类型），保留不动。

3. **独立数据结构的 `#[derive(IModel)]` 保留**：当 `#[derive(IModel)]` 后跟 `pub struct`（无 `#[component]`/`#[window]`）时，无宏自动生成 IModel，故保留。已通过源码 `expand_component_impls` 确认。

4. **不创建新文件**：仅编辑现有 12 个文件。

5. **不改变文档整体结构与内容**：仅修正陈旧语法引用，保留所有解释性文字、示例与格式。

6. **验证方式**：文档为 `.md` 文件，`cargo build` 不会编译它们。验证以 grep 为准（确认目标模式已清除）+ 可选 `cargo build --workspace` 作为项目健康检查（不验证文档正确性）。

## 验证步骤

1. **grep 验证无残留**：
   - `Grep "#\[component\(template"` 于 docs/ → 应仅在 12 文件之外有命中（本任务范围内 0 命中）
   - `Grep "#\[derive\(IModel\)\]" -A1` 于 12 文件 → 紧跟 `#[component]`/`#[window]` 的应为 0；紧跟 `pub struct` 的保留
   - `Grep "derive\(Model"` 于 12 文件 → 0 命中
   - `Grep "ViewContext<Self>"` 于 12 文件 → 0 命中

2. **可选**：`cargo build --workspace`（项目健康检查，非文档验证）

3. **报告**：列出已修改文件及编辑次数
