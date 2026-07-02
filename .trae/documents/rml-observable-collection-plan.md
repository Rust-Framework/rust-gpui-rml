# RML ObservableCollection 与响应式数据驱动能力增强计划

## Context

当前 RML 框架的响应式能力存在根本性缺口：集合数据（`Vec<T>`）的变更依赖 `#[command]` 宏的 AST 模式匹配（仅识别 `push`/`pop`/`clear` 等 6 个方法），无法检测间接修改（`let p = &mut self.items; p.push()`）和外部方法调用；且无细粒度通知，任何 Vec 变更都导致整个 `#[computed]` 缓存失效。

在实际应用中，这意味着大量样板代码：Demo 的 `refresh_shell_chrome` + `map_shell_chrome` + `subscribe_host_changes` 三件套手动桥接贡献点数据到 ViewModel 字段，每次贡献变更都要全量重建 menu/status/activity 面板。用户的目标是**数据 + 模板驱动界面渲染**——给集合增删改数据时不用额外写代码促成 UI 响应，且确保高性能。

本计划引入 WPF `ObservableCollection<T>` 等价能力，通过三个核心机制达成目标：
1. **`ObservableVec<T>`** —— 版本号驱动的集合类型，mutation 自动 bump version
2. **`#[computed_with_cx]`** —— 支持 `cx` 访问的计算属性，缓存键可包含外部 revision
3. **RML `each=` + `key=`** —— render 时 keyed diffing，element 复用，高性能列表更新

### 用户决策

| 决策点 | 选择 |
|--------|------|
| 通知模型 | 仅版本号 + render 时 key diffing（无 CollectionChange 事件） |
| `#[computed]` 扩展 | 纳入：实现 `#[computed_with_cx]` |
| Host API | `IContributionHost` trait 增加 `add`/`remove` 方法 |

---

## Phase A：`ObservableVec<T>` 核心类型

### 新建文件

**`crates/core/src/observable.rs`** —— 集合响应式核心类型

```rust
use std::sync::atomic::{AtomicU64, Ordering};

/// 版本号驱动的可观察集合。
///
/// Mutation 方法（push/insert/remove/swap/clear/replace_range）自动 bump version。
/// 与 RML 版本系统集成的桥梁：
/// - `__rml_get_version("field")` 路由到 `self.field.version()`
/// - `#[computed]` 缓存键自动包含集合版本
/// - `#[command]` 宏注入的 `__rml_bump_version` 对 ObservableVec 字段为 no-op
pub struct ObservableVec<T> {
    inner: Vec<T>,
    version: AtomicU64,
}

impl<T> ObservableVec<T> {
    pub fn new() -> Self { ... }
    pub fn with_capacity(n: usize) -> Self { ... }
    pub fn from(vec: Vec<T>) -> Self { ... }

    /// 当前版本号（每次 mutation 递增）
    pub fn version(&self) -> u64 { self.version.load(Ordering::Relaxed) }

    pub fn len(&self) -> usize { self.inner.len() }
    pub fn is_empty(&self) -> bool { self.inner.is_empty() }

    // —— mutation 方法：bump version ——
    pub fn push(&mut self, item: T) { self.inner.push(item); self.bump(); }
    pub fn insert(&mut self, index: usize, item: T) { self.inner.insert(index, item); self.bump(); }
    pub fn remove(&mut self, index: usize) -> T { let v = self.inner.remove(index); self.bump(); v }
    pub fn swap(&mut self, a: usize, b: usize) { self.inner.swap(a, b); self.bump(); }
    pub fn clear(&mut self) { self.inner.clear(); self.bump(); }
    pub fn replace_range(&mut self, range: std::ops::Range<usize>, items: impl IntoIterator<Item = T>) { ... self.bump(); }
    pub fn retain<F: FnMut(&T) -> bool>(&mut self, f: F) { self.inner.retain(f); self.bump(); }

    // —— 只读访问（支持 each= codegen 生成 .iter()）——
    pub fn iter(&self) -> std::slice::Iter<'_, T> { self.inner.iter() }
    pub fn get(&self, index: usize) -> Option<&T> { self.inner.get(index) }
    pub fn as_slice(&self) -> &[T] { &self.inner }

