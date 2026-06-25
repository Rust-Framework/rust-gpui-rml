# 10.4 构建流程

> **本节目标**：理解 RML 的构建集成机制，掌握 build.rs 配置、OUT_DIR 输出与自定义代码生成。

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
│      ├── 扫描 src/**/*.rml 与 **/*.rmlcss                │
│      ├── 解析为 AST                                        │
│      ├── 语义验证（绑定路径、命令签名、组件引用）         │
│      ├── 生成 Rust 代码到 OUT_DIR                         │
│      └── 告知 cargo 重新编译                              │
│      │                                                   │
│      ▼                                                   │
│  cargo 编译生成的代码 + 用户代码                            │
│      │                                                   │
│      ▼                                                   │
│  可执行文件                                               │
└──────────────────────────────────────────────────────────┘
```

RML 的构建发生在 `build.rs` 中，是 Cargo 标准的构建脚本机制。

## 10.4.2 最小 build.rs

```rust
// build.rs
fn main() {
    rml_build::build()
        .scan_dir("src")
        .scan_dir("assets/templates")
        .output_dir(std::env::var("OUT_DIR").unwrap())
        .build();
}
```

`rml_build::build()` 返回一个 Builder，链式配置后调用 `.build()` 触发编译。

## 10.4.3 配置选项

### 扫描目录

```rust
rml_build::build()
    .scan_dir("src/views")
    .scan_dir("src/components")
    .scan_dir("shared/templates") // 跨 crate 共享模板
```

默认扫描 `src`，可多次调用 `scan_dir` 增加目录。

### 输出目录

```rust
.output_dir(std::env::var("OUT_DIR").unwrap())
```

生成的代码默认输出到 Cargo 指定的 `OUT_DIR`，由 `include!` 引入。**不要**输出到 `src/` 下，避免污染源码。

### 自定义命名空间

```rust
.namespace("my_app::views")
```

生成的代码会放在指定模块路径下，便于组织。

### 严格模式

```rust
.strict(true) // 默认 true，把警告升级为错误
```

严格模式下，未使用的绑定、悬空的 `r:if` 等会直接编译失败。

## 10.4.4 OUT_DIR 与 include!

生成的代码在 `OUT_DIR/rml_generated.rs`，用户通过 `include!` 引入：

```rust
// src/views/login/login.rml.rs
mod generated {
    include!(concat!(env!("OUT_DIR"), "/rml_generated/login.rs"));
}

pub use generated::LoginView;
```

RML 提供宏简化这一步：

```rust
#[rml::view("login.rml")]
pub struct LoginViewModel { ... }
```

`#[rml::view]` 宏自动展开为 `include!` + 类型定义。

## 10.4.5 增量编译

RML 编译器记录每个 `.rml` 文件的哈希，未变化的文件跳过重新生成：

```
[INFO rml_build] 扫描 42 个 .rml 文件
[INFO rml_build] 3 个文件变化，重新生成
[INFO rml_build] 39 个文件未变化，跳过
[INFO rml_build] 生成完成，耗时 120ms
```

增量编译使大型项目的构建时间保持在秒级。

### 强制全量重新生成

```sh
cargo clean -p my-app && cargo build
```

或删除 `OUT_DIR` 中的 `rml_cache.json`。

## 10.4.6 依赖追踪

build.rs 通过 `cargo:rerun-if-changed` 告知 Cargo 哪些文件变化时需要重新执行：

```rust
// rml_build 内部已处理，用户无需关心
println!("cargo:rerun-if-changed=src/views/login/login.rml");
println!("cargo:rerun-if-changed=src/components/button/button.rml");
```

修改 `.rml` 文件会触发 build.rs 重新执行。

## 10.4.7 自定义代码生成

RML 允许通过插件扩展代码生成。常见场景：

- 生成国际化键
- 生成设计令牌
- 生成组件文档

### 示例：生成 i18n 键

```rust
// build.rs
fn main() {
    rml_build::build()
        .scan_dir("src")
        .plugin(I18nExtractor::new("assets/i18n/zh-CN.json"))
        .build();
}

struct I18nExtractor { path: PathBuf }
impl rml_build::Plugin for I18nExtractor {
    fn on_template(&self, template: &TemplateAst) {
        // 扫描模板中的 {t("key")} 调用，收集 key
        // 写入 self.path
    }
}
```

## 10.4.8 与 Cargo 特性集成

### 仅在 debug 启用热重载

```rust
// build.rs
fn main() {
    let mut builder = rml_build::build().scan_dir("src");
    #[cfg(feature = "hot-reload")]
    {
        builder = builder.hot_reload(true);
    }
    builder.build();
}
```

```toml
# Cargo.toml
[features]
hot-reload = ["rml/hot-reload"]
```

### 跨 crate 共享模板

```rust
// ui-kit crate 的 build.rs
rml_build::build()
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

- 用 `cargo rml-expand` 看生成代码
- 检查绑定路径是否在 ViewModel 中存在
- 检查命令签名是否匹配事件

### build.rs 死循环

- 检查是否在 build.rs 中写入了被 `rerun-if-changed` 监听的文件
- 生成的代码必须输出到 `OUT_DIR`，不能输出到 `src/`

### 增量编译失效

- 检查 `OUT_DIR` 是否被外部清理
- 检查 `rml_cache.json` 是否可写

下一节 → [10.5 IDE 支持](./ide-support.md)
