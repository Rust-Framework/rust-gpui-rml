# Phase B-2：Observable 字段追踪 + 计算属性缓存

## 摘要

在不改变用户语法（`self.count += 1`）的前提下，为 RML 引入 WPF 风格的可观察字段追踪系统：
- `#[command]` 自动注入版本号 bump 与 `cx.notify()`，用户无需手写
- `#[computed]` 方法基于依赖字段的版本号进行缓存，依赖未变时直接命中缓存
- 通过 `AtomicU64` 版本计数器 + `Mutex<HashMap>` 缓存满足 GPUI `Entity<T>: Send + Sync` 约束

## 当前状态分析

### 现状痛点
1. **`crates/demo/src/main_window.rml.rs:43-47`** — 用户必须在 `#[command]` 方法体手动调用 `cx.notify()`：
   ```rust
   #[command]
   pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
       self.count += 1;
       cx.notify();  // ← 用户手写，违反 MVVM 理念
   }
   ```
2. **`crates/macros/src/command.rs:38`** — `#[command]` 完全 pass-through（`quote! { #item }`），不做任何注入
3. **`crates/macros/src/computed.rs:45`** — `#[computed]` 完全 pass-through，无缓存逻辑
4. **`crates/engine/src/build/mod.rs:278-310`** — `scan_computed_methods` 用字符串匹配（非 syn），无法提取方法体依赖
5. **`crates/core/src/model.rs`** — `IModel` 仅返回 `&'static [FieldMeta]`，无版本号/dirty/订阅机制

### GPUI 渲染模型限制
- `cx.notify()` 触发**全量重渲染**，无部分重渲染机制
- `cx.cache()` 在当前 gpui 版本不存在
- `Entity<T>` 要求 `T: Send + Sync`，禁止使用 `RefCell`（不满足 Sync）

### WPF 级细粒度更新的实现路径
GPUI 无法在渲染层做到 WPF 的"仅更新使用变更字段的组件"，但可在**计算层**实现等效语义：
- **跳过未变更依赖的 `#[computed]` 重算** — 这是 WPF `DependencyProperty` 缓存的核心收益
- **避免非 observable 字段修改触发 `cx.notify()`** — 跳过无意义重渲染

这两点叠加即可达到 WPF 等效的细粒度更新效果。

## 设计决策

### 核心：保持 `pub count: i32` 不变
不重写为 `Observable<i32>` 包装类型。原因：
- `self.count += 1` 在 `Observable<i32>` 下需变成 `*self.count += 1`（DerefMut），违反"语法不变"要求
- `pub count: i32` 是 Rust 惯用法，重写为包装类型破坏与外部 API 互操作

替代方案：通过 `#[window]`/`#[component]` 宏为每个 pub 字段注入同名的 `AtomicU64` 版本计数器字段。

### 默认开启策略（所有 pub 字段都 observable）
不引入 `#[observable]` 属性，所有 `pub` 字段默认参与版本追踪。理由：
1. WPF ViewModelBase 约定所有 public 属性均触发 PropertyChanged
2. 用户偏好"最少样板代码"（保持 `self.count += 1` 即可）
3. `AtomicU64` 每 pub 字段 8 字节开销，可接受
4. 与 `IModel` 现有约定一致（所有 pub 字段即绑定字段）

如未来需要 opt-out，可后续追加 `#[no_observe]` 属性。

### 三层协作架构

```
┌─────────────────────────────────────────────────────────────┐
│  宏层 (crates/macros)                                       │
│  - #[window]/#[component]：注入 AtomicU64 + ComputedCache   │
│  - #[command]：注入 __rml_bump_version + cx.notify          │
│  - #[computed]：重命名 fn xxx → fn __rml_computed_xxx       │
└─────────────────────────────────────────────────────────────┘
                          ↓ (struct 已含追踪字段)
┌─────────────────────────────────────────────────────────────┐
│  build.rs 层 (crates/engine/src/build)                      │
│  - syn 解析 .rml.rs 提取 observable_fields / computed_deps  │
│  - 传入 CodegenCtx 供 codegen 使用                           │
└─────────────────────────────────────────────────────────────┘
                          ↓ (CodegenCtx 携带元信息)
┌─────────────────────────────────────────────────────────────┐
│  codegen 层 (crates/engine/src/compiler)                    │
│  - 生成 __rml_bump_version / __rml_get_version 方法          │
│  - 生成 __rml_computed_deps_version 方法                     │
│  - 生成 #[computed] 缓存包装方法                              │
└─────────────────────────────────────────────────────────────┘
```

