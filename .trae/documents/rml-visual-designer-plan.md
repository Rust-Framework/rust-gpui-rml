# RML 可视化设计器稳妥实现思路

## 1. 摘要

RML 已经具备声明式模板 + ViewModel 的 MVVM 架构、完整的 parser / AST / codegen 管线、sourcemap 双向映射、LSP 语义服务以及热重载机制。基于这些现有基础，可视化设计器不应另起炉灶造一套运行时，而应作为 **RML 编译器与 LSP 的设计时客户端** 存在：所有拖拽、属性编辑最终都落回到 `.rml` 源码与 AST 变更，再经标准编译流程生成渲染代码。这样既能复用框架核心能力，也能保证"设计时所见 = 运行时所得"。

## 2. 当前状态分析

### 2.1 已有基础

| 能力 | 现状 | 对设计器的价值 |
|------|------|---------------|
| **Parser / AST** | `crates/engine/src/parser/ast.rs` 定义了 `Node`、`Element`、`Attribute`、`Directive`、`Span` 等不可变结构 | 设计器可直接操作 AST 作为"文档模型" |
| **组件注册表** | `tags.rs` 的 `component_lookup`、`props_registry.rs` 的 `COMPONENT_PROPS` | 提供组件清单、可用属性、类型约束 |
| **代码生成** | `codegen::codegen()` 将 AST 转译为 Rust 源码 | 设计器预览必须走同一套生成逻辑，避免行为分叉 |
| **SourceMap** | `source_map.rs` 记录 RML span → 生成代码 (line, col) | 实现设计器选中 ↔ 源码位置的双向定位 |
| **LSP** | `rust-rml-lsp` 已提供 completion / hover / definition / diagnostics | 设计器属性面板、组件面板可直接调用 LSP 能力 |
| **热重载** | 文档描述 `.rml` 改动可在秒级反映到运行窗口 | 设计器实时预览可基于此机制 |
| **MVVM 元数据** | `build/scanner.rs` 扫描 `.rml.rs` 提取字段类型、computed、commands、slots | 提供数据绑定、事件命令、插槽的可用列表 |

### 2.2 主要缺口

1. **AST 可变性**：当前 AST 节点是简单 `#[derive(Debug, Clone)]` 结构，缺少带父指针/偏移追踪的可变语法树，设计器需要一套 "RmlDocument" 编辑模型。
2. **源码写回（Pretty Print）**：codegen 只生成 Rust，没有从 AST 还原格式化 `.rml` 源码的 printer。
3. **设计时渲染隔离**：需要一种方式在 host 应用内安全渲染被设计的子树，同时保留选中框、拖拽手柄等装饰。
4. **组件拖拽元数据**：当前 `tags.rs` 只有构造路径，缺少图标、分类、默认属性、布局占位等设计时元数据。
5. **属性编辑器类型系统**：`props_registry` 目前偏静态字符串列表，需要知道每个属性是字符串/数字/布尔/枚举/颜色/边距等，才能生成对应 UI 控件。

## 3. 总体架构

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          RML Visual Designer (GPUI App)                       │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────────┐ │
│  │  组件面板     │  │  画布/预览   │  │  属性面板     │  │  源码编辑器      │ │
│  │ (调用 LSP)   │  │ (RML Preview)│  │ (调用 LSP)   │  │  (双向 SourceMap)│ │
│  └──────┬───────┘  └──────┬───────┘  └──────┬───────┘  └────────┬─────────┘ │
└─────────┼─────────────────┼─────────────────┼───────────────────┼───────────┘
          │                 │                 │                   │
          └─────────────────┴─────────────────┴───────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         RML Design-Time Kernel (新 crate)                    │
│  ┌────────────────┐  ┌────────────────┐  ┌────────────────┐  ┌────────────┐ │
│  │ Document Model │  │ AST ↔ Source   │  │ Preview Host   │  │ LSP Client │ │
│  │ (可编辑 AST)   │  │ 双向同步       │  │ (沙箱渲染)     │  │ (IPC/stdio)│ │
│  └────────────────┘  └────────────────┘  └────────────────┘  └────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
          │
          ▼
┌─────────────────────────────────────────────────────────────────────────────┐
│                         复用现有 RML 基础设施                                 │
│   parser::parse()  →  ast::Node  →  codegen::codegen()  →  include!           │
│   tags::component_lookup   props_registry::COMPONENT_PROPS                    │
│   source_map::SourceMap    rust-rml-lsp (completion/hover/diagnostics)        │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3.1 关键设计原则

