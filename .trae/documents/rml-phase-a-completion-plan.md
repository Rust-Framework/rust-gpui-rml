# RML Phase A MVP 收尾计划

> **目标**：完成 Phase A MVP 闭环，让 `cargo build` 全通过 + `cargo run -p rml-demo` 窗口可打开。
> **范围**：仅补齐尚未实现的 Layer 3 / Layer 5 / Layer 6 + 验证。底层 trait / 宏 / 解析器 / 编译器（Layer 0-2）已在上一轮完成。
> **依据**：基于对 `docs/` 11 章文档与现有代码的逐文件核对，遵循已批准的 `rml-framework-architecture-plan.md`。
> **铁律**：所有 trait 以 `I` 开头；`#![forbid(unsafe_code)]`；生成代码只写 `OUT_DIR`；过程宏不做重活。

---

## 一、当前状态分析（Phase 1 探勘结论）

### 1.1 已完成 ✅

| 层 | 组件 | 关键文件 |
|----|------|---------|
| Layer 0 | `rml-core` 全部 trait | `crates/core/src/{model,view_model,view,command,component,event,events,lifecycle,binding,converter,two_way_binding,element_ref}.rs` + `prelude.rs`（含 `ViewContext<T>=Context<T>` 别名） |
| Layer 1 | `rml-macros` 7 个过程宏 | `crates/macros/src/{lib,derive_model,view,command,computed,lifecycle}.rs`（`#[element]` 为 `#[derive(IModel)]` helper attribute） |
| Layer 2 | `rml/parser` | `crates/rml/src/parser/{mod,ast,tokenizer}.rs` |
| Layer 2 | `rml/compiler` | `crates/rml/src/compiler/{mod,validator,codegen}.rs`，输出 `impl gpui::Render for <View>` |
| Layer 2 | `rml/tags` | `crates/rml/src/tags.rs`（19 个内置标签，Phase A 统一 `gpui::div()`） |
| Layer 4 | `rml/runtime` stub | `crates/rml/src/runtime/{mod,event_flow,component_registry,styling,watcher}.rs` |

### 1.2 阻塞编译的缺口 ❌（必须修复）

通过 `LS` 验证发现，**当前 workspace 无法编译**，存在 3 处悬空模块引用：

1. **`crates/rml/src/lib.rs:8`** 声明 `pub mod build;`，但 `crates/rml/src/build/` 目录不存在。
2. **`crates/app/src/lib.rs:7-9`** 声明 `pub mod application; pub mod resources; pub mod window;`，但 `crates/app/src/` 下只有 `lib.rs`。
3. **根 `Cargo.toml:2`** members 含 `"demo"`，但 `demo/` 目录不存在。

`crates/rml/src/prelude.rs:8` 还 `pub use crate::build::build as rml_build;` —— 同样悬空。

### 1.3 关键约束（来自探勘）

- **codegen 输出**：`impl gpui::Render for <View> { fn render(&mut self, _window: &mut gpui::Window, cx: &mut gpui::Context<Self>) -> impl gpui::IntoElement { ... } }`
- **`#[view]` 宏包裹**：生成 `const _: () = { include!(concat!(env!("OUT_DIR"), "/rml_generated/<snake>.rs")); };` —— 故 build.rs 输出文件只需含 `impl Render` 块本身。
- **输出路径约定**：`OUT_DIR/rml_generated/<snake_case_struct_name>.rs`（如 `Counter` → `counter.rs`）。
- **模板路径约定**：`<snake_case_struct_name>.rml`（如 `Counter` → `counter.rml`），从 `scan_dir` 递归查找。
- **`IRmlView: IViewModel`，`IViewModel: IModel + ILifecycle`**：`#[view]` 宏已生成全部 impl，demo 无需手写。
- **构造根视图**：`RmlApplication::run::<R>()` 需在内部构造 `R`。采用 `R: Default` bound（demo `Counter { count: i32 }` 可 `#[derive(Default)]`，count 默认 0 等价 `new()`）。

---

## 二、实施任务（按依赖顺序）

### Task 1 · Layer 3：`rml/build` 模块（解除 rml crate 编译阻塞）

**新建 3 个文件**：

#### 2.1.1 `crates/rml/src/build/mod.rs`
Builder API + 主流程入口。