### 数据流示例

用户代码：
```rust
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[computed]
    pub fn doubled(&self) -> i32 { self.count * 2 }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
    }
}
```

宏展开后（概念）：
```rust
pub struct MainWindow {
    pub count: i32,
    __rml_count_version: AtomicU64,           // #[window] 注入
    __rml_computed_cache: ComputedCache,       // #[window] 注入
    __rml_window_handle: Option<...>,
}

impl MainWindow {
    // #[computed] 重命名
    fn __rml_computed_doubled(&self) -> i32 { self.count * 2 }

    // #[command] 注入 bump + notify
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        self.__rml_bump_version("count");  // ← 注入
        cx.notify();                        // ← 注入
    }
}
```

codegen 生成（`include!` 到用户模块）：
```rust
impl MainWindow {
    fn __rml_bump_version(&self, field: &str) {
        match field {
            "count" => { self.__rml_count_version.fetch_add(1, Ordering::Relaxed); }
            _ => {}
        }
    }
    fn __rml_get_version(&self, field: &str) -> u64 {
        match field {
            "count" => self.__rml_count_version.load(Ordering::Relaxed),
            _ => 0,
        }
    }
    fn __rml_computed_deps_version(&self, computed: &str) -> u64 {
        match computed {
            "doubled" => self.__rml_get_version("count"),
            _ => 0,
        }
    }
    // #[computed] 缓存包装
    pub fn doubled(&self) -> i32 {
        let v = self.__rml_computed_deps_version("doubled");
        self.__rml_computed_cache.get_or_compute("doubled", v, || self.__rml_computed_doubled())
    }
}
```

运行时行为：
1. 用户点击按钮 → `on_click` 执行 → `count += 1` → bump_version("count") → cx.notify()
2. GPUI 触发重渲染 → 调用 `doubled()` → 检测到 `count` 版本号变化 → 重新计算
3. 第二次点击前若再次访问 `doubled()`（比如多个绑定引用）→ 版本未变 → 直接命中缓存

## 实施步骤

### Step 1：新增 `ComputedCache` 类型

**文件**：`crates/core/src/computed_cache.rs`（新建）

**做什么**：
- 实现 `ComputedCache` 类型，使用 `Mutex<HashMap<String, (u64, Box<dyn Any + Send>)>>` 存储
- 提供 `get_or_compute<T: Clone + Send + 'static>(key, version, compute)` 方法
- 实现 `Default`、`new()`、`invalidate(key)`、`clear()`

**为什么**：
- codegen 生成的 `#[computed]` 包装方法需要统一缓存入口
- 必须满足 `Send + Sync`（`Entity<T>` 要求）
- Mutex 内部 HashMap 存任意返回类型的缓存值

**怎么做**：
```rust
use std::any::Any;
use std::collections::HashMap;
use std::sync::Mutex;

pub struct ComputedCache {
    inner: Mutex<HashMap<String, (u64, Box<dyn Any + Send>)>>,
}

impl ComputedCache {
    pub fn new() -> Self {
        Self { inner: Mutex::new(HashMap::new()) }
    }

    /// 命中缓存返回克隆值；未命中调用 compute 计算并写入缓存。
    /// 关键：compute 在锁外执行，避免 #[computed] 嵌套调用导致死锁。
    pub fn get_or_compute<T: Clone + Send + 'static>(
        &self,
        key: &str,
        version: u64,
        compute: impl FnOnce() -> T,
    ) -> T {
        // 先尝试命中
        {
            let inner = self.inner.lock().unwrap();
            if let Some((cached_ver, cached_val)) = inner.get(key) {
                if *cached_ver == version {
                    return cached_val.downcast_ref::<T>().unwrap().clone();
                }
            }
        } // ← MutexGuard 释放，compute 在锁外执行

        let value = compute();
        let mut inner = self.inner.lock().unwrap();
        inner.insert(key.to_string(), (version, Box::new(value.clone())));
        value
    }

    pub fn invalidate(&self, key: &str) {
        self.inner.lock().unwrap().remove(key);
    }

    pub fn clear(&self) {
        self.inner.lock().unwrap().clear();
    }
}

impl Default for ComputedCache {
    fn default() -> Self { Self::new() }
}
```

