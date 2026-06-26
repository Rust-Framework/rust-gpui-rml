# 声明式根节点规范化计划

> **修订起因**：用户要求
> 1. `#[window]` 和 `#[component]` 两个宏**不再接受配置属性**
> 2. RML 根节点**必须**是 `<window>`、`<modern_window>` 或 `<component>`
> 3. 窗口属性（`title`/`width`/`height`/`chrome`）在 `.rml` 根节点上声明式配置，不在宏上配置
> 4. RML 框架要充分发挥声明式设计优势，**规范化**——不提供多种解决方案
> 5. demo 改造成类似 WPF 新建项目模板风格，让 WPF 开发者感到亲切

---

## 一、当前状态分析 Current State

### 问题

| 维度 | 当前状态 | 问题 |
|------|---------|------|
| `#[window]` 宏 | `#[window(title = "...", width = N, height = N)]` 接受属性参数 | 配置在 Rust 宏上，非声明式 |
| `#[component]` 宏 | `#[component(template = "path")]` 接受 `template` 参数 | 提供了多种模板路径配置方式 |
| RML 根节点 | 无约束（demo 用 `<div>`） | 缺乏根节点语义，无法从 `.rml` 配置窗口属性 |
| `<window>` 标签 | 不存在，编译器报 `unknown tag: <window>` | 无法声明式定义窗口 |
| `impl IWindow` 生成 | 由 `#[window]` 宏生成，属性值硬编码 | 与声明式设计冲突 |
| demo 文件结构 | `counter.rml` + `counter.rml.rs` | 不符合 WPF `MainWindow.xaml` 命名约定 |

### 影响范围

| 范围 | 引用数 | 说明 |
|------|--------|------|
| `#[window(...)]` 带属性 | ~3 处（demo + 宏代码） | 需改为 bare `#[window]` |
| `#[component(...)]` 带属性 | ~2 处（宏代码 + 文档） | 需改为 bare `#[component]` |
| `.rml` 根节点 `<div>` | 2 处（demo） | 需改为 `<window>`/`<component>` |
| 文档中 `#[window(...)]` | ~10 处 | 需更新为 bare 宏 + `.rml` 根节点配置 |

---

## 二、设计方案 Design

### 2.1 三种根节点的语义

| 根节点 | 生成内容 | 用途 |
|--------|---------|------|
| `<window title="..." width="N" height="N">` | `impl IWindow`（`chrome() = Transparent`）+ `impl Render`（使用子节点） | 基础窗口，透明标题栏 |
| `<modern_window title="..." width="N" height="N">` | `impl IWindow`（`chrome() = Native`）+ `impl Render`（使用子节点） | 现代窗口，原生标题栏 |
| `<component>` | 仅 `impl Render`（使用子节点） | 可复用组件 |

**关键设计决策**：
- `<window>` 和 `<modern_window>` 的**唯一区别**是 `chrome()` 返回值（`Transparent` vs `Native`）
- 两者都不包裹 `ModernWindowShell`——内容直接作为 `render()` 返回值
- `<component>` 不生成 `impl IWindow`，仅生成 `impl Render`
- **其他根节点报错**：`root must be <window>, <modern_window>, or <component>`

### 2.2 代码生成流程

```
.rml 文件
  ↓ parser::parse()
Node::Element { tag: "window", attributes: [...], children: [...] }
  ↓ codegen::codegen()
  ├─ 检测根节点 tag
  ├─ 若是 <window>/<modern_window>：
  │   ├─ 从 attributes 提取 title/width/height
  │   ├─ 生成 impl IWindow（含 open/handle/set_handle + 提取值 + chrome）
  │   └─ 生成 impl Render（使用 children，多个则包 div）
  ├─ 若是 <component>：
  │   └─ 生成 impl Render（使用 children，多个则包 div）
  └─ 否则：报错
  ↓ include!() 注入用户 crate
```

### 2.3 宏的简化