    fn bump(&self) { self.version.fetch_add(1, Ordering::Relaxed); }
}

impl<T> std::ops::Deref for ObservableVec<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { &self.inner }
}

// 无 DerefMut —— 防止绕过 version bump 的未追踪修改

impl<T: Send + Sync + 'static> Send for ObservableVec<T> {}
impl<T: Send + Sync + 'static> Sync for ObservableVec<T> {}
```

**关键设计决策：**
- **无 `DerefMut`** —— 强制通过专用 mutation 方法修改，确保 version 总是 bump
- **无 listener / 无 CollectionChange 事件** —— 版本号是唯一通知机制，GPUI 每次 render 重建元素，diffing 在 render 时通过 key 比对完成
- **`AtomicU64` version** —— 与现有 `__rml_<field>_version: AtomicU64` 系统一致，lock-free 读取
- **`Send + Sync`** —— `Vec<T: Send+Sync>` + `AtomicU64` 天然满足，无需 unsafe

**导出：** 在 `crates/core/src/lib.rs` 添加 `pub mod observable;` + `pub use observable::ObservableVec;`，在 `prelude.rs` 添加导出。

**单元测试：** 同文件 `#[cfg(test)]` 模块，验证 mutation 后 version 递增、`Deref` 读取正确、无 `DerefMut` 编译失败。

---

## Phase B：版本系统 + `#[computed]` 集成

### 目标

让 `ObservableVec<T>` 字段自动接入版本系统：
- `__rml_get_version("field")` 返回 `self.field.version()`（而非 `self.__rml_field_version.load()`）
- `#[computed]` 方法依赖 `ObservableVec` 字段时，缓存键自动包含集合版本
- `#[command]` 宏无需修改——注入的 `__rml_bump_version("field")` 对 ObservableVec 字段为 no-op（match 默认 arm `_ => {}`），`cx.notify()` 照常触发重渲

### 修改文件

**`crates/engine/src/build/scanner.rs`** —— 编译期检测 ObservableVec 字段类型

当前 `StructMetadata`（L42-60）已有 `field_types: HashMap<String, String>`。扩展：
```rust
pub struct StructMetadata {
    pub observable_fields: Vec<String>,
    pub computed_methods: Vec<String>,
    pub field_types: HashMap<String, String>,
    pub observable_vec_fields: Vec<String>,  // 新增：类型为 ObservableVec<...> 的字段
}
```
在 `scan_struct_metadata`（L87+）的字段类型解析逻辑中，当 `cleaned` 以 `"ObservableVec<"` 开头时，将字段名加入 `observable_vec_fields`。

**`crates/engine/src/compiler/codegen/observable.rs`** —— `gen_observable_impl` 版本路由

当前 L23-32 为每个 `version_fields` 字段生成 `bump_arms` + `get_arms`。修改逻辑：
- `bump_arms`：跳过 `observable_vec_fields` 中的字段（不生成 match arm，落入默认 `_ => {}` no-op）
- `get_arms`：对 `observable_vec_fields` 中的字段，生成 `self.{field}.version()` 而非 `self.__rml_{field}_version.load(...)`

```rust
for field in version_fields {
    if ctx.observable_vec_fields.contains(field) {
        // bump: skip (no-op via default arm)
        // get: route to ObservableVec::version()
        get_arms.push_str(&format!(
            "            \"{}\" => self.{}.version(),\n",
            field, field
        ));
    } else {
        bump_arms.push_str(&format!(...));  // 原逻辑
        get_arms.push_str(&format!(...));   // 原逻辑
    }
}
```

**`crates/macros/src/component.rs`** —— 跳过 ObservableVec 字段的 `__rml_<field>_version` 注入

当前 L94-101 为每个 observable 字段注入 `__rml_<field>_version: AtomicU64` 字段。修改：对 `observable_vec_fields` 中的字段跳过注入（ObservableVec 内部已有自己的 version）。

**`crates/engine/src/build/mod.rs`** —— 传递 `observable_vec_fields` 到 `CodegenCtx`

L295 附近将 `observable_vec_fields` 从 `StructMetadata` 传入 `CodegenCtx`。

### 效果

