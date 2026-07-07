# Engine 声明式代码生成引擎：Sourcemap 与架构规范性分析

## 任务说明

分析 `crates/engine` 声明式代码生成引擎是否：

1. 清晰记录代码转译过程信息（含 sourcemap，供 debug 引擎对接）；
2. 遵循架构设计规范（职责单一、模块化、高内聚低耦合）。

本文档为**分析评估报告**，非实现任务。若用户认可结论与改进项，再另行发起实现任务。

***

## 一、现状分析（基于实际代码探索）

### 1.1 代码转译过程信息记录现状

#### 已具备的能力

**AST 已携带 Span 字段**（[parser/ast.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/ast.rs)）：

| 节点类型                                | 是否携带 Span | 说明                                                                                                                                                    |
| ----------------------------------- | --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- |
| `Element.span`                      | ✅         | 覆盖 `<tag ...>...</tag>` 整个元素                                                                                                                          |
| `Attribute::Static/Bind/Event.span` | ✅         | 属性名+值的字节区间                                                                                                                                            |
| `Directive::* span`                 | ✅         | 指令名+值的字节区间（如 `if={cond}`）                                                                                                                             |
| `Node::Interpolation.span`          | ✅         | `{expr}` 的字节区间                                                                                                                                        |
| `TextSegment::Interpolation.span`   | ✅         | 混合文本中插值段的位置                                                                                                                                           |
| `Node::Text(String)`                | ❌         | 纯文本节点无位置                                                                                                                                              |
| `TextSegment::Literal(String)`      | ❌         | 混合文本字面量段无位置                                                                                                                                           |
| `EachClause`（item/index/iterable）   | ❌         | LSP token emitter 需"从 Directive::Each.span 内扫描源码提取"，注释明确见 [ast.rs:81-83](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/ast.rs#L81-L83) |

**Span 类型定义**（[parser/span.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/span.rs)）：

* 半开字节区间 `[start, end)`；

* 注释明确："LSP 与未来增量解析的基础设施"；

* 仅字节偏移，**无 line/column 工具函数**，需消费方自行换算。

#### 关键缺口（决定性证据）

**缺口 1：codegen 完全未消费 AST 的 span 字段**

在 `crates/engine/src/compiler/codegen/` 全目录搜索 `elem.span | element.span | attr.span | directive.span` —— **零匹配**。`gen_element` 函数（[codegen/node.rs:89-558](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs#L89-L558)）接收 `&Element` 但全程不读 `elem.span`，生成代码中没有任何位置标记。

**缺口 2：compile() 接口签名不携带 sourcemap**

[compiler/mod.rs:251-260](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L251-L260)：

```rust
pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<String, CompileError>
```

返回值仅为 Rust 源码字符串，**没有附带的 source map 数据结构**。要从根上支持 sourcemap，此签名必须扩展。

**缺口 3：build.rs 仅输出 .rs 文件，无 .rml.map**

[build/mod.rs:306-312](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs#L306-L312)：

```rust
match compile(&source, &ctx) {
    Ok(code) => {
        if let Err(e) = fs::write(&out_file, code) { ... }  // 仅写 .rs
    }
}
```

无 sourcemap 文件输出。`CodegenCtx`（[compiler/mod.rs:106-189](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L106-L189)）的 20+ 字段中**没有任何 sourcemap 收集器字段**。

**缺口 4：CodegenError 缺少源码位置**

[compiler/mod.rs:194-197](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L194-L197)：

```rust
pub struct CodegenError {
    pub message: String,  // 仅一行 message
}
```

对比 `ParseError` 有 `line / column / source_snippet`，`CodegenError` 完全无位置信息。codegen 报错（如 `<component>` 缺 `content` 属性、`else` 指令无前置 `if`）时无法回溯到 .rml 具体行号。

#### dap crate 已为 sourcemap 留好接口

[dap/src/source\_map/mapper.rs:7-12](file:///d:/GitCode/RF/rust-gpui-rml/crates/dap/src/source_map/mapper.rs#L7-L12) 注释明确承认缺口：

> 当前 rust-rml-engine 的 codegen 不输出 `.rml` 行号 → 生成 `.rs` 行号的源映射，因此 `FilePairMapper` 仅做文件级配对（`.rml` ↔ `.rml.rs`），不翻译行号。精确行级映射需 engine codegen 增强输出 `.rml.map`（另立任务），届时实现 `LineAccurateMapper` 即可，上层代码无需改动（依赖 `SourceMapper` trait）。

设计已预留 `SourceMapper` trait + `LineAccurateMapper` 扩展点，**只待 engine codegen 产出 sourcemap 数据**。这是 engine 与 dap 之间的契约缺口，不是 dap 侧的缺陷。

### 1.2 架构规范性评估

#### 符合规范的部分

| 项                         | 证据                                                                                                                                                                                                    |
| ------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| codegen 子模块拆分清晰           | [codegen/mod.rs:27-37](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L27-L37)：attribute/binding/lifecycle/model/node/observable/once/render/shell/text/window，按生成职责拆分 |
| dap crate 分层合理            | [dap/src/lib.rs:5-13](file:///d:/GitCode/RF/rust-gpui-rml/crates/dap/src/lib.rs#L5-L13)：engine / protocol / session / source\_map / lldb 五层各司其职                                                       |
| DebugEngine trait 接口零引擎依赖 | [dap/src/engine.rs:145-204](file:///d:/GitCode/RF/rust-gpui-rml/crates/dap/src/engine.rs#L145-L204)：方法签名只用 `Url/u32` 等中性类型                                                                            |
| component codegen 各组件独立目录 | `compiler/menu/`、`compiler/table/`、`compiler/accordion/` 等均为子目录形式                                                                                                                                     |

#### 违反规范的部分

**违反 1：`compiler/mod.rs`** **在 mod.rs 中写业务代码**

[compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs) 当前混合了三类内容：

* 第 5-29 行：24 个 `pub mod` 声明（合规的 re-export）；

* 第 36-189 行：定义了 `ValidationRule`、`ValidationRuleSet`、`InputHandlers`、`UserComponentInfo`、`CodegenCtx`、`CodegenError`、`CompileError` 等大量业务实体；

* 第 251-260 行：定义 `compile()` 编译主入口函数。

这违反 project memory 铁律："**所有 mod.rs 文件必须仅作为模块聚合与 re-export，禁止承载业务逻辑**"。约 250 行业务代码应拆出。

**违反 2：`parser/mod.rs`** **在 mod.rs 中写解析器实现**

[parser/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/parser/mod.rs) 当前混合：

* 第 6-14 行：`pub mod` 声明 + `Span` re-export（合规）；

* 第 16-90 行：`ParseError` 定义、`parse()` 入口函数；

* 第 92-295 行：`Parser` struct + `parse_children`/`build_element` 等核心解析逻辑；

* 第 297-419 行：`parse_each_expr`、`parse_event_handler`、`parse_text_segments`、`normalize_attr_name` 等辅助函数。

约 400 行解析器实现挤在 mod.rs 中。应拆到独立文件如 `parser/parser.rs` / `parser/text_segment.rs` / `parser/attr.rs`。

**违反 3：`codegen/node.rs`** **文件过大且职责过载**

[codegen/node.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/node.rs) 共 559 行（不含 tests，含 tests 约 1060 行），`gen_element` 函数体内包含 9 条独立分派路径：

1. `once` 指令处理（98-100）
2. `html` 指令降级（107-170）
3. `<component content={...} />` 透明容器（178-238）
4. `<slot />` 占位符（248-264）
5. 菜单容器标签（267-270）
6. 用户自定义 `#[component]`（273-279）
7. 扩展组件（282-288）
8. `model` 双向绑定 input（296-301）
9. 内置标签通用流程（303-557）

违反 project memory 铁律："**当某文件因合理演化超过 \~300 行或包含 2+ 独立** **`pub struct`/`pub trait`** **时，必须触发拆分**"。建议按分派路径抽到独立子模块（如 `codegen/dispatch/once.rs`、`codegen/dispatch/html.rs`、`codegen/dispatch/component.rs` 等），node.rs 仅保留分派骨架。

**违反 4：`CodegenCtx`** **单结构承担 4 类职责**

[compiler/mod.rs:106-189](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L106-L189) 的 `CodegenCtx` 持有 20+ 字段：

* 视图标识：`view_struct_name`、`view_module_path`

* 样式：`stylesheet`

* 元信息（扫描产物）：`computed_methods`、`observable_fields`、`version_fields`、`computed_deps`、`computed_returns`、`field_types`、`field_validations`

* 模型/输入：`model_fields`、`model_converters`、`model_input_handlers`

* 组件注册：`user_components`

* 生命周期：`lifecycle_hooks`、`has_manual_lifecycle_impl`、`is_contributehost`

* 配置开关：`strict`

违反"高内聚低耦合"原则。本可作为可读性改进项（不强制），但若要新增 sourcemap 收集器，会让该结构更臃肿，是改造时机点。

**违反 5：`build/scanner.rs`** **体量过大**

探索 agent 报告 `build/scanner.rs` 约 1200 行，建议拆分（本次未深入审查内部结构，留待后续单独评估）。

***

## 二、评估结论

### 2.1 关于"是否清晰记录代码转译过程信息"

**结论：不清晰。Sourcemap 完全缺失。**

* AST 层面有 Span 数据（仅字节偏移，无 line/column 工具函数）；

* **codegen 层面完全丢弃 Span**（grep 零匹配）；

* compile() 接口签名不携带 sourcemap 数据结构；

* build.rs 不输出 .rml.map 文件；

* CodegenError 无源码位置信息；

* dap 侧已预留 SourceMapper 扩展点，但 engine 不产出数据 → **.rml 调试能力实际无法启用**，调试时只能在 .rml.rs 生成代码上打断点（文件级配对，行号近似）。

### 2.2 关于"是否遵循架构设计规范"

**结论：部分违反。**

| 评估维度                        | 状态                                      |
| --------------------------- | --------------------------------------- |
| mod.rs 仅 re-export          | ❌ `compiler/mod.rs`、`parser/mod.rs` 均违反 |
| 一个 rs 文件 = 一个组件/职责          | ❌ `codegen/node.rs` 559 行 + 9 条分派路径     |
| 单文件 ≤300 行 / 1 个 pub struct | ⚠️ `build/scanner.rs` 约 1200 行（待复查）     |
| 组件代码与 codegen 分离            | ✅ compiler 子目录形式                        |
| 高内聚低耦合                      | ⚠️ `CodegenCtx` 20+ 字段混合职责              |
| 错误信息可回溯源码                   | ❌ CodegenError 无 span                   |

***

## 三、改进建议（分优先级，未在本次实施）

### P0（必做，决定 .rml 调试能力能否落地）

1. **定义 sourcemap 数据模型**

   * 在 engine crate 新增 `compiler/source_map.rs`（或独立子模块）；

   * 定义 `SourceMap { entries: Vec<SourceMapEntry> }`，每个 entry 携带 `(rml_span: Span, rust_line: u32, rust_col: u32)`；

   * 提供 `rml_to_rust(span) -> Option<(line, col)>` 与反向查询接口。

2. **扩展 compile() 接口**

   ```rust
   pub fn compile(source: &str, ctx: &CodegenCtx) -> Result<CompileOutput, CompileError>
   pub struct CompileOutput {
       pub code: String,
       pub source_map: SourceMap,  // 新增
   }
   ```

   保持向后兼容可通过 `compile_only()` 旧签名或 `CompileOutput::code` 字段访问。

3. **codegen 透传 span 到 sourcemap**

   * `CodegenCtx` 增加 `source_map: SourceMap` 字段（或通过独立的 `SourceMapCollector`）；

   * `gen_element`/`gen_attribute`/`gen_directive` 在生成关键 Rust 代码片段时，记录当前生成位置（行号）+ AST 的 `elem.span`，写入 sourcemap。

4. **build.rs 输出 .rml.map 文件**

   * `out_file` 改为 `out_file + out_map_file`（同目录 .rml.map）；

   * dap 的 `FilePairMapper::register_pair` 升级为 `LineAccurateMapper`，加载 .rml.map。

5. **CodegenError 携带源码位置**

   * 增加 `span: Option<Span>` 字段；

   * codegen 报错路径（如 `<component>` 缺 `content`、`else` 无前置 `if`）透传 AST 节点 span。

### P1（架构合规整改）

1. **拆分** **`compiler/mod.rs`**

   * 业务实体移至 `compiler/types.rs`（ValidationRule 等）、`compiler/codegen_ctx.rs`（CodegenCtx）、`compiler/error.rs`（CodegenError/CompileError）；

   * `compile()` 移至 `compiler/compile.rs`；

   * `mod.rs` 仅保留 `pub mod` + `pub use` 声明。

2. **拆分** **`parser/mod.rs`**

   * `Parser` struct + `parse_children` + `build_element` → `parser/parser.rs`；

   * 辅助函数 → `parser/text_segment.rs`、`parser/attr.rs`、`parser/each_expr.rs`；

   * `parse()` 入口 + `ParseError` 可留在 `parser/mod.rs`（属 re-export 性质）或独立 `parser/error.rs`。

3. **拆分** **`codegen/node.rs`**

   * 9 条分派路径抽到 `codegen/dispatch/` 子模块；

   * `node.rs` 仅保留 `gen_node` + `gen_element` 分派骨架（< 100 行）。

### P2（可选优化）

1. `CodegenCtx` 拆分为 `ViewIdentity` + `ScanMetadata` + `ModelBindings` + `ComponentRegistry` + `BuildConfig`（与 P0 第 3 项同步重构）；
2. `build/scanner.rs` 1200 行内部结构复查；
3. `parser/span.rs` 增加 `to_line_col(source) -> (u32, u32)` 工具函数。

***

## 四、假设与决策

* **假设 1**：用户问的是"现状评估"而非"立即实施"。本计划仅产出分析报告，待用户认可后再发起 P0/P1 实现任务。

* **假设 2**：sourcemap 数据模型放在 engine crate（产出方）而非 dap crate（消费方），dap 仅定义 `SourceMapper` trait 消费接口。这与 [mapper.rs:11-12](file:///d:/GitCode/RF/rust-gpui-rml/crates/dap/src/source_map/mapper.rs#L11-L12) 的注释一致。

* **决策**：本次不修改任何代码，仅产出本分析文档。

## 五、验证步骤

本任务为分析评估，验证方式：

1. 用户审阅本报告的事实证据（文件路径 + 行号）是否准确；
2. 用户审阅改进建议优先级是否合理；
3. 若用户认可，再另行发起实现任务，届时按 P0 → P1 → P2 顺序逐项实现，每项附测试用例。

