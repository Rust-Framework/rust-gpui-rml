# 10.4 构建流程

> **本节目标**：理解 RML 的构建集成机制，掌握 build.rs 配置（含 `.assets()` 资源双模式）、OUT_DIR 输出、`#[rml::main]` 单点入口与自定义代码生成。

## 10.4.1 构建流程总览

```
┌──────────────────────────────────────────────────────────┐
│                    RML 构建流程                            │
│                                                          │
│  cargo build                                             │
│      │                                                   │
│      ▼                                                   │
│  build.rs 执行                                            │
│      │                                                   │
│      ├── 扫描 src/**/*.rml 与 src/**/*.rml.rs            │
│      ├── syn 扫描 .rml.rs 提取元信息（computed/validate） │
│      ├── 解析 .rml 为 AST                                 │
│      ├── 语义验证（绑定路径、命令签名、组件引用）         │
│      ├── 生成 Rust 代码到 OUT_DIR/rml_generated/         │
│      ├── 生成资源注册代码到 OUT_DIR/rml_assets.rs         │
│      │   └── 内含 #[ctor::ctor] 函数（main 前自动注册）   │
│      └── 告知 cargo 重新编译                              │
│      │                                                   │
│      ▼                                                   │
│  cargo 编译生成的代码 + 用户代码                            │
│      │                                                   │
│      ▼                                                   │
│  可执行文件（含嵌入资源 + 已注册的资源源）                 │
└──────────────────────────────────────────────────────────┘
```

RML 的构建发生在 `build.rs` 中，是 Cargo 标准的构建脚本机制。

## 10.4.2 最小 build.rs

```rust
// build.rs
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

`rml::build()` 返回一个 Builder，链式配置后调用 `.build()` 触发编译。

> 注：包名统一为 `rust-rml-*` 前缀，源码中通过 `extern crate rust_rml_engine as rml` 别名引用。

## 10.4.3 配置选项

### 扫描目录

```rust
rml::build()
    .scan_dir("src/views")
    .scan_dir("src/components")
    .scan_dir("shared/templates") // 跨 crate 共享模板
```

默认扫描 `src`，可多次调用 `scan_dir` 增加目录。扫描同时识别 `.rml`（模板）与 `.rml.rs`（code-behind，用于提取 `#[computed]` / `#[validate]` 元信息）。

### 输出目录

```rust
.output_dir(std::env::var("OUT_DIR").unwrap())
```

生成的代码默认输出到 Cargo 指定的 `OUT_DIR/rml_generated/`，由 `#[component]` / `#[window]` 宏内部 `include!` 引入。**不要**输出到 `src/` 下，避免污染源码。

### 资源目录与双模式（核心）

`.assets(dir, embed)` 注册资源根目录并指定嵌入模式：

```rust
// 嵌入模式：编译期 include_bytes! 嵌入二进制
rml::build()
    .scan_dir("src")
    .assets("assets", true)
    .output_dir(std::env::var("OUT_DIR").unwrap())
    .build()

// 文件系统模式：运行期从磁盘读取，首次读取后 Box::leak 缓存
rml::build()
    .scan_dir("src")
    .assets("assets", false)   // false 是默认值，可省略 embed 参数语义
    .output_dir(std::env::var("OUT_DIR").unwrap())
    .build()
```

| 模式 | 二进制大小 | 资源泄露 | 适用场景 |
|---|---|---|---|
| `embed=true` | 大（资源全打入） | 无 | 发布构建、单文件分发 |
| `embed=false` | 小（仅代码） | 有（`Box::leak` 缓存） | 开发期、不关心泄露的内部工具 |

两种模式运行时 API 完全一致，均通过 `rml_core::assets::load(path)` / `load_str(path)` 查询，路径以相对 `assets/` 的正斜杠形式（如 `"themes/dark.css"`、`"i18n/zh-CN.json"`）。

**自动注册机制**：build.rs 会在 `OUT_DIR/rml_assets.rs` 生成一个 `#[rml_core::ctor::ctor]` 函数，在 `main` 之前自动调用 `rml_core::assets::init(AssetSource::...)` 完成注册。配合 `main.rs` 上的 `#[rml::main]` 属性宏（自动 `include!` 该文件），用户**无需手写任何资源初始化代码**。

### CSS 样式表

```rust
.with_style("styles/main.css")      // 显式注册（可多次，后者优先级更高）
```

除显式注册外，build.rs 还会自动扫描 `assets_dir` 根目录下的 `.css` 文件（不递归，避免误加载 `themes/` 子目录）。所有 CSS 合并为一个全局 `StyleSheet`，`:root` 变量跨文件共享。

### i18n 键提取

```rust
.extract_i18n("assets/i18n/zh-CN.json")
```

扫描 `.rml` 中的 `t("key")` 调用，合并写入指定 JSON 文件（缺失 key 以 key 自身为默认值）。