```rust
#[component]
pub struct MyView {
    items: ObservableVec<String>,  // 无 __rml_items_version 注入
    name: String,                  // 有 __rml_name_version 注入
}

#[computed]
fn item_count(&self) -> usize {
    self.items.len()  // 依赖 items，缓存键 = self.items.version()
}

#[command]
fn add_item(&mut self, cx: &mut Context<Self>) {
    self.items.push("new".into());  // ObservableVec 内部 bump version
    // 宏注入：self.__rml_bump_version("items"); → no-op（默认 arm）
    // 宏注入：cx.notify(); → 触发重渲
    // 重渲时：item_count() 缓存键变化 → 自动重算
}
```

---

## Phase C：`#[computed_with_cx]` 扩展

### 目标

当前 `#[computed]`（`crates/macros/src/computed.rs`）强制 `&self` 无参，无法访问 `cx`。这导致 computed 方法无法调用 `contribution_entries(host_id, cx)` 读取全局注册表数据——是 `refresh_shell_chrome` 样板代码存在的根本原因。

`#[computed_with_cx]` 是新宏属性，允许 `&self, cx: &Context<Self>` 签名，缓存键可包含外部 revision（如 `contribution_revision`）。

### 新建文件

**`crates/macros/src/computed_with_cx.rs`** —— 新宏实现

```rust
/// #[computed_with_cx(revision = expr)]
/// pub fn method(&self, cx: &Context<Self>) -> RetType { ... }
///
/// 生成的包装方法：
/// pub fn method(&self, cx: &Context<Self>) -> RetType {
///     let __v = self.__rml_computed_deps_version("method") + (expr);
///     self.__rml_computed_cache.get_or_compute::<RetType>("method", __v, || self.__rml_computed_with_cx_method(cx))
/// }
///
/// 其中 `revision = expr` 的 expr 在 &self + &Context<Self> 作用域内求值，
/// 典型用法：revision = contribution_revision(Self::ID, cx)
```

**校验规则：**
- 签名必须是 `&self, cx: &Context<Self>`（或 `cx: &Context<View>`）
- `revision =` 属性必填（提供外部版本源，否则退化为普通 `#[computed]`）
- 方法体重命名为 `__rml_computed_with_cx_{name}`

### 修改文件

**`crates/macros/src/lib.rs`** —— 注册新 proc-macro attribute

```rust
#[proc_macro_attribute]
pub fn computed_with_cx(attr: TokenStream, item: TokenStream) -> TokenStream { ... }
```

**`crates/engine/src/compiler/codegen/observable.rs`** —— `gen_computed_wrappers` 扩展

新增 `gen_computed_with_cx_wrappers` 函数，生成带 `cx` 参数的包装方法。缓存键 = `__rml_computed_deps_version("method") + revision_expr_value`。

**`crates/engine/src/build/scanner.rs`** —— 检测 `#[computed_with_cx]` 方法

在方法扫描逻辑中识别 `#[computed_with_cx]` 属性，记录方法名 + revision 表达式到 `StructMetadata.computed_with_cx_methods: Vec<(String, String)>`。

### 使用示例

```rust
#[computed_with_cx(revision = contribution_revision(Self::ID, cx))]
pub fn menu_items(&self, cx: &Context<Self>) -> MenuItems {
    let entries = contribution_entries(Self::ID, cx);
    map_menu_items(entries, &self.menu_commands)
}
```

贡献点注册 → `ObservableVec` bump version → `contribution_revision` 递增 → `menu_items` 缓存键变化 → 自动重算 → RML 模板读取新值 → UI 更新。**零样板代码。**

---

## Phase D：`IContributionHost` trait + 存储重构

### 目标

1. `IContributionHost` trait 增加 `add`/`remove` 方法，提供 `host.add(contribution)` API
2. `ContributionHost` 存储改为 `ObservableVec<ContributedEntry>`，version 驱动响应式
3. `subscribe_host_changes` 简化（从手动 refresh 回调退化为 `cx.notify()`）

### 修改文件

**`crates/core/src/contribution.rs`** —— trait 扩展

