# ComputedCache 分析：RML 为什么需要缓存而手写 GPUI 不需要

> 分析对象：`crates/core/src/computed_cache.rs`
> 核心问题：RML 将 `.rml` 翻译为 gpui 原生代码，与手写本质相同，为何手写不需要缓存？

---

## 一、`computed_cache.rs` 是做什么的

### 1.1 数据结构

`ComputedCache` 是 `#[computed]` 方法的运行时缓存存储，核心只有一个字段：

```rust
pub struct ComputedCache {
    inner: Mutex<HashMap<String, (u64, Box<dyn Any>)>>,
}
```

- **key**：`#[computed]` 方法名（如 `"completed_count"`）
- **value**：`(版本号, 类型擦除的缓存值)`，`u64` 是依赖字段版本号之和，`Box<dyn Any>` 存任意可克隆返回值

### 1.2 核心方法 `get_or_compute`

```rust
pub fn get_or_compute<T: Clone + 'static>(
    &self, key: &str, version: u64, compute: impl FnOnce() -> T,
) -> T {
    // 1. 锁内尝试命中
    { let inner = self.inner.lock().unwrap();
      if let Some((v, val)) = inner.get(key) {
          if *v == version { return val.downcast_ref::<T>().unwrap().clone(); }
      }
    } // ← 释放锁
    // 2. 锁外计算（支持 #[computed] A 调用 #[computed] B 的嵌套，避免死锁）
    let value = compute();
    // 3. 写入缓存
    let mut inner = self.inner.lock().unwrap();
    inner.insert(key.to_string(), (version, Box::new(value.clone())));
    value
}
```

关键设计：
- **锁外计算**：`compute` 闭包在 `MutexGuard` 释放后执行，支持 `#[computed] A` 内部调用 `#[computed] B`（B 会再次 `get_or_compute` 同一 cache），避免重入死锁
- **返回 Clone 而非引用**：避免返回引用穿过 `MutexGuard`，`T: Clone` 约束
- **`unsafe impl Send + Sync`**：缓存值经 `Box<dyn Any>` 类型擦除，可能含非 `Send` 的 GPUI 类型（如 `Vec<TabItem>` 含 `Rc`）；安全性靠 `Mutex` 串行化 + `#[computed]` 仅在 render 线程调用保证

### 1.3 它在整个 `#[computed]` 管线中的位置

`ComputedCache` 是 `#[computed]` 三阶段管线的运行时落点：

| 阶段 | 位置 | 职责 |
|------|------|------|
| 1. 宏改写 | `crates/macros/src/computed.rs` | `fn name` → `fn __rml_computed_name`（保留方法体，仅改名） |
| 2. 依赖扫描 | `crates/engine/src/build/scanner.rs` | `syn::Visit` 扫描方法体中 `self.<field>` 访问，收集 `computed_deps: HashMap<方法名, Vec<依赖字段>>` |
| 3. 代码生成 | `crates/engine/src/compiler/codegen/observable.rs` | 生成版本方法 + 缓存包装方法 |

