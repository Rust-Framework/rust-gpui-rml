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
rml = { git = "https://github.com/your-org/rml-framework.git" }       # 引擎 + 过程宏
rml-app = { git = "https://github.com/your-org/rml-framework.git" }  # 应用启动器
gpui = { git = "https://github.com/zed-industries/zed.git" }
gpui-component = { git = "https://github.com/your-org/gpui-component.git" }

[build-dependencies]
rml = { git = "https://github.com/your-org/rml-framework.git" }      # build.rs 调用同一 crate
```

> 注：包名统一为 `rust-rml-*` 前缀（ crates 各自的 `Cargo.toml`），但通过 `extern crate rust_rml_engine as rml` 别名机制，用户源码中可以直接 `use rml::prelude::*`。

## 1.3.3 配置 build.rs

在项目根目录创建 `build.rs`，告诉 RML 编译器去哪里找 `.rml` 文件，并声明资源目录：

```rust
// build.rs
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")                          // 扫描 .rml 与 .rml.rs
        .assets("assets", true)                    // 嵌入模式：CSS/i18n 等打入二进制
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

💡 **资源两种模式**：`.assets(path, true)` 编译期 `include_bytes!` 嵌入二进制（无资源泄露，二进制较大）；`.assets(path, false)` 运行期按需从磁盘读取并 `Box::leak` 缓存（二进制小，不关心资源泄露）。两种模式运行时 API 一致（`rml_core::assets::load(path)`）。资源注册由 build.rs 生成的 `#[ctor::ctor]` 函数在 `main` 之前自动完成，main.rs 中无需任何资源初始化代码。

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
        <button class="btn primary" on-click={increment}>➕ 增加</button>
        <button class="btn danger" on-click={decrement} if={count > 0}>➖ 减少</button>
        <button class="btn secondary" on-click={reset}>↺ 重置</button>
    </div>
</div>
```

**逐行解读**：

- `{count}` —— 单向绑定，显示 ViewModel 的 `count` 字段
- `if={count > 10}` —— 条件渲染指令，表达式为真时渲染
- `on-click={increment}` —— 事件绑定，调用 ViewModel 的 `increment` 命令
- `class="btn primary"` —— 标准 HTML class 属性，映射到 GPUI 样式

## 1.3.5 编写业务逻辑（`.rml.rs`）

创建 `src/views/counter.rml.rs`：

```rust
// src/views/counter.rml.rs
use rml::prelude::*;

#[derive(IModel)]
#[component]  // 极简宏：标记为 RML 组件
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
        // 宏自动注入：self.__rml_bump_version("count"); cx.notify();
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        if self.count > 0 {
            self.count -= 1;
            // 宏自动注入：bump_version("count") + cx.notify()
        }
    }

    #[command]
    pub fn reset(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count = 0;
        // 宏自动注入：bump_version("count") + cx.notify()
    }
}
```

**关键点**：

- `#[derive(IModel)]` —— 让结构体成为 GPUI Entity，字段自动成为响应式状态
- `#[component]` —— 标记为 RML 组件，编译器会为其生成 `Render` 实现
- `#[command]` —— 标记方法为 UI 可调用的命令，`.rml` 中的 `on-click={increment}` 直接绑定到这里
- **MVVM 数据驱动**：宏自动追踪 `self.<field>` 的修改并自动注入 `bump_version` + `cx.notify()`，**用户无需手写 `cx.notify()`**

## 1.3.6 编写入口（`main.rs`）

创建 `src/main.rs`：

```rust
// src/main.rs
extern crate rust_rml_engine as rml;     // 引入 rml（提供 #[rml::main]）
extern crate rust_rml_app as rml_app;    // 引入 rml_app（RmlApplication）

mod views;

// `#[rml::main]` 自动注入 `rml::embed_assets!()`（include build.rs 生成的 rml_assets.rs）。
// 生成文件内的 `#[ctor::ctor]` 函数在 main 之前自动调用 `rml_core::assets::init(...)`,
// 因此此处无需手写资源初始化代码。模式（嵌入/文件系统）由 build.rs 的 `.assets(path, embed)` 决定。
#[rml::main]
fn main() {
    rml_app::RmlApplication::new()
        .main_window::<views::counter::Counter>()
        .run();
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

## 1.3.8 体验热重载（开发中）

> ⚠️ 热重载属于 Phase 4 路线图能力，当前版本未实现。修改 `.rml` 文件后仍需 `cargo run` 重新编译。独立的 `.rml` 文件设计为未来热重载奠定了天然基础。

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
| 编译报错"找不到 RmlView"   | `build.rs` 未配置或 `scan_dir` 路径错误 | 检查 `build.rs` 中的路径              |
| UI 不更新              | 间接修改字段（如 `let p = &mut self.x; *p = 1;` 或方法调用 `self.items.retain()`）宏无法识别 | 改为直接 `self.<field> = ...` 赋值，或在方法末尾手动 `cx.notify()` |
| UI 不更新（异步任务）         | `cx.spawn` 闭包内修改字段，宏不注入 notify | 闭包内 `this.update(...)` 末尾手动 `cx.notify()` |
| 事件不响应               | 方法未标注 `#[command]`          | 给 UI 调用的方法加 `#[command]`        |
| 绑定无值                | 字段未声明为 `pub`                | ViewModel 字段必须 `pub`            |
| 资源加载失败              | `assets/` 目录未配置或路径不对 | `build.rs` 中 `.assets("assets", true)`；运行时用 `rml_core::assets::load("themes/dark.css")` |

📋 **清单**：完成本节后，你应该能够：

- [ ] 创建三件套文件结构
- [ ] 用 `#[derive(IModel)]` + `#[component]` 定义 ViewModel
- [ ] 用 `#[command]` 暴露方法给 UI（无需手写 `cx.notify()`）
- [ ] 在 `.rml` 中使用 `{}`、`if`、`on-click` 三种基础语法
- [ ] 配置 `build.rs` 的 `.assets(path, embed)` 双模式资源

下一节 → [1.4 与原生 GPUI 的对比](./comparison.md)