```rust
pub trait IContributionHost: Send + Sync + 'static {
    const ID: &'static str;

    /// 向 host 注册贡献。默认实现路由到全局注册表。
    fn add(&self, contribution: Arc<dyn IContribution>, options: ContributionOptions, cx: &mut gpui::App) {
        cx.register(Self::ID, contribution, options);
    }

    /// 从 host 注销贡献。默认实现路由到全局注册表。
    fn remove(&self, contribution_id: &str, cx: &mut gpui::App) -> bool {
        cx.unregister(Self::ID, contribution_id)
    }
}
```

**设计决策：** trait 方法提供默认实现（路由到 `ContributionExt`），用户通常无需重写。这保留了"trait = 契约，存储 = app 关注点"的分层——trait 定义 API，存储仍由全局注册表管理。`Send + Sync + 'static` bound 确保 host 可作为 `Entity<T>` 存储。

**`crates/app/src/contribution/host.rs`** —— 存储改用 ObservableVec

```rust
pub struct ContributionHost {
    id: String,
    entries: ObservableVec<ContributedEntry>,  // 原 Vec<ContributedEntry>
    // revision: AtomicU64 移除（ObservableVec 内部已有）
}

impl ContributionHost {
    pub fn revision(&self) -> u64 { self.entries.version() }

    pub fn add(&mut self, entry: ContributedEntry, _cx: &mut App) {
        let id = entry.contribution.id().to_string();
        // dedup + sort 逻辑保留，但通过 ObservableVec 的 mutation 方法操作
        // 需要先 remove 旧的同 id 条目，再 push 新的，最后 sort
        // 注意：sort 需要直接访问 inner Vec —— 提供 ObservableVec::sort_by_mut 或在 host 层处理
        self.entries.retain(|e| e.contribution.id() != id);
        self.entries.push(entry);
        self.sort_entries();  // 需要 &mut Vec 访问 —— 见下方"sort 方案"
    }
}
```

**sort 方案：** `ObservableVec` 不暴露 `&mut Vec<T>`（会绕过 version bump）。为支持 sort，在 `ObservableVec` 增加 `sort_by_mut<F: FnMut(&T, &T) -> Ordering>(&mut self, f: F)` 方法，内部调用 `self.inner.sort_by(f)` + `self.bump()`。这是一个有意的 mutation 入口，bump version。

**`crates/app/src/contribution/global.rs`** —— `contribution_revision` 读取 ObservableVec version

```rust
pub fn contribution_revision<C>(host_id: &str, cx: &Context<C>) -> u64 {
    if !cx.has_global::<ContributionRegistryGlobal>() { return 0; }
    cx.global::<ContributionRegistryGlobal>().0.revision(host_id)
    // revision() 内部返回 entries.version()
}
```

**`crates/app/src/contribution/registry.rs`** —— `notify_host` 保留，`revision` 改读 ObservableVec

`ContributionRegistry` 的 `notify_host`（L74-80）保留——它负责将变更通知到订阅 Entity。`HostListener` 回调签名不变（`Fn(&mut App)`），但消费者的回调逻辑简化为 `|_, cx| cx.notify()`（不再需要 `refresh_shell_chrome`）。

### 效果

```rust
// 贡献注册（#[contribute] 宏生成的代码）：
MainWindow::add(&MainWindow, Arc::new(MyCase::default()), options, cx);
// 或等价的：cx.register("demo.shell", contribution, options);

// 内部流程：
// 1. ContributionExt::register → registry.register_entry → host.add(entry, cx)
// 2. host.add → ObservableVec::push (bump version) → registry.notify_host
// 3. notify_host → HostListener 回调 → entity.update(cx, |_, cx| cx.notify())
// 4. 重渲 → #[computed_with_cx(revision = contribution_revision(...))] 缓存失效 → 自动重算
// 5. RML each=+key= keyed diffing → element 复用 → UI 更新
```

---

## Phase E：RML `each=` + `key=` keyed diffing

### 目标

当前 RML `each=` 生成 `self.field.iter().map(|item| { code })` → `.children(iter)`，每次 render 全量重建元素。引入 `key=` 指令后，render 时通过 key 比对复用已存在的元素，避免状态丢失 + 提升性能。

### 修改文件

**`crates/core/src/observable.rs`** —— 新增 `reconcile` 辅助函数