```rust
//! RML 构建集成
//!
//! 在用户 build.rs 中调用，扫描 .rml、调用编译器、输出到 OUT_DIR。
//! 详见文档 §10.4 构建流程。

pub mod cache;
pub mod scanner;

use crate::compiler::{compile, CodegenCtx};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct BuildError { pub message: String }
impl std::fmt::Display for BuildError { /* ... */ }
impl std::error::Error for BuildError {}

pub struct Builder {
    scan_dirs: Vec<PathBuf>,
    output_dir: Option<PathBuf>,
    namespace: Option<String>,
    strict: bool,
    hot_reload: bool,
    public: bool,
}

pub fn build() -> Builder { Builder::new() }

impl Builder {
    pub fn new() -> Self { /* 默认 scan_dirs=["src"], strict=true, 其余 false/None */ }
    pub fn scan_dir(mut self, dir: impl Into<PathBuf>) -> Self { ... }
    pub fn output_dir(mut self, dir: impl Into<PathBuf>) -> Self { ... }
    pub fn namespace(mut self, ns: impl Into<String>) -> Self { ... }
    pub fn strict(mut self, on: bool) -> Self { ... }
    pub fn hot_reload(mut self, on: bool) -> Self { ... }
    pub fn public(mut self, on: bool) -> Self { ... }
    pub fn build(self) -> Result<(), BuildError> { ... }
}
```

**`build()` 主流程**：
1. `output_dir` 必须来自 `env::var("OUT_DIR")`，否则报错。生成目录 `<output_dir>/rml_generated/`。
2. 用 `scanner::scan` 递归收集 `scan_dirs` 下所有 `*.rml` 文件。
3. 对每个 `.rml` 打印 `cargo:rerun-if-changed=<path>`。
4. 读取 `<output_dir>/rml_cache.json`（`cache::Cache`），对每个文件算 sha256，命中缓存则跳过。
5. 对每个待编译文件：
   - 文件名 → 视图结构名：`counter.rml` → `Counter`（snake_case → PascalCase）
   - 读源码，调 `compile(source, &CodegenCtx { view_struct_name, view_module_path: namespace.clone().unwrap_or_default() })`
   - 写入 `<output_dir>/rml_generated/<snake>.rs`
   - 失败时 `println!("cargo:warning=RML error in {path}: {err}")` 并返回 `Err`
6. 写回 `rml_cache.json`，对其打印 `cargo:rerun-if-changed`。
7. `namespace`/`public`/`hot_reload`/`strict` 在 Phase A 仅做记录，不影响生成代码（Phase B 才用）。

#### 2.1.2 `crates/rml/src/build/scanner.rs`
```rust
//! 递归扫描 .rml 文件
use std::path::{Path, PathBuf};

pub fn scan(dirs: &[PathBuf]) -> Vec<PathBuf> {
    // 用 walkdir::WalkDir 递归，过滤扩展名 .rml，排序保证稳定
}
```

#### 2.1.3 `crates/rml/src/build/cache.rs`
```rust
//! 增量缓存（JSON）
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Default)]
pub struct Cache {
    /// 相对路径 → sha256 hex
    pub entries: HashMap<String, String>,
}

impl Cache {
    pub fn load(path: &Path) -> Self { /* 读 JSON，失败返回 default */ }
    pub fn save(&self, path: &Path) -> std::io::Result<()> { /* 写 JSON */ }
}
```

**依赖确认**：`crates/rml/Cargo.toml` 已含 `serde/serde_json/walkdir/sha2`（上一轮已加），无需改 Cargo.toml。

#### 2.1.4 `crates/rml/src/lib.rs` 无需改
已声明 `pub mod build;`，补齐文件即可。

---

### Task 2 · Layer 5：`rml-app` 模块（解除 app crate 编译阻塞）

**新建 3 个文件**：

#### 2.2.1 `crates/app/src/application.rs`
```rust
//! RmlApplication —— 应用启动器
//!
//! 封装 GPUI Application + 单窗口创建。
//! 详见文档 §1.3.6 入口编写。

use gpui::{
    App, Application, Context, Entity, IntoElement, Pixels, Render, SharedString, TitlebarOptions,
    Window, WindowBounds, WindowOptions,
};
use rml_core::view::IRmlView;
use std::marker::PhantomData;

pub struct RmlApplication {
    title: SharedString,
    width: Pixels,
    height: Pixels,
}

impl RmlApplication {
    pub fn new() -> Self {
        Self { title: "RML App".into(), width: px(800.), height: px(600.) }
    }
    pub fn title(mut self, t: impl Into<SharedString>) -> Self { self.title = t.into(); self }
    pub fn size(mut self, w: Pixels, h: Pixels) -> Self { self.width = w; self.height = h; self }

    /// 启动应用，以 R 为根视图。
    /// R 必须实现 IRmlView + Render + Default（Default 用于构造初始实例）。
    pub fn run<R>(self)
    where
        R: IRmlView + Render + Default + 'static,
    {
        let title = self.title;
        let (w, h) = (self.width, self.height);
        Application::new().run(move |cx: &mut App| {
            let bounds = WindowBounds::Windowed(gpui::Bounds {
                origin: Default::default(),
                size: gpui::Size { width: w, height: h },
            });
            let options = WindowOptions {
                window_bounds: Some(bounds),
                titlebar: Some(TitlebarOptions { title: Some(title.clone()), ..Default::default() }),
                ..Default::default()
            };
            cx.open_window(options, |_window, cx| {
                cx.new(|_cx| R::default())
            }).expect("failed to open window");
        });
    }
}

impl Default for RmlApplication {
    fn default() -> Self { Self::new() }
}

fn px(f: f32) -> Pixels { Pixels(f) }
```

