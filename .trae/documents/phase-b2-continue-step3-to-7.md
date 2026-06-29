# Phase B-2 续作：Step 3 修复 + Step 4-7 实施

## 摘要

延续上一会话 Phase B-2 工作，完成 Step 3 失败测试修复并推进至 Step 7。
目标：让 `#[command]` 自动注入 `bump_version + cx.notify()`，`#[computed]` 自动缓存，
用户保持 `self.count += 1` 语法无需手动 `cx.notify()`。

## 当前状态分析

### 已完成
1. **Step 1 ComputedCache**：[crates/core/src/computed_cache.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/core/src/computed_cache.rs) 实现完毕，10/10 测试通过
2. **Step 2 字段注入**：
   - [crates/macros/src/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/component.rs#L71-L105) `inject_tracking_fields` 为每个 pub 字段注入 `__rml_<field>_version: AtomicU64` + `__rml_computed_cache: ComputedCache`
   - [crates/macros/src/window.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/window.rs#L65) 调用同一函数
3. **Step 3 部分**：[crates/engine/src/build/scanner.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/scanner.rs) 已重写为 syn 解析，`StructMetadata` 已定义，`ComputedDepVisitor` 已实现，5/6 测试通过

### 失败的测试
`scans_computed_method_deps` 失败：`format!("{} {}", self.count, self.name)` 中的 `self.count`/`self.name` 未被识别为依赖。

**根因**：`node.mac.tokens.to_string()` 输出形如 `"{} {}", self . count, self . name`（syn 在 punct 周围插入空格），而 `scan_self_field_accesses` 用字节匹配 `b"self."`（无空格），导致 `self . count` 无法匹配。

### 未开始
- **Step 4**：`CodegenCtx` 已有 `observable_fields`/`computed_deps`，但缺 `computed_returns`；codegen 未生成 `__rml_bump_version`/`__rml_get_version`/`__rml_computed_deps_version` 三方法
- **Step 5**：[crates/macros/src/command.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs) 仍为 pass-through
- **Step 6**：[crates/macros/src/computed.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/computed.rs) 仍为 pass-through
- **Step 7**：Demo 仍含手动 `cx.notify()`，无集成测试

## 设计决策（细化）

### D1：宏参数扫描改用 `parse_body_with` + 字符串兜底
对 `format!`/`println!` 类宏，token stream 是逗号分隔的表达式列表。优先用 `node.mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)` 解析后递归 `visit_expr`，命中 `visit_expr_field`；解析失败回退到改进版字符串扫描（容忍 `self . field` 形式的空格）。

### D2：computed 返回类型由扫描器提取
`#[computed]` 包装方法需 `get_or_compute::<T, _>(...)`，T 必须显式。`StructMetadata` 新增 `computed_returns: HashMap<String, String>`，扫描器从 `method.sig.output` 提取返回类型字符串（如 `"i32"`、`"Vec<MenuItem>"`）。`CodegenCtx` 同步添加该字段。

### D3：#[command] 注入策略
- 用 `syn::visit::Visit` 遍历方法体，识别 `Expr::Assign`/`Expr::AssignOp` 的 LHS 为 `self.<ident>` 的语句
- 在该语句后插入 `self.__rml_bump_version("<field>");`
- 检测 `&mut Context<Self>` 参数名（通常为 `cx`），方法末尾追加 `<cx>.notify();`
- 用户已写的 `cx.notify()` 不剥离（GPUI 多次 notify 幂等）

### D4：#[computed] 重命名策略
- `fn xxx` 重命名为 `fn __rml_computed_xxx`，保留可见性、返回类型、方法体不变
- codegen 生成原签名 `pub fn xxx(...) -> RetType` 的包装方法，内部调用 cache

### D5：observable_impl 即使无 observable 字段也生成空 match
保证 `#[command]` 注入的 `bump_version` 调用不会因 match 缺失而编译失败。空 match 用 `_ => {}` 兜底。

## 实施步骤

### Step 3-finalize：修复失败测试 + 提取 computed 返回类型

**文件**：
- [crates/engine/src/build/scanner.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/scanner.rs)

**做什么**：
1. 重写 `visit_expr_macro`：先尝试 `node.mac.parse_body_with(Punctuated::<Expr, Token![,]>::parse_terminated)`，对每个解析成功的 `Expr` 调 `self.visit_expr(expr)`；解析失败回退到改进版字符串扫描
2. 重写 `visit_macro`：同上策略，用 `node.parse_body_with(...)` 或 `node.mac.parse_body_with(...)`
3. 改进 `scan_self_field_accesses`：处理 `self` 与 `.` 之间的空格、`.` 与 ident 之间的空格
4. `StructMetadata` 添加 `pub computed_returns: HashMap<String, String>`
5. 扫描器在收集 `#[computed]` 方法时，从 `method.sig.output` 提取返回类型字符串（用 `quote!(#output).to_string()` 后清理空格），存入 `computed_returns`

**验证**：`cargo test -p rust-rml-engine --lib build::scanner` 全部通过（6/6）。

### Step 4：CodegenCtx 扩展 + codegen 生成版本管理方法 + 计算属性包装

**文件**：
- [crates/engine/src/compiler/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/mod.rs#L17-L45) - `CodegenCtx` 添加 `computed_returns: HashMap<String, String>`
- [crates/engine/src/compiler/codegen.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen.rs) - 新增 `gen_observable_impl` + `gen_computed_wrappers`，在 `codegen()` 主流程末尾调用
- [crates/engine/src/build/mod.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/build/mod.rs#L211-L218) - 构造 `CodegenCtx` 时传入 `computed_returns`
- [crates/engine/src/compiler/component.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L339-L348)（测试）和 [crates/engine/src/compiler/event.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/event.rs#L174-L183)（测试）的 `ctx()` 函数添加 `computed_returns: HashMap::new()`

**做什么**：

1. `CodegenCtx` 添加 `pub computed_returns: HashMap<String, String>`

2. 新增 `gen_observable_impl(ctx) -> String`，生成一个 `impl <View> { ... }` 块，包含：
   - `fn __rml_bump_version(&self, field: &str)`：对每个 `observable_fields` 生成 match 臂 `"x" => { self.__rml_x_version.fetch_add(1, Relaxed); }`，末尾 `_ => {}`
   - `fn __rml_get_version(&self, field: &str) -> u64`：对每个生成 `"x" => self.__rml_x_version.load(Relaxed),`，末尾 `_ => 0`
   - `fn __rml_computed_deps_version(&self, computed: &str) -> u64`：对每个 `computed_methods` 查 `computed_deps`，sum 依赖字段版本号；空依赖返回 0；末尾 `_ => 0`

3. 新增 `gen_computed_wrappers(ctx) -> String`，生成一个 `impl <View> { ... }` 块，对每个 `computed_methods` 中的方法生成包装：
   ```rust
   pub fn <name>(&self) -> <RetType> {
       let __v = self.__rml_computed_deps_version("<name>");
       self.__rml_computed_cache.get_or_compute::<<RetType>, _>("<name>", __v, || self.__rml_computed_<name>())
   }
   ```
   - `<RetType>` 从 `computed_returns` 取（找不到则 panic）
   - 方法签名（pub/无参/&self）由 codegen 生成

4. `codegen()` 主流程末尾追加：
   ```rust
   out.push_str(&gen_observable_impl(ctx));
   out.push('\n');
   out.push_str(&gen_computed_wrappers(ctx));
   ```

**验证**：`cargo build -p rust-rml-engine` 通过；现有测试不破坏。

### Step 5：#[command] 宏改造

**文件**：[crates/macros/src/command.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/command.rs)

**做什么**：
1. 引入 `syn::visit::Visit`、`syn::{Expr, ExprAssign, ExprAssignOp, Member, Stmt}`
2. 定义 `FieldMutationVisitor { mutated_fields: Vec<String> }`：
   - `visit_expr_assign`：检测 LHS 为 `Expr::Field` 且 base 是 `self`，提取 member ident 加入 `mutated_fields`
   - `visit_expr_assign_op`：同上
3. 重写 `expand`：
   - 校验 `&self`/`&mut self`（保留现有校验）
   - 提取 `&mut Context<Self>` 参数名（默认 `cx`，从 `FnArg::Typed` 的 `Pat::Ident` 提取）
   - 遍历 `item.block.stmts`，对每个 `Stmt::Semi(Expr::Assign|ExprAssignOp, ..)` 检测字段修改
   - 重构 stmt 列表：原 stmt 后插入 `self.__rml_bump_version("<field>");`
   - 若 `mutated_fields` 非空且有 Context 参数，末尾追加 `<cx>.notify();`
   - 用 `parse_quote!` 构造注入 stmt
4. 保留现有的 `extract_event_type`/`extract_params`（仍可能用于元信息）

**验证**：
- `cargo build -p rust-rml-macros` 通过
- `cargo test -p rust-rml-macros`（若有测试）通过

### Step 6：#[computed] 宏重命名

**文件**：[crates/macros/src/computed.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/macros/src/computed.rs)

**做什么**：
1. 保留 `&self` + 无参校验
2. 提取 `item.sig.ident` 为 `original_name`
3. 构造 `__rml_computed_<original_name>` 作为新 ident
4. 修改 `item.sig.ident = new_ident`
5. 可选：保留 `#[allow(non_snake_case)]`（虽然 `__rml_computed_xxx` 是 snake_case，无需）
6. 输出 `quote! { #item }`

**验证**：
- `cargo build -p rust-rml-macros` 通过
- codegen 生成的包装方法调用 `self.__rml_computed_<name>()` 能匹配重命名后的方法

### Step 7：Demo 验证 + 集成测试

**文件**：
- [demo/src/main_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/main_window.rml.rs#L43-L47) - 删除 `cx.notify()`
- `crates/engine/tests/observable_test.rs`（新建）- 集成测试

**做什么**：

1. Demo `on_click` 方法体简化为：
   ```rust
   #[command]
   pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
       self.count += 1;
   }
   ```

2. 集成测试 `crates/engine/tests/observable_test.rs`：
   - 测试 1：构造有 `#[window]` 标注的 struct，验证 `__rml_bump_version("count")` 后 `__rml_get_version("count")` 从 0 变 1
   - 测试 2：构造 `#[computed]` 方法（依赖 `count`），手动 bump 后再次调用应重算；不 bump 时再次调用应命中缓存
   - 测试 3：嵌套 `#[computed]` 调用不死锁

3. 运行完整测试套件：
   - `cargo build --workspace`
   - `cargo test --workspace`
   - `cargo run -p rust-rml-demo`（手动验证 UI 行为）

**验证**：
- Demo 启动后点击按钮，`count` 自增、UI 更新正常（证明自动 notify 生效）
- 命令行测试全部通过
- 缓存测试：在 `#[computed]` 中加 `eprintln!("recompute")`，验证多次访问只在依赖变更后打印一次

## 假设与决策

1. **不修改 ComputedCache**：Step 1 已完成且测试充分，无需变更
2. **不修改字段注入宏**：Step 2 已完成，inject_tracking_fields 工作正常
3. **保持 `__rml_` 前缀**：这是宏卫生注入的私有字段（避免与用户字段冲突），与用户偏好的"避免冗余 rml_ 前缀"是不同概念（后者针对用户可见 API）
4. **codegen 生成的方法用 `#[allow(dead_code)]`**：在无 `#[command]`/`#[computed]` 使用时这些方法不被调用，需避免告警
5. **不引入 `#[observable]` 属性**：所有 pub 字段默认追踪（与设计文档一致）
6. **Step 5 上下文参数识别**：仅识别 `&mut Context<Self>` 类型的参数，将其 Pat::Ident 名作为 cx 参数名；找不到则不注入 notify（仍注入 bump）
7. **return type 提取用 quote!.to_string()**：保留源码形式（如 `Vec<MenuItem>`），codegen 直接插入

## 验证步骤

1. `cargo test -p rust-rml-engine --lib build::scanner` → 6/6 通过
2. `cargo build --workspace` → 全部编译通过
3. `cargo test --workspace` → 全部测试通过（含原 219 个 + 新增）
4. `cargo run -p rust-rml-demo` → 启动成功，点击按钮 count 自增、UI 更新正常

## 关键文件改动清单

| 文件 | 操作 | 描述 |
|------|------|------|
| `crates/engine/src/build/scanner.rs` | 修改 | 修复宏参数扫描 + 提取 computed_returns |
| `crates/engine/src/compiler/mod.rs` | 修改 | CodegenCtx 添加 `computed_returns` |
| `crates/engine/src/compiler/codegen.rs` | 修改 | 新增 `gen_observable_impl` + `gen_computed_wrappers` |
| `crates/engine/src/build/mod.rs` | 修改 | CodegenCtx 传入 computed_returns |
| `crates/engine/src/compiler/component.rs` | 修改 | 测试 ctx() 添加 computed_returns |
| `crates/engine/src/compiler/event.rs` | 修改 | 测试 ctx() 添加 computed_returns |
| `crates/macros/src/command.rs` | 重写 | Visit 检测字段修改 + 注入 bump/notify |
| `crates/macros/src/computed.rs` | 修改 | 重命名 `fn xxx` → `fn __rml_computed_xxx` |
| `demo/src/main_window.rml.rs` | 修改 | 删除手动 `cx.notify()` |
| `crates/engine/tests/observable_test.rs` | 新建 | 集成测试 |

## 依赖顺序

```
Step 3-finalize (修复 + computed_returns 提取)
       ↓
Step 4 (CodegenCtx.computed_returns + gen_observable_impl + gen_computed_wrappers)
       ↓
Step 5 (#[command] 注入)    Step 6 (#[computed] 重命名)
       ↓                          ↓
       └──────────┬────────────────┘
                  ↓
            Step 7 (Demo + 测试)
```

Step 5 和 Step 6 在 Step 4 完成后可并行，但 Step 6 必须在 Step 4 的 `gen_computed_wrappers` 完成后才能验证完整功能（包装方法调用 `__rml_computed_<name>`，需 `#[computed]` 重命名配合）。