```rust
/// Keyed reconciliation：比对前一次 render 的 (key, element) 列表与当前 items，
/// 复用匹配 key 的 element，为新 key 调用 builder 构建 element，移除消失的 key。
///
/// 返回新的 (key, element) 列表（供下一次 render 比对）+ 元素引用迭代器。
pub fn reconcile<T, K, F>(
    prev: Vec<(K, gpui::AnyElement)>,
    items: impl IntoIterator<Item = (K, T)>,
    builder: F,
) -> Vec<(K, gpui::AnyElement)>
where
    K: Eq + std::hash::Hash + Clone,
    F: Fn(&T) -> gpui::AnyElement,
{
    // 1. 构建 prev 的 key→element HashMap
    // 2. 遍历 items，对每个 key：
    //    - 若 prev 中存在，复用 element（不调用 builder）
    //    - 若不存在，调用 builder 构建新 element
    // 3. 返回新的 (key, element) Vec，顺序与 items 一致
}
```

**`crates/engine/src/compiler/codegen/mod.rs`** —— `gen_node` each= 分支

当前 L433-440 的 `each=` codegen 无条件生成 `.iter().map(...)`。修改为三分支：

```rust
// 伪代码
let is_observable_vec = ctx.observable_vec_fields.contains(&clause.iterable);
let has_key = directives.iter().any(|d| matches!(d, Directive::Key(_)));

if is_observable_vec && has_key {
    // Case B：ObservableVec + key= → keyed diffing
    let key_expr = extract_key_expr(directives);
    format!("{{ ... reconcile ... }}")
} else {
    // Case A/C：plain Vec 或无 key= → 原逻辑 .iter().map(...)
    format!("self.{}.iter().map(|{}| {{ {} }})", ...)
}
```

**生成的 keyed diffing 代码模式：**

```rust
// RML 模板：<div each="item in items" key={item.id}>...</div>
// 生成代码：
{
    let __rml_key_fn = |item: &Item| item.id.clone();
    let __rml_new_keys: Vec<_> = self.items.iter().map(__rml_key_fn).collect();
    self.__rml_items_children = rml_core::observable::reconcile(
        std::mem::take(&mut self.__rml_items_children),
        self.items.iter().map(|item| (__rml_key_fn(item), item)),
        |item| {
            // 原始 each= body 代码，item 已在作用域内
            { /* generated element code */ }
        },
    );
    self.__rml_items_children.iter().map(|(_, el)| el.clone())
}
// 父元素使用 .children(上述迭代器)
```

**`crates/macros/src/component.rs`** —— 注入 `__rml_{field}_children` 字段

对 RML 模板中使用了 `each=` + `key=` 的 `ObservableVec` 字段，注入：
```rust
#[allow(non_snake_case)]
__rml_items_children: Vec<(String, gpui::AnyElement)>,
```
类似当前 `__rml_input_states` 的惰性初始化模式。扫描器需检测 RML 模板中的 `each=` + `key=` 组合并记录对应字段。

**`crates/engine/src/parser/ast.rs`** —— `Directive::Key` 已解析（L36），需在 codegen 中消费

当前 `key=` 被解析为 `Directive::Key(Expr)` 但无 codegen 消费者。Phase E 在 `gen_node` 中提取 `Directive::Key` 的表达式并传入 keyed diffing 分支。

### 性能特性

- **Element 复用：** 相同 key 的 element 跨 render 复用，保留内部状态（如 InputState、滚动位置）
- **增量构建：** 仅新 key 触发 builder 调用，已存在 key 的 builder 不执行
- **顺序保持：** 输出顺序与 items 迭代顺序一致，支持 reorder
- **复杂度：** O(n) reconcile（HashMap 查找），n = items 长度

---

## Phase F：Demo 样板代码消除

### 目标

消除 `demo/src/shell/` 中的响应式桥接样板：
- `refresh_shell_chrome` 方法删除
- `subscribe_host_changes` 回调简化为 `|_, cx| cx.notify()`
- `map_shell_chrome` 转为 `#[computed_with_cx]` 方法
- `menu_shell_contribs.rs` 的菜单定义可保留为 `#[contribute]` 声明（贡献点机制仍用于扩展），但 shell chrome 映射层消除

### 修改文件

**`demo/src/shell/main_window.rml.rs`** —— MainWindow 重构

