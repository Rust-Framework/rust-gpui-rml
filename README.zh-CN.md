# RML —— 面向 GPUI 的 Rust 标记语言

[English](README.md) | **简体中文**

[![MIT License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org)

> 一个基于 [GPUI](https://github.com/zed-industries/zed)
> —— Zed 编辑器所采用、GPU 加速的 Rust UI 框架 —— 构建的 **HTML 友好型声明式 UI 框架**。
>
> 标记写在 `.rml` 文件中，业务逻辑写在 `.rml.rs` Code-Behind 文件中，并借助 **编译期**
> 直接生成原生 GPUI 渲染代码（零运行时开销）。
> 设计哲学：汲取 WPF XAML 的设计精髓，拥抱 HTML 的语法亲和力。

## 目录

- [RML 是什么？](#rml-是什么)
- [动机](#动机)
- [核心特性](#核心特性)
- [架构](#架构)
- [仓库结构](#仓库结构)
- [快速开始](#快速开始)
- [RML 速览](#rml-速览)
- [文档](#文档)
- [构建](#构建)
- [License](#license)

## RML 是什么？

RML（**R**ust **M**arkup **L**anguage，Rust 标记语言）是一个基于 GPUI、面向开发者与设计师的
UI 框架。它将 WPF/XAML 与现代 Web 框架（Vue / React）的工业化 UI 开发模式带到原生 Rust
桌面应用中来：

- **`window.rml`** —— 一个独立的、类似 HTML 的标记文件，描述 UI 结构、布局、数据绑定与事件绑定。
- **`window.rml.rs`** —— *Code-Behind（后台代码）*，包含纯 Rust 状态、事件处理器、计算属性与
  生命周期回调，扮演 MVVM 中的 `ViewModel` 角色。
- **`build.rs`** —— 把每个 `.rml` 编译成原生 GPUI 渲染代码（`impl Render`），因此在运行时
  无需任何解释执行。

因为标记会被编译成纯 GPUI 代码，所以本框架**零运行时开销**，并保留对 GPUI 那种 GPU 加速、
即时/保留混合渲染模型的完整访问能力。

> **状态说明：** 热重载与 VS Code 扩展已列入路线图，但**尚未实现**。
> 当前已经可用的能力，请参见[核心特性](#核心特性)。

## 动机

直接用 GPUI 编写 UI 相当冗长，且 UI 结构与业务逻辑、事件处理深度耦合：

```rust
// 原生 GPUI —— 命令式链式调用，UI 与逻辑交织
div()
    .flex()
    .flex_col()
    .gap(px(16.0))
    .p(px(24.0))
    .child(
        div()
            .text_xl()
            .font_weight(FontWeight::BOLD)
            .child(Label::new("Hello World")),
    )
    .child(
        Button::new("Click me").on_click(cx.listener(|this, _ev, cx| {
            this.count += 1;
            cx.notify();
        })),
    );
```

这带来真实成本：UI 逻辑与 Rust 深度耦合、代码冗长且嵌套、设计师无法参与，也没有统一的 UI
标记标准。

RML 着力解决这些问题：

| 目标 | 价值 |
|------|------|
| **关注点彻底分离** | UI 结构（`.rml`）与业务逻辑（`.rml.rs`）完全独立 |
| **HTML 语法亲和** | 使用标准 HTML 标签、属性与事件——对 Web 开发者近乎零学习成本 |
| **WPF 级数据绑定** | 单向 / 双向绑定、值转换器、命令系统 |
| **零运行时开销** | `.rml` 在构建期编译为原生 GPUI 渲染代码 |
| **设计师友好** | 纯标记语言，可配合任意 XML/HTML 工具使用 |
| **热重载就绪** | 独立文件为未来实时编辑提供了天然基础 |

## 核心特性

**面向 GPUI 的标记语言**
- 标准 HTML 标签（`div`、`p`、`span`、`button`、`input`、`textarea`、`ul`/`li`、`h1`–`h6`、`img`、`label` 等）映射到原生 GPUI 元素。
- PascalCase 标签（`<Button>`、`<Input>`、`<Dialog>` 等）经 `rust-rml-ui` 扩展 crate 路由到 [`gpui-component`](https://github.com/longbridge/gpui-component) 组件。
- 标准 HTML 属性（`class`、`id`、`style`、`placeholder`、`type`、`disabled` 等）。

**MVVM 数据绑定**（WPF 级能力矩阵）
- 单向绑定：`{field}` / `attr={expr}` —— ViewModel → View 自动同步。
- 双向绑定：`model={field}` —— 完整双向数据流并带循环防护。
- 值转换器：`model={field | Converter}` —— 内置 `UpperCase` / `LowerCase` / `Trim` / `Currency` / `Percent` / `BoolToYesNo`，并支持自定义 `IConverter`。
- 计算属性：`#[computed]`，带依赖追踪与 `ComputedCache` 失效。
- 命令系统：`#[command]` + `onclick={method}`（强类型直接调用）与声明式 `command={field}`（对齐 WPF `ICommand`，经 `can_execute`/`execute` 动态调度）。
- 字段校验：`#[validate(range/length/required/regex/custom/IValidate)]`，错误消息自动管理。
- 防抖 / 节流：`#[command(debounce = "300ms")]`。

**指令与事件**（无任何框架前缀）
- `if` / `else` / `each` / `key` / `model` / `show` / `once` / `html` / `ref` / `slot`。
- 标准事件模型：`onclick`、`oninput`、`onchange`、`onkeydown`、`onkeyup`、`onmouseenter`、`onmouseleave` 等。

**组件系统**
- 自定义组件以 `#[component]` 结构体 + 对应 `.rml` 模板定义。
- 具名插槽用于组合，`ref`/`#[element]` 元素引用，以及 `#[on_loaded]` / `#[on_unloaded]` 生命周期回调。

**样式、主题与国际化（i18n）**
- CSS 样式表支持：`.rml`/`.css` 被解析并映射为 GPUI 样式；配合 `assets/themes/*.css`（dark / light / 自定义）主题。
- 内置主题运行时（`assets/themes`），以及一套在构建期提取键的 i18n 层（`t("key")`）。

**工具链**
- 内置 RML 语法高亮 / 注入的 **tree-sitter grammar**（供 helix/neovim 使用的 highlights + injections）。
- 一个 **LSP 服务器**（`rust-rml-lsp`），提供补全、悬停、诊断、格式化、定义/引用、重命名等能力——包括 `.rml` 与 `.rml.rs`（以及经 rust-analyzer 的 Rust）之间的跨语言跳转。
- 增量式 `build.rs` 编译，三层缓存（`.rml` 哈希 + Code-Behind 哈希 + engine 源码哈希）。
- 一个嵌入式终端组件（`rust-rml-ui-term`，基于 `alacritty_terminal`）。

## 架构

### 三层架构

```
┌──────────────────────────────────────────────────────────────────────┐
│                          表现层（Presentation）                       │
│   window.rml（UI 标记 / 绑定 / 事件）  +  window.rml.rs                │
│   （Code-Behind：状态 · 处理器 · 计算属性 · 生命周期）                 │
└──────────────────────────────────────────────────────────────────────┘
                              │ 经编译
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                     RML 编译器（build.rs + 过程宏）                    │
│   .rml → 词法分析 → AST → 语义验证 → GPUI 代码生成                    │
└──────────────────────────────────────────────────────────────────────┘
                              ▼
┌──────────────────────────────────────────────────────────────────────┐
│                           框架层（Framework）                          │
│   GPUI（渲染引擎）· gpui-component（组件库）· RML 运行时                │
│   （绑定系统 · 双向绑定 · 转换器 · 计算缓存）                           │
└──────────────────────────────────────────────────────────────────────┘
```

整条流水线遵循 **MVVM**：一个纯 Rust 的 `Model`、一个 `ViewModel`（即 `.rml.rs` Code-Behind，
持有响应式状态与命令的 entity）与一个 `View`（即通过绑定消费状态的 `.rml` 标记）。

### 编译期流程

```
1. build.rs 执行
   ├── 扫描 src/**/*.rml
   ├── 将每个 .rml 解析为 AST
   ├── 校验语法与绑定路径
   └── 向 OUT_DIR/ 写入 *.generated.rs
2. rustc 编译 .rml.rs（你的逻辑），并通过 include! 引入生成的 Render 实现
3. 最终二进制完成链接——全程无需运行时解释器
```

每个 crate 尽可能启用 `#![forbid(unsafe_code)]`。

## 仓库结构

这是一个 Cargo workspace。核心 crates：

| Crate | 职责 |
|-------|------|
| `crates/core` | 契约层：`IModel` / `IViewModel` / `IComponent` / `IWindow` / `ICommand` / `IConverter` / `ITwoWayBinding` / `ILifecycle` 及事件 / 标记类型。 |
| `crates/macros` | 过程宏：`#[derive(IModel)]`、`#[component]`、`#[window]`、`#[command]`、`#[computed]`、`#[validate]`、`#[rml::main]` 等。 |
| `crates/engine` | 编译器：词法分析 → AST → 校验 → 代码生成，另含 CSS 映射、资源处理、i18n 提取，以及 `build.rs` API。 |
| `crates/ui` | 封装 `gpui-component` 的扩展组件库（Dialog / Form / List / …）及内置窗口类型。 |
| `crates/ui-term` | 嵌入式终端组件（`TerminalView`）。 |
| `crates/app` | WPF 风格启动器：`RmlApplication::new().main_window::<W>().run::<L>()`。 |
| `crates/rml` | RML 客户端：tree-sitter grammar + LSP 客户端钩子 + 代码编辑器 provider。 |
| `crates/lsp` | RML 语言服务器（`rml-lsp` 二进制），带跨语言协调器。 |
| `crates/dap` | 已从 workspace 构建中排除（重型 `lldb` git 依赖）；需要时在其独立目录中构建。 |
| `demo` | 示例应用，用 100+ 个组件用例验证 `.rml` + `.rml.rs` + `build.rs` 三件套闭环。 |
| `studio/*` | **Arc Studio**，一个构建在框架之上的示例 IDE 产品（shell / editor / explorer / chat + core DI）。 |

## 快速开始

前置条件：Rust 工具链与网络连接（GPUI 与 `gpui-component` 从 git 拉取，并锁定到特定修订版；
见 `Cargo.toml`）。

**1. 克隆并构建**

```bash
git clone https://github.com/Rust-Framework/rust-gpui-rml.git
cd rust-gpui-rml
cargo build
```

> workspace 有意将 `crates/dap` 放入 `exclude`，因为其重型 `lldb` git 依赖会在受限网络下
> 阻塞编译。如需使用请单独构建。

**2. 运行展示 demo**

```bash
cargo run -p rust-rml-demo
```

这会启动 RML 展示应用——一个覆盖 100+ 个 `.rml` 用例（按钮、表单、表格、菜单、对话框、
i18n、主题、终端等）的标签式窗口。

**3. 运行 Arc Studio**

```bash
cargo run -p arc-studio
```

Arc Studio 是一个完全用 `.rml` 视图构建的小型 IDE（项目资源管理器、编辑器、聊天面板、
状态栏）——是框架的一个真实端到端示例。

## RML 速览

标记（`counter.rml`）：

```html
<div class="app">
    <h1>计数：{count}</h1>
    <button onclick={increment}>+1</button>
    <button onclick={decrement} if={count > 0}>-1</button>
</div>
```

Code-Behind（`counter.rml.rs`）：

```rust
use rml::prelude::*;

#[component]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        // 宏会自动注入 bump_version("count") + cx.notify()
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.count > 0 {
            self.count -= 1;
        }
    }
}
```

入口（`main.rs`）：

```rust
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;

mod counter;

#[rml::main] // 自动注入 rml::embed_assets!();（资源注册）
fn main() {
    rml_app::RmlApplication::new()
        .main_window::<counter::Counter>()
        .run();
}
```

无需手写 GPUI 链式调用——UI 与逻辑彻底分离，状态变更自动驱动重绘。

## 文档

- **`CLAUDE.md`** —— 仓库针对大语言模型编码智能体的显式行为准则（先思考后编码、简单优先、
  外科手术式改动、目标驱动执行）。如果你用 AI 助手协同这个代码库，请让它先读这里。
- **`demo/`** —— `demo/src/cases/` 下 100+ 个可运行展示用例。
- **`crates/*/README.md`** —— 每个 crate 的设计文档（`core`、`macros`、`engine`、`ui`、`app`
  等），含约束、trait 与设计规则。
- **`.trae/documents/`** —— 详细设计与规划文档（架构规划、MVVM 完善、RML 迭代、组件规范等）。

## 构建

```bash
cargo build                  # 构建整个 workspace（不含 crates/dap）
cargo build -p rust-rml-demo # 仅构建 demo
cargo test -p rust-rml-engine# 运行 engine 的代码生成 / CSS / e2e 测试
```

说明：

- 为可复现起见，`Cargo.toml` 中 GPUI 与 `gpui-component` 锁定到特定 git 修订版。升级时按需
  重新生成 / 锁定。
- 资源模式（嵌入 vs 文件系统）在 `build.rs` 中经 `.assets(path, embed)` 一次性配置；两种模式下
  运行时 API 保持一致。
- `crates/dap` 已被排除，因此默认 workspace 构建无需 `lldb` 绑定即可工作。

## License

[MIT](LICENSE)