codegen 生成的包装方法（[observable.rs:118-124](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/observable.rs#L118-L124)）：

```rust
pub fn completed_count(&self) -> usize {
    let __v = self.__rml_computed_deps_version("completed_count");
    self.__rml_computed_cache
        .get_or_compute::<usize>("completed_count", __v, || self.__rml_computed_completed_count())
}
```

其中 `__rml_computed_deps_version` 把依赖字段的 `AtomicU64` 版本号求和作为缓存键：

```rust
fn __rml_computed_deps_version(&self, computed: &str) -> u64 {
    match computed {
        "completed_count" => self.__rml_get_version("todos"),
        "pending_count" => self.__rml_get_version("todos") + self.__rml_get_version("completed_count"),
        ...
    }
}
```

字段版本号由 `#[command]` 宏在修改字段后注入的 `__rml_bump_version("field")` 递增。

---

## 二、为什么组件需要缓存

### 2.1 问题：声明式绑定产生「独立调用点」

RML 模板（`.rml`）中每个 `{binding}` 都被 codegen **独立翻译**为一个方法调用（[mod.rs:585-601](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/mod.rs#L585-L601)）：

```rust
// gen_expr_code: 引用名若命中 computed_methods，生成 self.name()
Ok(expr::Expr::Field(name)) if computed.iter().any(|c| *c == name) => {
    format!("self.{}()", name)
}
```

考虑文档 [computed.md](file:///d:/GitCode/RF/rust-gpui-rml/docs/03-binding/computed.md) 中的例子：

```html
<p>已完成: {completed_count}</p>
<p>待办: {pending_count}</p>
<p>进度: {progress}</p>
```

```rust
#[computed] pub fn completed_count(&self) -> usize { self.todos.iter().filter(|t| t.done).count() }
#[computed] pub fn pending_count(&self) -> usize { self.todos.len() - self.completed_count() }
#[computed] pub fn progress(&self) -> f64 { self.completed_count() as f64 / self.todos.len() as f64 * 100.0 }
```

codegen 生成的 `render()` 中，三个绑定互不感知，各自展开为：

```rust
format!("已完成: {}", self.completed_count())      // 调用 1
format!("待办: {}", self.pending_count())          // 内部再调 completed_count() → 调用 2
format!("进度: {}", self.progress())               // 内部再调 completed_count() → 调用 3
```

**单次 render 内 `completed_count` 被调用 3 次，`todos.iter().filter().count()` 遍历 3 次。**

这是声明式模板的固有特性：**绑定是树节点的独立属性，codegen 逐节点翻译，无法跨兄弟节点共享局部变量。** 缓存让第 2、3 次调用命中（版本号未变），把 3 次遍历降为 1 次。

### 2.2 问题：跨 render 的重复计算

GPUI 的 `cx.notify()` 触发整个 `render()` 重跑。手写代码每次 render 都全量重算；RML 同样如此——但 RML 拥有字段版本号系统。

即使两次 render 间 `todos` 没变（可能是别的字段变了触发 `cx.notify()`），`completed_count` 的依赖版本号未变，缓存直接命中，跳过遍历。

这是 **跨 render 记忆化**，手写代码需要开发者手动维护（存字段 + 变更时更新）。

---

## 三、为什么手写 GPUI 不需要缓存（挑战前提）

用户的提问预设「手写不需要缓存」。这个前提需要拆解——**手写不是不需要，而是用另一种方式解决了同一个问题。**

### 3.1 手写的「缓存」= 局部变量（结构性免费）

手写 `render()` 是**单一函数、共享作用域**，开发者天然用局部变量去重：

```rust
fn render(&mut self, _, cx) -> impl IntoElement {
    let count = self.todos.iter().filter(|t| t.done).count();  // 算一次
    let pending = self.todos.len() - count;                     // 复用 count
    let progress = count as f64 / self.todos.len() as f64;      // 复用 count
    div()
        .child(format!("已完成: {}", count))
        .child(format!("待办: {}", pending))
        .child(format!("进度: {}", progress))
}
```

- 编译器原生支持局部变量，零运行时开销
- 开发者有语义感知，知道 `count` 被多处使用，主动 hoist
- 这是**结构性优势**：手写代码是线性流程，RML 模板是树状独立节点

**RML 的 codegen 无法做这种 hoist**：模板的三个 `<p>` 是兄弟节点，各自独立翻译，生成的代码没有「先算 count 再用三次」的共享局部作用域。要 hoist 需要 codegen 做跨节点别名分析（同一 computed 在多处引用 → 抽到 render 顶部），实现复杂且只能解决渲染内去重，无法解决跨 render 记忆化。`ComputedCache` 用统一机制同时解决两者。

### 3.2 手写接受「全量重算」，RML 提供「细粒度记忆化」作为特性

| 维度 | 手写 GPUI | RML |
|------|-----------|-----|
| render 内去重 | 局部变量（开发者手动 hoist） | `ComputedCache`（自动） |
| 跨 render 记忆化 | 默认全量重算；如需缓存，开发者手动存字段 + 在变更点更新 | `#[computed]` + 版本号系统自动失效 |
| 变更感知 | 无字段级追踪，`cx.notify()` 一刀切全量重渲染 | `#[command]` 注入 `bump_version`，字段级版本追踪 |

手写代码里，如果 `completed_count` 真的很贵，开发者**也会手动缓存**：

```rust
struct TodoView { cached_completed: Option<usize>, cached_version: u64, ... }
// 在 render 开头检查 todos 版本，未变则用 cached_completed
```

这正是 `ComputedCache` 自动做的事。**手写不是不需要缓存，而是把缓存逻辑交给开发者判断——简单场景局部变量够了，昂贵场景手动 memo。** RML 把这个决策自动化了：所有 `#[computed]` 默认带缓存，开发者无需评估「这个方法够不够贵到要 memo」。

### 3.3 根本差异：声明式 vs 命令式的作用域模型

这才是问题的本质，而非「翻译后代码一样所以需求一样」：

```
手写 render()                    RML 模板
┌─────────────────┐              ┌──────────────────┐
│ 单一函数作用域    │              │ 树状独立节点       │
│ let x = ...;    │              │ <p>{x}</p>       │
│ use x 3 times   │              │ <p>{y(x)}</p>    │
│ ↑ 局部变量天然去重 │              │ <p>{z(x)}</p>    │
└─────────────────┘              │ ↑ 三处独立调用点    │
                                 │ 无共享作用域，需缓存 │
                                 └──────────────────┘
```

**「翻译为原生代码」描述的是最终产物，但翻译过程引入了手写不存在的结构：独立调用点。** 缓存是对这种结构差异的补偿。

类比：手写 SQL `SELECT ... WHERE x IN (1,2,3)` 与 ORM 生成的参数化查询，最终都执行 SQL，但 ORM 多了对象映射开销——「最终都是 SQL」不等于「开销相同」。RML 同理：最终都是 `render()`，但绑定展开方式引入了冗余调用，缓存消除冗余。

---

## 四、结论

1. **`ComputedCache` 是 `#[computed]` 方法的版本化记忆化存储**，靠 `Mutex<HashMap<方法名, (版本号, 值)>>` + 锁外计算实现线程安全与嵌套安全。

2. **RML 需要缓存的根因是声明式模板的「独立调用点」结构**：每个 `{binding}` 独立翻译为方法调用，跨节点无共享局部作用域，同一 computed 在一次 render 内可能被调多次。缓存去重这些调用。

3. **手写不需要缓存的提法不成立**——手写用局部变量做渲染内去重（结构性免费），用手动 memo 做跨渲染缓存（按需）。RML 的缓存同时自动化了这两件事：补偿 codegen 无法 hoist 局部变量的缺陷 + 提供版本号驱动的自动记忆化特性。

4. **「翻译为原生代码相同」≠「运行时行为相同」**：翻译过程引入的结构（独立调用点）是手写没有的冗余，缓存正是对此的补偿，外加框架级的开发便利。

5. **理论上 RML 也可以不用缓存**：若 codegen 做跨节点别名分析把共享 computed hoist 到 render 顶部局部变量，可消除渲染内冗余。但跨渲染记忆化仍需版本号系统，且 hoist 实现复杂度高。`ComputedCache` 用统一机制覆盖两层需求，是更简洁的工程选择。