**修改**：`crates/core/src/lib.rs` 添加 `pub mod computed_cache;`

### Step 2：`#[window]`/`#[component]` 宏注入追踪字段

**文件**：
- `crates/macros/src/component.rs`（修改 `expand_component_impls` 签名或新增字段注入逻辑）
- `crates/macros/src/window.rs`（在 `expand` 中调用字段注入）

**做什么**：
- 扫描所有 pub 字段（与 `IModel` 一致）
- 为每个 pub 字段注入 `__rml_<field>_version: std::sync::AtomicU64`
- 注入一个 `__rml_computed_cache: rml_core::computed_cache::ComputedCache` 字段
- 清理 struct 时保留这些注入字段（不剥离）

**为什么**：
- pub 字段即 observable 字段（设计决策：默认开启）
- AtomicU64 满足 `Send + Sync`
- `ComputedCache::default()` 返回空 map，`#[derive(Default)]` 兼容

**怎么做**：
- 在 `expand_component_impls` 中新增 `inject_observable_fields(&mut item.fields)` 函数
- 该函数遍历 `Fields::Named`，对每个 `pub` 字段 push 一个对应的 `__rml_<name>_version: AtomicU64` 字段
- 最后追加一个 `__rml_computed_cache: ComputedCache` 字段
- `#[window]` 已有 `__rml_window_handle` 注入逻辑，复用此模式

注入字段示例：
```rust
// 用户 struct
pub struct MainWindow { pub count: i32, pub name: String }

// 宏展开后
pub struct MainWindow {
    pub count: i32,
    pub name: String,
    #[allow(non_snake_case)]
    __rml_count_version: std::sync::AtomicU64,
    #[allow(non_snake_case)]
    __rml_name_version: std::sync::AtomicU64,
    #[allow(dead_code)]
    __rml_computed_cache: rml_core::computed_cache::ComputedCache,
    #[allow(dead_code, non_snake_case)]
    __rml_window_handle: Option<gpui::AnyWindowHandle>,
}
```

**验证**：`#[derive(Default)]` 仍可用（`AtomicU64: Default = 0`，`ComputedCache::default() = 空 map`）。

### Step 3：build.rs 升级为 syn 解析

**文件**：
- `crates/engine/src/build/scanner.rs`（修改）
- `crates/engine/src/build/mod.rs`（修改 `scan_computed_methods` + 新增 `scan_observable_fields` + `scan_computed_deps`）
- `crates/engine/Cargo.toml`（添加 `syn = { workspace = true }` 依赖）

**做什么**：
1. **`scan_observable_fields(rml_files) -> Vec<String>`**：syn 解析 `.rml.rs`，提取所有 pub 字段名（与 `IModel::rml_fields` 等价，但供 codegen 使用以生成 bump_version match 臂）
2. **`scan_computed_methods(rml_files) -> Vec<String>`**：升级为 syn 解析，保留原签名
3. **`scan_computed_deps(rml_files) -> HashMap<String, Vec<String>>`**：对每个 `#[computed]` 方法用 `syn::visit::Visit` 遍历方法体，提取所有 `self.<ident>` 读访问作为依赖

**为什么**：
- 字符串匹配脆弱（注释、宏展开前源码等场景易出错）
- syn 可靠提取方法体依赖，是细粒度缓存的关键
- build.rs 在用户 crate 编译时执行，可读用户 `.rml.rs` 源码

**怎么做**：
```rust
// scanner.rs 新增
use syn::{ItemStruct, ItemImpl, ImplItem, FnArg, Receiver, Visit};
use std::collections::HashMap;

pub struct ComputedDepVisitor {
    pub deps: Vec<String>,
}

impl<'ast> Visit<'ast> for ComputedDepVisitor {
    fn visit_expr_field(&mut self, node: &'ast syn::ExprField) {
        // 检测 self.<ident> 模式
        if let syn::Expr::Path(syn::ExprPath { path, .. }) = &*node.base {
            if path.is_ident("self") {
                if let Some(ident) = node.member.get_ident() {
                    let name = ident.to_string();
                    if !self.deps.contains(&name) {
                        self.deps.push(name);
                    }
                }
            }
        }
        syn::visit::visit_expr_field(self, node);
    }
}
```