**`#[window]` 宏（变更后）**：
- 添加 `__rml_window_handle: Option<AnyWindowHandle>` 字段
- 生成 `impl IModel` + `impl ILifecycle` + `impl IViewModel` + `impl IComponent`
- **不再生成 `impl IWindow`**——由编译器从 `.rml` 根节点生成
- `include!(OUT_DIR/rml_generated/<snake>.rs)` 注入编译器生成的 `impl IWindow` + `impl Render`
- **拒绝任何属性参数**：`#[window(title = "...")]` → 编译错误

**`#[component]` 宏（变更后）**：
- 生成 `impl IModel` + `impl ILifecycle` + `impl IViewModel` + `impl IComponent`
- 模板路径固定为 `<snake_case>.rml`（不再支持 `template=` 参数）
- `include!(OUT_DIR/rml_generated/<snake>.rs)` 注入编译器生成的 `impl Render`
- **拒绝任何属性参数**：`#[component(template = "...")]` → 编译错误

### 2.4 demo 改造（WPF 新建项目模板风格）

WPF 新建项目结构：
```
MainWindow.xaml      ← 窗口 XAML
MainWindow.xaml.cs   ← code-behind
App.xaml             ← 应用配置（RML 用 main.rs 替代）
```

RML 对应改造：
```
demo/src/
  main.rs            ← 应用入口（RmlApplication::new().main_window::<MainWindow>().run()）
  main_window.rml    ← 主窗口模板（<window title="MainWindow" ...>）
  main_window.rml.rs ← code-behind（#[window] struct MainWindow）
  styles.css         ← 样式表（简化）
  build.rs           ← 构建脚本（不变）
```

**`main_window.rml` 内容**（WPF 风格 + RML 声明式特性）：
```xml
<window title="MainWindow" width="800" height="450">
    <div class="container">
        <h1 ref="title">Hello, RML!</h1>
        <p class="count">点击次数：{count}</p>
        <Button ref="click_btn" label="点击我" primary="" onclick={on_click} />
    </div>
</window>
```

**`main_window.rml.rs` 内容**：
```rust
use rml::prelude::*;

#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}
```

---

## 三、实施步骤 Implementation Steps

### Step 1：宏简化

**文件**：`crates/macros/src/window.rs`
- 删除 `WindowArgs` 结构体
- 删除 `parse_window_args()` 函数
- 删除 `gen_impl_iwindow()` 函数
- `expand()` 函数：
  - 若 `args` 非空，返回编译错误：`"#[window] takes no arguments; configure window properties in .rml root element (<window title=\"...\" width=\"N\" height=\"N\">)"`
  - 保留添加 `__rml_window_handle` 字段的逻辑
  - 保留 `expand_component_impls()` 调用
  - 保留 `include!` 语句
  - **不再生成 `impl IWindow`**

**文件**：`crates/macros/src/component.rs`
- 删除 `parse_template_arg()` 函数
- `expand()` 函数：
  - 若 `args` 非空，返回编译错误：`"#[component] takes no arguments; template path is fixed as <snake_case>.rml"`
  - 模板路径固定为 `format!("{}.rml", snake)`
  - 保留 `expand_component_impls()` 调用
  - 保留 `include!` 语句

**验证**：`cargo build -p rust-rml-macros` 通过

### Step 2：编译器识别根节点

**文件**：`crates/engine/src/tags.rs`
- 新增 `is_root_tag(tag: &str) -> bool`：返回 `tag == "window" || tag == "modern_window" || tag == "component"`
- 新增 `RootTag` 枚举：`Window` / `ModernWindow` / `Component`
- 新增 `root_tag_lookup(tag: &str) -> Option<RootTag>`：将字符串映射到枚举
- **不加入 `BuiltinTag`**——这些是根节点标记，不是普通 HTML 标签

**文件**：`crates/engine/src/compiler/codegen.rs`
- 重构 `codegen()` 函数：
  ```rust
  pub fn codegen(root: &Node, ctx: &CodegenCtx) -> Result<String, CodegenError> {
      match root {
          Node::Element(elem) if tags::is_root_tag(&elem.tag) => {
              gen_root_element(elem, ctx)
          }
          _ => Err(CodegenError {
              message: format!(
                  "root element must be <window>, <modern_window>, or <component>; got <{}>",
                  root.tag_str()
              ),
          })
      }
  }
  ```