1. **源码为唯一真相源**：设计器不保存私有格式，只编辑 `.rml` 文件。所有操作 = AST 变换 → printer 写回源码 → cargo build 生成预览。
2. **不碰 `.rml.rs` 业务代码**：ViewModel、命令、计算属性仍由开发者手写；设计器只读取 scanner 元数据，不修改。
3. **标准编译流程生成预览**：预览必须调用 `codegen::codegen()` 生成 Rust 代码，再交给 Rust 编译执行，杜绝"设计器专用渲染"带来的差异。
4. **沙箱式预览**：被设计页面作为子 view 运行在独立 entity 中，装饰层（选中框、拖拽手柄、辅助线）叠加在上方，不污染目标 AST。
5. **渐进式增强**：先做"源码 + 实时预览 + 属性面板"，再补拖拽；先做静态属性编辑，再补表达式绑定与事件选择。

## 4. 分阶段实现方案

### Phase 0：前置能力补齐（不急着写设计器 UI）

| 任务 | 文件/模块 | 说明 |
|------|----------|------|
| 引入可编辑文档模型 | `crates/engine/src/parser/document.rs`（新文件） | 在 `ast::Node` 之上包装带父指针、版本号、dirty 标记的 `RmlDocument`，支持增删改节点并追踪 span 漂移 |
| RML AST Printer | `crates/engine/src/parser/printer.rs`（新文件） | 将 `ast::Node` 输出为格式化 `.rml` 源码，作为设计器"写回"的标准化步骤 |
| 设计时元数据扩展 | `crates/ui/src/components/meta.rs`（新文件） | 为每个组件注册：分类、图标、默认子节点、可接受子组件、布局行为（block/inline/flex item） |
| 属性类型描述 | `crates/engine/src/compiler/props_registry.rs` | 在 `PropInfo` 中增加 `value_kind: PropKind`（String / Bool / Number / Enum / Color / Size / Shadow / ...） |
| SourceMap 稳定性增强 | `crates/engine/src/compiler/source_map.rs` | printer 写回后重新 parse，验证 span 漂移可控；必要时为 Element 增加 stable id |

### Phase 1：最小可用设计器（预览 + 源码双向定位）

目标：做一个能打开任意 `.rml` 文件、右侧实时预览、点击预览元素高亮源码行、修改源码后预览热重载的最小工具。

1. **设计器宿主应用**：新建 `tools/rml-designer`（或 `demo` 子 crate），用 GPUI 实现三栏布局：左侧文件树、中间源码编辑器、右侧预览窗口。
2. **预览机制**：
   - 工具监听 `.rml` 保存事件。
   - 调用 `rust-rml-engine` 的 parse + codegen，生成临时 `.rml.rs` 到 `target/rml-designer/`。
   - 通过动态加载或独立进程渲染目标组件（优先选独立进程，避免宿主崩溃）。
   - 对 `<window>` / `<modern-window>` 根节点，截取内容区作为子 view 嵌入；对 `<component>` 根节点，直接嵌入。
3. **双向定位**：
   - 点击源码 → 通过 `SourceMap::rml_to_rust` 找到生成代码位置，再映射到预览中的元素 ID。
   - 点击预览 → 通过元素 ID 反查 `SourceMap::rust_to_rml`，得到 RML span，高亮源码。
4. **状态隔离**：预览 view 与装饰层分离，装饰层通过 `Overlay` 或同级绝对定位元素实现。

### Phase 2：组件与属性面板

目标：不手写源码也能完成 80% 的常见结构调整。

1. **组件面板**：读取 `tags.rs` + `meta.rs`，按分类（Layout / Form / Data / Feedback / Navigation）展示可拖拽组件。
2. **属性面板**：
   - 选中画布元素后，查询 `props_registry` 得到该组件支持的所有属性。
   - 根据 `PropKind` 渲染不同编辑器：布尔 → toggle；枚举 → dropdown；颜色 → color picker；size → 预设按钮组；字符串/数字 → input。
   - 静态属性直接修改 AST 并写回源码；绑定属性 `{expr}` 提供 ViewModel 字段/计算属性补全（调用 LSP）。
3. **结构操作**：
   - 右键删除、复制、包裹（wrap with `<div>` / `<Card>`）。
   - 拖拽调整同层兄弟节点顺序（通过 AST children 重排）。
4. **实时校验**：每次修改后调用 LSP diagnostics，错误在属性面板和画布边缘提示。

### Phase 3：拖拽与可视化布局

目标：支持从组件面板拖入画布、在画布内调整位置。

1. **拖放语义**：
   - 拖入容器：根据目标组件 `meta.accepts_children` 决定能否放置。
   - 拖入 flex 行/列：通过鼠标位置计算插入索引，使用视觉指示条反馈。
   - 拖拽元素调整顺序：同级元素间交换 AST children 顺序。