**修改**：`Builder::build()` 中调用三个扫描器，结果合并入 `CodegenCtx`。

### Step 4：CodegenCtx 扩展 + codegen 生成版本管理方法

**文件**：
- `crates/engine/src/compiler/mod.rs`（修改 `CodegenCtx`）
- `crates/engine/src/compiler/codegen.rs`（修改 `codegen` 主流程 + 新增 `gen_observable_impl`）
- `crates/engine/src/build/mod.rs`（构造 CodegenCtx 时传入新字段）

**做什么**：
1. `CodegenCtx` 新增字段：
   ```rust
   pub observable_fields: Vec<String>,
   pub computed_deps: HashMap<String, Vec<String>>,
   ```
2. codegen 主流程在 `gen_window_impl` 后追加 `gen_observable_impl`，生成：
   - `fn __rml_bump_version(&self, field: &str)`：match 每个 observable 字段 → `fetch_add(1, Relaxed)`
   - `fn __rml_get_version(&self, field: &str) -> u64`：match 每个 observable 字段 → `load(Relaxed)`
   - `fn __rml_computed_deps_version(&self, computed: &str) -> u64`：match 每个 computed 方法 → sum 依赖字段的 version

**为什么**：
- 这三个方法是 `#[command]` 和 `#[computed]` 包装的运行时支撑
- 必须由 codegen 生成，因为只有 codegen 知道哪些字段是 observable（从 build.rs 传入）

**怎么做**：
```rust
fn gen_observable_impl(elem: &Element, ctx: &CodegenCtx) -> String {
    let view_name = &ctx.view_struct_name;
    let mut bump_arms = String::new();
    let mut get_arms = String::new();
    for field in &ctx.observable_fields {
        bump_arms.push_str(&format!(
            "            \"{}\" => {{ self.__rml_{}_version.fetch_add(1, std::sync::atomic::Ordering::Relaxed); }}\n",
            field, field
        ));
        get_arms.push_str(&format!(
            "            \"{}\" => self.__rml_{}_version.load(std::sync::atomic::Ordering::Relaxed),\n",
            field, field
        ));
    }
    // ...format 三个方法...
}
```

**关键**：observable_fields 为空时仍生成空 match（`_ => {}`），保证 `#[command]` 注入的 bump 调用不会编译失败。

### Step 5：`#[command]` 宏改造为方法体分析 + bump/notify 注入

**文件**：`crates/macros/src/command.rs`（重写 `expand`）

**做什么**：
1. 用 `syn::visit::Visit` 遍历方法体，检测 `self.<ident> =`、`self.<ident> +=`、`-=`、`*=`、`/=`、`%=`、`&=`、`|=`、`^=`、`<<=`、`>>=` 模式
2. 收集所有被修改的字段名（即使非 pub 或非 observable，靠 codegen 的 match 兜底）
3. 在每个修改语句后插入 `self.__rml_bump_version("<field>");`
4. 检测 `&mut Context<Self>` 参数名（通常为 `cx`，但需泛化）
5. 若至少检测到一个修改且存在 Context 参数，在方法末尾插入 `<cx>.notify();`

**为什么**：
- 满足用户"无需手动 notify"要求
- bump_version 调用对所有字段修改注入（即使非 observable），codegen 的 match 兜底保证非 observable 字段静默忽略

**怎么做**：
```rust
pub fn expand(input: TokenStream) -> TokenStream {
    let mut item: ItemFn = match syn::parse2(input.clone()) {
        Ok(i) => i,
        Err(e) => return e.to_compile_error(),
    };
    // ... 校验 &self/&mut self ...

    let cx_ident = extract_context_param(&item.sig.inputs);

    // 遍历方法体，识别字段修改语句
    let mut visitor = FieldMutationVisitor::default();
    visitor.visit_block(&item.block);

    // 对每个识别到的修改语句，在其后插入 bump_version 调用
    // （通过 stmt 重构）
    let mut new_stmts: Vec<Stmt> = Vec::new();
    for stmt in item.block.stmts.drain(..) {
        let mutated_field = detect_field_mutation(&stmt);
        new_stmts.push(stmt.clone());
        if let Some(field) = mutated_field {
            let bump: Stmt = parse_quote! {
                self.__rml_bump_version(#field);
            };
            new_stmts.push(bump);
        }
    }

    // 若检测到修改且有 cx 参数，追加 cx.notify()
    if !visitor.mutated_fields.is_empty() {
        if let Some(cx) = cx_ident {
            let notify: Stmt = parse_quote! { #cx.notify(); };
            new_stmts.push(notify);
        }
    }

    item.block.stmts = new_stmts;
    quote! { #item }
}
```

