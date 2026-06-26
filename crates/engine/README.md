# rust-rml-engine

> RML 解析引擎与编译器（`.rml` → Rust/GPUI 代码生成）。

## 职责

`rust-rml-engine` 是框架的核心引擎层，负责将 `.rml` 模板文件编译为原生 GPUI 渲染代码。包含四阶段流水线：词法分析 → AST 构建 → 语义验证 → 代码生成，以及构建集成（`build.rs` 支持）和运行时支持（事件流、组件注册表、样式、热重载）。

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
│   ├── mod.rs              # 编译器入口（parse → validate → codegen）
│   ├── codegen.rs          # 代码生成器（AST → impl Render 源码）
│   └── validator.rs        # 语义验证器
├── build/
│   ├── mod.rs              # 构建集成入口（build.rs 调用）
│   ├── scanner.rs          # .rml 文件递归扫描
│   └── cache.rs            # 增量缓存（sha256 哈希）
└── runtime/
    ├── mod.rs              # 运行时模块声明
    ├── event_flow.rs       # GPUI→RML 事件转换 + 三阶段调度
    ├── component_registry.rs # 全局组件注册表（Phase B）
    ├── styling.rs          # CSS 子集解析（Phase B）
    └── watcher.rs          # 热重载文件监听（Phase B）
```

## Features

| Feature | 默认 | 说明 |
|---------|------|------|
| `gpui-component` | 开启 | 引入 `gpui-component` 依赖，启用扩展组件支持 |
| `hot-reload` | 关闭 | 引入 `notify` 依赖，启用 `.rml` 文件热重载 |

## 设计规范

1. **生成代码风格**：全限定路径（`gpui::div()` 而非 `div()`），函数内 `use` 引入 trait，避免与用户 import 冲突
2. **事件转换**：GPUI 事件 → RML 事件使用自由函数（`from_gpui_click` 等），避免 orphan 规则冲突
3. **元素 ID**：有事件处理器的元素自动 `.id(("rml_el", n))` 成为 `Stateful<Div>`，满足 `StatefulInteractiveElement` 要求
4. **条件渲染**：`if`/`show` 指令用 `if/else` + `into_any_element()` 实现真正条件渲染，而非 `.when()`（后者不会隐藏元素）
5. **增量缓存**：`.rml` 文件 sha256 哈希未变则跳过重新生成，加速构建
6. **指令零前缀**：`if`/`each`/`model`/`show`/`once`/`html`/`ref`/`slot`/`else`/`key` 无冒号前缀
7. **事件属性**：`on*` 无冒号前缀（`onclick`/`oninput`/`onchange`）