```rust
// —— 删除 ——
// fn refresh_shell_chrome(&mut self, cx: &mut Context<Self>) { ... }
// subscribe_host_changes(Self::ID, cx, |this, cx| { this.refresh_shell_chrome(cx); cx.notify(); });

// —— 替换为 ——
// on_loaded 末尾：
subscribe_host_changes(Self::ID, cx, |_, cx| {
    cx.notify(); // 仅触发重渲，computed_with_cx 自动重算
});

// —— 新增 #[computed_with_cx] 方法 ——
#[computed_with_cx(revision = contribution_revision(Self::ID, cx))]
pub fn menu_items(&self, cx: &Context<Self>) -> MenuItems {
    let entries = contribution_entries(Self::ID, cx);
    map_menu_items(entries, &self.menu_commands)
}

#[computed_with_cx(revision = contribution_revision(Self::ID, cx))]
pub fn status_items(&self, cx: &Context<Self>) -> StatusBarItems {
    let entries = contribution_entries(Self::ID, cx);
    map_status_items(entries)
}

#[computed_with_cx(revision = contribution_revision(Self::ID, cx))]
pub fn activity_panels(&self, cx: &Context<Self>) -> ActivityPanels {
    let entries = contribution_entries(Self::ID, cx);
    map_activity_panels(entries)
}
```

**`demo/src/shell/shell_chrome.rs`** —— `map_shell_chrome` 拆分为独立 projection 函数

`map_shell_chrome`（L135-145）拆分为 `map_menu_items` / `map_status_items` / `map_activity_panels`，各自作为 `#[computed_with_cx]` 方法的 body。`ShellChromeBindings` 结构体可删除。

**`demo/src/shell/main_window.rml`** —— 模板绑定到 computed_with_cx 方法

```rml
// 修改前
<menu items={menu_items} />
<status_bar items={status_items} />

// 修改后（computed_with_cx 方法通过字段绑定语法访问）
<menu items={menu_items} />
<status_bar items={status_items} />
```

RML 模板无需改动——`menu_items`/`status_items` 现在是 `#[computed_with_cx]` 方法而非普通字段，但 RML 的 `items={menu_items}` 绑定生成的 `self.menu_items(cx)` 调用需要 codegen 支持 computed_with_cx 方法的 cx 参数传递。

**Codegen 调整：** `crates/engine/src/compiler/codegen/` 中，当绑定目标为 `computed_with_cx` 方法时，生成 `self.{method}(cx)` 而非 `self.{field}.clone()`。需要 scanner 标记 computed_with_cx 方法名，codegen 查表区分。

### 复杂多层级数据结构（树形菜单）

菜单树通过 `parent_id` 链构建层级。`map_menu_items` 内部递归分组：
```rust
fn map_menu_items(entries: &[ContributedEntry], commands: &HashMap<...>) -> MenuItems {
    // 1. 按 parent_id 分组：HashMap<parent_id, Vec<entry>>
    // 2. 递归构建树：root entries (parent_id=None) → children → grandchildren
    // 3. 返回扁平化的 MenuItems（MenuBar 内部处理层级展开）
}
```

当贡献点增删时，`contribution_revision` 变化 → `menu_items` computed_with_cx 缓存失效 → 自动重新构建整棵树 → RML `each=` + `key=` keyed diffing 复用现有 element。**树形结构的 CRUD 无需额外 UI 代码。**

---

## 验证步骤

### Phase A 验证
```bash
cargo test -p rust-rml-core -- observable
```
验证 ObservableVec mutation 后 version 递增、`Deref<[T]>` 读取、无 `DerefMut`。

### Phase B 验证
```bash
cargo build -p rust-rml-engine -p rust-rml-macros
cargo test -p rust-rml-engine -- codegen::observable
```
验证 `__rml_get_version` 对 ObservableVec 字段路由到 `self.field.version()`，`__rml_bump_version` 对 ObservableVec 字段为 no-op。

### Phase C 验证
```bash
cargo build -p rust-rml-macros
cargo test -p rust-rml-engine -- computed_with_cx
```
验证 `#[computed_with_cx(revision = ...)]` 生成正确的缓存包装，缓存键包含 revision 表达式值。