**关键细节**：
- 用户已写的 `cx.notify()` 不剥离（GPUI 多次 notify 幂等，开销可忽略）
- `detect_field_mutation` 通过模式匹配 `Stmt::Semi(Expr::Assign, ..)` 中的 LHS 是否为 `self.<ident>` 判定
- 复合赋值（`+=` 等）通过 `Expr::AssignOp` 检测

### Step 6：`#[computed]` 宏重命名 + codegen 缓存包装

**文件**：
- `crates/macros/src/computed.rs`（修改 `expand`）
- `crates/engine/src/compiler/codegen.rs`（新增 `gen_computed_wrappers`）

**做什么**：
1. `#[computed]` 宏：将 `fn <name>` 重命名为 `fn __rml_computed_<name>`，其余不变
2. codegen 在 `gen_observable_impl` 之后追加 `gen_computed_wrappers`，为每个 `computed_methods` 中的方法生成：
   ```rust
   pub fn <name>(&self) -> <RetType> {
       let v = self.__rml_computed_deps_version("<name>");
       self.__rml_computed_cache.get_or_compute::<RetType, _>(
           "<name>", v, || self.__rml_computed_<name>()
       )
   }
   ```

**为什么**：
- `#[computed]` 包装层拦截每次调用，命中缓存直接返回
- 重命名让 codegen 可以"插入"包装方法而不冲突
- 依赖版本号变化时才重算（WPF `DependencyProperty` 等效语义）

**怎么做**：
```rust
// computed.rs
pub fn expand(input: TokenStream) -> TokenStream {
    let mut item: ItemFn = ...;
    let original_name = item.sig.ident.clone();
    let new_name = format_ident!("__rml_computed_{}", original_name);
    item.sig.ident = new_name;
    // 可选：添加 #[allow(non_snake_case)] 等属性
    quote! { #item }
}
```

codegen 生成（在 `gen_observable_impl` 同一 impl 块中）：
```rust
pub fn <name>(&self) -> <RetType> {
    let v = self.__rml_computed_deps_version("<name>");
    self.__rml_computed_cache.get_or_compute::<RetType, _>(
        "<name>", v, || self.__rml_computed_<name>(),
    )
}
```

**关键**：RetType 必须由 build.rs 扫描时一并提取（`scan_computed_methods` 升级为返回 `Vec<(String, String)>` 即 `(name, ret_type_str)`）。codeguctx 添加 `computed_returns: HashMap<String, String>`。

### Step 7：Demo 验证 + 测试

**文件**：
- `crates/demo/src/main_window.rml.rs`（删除手动 `cx.notify()`，验证自动注入生效）
- `crates/engine/tests/observable_test.rs`（新增集成测试）

**做什么**：
1. Demo 中移除 `on_click` 体内的 `cx.notify()`，编译运行验证功能不变
2. 添加 observable 字段修改追踪的单元测试（验证版本号自增、缓存命中）
3. 添加嵌套 computed 调用的死锁测试

**为什么**：
- 验证用户原诉求"无需手动 notify"已满足
- 验证缓存正确性（依赖未变命中、依赖变化重算）
- 回归 219 个现有测试不破坏

## 范围之外（Phase B-3 处理）

以下功能原本列入 Phase B-2 但与本次 observable 系统正交，留待 Phase B-3：
- **ref 元素注入**：`ref="name"` 的运行时 ElementRef handle API（codegen 已生成 `.id("rml_ref:name")`，缺少用户侧 `self.refs.name` 访问）
- **html 指令**：`html={raw}` 渲染原始 HTML 字符串
- **else/once 指令**：codegen 已识别但不生成对应代码
- **IConverter 扩展**：转换器管道语法已在 expr.rs 支持，缺少标准转换器库
- **BindingPath 运行时订阅**：`IBindingContext` 仍为标记 trait

## 假设与决策