- 新增 `gen_root_element(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError>`：
  - 根据 `elem.tag` 调度到 `gen_window_impl()` / `gen_modern_window_impl()` / `gen_component_only()`
  - 调用 `gen_render_impl_from_children(elem, ctx)` 生成 `impl Render`
- 新增 `gen_window_impl(elem: &Element, ctx: &CodegenCtx) -> String`：
  - 从 `elem.attributes` 提取 `title`（默认 `"RML Window"`）、`width`（默认 `800.0`）、`height`（默认 `600.0`）
  - 生成完整 `impl IWindow` 代码块，包含：
    - `title()` / `width()` / `height()` 返回提取值
    - `open()`：调用 `rml_ui::init(cx)` + `cx.open_window(...)` + `rml_ui::Root::new(view, window, cx)`
    - `handle()` / `set_handle()`：访问 `self.__rml_window_handle`
- 新增 `gen_modern_window_impl(elem: &Element, ctx: &CodegenCtx) -> String`：
  - 同 `gen_window_impl`，但额外生成 `chrome() -> WindowChrome::Native`
- 新增 `gen_render_impl_from_children(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError>`：
  - 生成 `impl Render` 代码块
  - 函数体：遍历 `elem.children`，对每个子节点调用 `gen_node()`
  - 单个子节点：直接使用其代码
  - 多个子节点：包裹在 `gpui::div()` 中，逐个 `.child(...)`
- 新增 `gen_component_only(elem: &Element, ctx: &CodegenCtx) -> Result<String, CodegenError>`：
  - 仅调用 `gen_render_impl_from_children()`

**属性提取细节**：
- `title="..."` → `Attribute::Static { name: "title", value: "..." }` → 直接使用 `value`
- `width="800"` 或 `width="800.0"` → 解析为 `f32`，生成 `gpui::px(800.0)`
- `height="450"` → 同上
- 其他属性 → 忽略（不报错，避免阻碍）

**文件**：`crates/engine/src/parser/ast.rs`（可能需要）
- 若 `Node` 没有 `tag_str()` 方法，新增一个辅助方法或用 `match` 处理

**验证**：
- `cargo build -p rust-rml-engine` 通过
- 现有测试可能需要更新（见 Step 5）

### Step 3：demo 改造

**删除文件**：
- `demo/src/counter.rml`
- `demo/src/counter.rml.rs`
- `demo/src/todos.rml`
- `demo/src/todos.rml.rs`

**新建文件**：`demo/src/main_window.rml`
```xml
<window title="MainWindow" width="800" height="450">
    <div class="container">
        <h1 ref="title">Hello, RML!</h1>
        <p class="count">点击次数：{count}</p>
        <Button ref="click_btn" label="点击我" primary="" onclick={on_click} />
    </div>
</window>
```

**新建文件**：`demo/src/main_window.rml.rs`
```rust
use rml::prelude::*;

#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}
```

**修改文件**：`demo/src/main.rs`
```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;
extern crate rust_rml_core as rml_core;
extern crate rust_rml_ui as rml_ui;

use rml_app::RmlApplication;

#[path = "main_window.rml.rs"]
mod main_window;

fn main() {
    RmlApplication::new()
        .main_window::<main_window::MainWindow>()
        .run();
}
```

**修改文件**：`demo/src/styles.css`
- 简化为 `.container` + `.count` 样式（移除 counter 专用样式）

**验证**：`cargo build -p rust-rml-demo` 通过

### Step 4：编译器测试更新

**文件**：`crates/engine/tests/` 下的测试
- 检查所有测试用的 `.rml` 片段
- 若测试直接调用 `codegen::codegen()` 且使用 `<div>` 作为根节点，需更新为 `<component>` 根节点
- 或：在测试中保留 `<div>` 根节点，但更新 `codegen()` 签名以支持测试模式

**策略**：
- 现有测试大多使用 `compile(source, ctx)` 入口
- 更新测试用例的 `.rml` 根节点为 `<component>` 或 `<window>`
- 对于纯元素测试（如测试 `<div>` 渲染），包裹在 `<component>` 中：
  ```rust
  let source = "<component><div>hello</div></component>";
  ```

