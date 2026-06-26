# 1.3 快速开始

> **本节目标**：15 分钟内从零跑通一个 RML 计数器应用，建立"`.rml` + `.rml.rs` + `main.rs`"三件套的肌肉记忆。

## 1.3.1 前置准备

| 依赖              | 版本要求           | 说明                       |
| --------------- | -------------- | ------------------------ |
| Rust 工具链        | stable 1.75+   | `rustup default stable`  |
| GPUI 系统依赖       | 见 GPUI 文档      | Linux 需安装 `gtk`、`webkit2gtk` 等 |
| RML 框架          | 最新 main 分支     | 通过 git 依赖引入               |

## 1.3.2 创建项目

```bash
cargo new rml-counter --bin
cd rml-counter
```

编辑 `Cargo.toml`：

```toml
[package]
name = "rml-counter"
version = "0.1.0"
edition = "2021"

[dependencies]
rml = { git = "https://github.com/your-org/rml-framework.git" }
rml-app = { git = "https://github.com/your-org/rml-framework.git" }
gpui = { git = "https://github.com/zed-industries/zed.git" }
gpui-component = { git = "https://github.com/your-org/gpui-component.git" }

[build-dependencies]
rml-compiler = { git = "https://github.com/your-org/rml-framework.git" }
```

## 1.3.3 配置 build.rs

在项目根目录创建 `build.rs`，告诉 RML 编译器去哪里找 `.rml` 文件：

```rust
// build.rs
use rml_compiler::RmlBuild;

fn main() {
    RmlBuild::new()
        .input_dir("src/views")        // 扫描视图文件
        .input_dir("src/components")   // 扫描组件文件
        .output_dir(std::env::var("OUT_DIR").unwrap())
        .with_watch(true)              // 开发模式启用文件监听
        .compile()
        .unwrap();

    println!("cargo:rerun-if-changed=src/views");
    println!("cargo:rerun-if-changed=src/components");
}
```

💡 **提示**：`with_watch(true)` 仅在 debug 构建生效，release 构建会自动关闭以避免运行时开销。

## 1.3.4 编写 UI 标记（`.rml`）

创建 `src/views/counter.rml`：

```html
<!-- src/views/counter.rml -->
<div class="counter-container">
    <h1 class="counter-title">⚡ RML 计数器</h1>

    <div class="counter-display">
        <span class="counter-value">{count}</span>
        <span class="counter-status" if={count > 10}>
            🚀 超过十啦！
        </span>
    </div>

    <div class="counter-buttons">
        <button class="btn primary" onclick={increment}>➕ 增加</button>
        <button class="btn danger" onclick={decrement} if={count > 0}>➖ 减少</button>
        <button class="btn secondary" onclick={reset}>↺ 重置</button>
    </div>
</div>
```

**逐行解读**：

- `{count}` —— 单向绑定，显示 ViewModel 的 `count` 字段
- `if={count > 10}` —— 条件渲染指令，表达式为真时渲染
- `onclick={increment}` —— 事件绑定，调用 ViewModel 的 `increment` 命令
- `class="btn primary"` —— 标准 HTML class 属性，映射到 GPUI 样式

## 1.3.5 编写业务逻辑（`.rml.rs`）

创建 `src/views/counter.rml.rs`：

```rust
// src/views/counter.rml.rs
use rml::prelude::*;

#[derive(Model)]
#[component]  // 极简宏：标记为 RML 组件
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Self {
        Self { count: 0 }
    }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count += 1;
        cx.notify();  // 触发 UI 重绘
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.count > 0 {
            self.count -= 1;
            cx.notify();
        }
    }

    #[command]
    pub fn reset(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count = 0;
        cx.notify();
    }
}
```

**关键点**：

- `#[derive(Model)]` —— 让结构体成为 GPUI Entity，字段自动成为响应式状态
- `#[component]` —— 标记为 RML 组件，编译器会为其生成 `Render` 实现
- `#[command]` —— 标记方法为 UI 可调用的命令，`.rml` 中的 `onclick={increment}` 直接绑定到这里
- `cx.notify()` —— 状态变更后必须调用，否则 UI 不会更新

## 1.3.6 编写入口（`main.rs`）

创建 `src/main.rs`：

```rust
// src/main.rs
use rml_app::RmlApplication;

mod views;

fn main() {
    RmlApplication::new()
        .with_hot_reload(vec!["src/views".into()])  // 启用热重载
        .main_window::<views::counter::Counter>()
        .run()
        .unwrap();
}
```

在 `src/views/mod.rs` 中导出模块：

```rust
// src/views/mod.rs
pub mod counter;
```

## 1.3.7 运行应用

```bash
cargo run
```

如果一切正常，你会看到一个窗口，标题"⚡ RML 计数器"，点击"➕ 增加"按钮，数字会递增；超过 10 时显示"🚀 超过十啦！"；点击"➖ 减少"在 count > 0 时可用；点击"↺ 重置"归零。

## 1.3.8 体验热重载

保持应用运行，编辑 `src/views/counter.rml`，把标题改为：

```html
<h1 class="counter-title">🎯 我的热重载计数器</h1>
```

保存文件，应用窗口会实时更新，**无需重新编译 Rust 代码**。这是 RML 独立文件设计带来的核心开发体验优势。

## 1.3.9 三件套肌肉记忆

每个 RML 视图都是这三件套：

```
views/counter.rml       ← UI 结构、样式、绑定、事件
views/counter.rml.rs    ← ViewModel：状态、命令、生命周期
views/mod.rs            ← 模块导出
```

记住这个结构，你就掌握了 RML 开发的基本单元。后续章节会在此基础上扩展组件、样式、生命周期等能力。

## 1.3.10 常见问题排查

| 现象                  | 原因                          | 解决                              |
| ------------------- | --------------------------- | ------------------------------- |
| 编译报错"找不到 RmlView"   | `build.rs` 未配置或 `input_dir` 路径错误 | 检查 `build.rs` 中的路径              |
| UI 不更新              | 忘记调用 `cx.notify()`          | 在所有修改状态的命令末尾加 `cx.notify()`     |
| 事件不响应               | 方法未标注 `#[command]`          | 给 UI 调用的方法加 `#[command]`        |
| 绑定无值                | 字段未声明为 `pub`                | ViewModel 字段必须 `pub`            |
| 热重载不生效              | 未调用 `with_hot_reload`       | 在 `main.rs` 中启用热重载              |

📋 **清单**：完成本节后，你应该能够：

- [ ] 创建三件套文件结构
- [ ] 用 `#[derive(Model)]` + `#[component]` 定义 ViewModel
- [ ] 用 `#[command]` 暴露方法给 UI
- [ ] 在 `.rml` 中使用 `{}`、`if`、`onclick` 三种基础语法
- [ ] 启用热重载并体验实时更新

下一节 → [1.4 与原生 GPUI 的对比](./comparison.md)