1. **默认 observable**：所有 pub 字段自动追踪，不引入 `#[observable]` 属性。理由：用户偏好最少样板代码，且与 WPF 约定一致。如未来需 opt-out，可追加 `#[no_observe]` 属性。
2. **不剥离用户 `cx.notify()`**：用户已写的 `cx.notify()` 调用保留，宏再注入一个。GPUI 多次 notify 幂等无副作用。
3. **bump_version 对所有字段修改注入**：`#[command]` 不区分 observable/非 observable，对 `self.xxx =` 等模式一律注入 bump 调用。codegen 的 match 对非 observable 字段静默忽略（`_ => {}`）。
4. **ComputedCache 死锁规避**：`get_or_compute` 在调用 compute 前释放 MutexGuard，支持 `#[computed]` 嵌套调用（A 调 B 不死锁）。
5. **computed 返回类型约束**：要求 `T: Clone + Send + 'static`。`Vec<MenuItem>` 等 gpui-component 类型应满足（需在 Step 6 验证）。
6. **build.rs 同步执行**：syn 扫描在 build.rs 中执行，对用户编译时间影响可接受（.rml.rs 通常较小）。
7. **`#[derive(Default)]` 兼容**：注入字段全部实现 `Default`（`AtomicU64: Default = 0`，`ComputedCache: Default = 空 map`），不破坏用户 `#[derive(Default)]`。

## 验证步骤

1. **编译验证**：`cargo build --workspace` 通过
2. **测试验证**：`cargo test --workspace` 全部通过（含 219 个现有测试）
3. **Demo 验证**：`cargo run -p rust-rml-demo` 启动，点击按钮 `count` 自增、`doubled`（若添加）同步更新
4. **行为验证**：在 `on_click` 中删除手动 `cx.notify()`，重新编译运行，UI 仍正确更新
5. **缓存验证**：在 `#[computed]` 方法中添加 `eprintln!("recompute")`，多次访问时只在依赖变更后打印一次
6. **死锁验证**：构造 `#[computed] A` 调用 `#[computed] B` 的场景，运行不阻塞

## 依赖顺序

```
Step 1 (ComputedCache) → Step 2 (字段注入)
                       ↘
Step 3 (build.rs syn 扫描) → Step 4 (CodegenCtx + gen_observable_impl)
                                                ↓
                         Step 6 (#[computed] 重命名 + 缓存包装)
                                                ↓
Step 5 (#[command] bump/notify 注入) → Step 7 (Demo + 测试)
```

Step 1-2 可并行，Step 3-4 串行，Step 5-6 在 4 完成后可并行，Step 7 在 5-6 完成后执行。

## 关键文件改动清单

| 文件 | 操作 | 描述 |
|------|------|------|
| `crates/core/src/computed_cache.rs` | 新建 | ComputedCache 类型 + Default |
| `crates/core/src/lib.rs` | 修改 | 添加 `pub mod computed_cache;` |
| `crates/macros/src/component.rs` | 修改 | `expand_component_impls` 注入 AtomicU64 + ComputedCache 字段 |
| `crates/macros/src/window.rs` | 修改 | 复用 `expand_component_impls` 的字段注入逻辑 |
| `crates/macros/src/command.rs` | 重写 | `syn::visit::Visit` 检测字段修改 + 注入 bump/notify |
| `crates/macros/src/computed.rs` | 修改 | 重命名 `fn xxx` → `fn __rml_computed_xxx` |
| `crates/engine/Cargo.toml` | 修改 | 添加 `syn = { workspace = true }` 依赖 |
| `crates/engine/src/build/scanner.rs` | 修改 | 新增 `ComputedDepVisitor` + syn 解析 |
| `crates/engine/src/build/mod.rs` | 修改 | 调用三个扫描器，结果传入 CodegenCtx |
| `crates/engine/src/compiler/mod.rs` | 修改 | CodegenCtx 添加 `observable_fields`、`computed_deps`、`computed_returns` |
| `crates/engine/src/compiler/codegen.rs` | 修改 | 新增 `gen_observable_impl` + `gen_computed_wrappers` |
| `crates/demo/src/main_window.rml.rs` | 修改 | 删除手动 `cx.notify()`，验证自动注入 |
| `crates/engine/tests/observable_test.rs` | 新建 | 集成测试：版本追踪、缓存命中、嵌套 computed |