### Step 5：文档更新

**文件**（按优先级）：

| 优先级 | 文件 | 变更 |
|--------|------|------|
| P0 | `crates/macros/README.md` | 更新 `#[window]` / `#[component]` 说明（移除属性参数） |
| P0 | `crates/core/README.md` | 更新 `IWindow` 说明（属性在 `.rml` 配置） |
| P1 | `docs/04-code-behind/macros.md` | 全面更新宏参考文档 |
| P1 | `docs/06-components/custom-components.md` | 更新自定义组件示例为 `<component>` 根节点 |
| P1 | `docs/04-code-behind/viewmodel-structure.md` | 更新 ViewModel 结构示例 |
| P2 | `docs/**` 其余 | 批量更新 `#[window(...)]` → `#[window]` + `.rml` 根节点配置 |
| P2 | `README.md`（根） | 更新快速开始示例 |

**批量替换规则**：
- `#[window(title = "...", width = N, height = N)]` → `#[window]` + `.rml` 根节点 `<window title="..." width="N" height="N">`
- `#[component(template = "...")]` → `#[component]`
- `.rml` 示例中 `<div>` 根节点 → `<window>` 或 `<component>` 根节点

### Step 6：全量验证

```bash
cargo build -p rust-rml-core
cargo build -p rust-rml-macros
cargo build -p rust-rml-engine
cargo build -p rust-rml-app
cargo build -p rust-rml-ui
cargo build -p rust-rml-demo
cargo build --workspace

cargo test -p rust-rml-engine
cargo test -p rust-rml-core
cargo test --workspace

cargo run -p rust-rml-demo
```

期望：所有命令通过，demo 窗口正常打开，显示 "Hello, RML!" 和点击计数按钮。

---

## 四、假设与决策 Assumptions & Decisions

### 假设
1. `rml_ui::init()` 和 `rml_ui::Root::new()` 在编译器生成的代码中可解析（用户 crate 有 `extern crate rust_rml_ui as rml_ui`）
2. `ModernWindowShell` 不在 `<modern_window>` 中自动包裹——保持简单，用户可手动在内容中使用
3. `<window>` / `<modern_window>` 的子节点可以是多个（自动包裹在 `div` 中）
4. 未识别的根节点属性（如 `resizable`）被忽略，不报错
5. 现有编译器测试可更新为使用 `<component>` 根节点

### 决策
1. **宏不接受任何属性参数**：`#[window]` 和 `#[component]` 均为 bare 宏，非空参数报编译错误
2. **`impl IWindow` 由编译器生成**：从 `.rml` `<window>` 根节点属性提取，宏不再生成
3. **`<window>` vs `<modern_window>` 唯一区别**：`chrome()` 返回值（`Transparent` vs `Native`）
4. **不自动包裹 `ModernWindowShell`**：保持 codegen 简单，用户自行选择是否使用
5. **模板路径固定**：`<snake_case>.rml`，不再支持 `template=` 参数
6. **demo 简化为 WPF 模板风格**：`MainWindow` + 简单计数器，文件命名对齐 WPF 约定
7. **删除未使用的 `todos.rml` / `todos.rml.rs`**：减少噪音，聚焦主窗口
8. **`Window` / `ModernWindow` 内置类型保留**：作为无 `.rml` 模板时的开箱即用窗口（非"多种方案"，是不同工具）

### 待实现时确认的细节
1. `<window>` 子节点为空时 `impl Render` 返回什么？建议 `gpui::div()`
2. `width="800"` 和 `width="800.0"` 都应支持解析为 `f32`
3. 编译器测试失败时的修复策略：逐个更新测试用例

---

## 五、文件变更清单 File Change List

### Macros crate（crates/macros）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/macros/src/window.rs` | 修改 | 移除属性解析 + `impl IWindow` 生成；拒绝非空 args |
| `crates/macros/src/component.rs` | 修改 | 移除 `parse_template_arg()`；拒绝非空 args |
| `crates/macros/src/lib.rs` | 不变 | 宏签名保持不变（args 透传给 expand） |