2. **布局辅助**：
   - 选中元素显示 margin / padding / gap 的可视化标注。
   - 支持通过拖拽手柄调整 `flex:`、`size`、`p-`、`m-` 等 class。
3. **约束**：拖拽只修改静态结构和 class/style；不修改 `if`/`each` 指令内部表达式，避免引入不可控副作用。

### Phase 4：与 LSP / 工程深度集成

1. **设计器 ↔ LSP 协同**：设计器作为 LSP 客户端，所有补全/诊断/跳转走 `rust-rml-lsp`，不重复实现语义分析。
2. **项目级索引**：复用 `workspace/project_index.rs`，支持跨文件组件、自定义组件、插槽定义的发现。
3. **重构联动**：在设计器重命名 class、组件、ref 名时，调用 LSP `workspace/rename` 同步所有引用。
4. **多根窗口支持**：预览 `<tab-window>` / `<modern-window>` 时，能在设计器中切换 Top/Left/Right/Bottom/Main 等 shell 区域。

## 5. 风险与规避

| 风险 | 规避方案 |
|------|---------|
| AST 变换后源码格式崩坏 | 引入标准化 printer，并增加 roundtrip 测试：parse → AST → print → parse 结果一致 |
| 预览渲染崩溃拖垮设计器 | 预览运行在独立进程或独立 thread + catch_unwind，错误时显示占位图 |
| 设计器行为与运行时行为不一致 | 强制走同一 codegen 路径，禁止为设计器单独实现渲染 |
| ViewModel 字段缺失导致绑定失效 | 属性面板绑定下拉框只显示 scanner 已发现的字段，缺失时明确提示"需要在 .rml.rs 中添加字段" |
| 复杂指令（each/slot）的拖拽语义混乱 | 拖拽不进入 `each` / `template slot` 内部，这些区域在画布上以"数据驱动区域"标识，仍需源码编辑 |
| 组件元数据维护负担 | 将元数据纳入 `props_registry::tests` 已有护栏，新增组件必须同时登记设计时元数据 |

## 6. 假设与决策

1. **假设 RML 的 `.rml` 源码始终可解析为合法 AST**。非法文件进入设计器时，先以只读源码模式打开，修复后再启用可视化编辑。
2. **假设设计器与目标项目在同一台机器**。跨机器/浏览器端设计器需要另建 server-side 编译服务，不在本阶段考虑。
3. **决策：设计器优先使用 GPUI 实现**，而非 Electron/Webview。理由是团队熟悉 GPUI，且能直接复用 `gpui-component` 做设计器自身 UI。
4. **决策：不修改现有 codegen 接口**，只新增 `document.rs` / `printer.rs` 等辅助模块，保持现有 demo / LSP 不受影响。
5. **决策：属性编辑粒度为"单个属性"**，不自动拆分或合并 class。class 编辑由独立 CSS 面板处理，避免与 `class="..."` 字符串语义冲突。

## 7. 验证步骤

1. **单元测试**：`document.rs` 提供 `RmlDocument` 的增删改 API 测试；`printer.rs` 提供 roundtrip 测试。
2. **集成测试**：在 `tools/rml-designer/tests` 中，对 `demo/src/cases/button_case.rml` 等典型文件执行：打开 → 修改一个静态属性 → 写回源码 → 重新 parse → 属性值正确。
3. **编译回归**：每次设计器相关改动后，运行 `cargo test -p rust-rml-engine --lib` 与 `cargo build -p rust-rml-demo`，确保 codegen 行为不变。
4. **预览一致性**：选取 10 个 demo cases，比较"源码编译运行截图"与"设计器预览截图"，像素级一致（除装饰层外）。
5. **LSP 协同测试**：设计器属性面板调用 LSP 补全，验证返回的字段列表与 `scanner.rs` 提取的一致。

## 8. 建议的下一步

若决定启动，建议按以下顺序推进：

1. 新建 `crates/engine/src/parser/document.rs` 与 `printer.rs`，为 RML 提供可编辑文档模型。
2. 在 `crates/engine` 中补充 roundtrip 测试，确保 AST ↔ 源码转换稳定。
3. 新建最小 `tools/rml-designer` crate，实现"打开 `.rml` → 预览 → 源码高亮"闭环。
4. 逐步叠加组件面板、属性面板、拖拽能力。

---

**结论**：RML 可视化设计器的稳妥路线不是重建 UI 系统，而是把设计器作为 RML 编译器的图形化外壳。所有设计时操作最终都落回 `.rml` 源码，再经同一套 parser / codegen 生成真实运行代码。这样既能控制复杂度，也能保证设计器产出与手写源码完全等价。