> **注**：`cx.new(|_cx| R::default())` 创建 `Entity<R>`。`open_window` 闭包返回 `Entity<R>` 即可，GPUI 会自动要求 `R: Render`。若 GPUI 实际 API 签名要求 `impl IntoElement`，则改为返回 `cx.new(|_cx| R::default())`（Entity 实现 IntoElement）。实现时以实际 GPUI git 版本 API 为准，必要时调整闭包返回类型。

#### 2.2.2 `crates/app/src/window.rs`
Phase A stub（多窗口 Phase B）：
```rust
//! 窗口管理 helper（Phase A stub，Phase B 实现多窗口）
```
仅含模块文档注释，确保 `pub mod window;` 编译通过。

#### 2.2.3 `crates/app/src/resources.rs`
Phase A stub：
```rust
//! 资源加载（Phase A stub，Phase B 实现图标/字体/i18n）
```

**依赖确认**：`crates/app/Cargo.toml` 已含 `rml-core` + `gpui`，无需改。

---

### Task 3 · Layer 6：`demo` 包（解除 workspace 编译阻塞 + 验证闭环）

**新建 5 个文件**：

#### 2.3.1 `demo/Cargo.toml`
```toml
[package]
name = "rml-demo"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
rml = { workspace = true }
rml-app = { workspace = true }
rml-core = { workspace = true }
gpui = { workspace = true }

[build-dependencies]
rml = { workspace = true }
```

#### 2.3.2 `demo/build.rs`
```rust
fn main() {
    rml::build()
        .scan_dir("src")
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

#### 2.3.3 `demo/src/counter.rml`
```html
<div class="counter">
    <h1>计数器</h1>
    <p class="count">{count}</p>
    <div class="buttons">
        <button onclick={decrement}>-</button>
        <button onclick={increment}>+</button>
    </div>
</div>
```

> 与现有 codegen 能力对齐：`{count}` → `Label::new(format!("{}", self.count))`；`onclick={increment}` → `on_click(cx.listener(...))`。不含 `if`/`each`/`model`，规避 Phase A codegen 未完成的指令。

#### 2.3.4 `demo/src/counter.rml.rs`
```rust
use rml::prelude::*;

#[derive(IModel, Default)]
#[view]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count -= 1;
        cx.notify();
    }
}
```

> - `#[derive(IModel, Default)]`：`IModel` 由宏生成字段元信息；`Default` 让 `count: i32` 默认 0（等价 `new()`），供 `RmlApplication::run::<Counter>()` 构造。
> - 不写 `new()`，统一用 `Default` 构造，与 `run` 的 bound 对齐。
> - `#[command]` Phase A 为 pass-through，仅校验签名。
> - `Context<Self>` 用 core prelude 的别名（文档兼容 `ViewContext`）。

#### 2.3.5 `demo/src/main.rs`
```rust
use rml_app::RmlApplication;
mod counter;

fn main() {
    RmlApplication::new()
        .title("RML Counter Demo")
        .size(px(400.), px(300.))
        .run::<counter::Counter>();
}

fn px(f: f32) -> gpui::Pixels { gpui::Pixels(f) }
```

---

### Task 4 · 验证：编译 + 运行

按顺序执行（每步必须通过才进下一步）：

1. `cargo build -p rml-core` —— trait 层编译
2. `cargo build -p rml-macros` —— 过程宏编译
3. `cargo build -p rml` —— 含 build/runtime/compiler，必须通过
4. `cargo build -p rml-app` —— 启动器编译
5. `cargo build -p rml-demo` —— 触发 build.rs，生成 `OUT_DIR/rml_generated/counter.rs`，编译 demo
6. `cargo run -p rml-demo` —— 窗口打开，显示"计数器"标题、当前数字 0、点击 +/- 数字变化