### Engine crate（crates/engine）

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/engine/src/tags.rs` | 修改 | 新增 `RootTag` 枚举 + `is_root_tag()` + `root_tag_lookup()` |
| `crates/engine/src/compiler/codegen.rs` | 修改 | 重构 `codegen()` 检测根节点；新增 `gen_root_element()` / `gen_window_impl()` / `gen_modern_window_impl()` / `gen_render_impl_from_children()` |
| `crates/engine/tests/*.rs` | 修改 | 更新测试用 `.rml` 片段为 `<component>` 根节点 |

### Demo

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `demo/src/counter.rml` | **删除** | 重命名为 `main_window.rml` |
| `demo/src/counter.rml.rs` | **删除** | 重命名为 `main_window.rml.rs` |
| `demo/src/todos.rml` | **删除** | 未使用 |
| `demo/src/todos.rml.rs` | **删除** | 未使用 |
| `demo/src/main_window.rml` | **新建** | WPF 风格主窗口模板 |
| `demo/src/main_window.rml.rs` | **新建** | `#[window] struct MainWindow` |
| `demo/src/main.rs` | 修改 | 引用 `main_window` 模块 |
| `demo/src/styles.css` | 修改 | 简化为 `MainWindow` 样式 |

### 文档

| 文件 | 变更类型 | 说明 |
|------|---------|------|
| `crates/macros/README.md` | 修改 | 更新宏说明（bare 宏，无属性） |
| `crates/core/README.md` | 修改 | 更新 `IWindow` 说明（属性在 `.rml`） |
| `docs/04-code-behind/macros.md` | 修改 | 全面更新宏参考 |
| `docs/06-components/custom-components.md` | 修改 | 更新为 `<component>` 根节点 |
| `docs/04-code-behind/viewmodel-structure.md` | 修改 | 更新示例 |
| `docs/**` 其余 | 修改 | 批量更新 `#[window(...)]` → `#[window]` + `.rml` 根节点 |
| `README.md`（根） | 修改 | 更新快速开始 |

### 零影响

| 范围 | 说明 |
|------|------|
| `crates/app/**` | `RmlApplication` 不变（已内置 `main_window`） |
| `crates/core/src/window.rs` | `IWindow` trait 不变（默认实现已完整） |
| `crates/core/src/component.rs` | `IComponent` trait 不变 |
| `crates/ui/src/window/builtin_window.rs` | 内置 `Window`/`ModernWindow` 保留（手动实现） |

---

## 六、执行顺序 Execution Order

1. **Step 1**：宏简化 → `cargo build -p rust-rml-macros` 通过
2. **Step 2**：编译器识别根节点 → `cargo build -p rust-rml-engine` 通过
3. **Step 3**：demo 改造 → `cargo build -p rust-rml-demo` 通过
4. **Step 4**：编译器测试更新 → `cargo test -p rust-rml-engine` 通过
5. **Step 5**：文档更新
6. **Step 6**：全量验证 → `cargo build --workspace` + `cargo test --workspace` + `cargo run -p rust-rml-demo`

每个步骤完成后立即验证编译，避免错误累积。

---

## 七、与既有计划的关系

本计划是 `wpf-style-window-and-application-api-plan.md` 的**后续规范化**：

| 维度 | WPF 风格计划（已完成） | 本计划 |
|------|----------------------|--------|
| `#[window]` 宏 | 接受 `title`/`width`/`height` 属性 | **不接受任何属性** |
| `#[component]` 宏 | 接受 `template` 属性 | **不接受任何属性** |
| RML 根节点 | 无约束（`<div>` 等） | **必须** `<window>`/`<modern_window>`/`<component>` |
| 窗口属性配置 | 宏属性参数 | `.rml` 根节点属性 |
| `impl IWindow` 生成 | 宏生成 | 编译器生成 |
| demo 文件命名 | `counter.rml` | `main_window.rml`（WPF 风格） |

**保持不变**：
- `IWindow` trait 设计（默认实现完整）
- `RmlApplication` 内置 `main_window::<W>()` API
- `app` crate 不依赖 `ui` crate 架构
- 引擎零影响原则（编译器仍不引用 trait）
