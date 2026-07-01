# rust-rml-engine

> RML 解析引擎与编译器（`.rml` → Rust/GPUI 代码生成）。

## 职责

`rust-rml-engine` 是框架的核心引擎层，负责将 `.rml` 模板文件编译为原生 GPUI 渲染代码。包含四阶段流水线：词法分析 → AST 构建 → 语义验证 → 代码生成，以及构建集成（`build.rs` 支持，含资源处理与 i18n 提取）和运行时支持（事件流、组件注册表、样式、热重载）。

**核心约束**：
- `#![forbid(unsafe_code)]` 全 crate 启用
- 生成代码只写 `OUT_DIR`，禁止写 `src/`
- 过程宏不做重活，模板编译由 `build.rs` 调用 engine 完成
- 双轨制组件策略：原生轨（HTML 标签 → GPUI 原生元素）+ 扩展轨（`crates/ui` + feature flag 引入 `gpui-component`）

## 模块结构

```
engine/src/
├── lib.rs                  # crate 入口，pub extern crate 别名 + 模块声明
├── prelude.rs              # engine prelude，重导出 core/macros + build/compile
├── tags.rs                 # HTML 标签 → GPUI 构造器映射表（19 个内置标签）
├── parser/
│   ├── mod.rs              # 解析器入口（.rml → AST）
│   ├── tokenizer.rs        # 词法分析器（.rml → Token 流）
│   └── ast.rs              # AST 数据结构（Node/Element/Attribute/Directive）
├── compiler/
│   ├── mod.rs              # 编译器入口（parse → validate → codegen），含 CodegenError
│   ├── codegen.rs          # 代码生成器（AST → impl Render 源码）
│   ├── component.rs        # gpui-component 扩展组件 codegen
│   ├── event.rs            # 事件绑定 codegen（onhover 等特殊处理）
│   ├── expr.rs             # 表达式解析器（field/member/index/method-call/binary-op）
│   └── validator.rs        # 语义验证器
├── build/
│   ├── mod.rs              # Builder 入口（build.rs 调用），含 .assets(path, embed) API
│   ├── scanner.rs          # .rml / .rml.rs 文件递归扫描
│   ├── cache.rs            # 增量缓存（sha256 哈希 + code-behind 哈希 + engine 源码哈希）
│   ├── assets_processor.rs # 资源处理：扫描 assets/ 生成 rml_assets.rs（含 #[ctor::ctor] 自动注册）
│   └── i18n_extractor.rs   # i18n 键提取：扫描 .rml 中的 t("key") 合并写入 JSON
├── css/
│   ├── ast.rs              # CSS AST（StyleSheet/Rule/Declaration/Value/Selector）
│   ├── parser.rs           # 递归下降解析器（支持 :root 变量、var()）
│   ├── matcher.rs          # 选择器匹配 + 生成 styles
│   └── mapper.rs           # CSS 声明 → GPUI 方法调用映射
└── runtime/
    ├── mod.rs              # 运行时模块声明
    ├── event_flow.rs       # GPUI→RML 事件转换 + 三阶段调度
    ├── component_registry.rs # 全局组件注册表（Phase B）
    ├── styling.rs          # 样式运行时支持（Phase B）
    └── watcher.rs          # 热重载文件监听（Phase B）
```

## Features

| Feature | 默认 | 说明 |
|---------|------|------|
| `gpui-component` | 开启 | 引入 `gpui-component` 依赖，启用扩展组件支持 |
| `hot-reload` | 关闭 | 引入 `notify` 依赖，启用 `.rml` 文件热重载 |

## 构建集成（`build.rs`）

`rust-rml-engine` 在 `build.rs` 中通过 `extern crate rust_rml_engine as rml` 别名引入，调用 `rml::build()` 链式 API 配置。

### 资源双模式（核心）

`.assets(path, embed: bool)` 是资源管理的单点配置入口：

| 模式 | 调用 | 嵌入方式 | 二进制大小 | 运行时 API |
|------|------|----------|------------|-----------|
| 嵌入模式 | `.assets("assets", true)` | `include_bytes!` 编译期嵌入 | 较大 | `rml_core::assets::load` 返回 `Option<&'static [u8]>` |
| 文件系统模式 | `.assets("assets", false)` | 运行期按需磁盘读取 + `Box::leak` 缓存 | 较小 | `rml_core::assets::load` / `load_owned` |

资源注册由 `AssetsProcessor` 生成的 `rml_assets.rs` 中的 `#[ctor::ctor]` 函数在 `main` 之前自动完成，
`main.rs` 中无需手写初始化代码，但需通过 `#[rml::main]` 属性宏注入 `rml::embed_assets!()` 来触发 `include!`。

### 最小 build.rs

```rust
extern crate rust_rml_engine as rml;

fn main() {
    rml::build()
        .scan_dir("src")                       // 扫描 .rml 与 .rml.rs
        .assets("assets", true)                 // 嵌入模式；false = 文件系统模式
        .with_style("styles/main.css")          // 可选：注册 CSS 样式表（可多次调用）
        .extract_i18n("assets/i18n/zh-CN.json") // 可选：提取 i18n 键
        .output_dir(std::env::var("OUT_DIR").expect("OUT_DIR not set"))
        .build()
        .expect("RML build failed");
}
```

## 设计规范

1. **生成代码风格**：全限定路径（`gpui::div()` 而非 `div()`），函数内 `use` 引入 trait，避免与用户 import 冲突
2. **事件转换**：GPUI 事件 → RML 事件使用自由函数（`from_gpui_click` 等），避免 orphan 规则冲突；`onhover` 因回调签名差异单独处理
3. **元素 ID**：有事件处理器的元素自动 `.id(("rml_el", n))` 成为 `Stateful<Div>`，满足 `StatefulInteractiveElement` 要求
4. **条件渲染**：`if`/`show` 指令用 `if/else` + `into_any_element()` 实现真正条件渲染，而非 `.when()`（后者不会隐藏元素）
5. **增量缓存**：`.rml` 文件 sha256 哈希 + `.rml.rs` code-behind 哈希 + engine 源码哈希三层校验，任一变化触发重新生成
6. **指令零前缀**：`if`/`each`/`model`/`show`/`once`/`html`/`ref`/`slot`/`else`/`key` 无冒号前缀
7. **事件属性**：`on*` 无冒号前缀（`onclick`/`oninput`/`onchange`）
8. **资源单点配置**：资源模式仅由 build.rs 的 `.assets(path, embed)` 决定，生成的 `rml_assets.rs` 中 `#[ctor::ctor]` 函数自动注册到 `rml_core::assets::init`，运行时 API 统一
9. **CSS 自动发现**：除 `.with_style()` 显式注册的文件外，自动扫描 `assets_dir` 根目录下的 `.css` 文件（不递归子目录，避免误加载 `themes/` 等主题文件）
10. **MVVM 数据驱动 codegen**：codegen 通过 `CodegenCtx` 携带 observable_fields / computed_methods / computed_deps / computed_returns / field_validations 等元信息，生成 `__rml_bump_version` / `__rml_get_version` / `__rml_computed_deps_version` 等版本管理方法，支撑 `#[command]` 宏的自动 notify 与 `#[computed]` 缓存失效