### Phase D 验证
```bash
cargo build -p rust-rml-core -p rust-rml-app
cargo test -p rust-rml-app -- contribution
```
验证 `IContributionHost::add`/`remove` 默认实现路由正确，`ContributionHost.entries` 为 ObservableVec，`revision()` 返回 ObservableVec::version()。

### Phase E 验证
```bash
cargo test -p rust-rml-engine -- codegen::each_key
cargo run -p rust-rml-demo
```
验证 `each=` + `key=` 生成 keyed diffing 代码，运行 demo 切换 tab/打开 case 时 element 复用（可通过日志或视觉验证无闪烁）。

### Phase F 验证
```bash
cargo build -p rust-rml-demo
cargo run -p rust-rml-demo
```
验证：
1. Demo 启动后 menu/status/activity 面板正确显示
2. 通过菜单打开 case → tab 新增 → UI 更新（无 refresh_shell_chrome 调用）
3. 切换语言 → 菜单标题更新（computed_with_cx 缓存失效）
4. ActivityBar 面板切换正常

---

## 关键文件清单

| 文件 | Phase | 操作 |
|------|-------|------|
| `crates/core/src/observable.rs` | A | 新建 |
| `crates/core/src/lib.rs` | A | 导出 observable 模块 |
| `crates/core/src/prelude.rs` | A | 导出 ObservableVec |
| `crates/engine/src/build/scanner.rs` | B, C, E | 检测 ObservableVec 字段 + computed_with_cx 方法 |
| `crates/engine/src/compiler/codegen/observable.rs` | B, C | 版本路由 + computed_with_cx wrapper 生成 |
| `crates/macros/src/component.rs` | B, E | 跳过 ObservableVec version 注入 + children 字段注入 |
| `crates/macros/src/computed_with_cx.rs` | C | 新建 |
| `crates/macros/src/lib.rs` | C | 注册 computed_with_cx proc-macro |
| `crates/engine/src/compiler/codegen/mod.rs` | E | each= + key= keyed diffing 分支 |
| `crates/core/src/contribution.rs` | D | IContributionHost trait add/remove |
| `crates/app/src/contribution/host.rs` | D | 存储改用 ObservableVec |
| `crates/app/src/contribution/global.rs` | D | contribution_revision 读 ObservableVec version |
| `crates/app/src/contribution/registry.rs` | D | revision 路由 |
| `demo/src/shell/main_window.rml.rs` | F | 删除 refresh_shell_chrome + computed_with_cx 方法 |
| `demo/src/shell/shell_chrome.rs` | F | 拆分为独立 projection 函数 |

---

## 假设与风险

### 假设
1. `ObservableVec` 的 `sort_by_mut` 方法是有意 mutation 入口，bump version——满足 `ContributionHost::add` 的 dedup+sort 需求
2. `#[computed_with_cx]` 的 `revision = expr` 表达式在 `&self` + `&Context<Self>` 作用域内求值——`contribution_revision(host_id, cx)` 满足此约束
3. RML `each=` + `key=` 的 key 表达式返回 `String`（或可 `Eq + Hash + Clone`），通过 `key={item.id}` 语法指定
4. `computed_with_cx` 方法在 RML 模板中通过 `items={method_name}` 绑定，codegen 识别并生成 `self.method_name(cx)` 调用

### 风险
1. **`computed_with_cx` 缓存键碰撞：** 若 `revision` 表达式返回的值与 `__rml_computed_deps_version` 求和后碰撞，可能导致缓存未失效。缓解：revision 使用 `u64`，碰撞概率极低
2. **keyed diffing 的 element clone 开销：** `gpui::AnyElement::clone()` 可能非廉价（取决于内部 `Arc` 引用计数）。缓解：AnyElement 通常是 `Arc`-backed，clone 为引用计数递增
3. **`computed_with_cx` 方法的 cx 借用冲突：** 若方法体内同时需要 `&self` 读取和 `cx` 读取，可能触发借用检查错误。缓解：方法体内先 clone 需要的数据再释放 `&self` 借用
4. **`menu_shell_contribs.rs` 的 `#[contribute]` 声明保留：** 贡献点声明本身是数据驱动的（宏自动注册），不应消除。消除的是 `shell_chrome.rs` 的映射层 + `refresh_shell_chrome` 的手动刷新层