**失败排查路径**：
- 若 step 5 build.rs 报错 → 检查 `OUT_DIR/rml_generated/counter.rs` 是否生成、内容是否为合法 `impl Render`
- 若 step 5 编译失败 → 用 `cargo expand -p rml-demo counter` 查看 `#[view]` 展开后的 `include!` 路径与生成代码
- 若 GPUI API 签名不符 → 调整 `application.rs` 的 `open_window` 闭包返回类型（`Entity<R>` vs `impl IntoElement`）

---

## 三、假设与决策

| 决策点 | 选择 | 理由 |
|--------|------|------|
| 根视图构造方式 | `R: Default` bound | `Counter { count: i32 }` 默认 0 等价 `new()`；避免引入新 trait |
| demo 模板不含 if/each/model | 简化为 div+p+button | 现有 codegen 对 if/each/model 仅 stub，会生成无效代码；Phase A 闭环优先 |
| `namespace`/`public`/`hot_reload` | Phase A 仅记录不生效 | 文档 §10.4 这些是 Phase B 特性，先打通主流程 |
| 缓存格式 | JSON（`rml_cache.json`） | 与文档 §10.4.5 一致 |
| 错误输出 | `cargo:warning=RML error in ...` | 文档 §10.4.10 约定 |
| `build()` 入口函数名 | `rml::build()` 返回 `Builder` | 与 `prelude.rs` 现有 `pub use crate::build::build as rml_build;` 一致 |
| 文件名 → 视图名转换 | snake_case → PascalCase | `counter.rml` → `Counter`，与 `#[view]` 宏的 `to_snake_case` 反向 |
| `crates/app` 不依赖 `rml` | 仅依赖 `rml-core` + `gpui` | app 层只需 `IRmlView` marker，不需要编译器 |

---

## 四、与文档的功能匹配度（Phase A 范围内）

| 文档章节 | 验收项 | 本计划覆盖 |
|---------|--------|-----------|
| §1.2 三层架构 | 5 crate 依赖正确 | ✅ 补齐 build/app/demo 后全通 |
| §1.3 快速开始 | 三件套 + build.rs + main | ✅ demo 完整覆盖（API 名称按铁律用 `IModel` 而非文档的 `Model`） |
| §2.1 标签映射 | 内置 HTML 标签 | ✅ tags.rs 已有 19 个 |
| §2.3 属性系统 | 标准/绑定/事件/指令 | ✅ parser 已分类 |
| §2.5 插值 | 文本/属性/混合插值 | ✅ parser+codegen 已实现 |
| §3.1 单向绑定 | `{field}` | ✅ codegen 生成 `format!("{}", self.field)` |
| §4.1 ViewModel | `#[derive(IModel)]` + `#[view]` | ✅ 宏已实现 |
| §4.4 命令 | `#[command]` 可被 onclick 调用 | ✅ pass-through + codegen 生成 `cx.listener` |
| §5.1 事件绑定 | onclick 等 | ✅ codegen 生成 `on_click` |
| §10.4 build.rs | Builder API | ✅ Task 1 |
| §10.6 代码生成 | `OUT_DIR/rml_generated/<name>.rs` | ✅ Task 1 主流程 |

Phase B 范围（else/html/slot 指令、computed 缓存、converter、绑定引擎校验、element 联动、事件流三阶段、防抖、组件系统、样式系统、生命周期自动注入、热重载、LSP）不在本计划内，留待下一阶段。

---

## 五、交付物清单

**新建文件（8 个）**：
```
crates/rml/src/build/mod.rs          # Builder API + 主流程
crates/rml/src/build/scanner.rs      # .rml 递归扫描
crates/rml/src/build/cache.rs        # 增量缓存 JSON
crates/app/src/application.rs        # RmlApplication 启动器
crates/app/src/window.rs             # stub
crates/app/src/resources.rs          # stub
demo/Cargo.toml                      # demo 包配置
demo/build.rs                        # demo 构建脚本
demo/src/main.rs                     # demo 入口
demo/src/counter.rml                 # counter 模板
demo/src/counter.rml.rs              # counter ViewModel
```

**修改文件（0 个）**：所有依赖（Cargo.toml、lib.rs 声明）已在上一轮就绪，本轮纯新建补齐。

**验证里程碑**：`cargo run -p rml-demo` 窗口可打开且计数器可点击。