### 严格模式

```rust
.strict(true) // 默认 true，把警告升级为错误
```

严格模式下，未使用的绑定、悬空的 `if` 等会直接编译失败。

### 命名空间

```rust
.namespace("my_app::views")
```

生成的代码会放在指定模块路径下，便于组织。

## 10.4.4 `#[rml::main]` 单点入口

build.rs 生成的 `rml_assets.rs` 需要在 `main.rs` 中 `include!` 引入，以便其中的 `#[ctor::ctor]` 函数被链接到最终二进制。RML 提供 `#[rml::main]` 属性宏自动完成这一步：

```rust
// src/main.rs
extern crate rust_rml_engine as rml;
extern crate rust_rml_app as rml_app;

#[rml::main]   // 自动注入：rml::embed_assets!();
fn main() {
    rml_app::RmlApplication::new()
        .main_window::<MyWindow>()
        .run();
}
```

`#[rml::main]` 展开为：

```rust
rml::embed_assets!();   // include!(concat!(env!("OUT_DIR"), "/rml_assets.rs"));
fn main() { ... }
```

> ⚠️ 若 `main.rs` 未标注 `#[rml::main]` 且未手写 `rml::embed_assets!()`，则 `rml_assets.rs` 不会被链接，运行时 `rml_core::assets::load()` 永远返回 `None`。

## 10.4.5 OUT_DIR 与 include!

`#[component]` / `#[window]` 宏内部会自动展开为类似下面的 `include!`：

```rust
// 由 #[window] 宏生成（用户无需手写）
include!(concat!(env!("OUT_DIR"), "/rml_generated/main_window.rs"));
```

生成文件含 `impl Render for MainWindow { ... }`，必须位于模块顶层（不能在 `const _: () = { ... }` 块内）。

## 10.4.6 增量编译

RML 编译器记录每个 `.rml` 文件及其 `.rml.rs` code-behind 的 sha256 哈希，任一未变化则跳过重新生成。同时记录 engine 源码哈希——engine 任何 `src/**/*.rs` 变化会让缓存中的所有条目失效，强制重新生成（确保 codegen 行为变更立即生效）。

### 强制全量重新生成

```sh
cargo clean -p my-app && cargo build
```

或删除 `OUT_DIR` 中的 `rml_cache.json`。

## 10.4.7 依赖追踪

build.rs 通过 `cargo:rerun-if-changed` 告知 Cargo 哪些文件变化时需要重新执行：

```rust
// rml::build() 内部已处理，用户无需关心
println!("cargo:rerun-if-changed=src/views/login/login.rml");
println!("cargo:rerun-if-changed=src/views/login/login.rml.rs");
println!("cargo:rerun-if-changed=assets/themes/dark.css");
```

修改 `.rml` / `.rml.rs` / `assets/` 下文件会触发 build.rs 重新执行。

## 10.4.8 与 Cargo 特性集成

### 跨 crate 共享模板

```rust
// ui-kit crate 的 build.rs
rml::build()
    .scan_dir("src")
    .output_dir(std::env::var("OUT_DIR").unwrap())
    .public(true) // 生成的代码标记为 pub，供下游 crate 使用
    .build();
```

下游 crate：

```rust
// app crate
use ui_kit::Button; // 直接使用上游生成的组件
```

## 10.4.9 构建产物检查

### 查看生成的代码

```sh
ls target/debug/build/my-app-*/out/rml_generated/
cat target/debug/build/my-app-*/out/rml_assets.rs   # 资源注册代码
```

### 构建时间分析

```sh
cargo build --timings
```

打开 `target/cargo-timings/cargo-timing.html` 查看 build.rs 耗时占比。

## 10.4.10 常见构建问题

### 找不到 .rml 文件

- 检查 `scan_dir` 路径是否正确
- 检查文件扩展名是否为 `.rml`（不是 `.rml.rs`）

### 生成的代码编译失败

- 查看 `OUT_DIR/rml_generated/<view>.rs` 排查
- 检查绑定路径是否在 ViewModel 中存在
- 检查命令签名是否匹配事件

### 资源加载返回 None

- `main.rs` 是否标注 `#[rml::main]`（或手写 `rml::embed_assets!()`）
- `build.rs` 是否调用 `.assets("assets", ...)` 且 `assets/` 目录存在
- 运行时 `load("themes/dark.css")` 路径是否相对 `assets/` 的正斜杠形式

### build.rs 死循环

- 检查是否在 build.rs 中写入了被 `rerun-if-changed` 监听的文件
- 生成的代码必须输出到 `OUT_DIR`，不能输出到 `src/`

### 增量编译失效

- 检查 `OUT_DIR` 是否被外部清理
- 检查 `rml_cache.json` 是否可写
- engine 源码变化会自动失效全部缓存（这是预期行为）

下一节 → [10.5 IDE 支持](./ide-support.md)
