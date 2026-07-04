# MainWindow 直接实现 IContributionHost：Entity 感知 Registry 方案

## 摘要

消除 `MainWindowHostHandle` 适配器，让 `MainWindow` 直接 `impl IContributionHost for MainWindow`。

**根因分析**：`registry.add(host: Arc<dyn IContributionHost>)` 要求 `Arc`，但 GPUI Entity 不暴露 `Arc<T>`，故需 `MainWindowHostHandle` 共享 `Arc<RwLock<Vec<...>>>` 作桥接。

**改进方向**：让 registry 直接感知 GPUI Entity —— `add` 接受 `WeakEntity<T>`（经 helper 转为类型擦除闭包），`register`/`unregister` 接受 `cx: &mut App` 以便经 `weak.update(cx, |h, _| h.add(...))` 调用 Entity 的 `add`/`remove`。

**此模式已有先例**：`RelayCommand`（[command.rs:135-138](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/command.rs#L135-L138)）捕获 `WeakEntity<T>` + 闭包，`execute` 时 `weak.update(cx, |this, cx| ...)`。

**架构转变**：

```
旧：MainWindow → Arc::new(MainWindowHostHandle{entries: shared}) → registry.add(arc) → host.add() → entries.push
新：MainWindow → cx.register_host(ID, cx.weak_entity()) → registry 存闭包 → register(cx) → weak.update(cx, h.add()) → entries.push
```

---

## 从 `registry.add` 定义与调用方分析

### 定义（[contribution.rs:190](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs#L190)）

```rust
fn add(&self, host: Arc<dyn IContributionHost>);
```

要求 `Arc<dyn IContributionHost>` —— 必须是 `Arc` 可共享的独立对象。

### 调用方（唯一：[main_window.rml.rs:147-151](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L147-L151)）

```rust
let handle = Arc::new(MainWindowHostHandle {
    id: Self::ID,
    entries: self.entries.clone(),  // 共享 Arc<RwLock<Vec<ContribEntry>>>
});
cx.get_contribution_registry().add(handle);
```

调用方**有 `cx`**（`&mut Context<Self>`），且 `cx.weak_entity()` 可直接获得 `WeakEntity<MainWindow>`。

### 为何 `Arc<dyn IContributionHost>` 是问题

- `MainWindow` 是 GPUI Entity —— 内部由 GPUI 以 `Arc` 管理生命周期，但不暴露 `Arc<MainWindow>`
- 无法从 `Entity<MainWindow>` 提取 `Arc<MainWindow>`
- 故需 `MainWindowHostHandle` 作 `Arc` 可共享的替身，共享 `entries` 字段

### 为何 `take_pending` 不是好解法

`take_pending` 不改 `add` 签名，而是绕过它：不注册 host，贡献入 pending 队列，host 自行 drain。问题：
1. **pending 队列是多余的状态** —— registry 本是路由表，不应暂存贡献
2. **改变了 `register` 语义** —— 从"路由到 host"变为"入队或路由"，行为分叉
3. **一次性 drain 语义** —— 后续动态注册的贡献无法到达 host
4. **host 仍需循环 `self.add()`** —— 把路由逻辑推给业务代码

### 正确方向：Entity 感知 registry

registry 的职责是路由 `register(host_id, ...)` 到 `host.add(...)`。既然 host 是 Entity，registry 应直接支持 Entity：

| 维度 | `Arc<dyn IContributionHost>`（旧） | `WeakEntity<T>` + 闭包（新） |
| --- | --- | --- |
| host 注册 | `registry.add(Arc::new(handle))` —— 需 adapter | `cx.register_host(ID, cx.weak_entity())` —— 直接 |
| host.add 调用 | `host.add(c, o)` —— `&self`，无需 cx | `weak.update(cx, \|h, \| h.add(c, o))` —— 需 cx |
| register 签名 | `register(host_id, c, o)` —— 无 cx | `register(host_id, c, o, cx)` —— 有 cx |
| 适配器 | `MainWindowHostHandle` 必需 | 不需要 |

`cx` 从哪来？`#[contribute]` 宏生成的 `__rml_register_xxx(cx: &mut App)` **已有 `cx`**（[contribute.rs:353](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs#L353)），只需透传给 `register`。

---

## 变更清单

### Phase 1：`IContributionRegistry` trait 签名调整

**文件**：[crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs#L188-L205)

```rust
pub trait IContributionRegistry: Send + Sync {
    /// 注册 Entity host。`add_fn`/`remove_fn` 由 `register_host` helper 从 `WeakEntity<T>` 构造。
    /// registry 按 `host_id` 存储闭包，`register` 时经闭包调 `weak.update(cx, |h, _| h.add(...))`。
    fn add(
        &self,
        host_id: &str,
        add_fn: Box<dyn Fn(Arc<dyn IContribution>, Option<ContributionOptions>, &mut App) + Send + Sync>,
        remove_fn: Box<dyn Fn(&str, &mut App) + Send + Sync>,
    );

    fn remove(&self, host_id: &str);

    /// 向 host 注册贡献。`cx` 用于 `weak.update(cx, ...)` 调用 host.add。
    fn register(
        &self,
        host_id: &str,
        contribution: Arc<dyn IContribution>,
        options: Option<ContributionOptions>,
        cx: &mut App,
    );

    fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool;
}
```

**变化**：
- `add`：`Arc<dyn IContributionHost>` → `host_id: &str` + 两个类型擦除闭包
- `register`：新增 `cx: &mut App` 参数
- `unregister`：新增 `cx: &mut App` 参数
- `IContributionHost` trait **不变**（`id`/`add`/`remove` 签名保持）

### Phase 2：`ContributionRegistry` impl 调整

**文件**：[crates/app/src/contribution/registry.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/registry.rs)

存储改为闭包：

```rust
type AddFn = Box<dyn Fn(Arc<dyn IContribution>, Option<ContributionOptions>, &mut App) + Send + Sync>;
type RemoveFn = Box<dyn Fn(&str, &mut App) + Send + Sync>;

struct HostEntry {
    add_fn: AddFn,
    remove_fn: RemoveFn,
}

pub struct ContributionRegistry {
    hosts: RwLock<HashMap<String, HostEntry>>,
}
```

impl：
```rust
fn add(&self, host_id: &str, add_fn: AddFn, remove_fn: RemoveFn) {
    self.hosts.write().unwrap().insert(host_id.to_string(), HostEntry { add_fn, remove_fn });
}

fn register(&self, host_id: &str, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>, cx: &mut App) {
    let hosts = self.hosts.read().unwrap();
    if let Some(entry) = hosts.get(host_id) {
        (entry.add_fn)(contribution, options, cx);
    }
    // host 未注册时贡献丢弃（保持原语义，不再需要 pending 队列）
}

fn unregister(&self, host_id: &str, contribution_id: &str, cx: &mut App) -> bool {
    let hosts = self.hosts.read().unwrap();
    if let Some(entry) = hosts.get(host_id) {
        (entry.remove_fn)(contribution_id, cx);
        true
    } else {
        false
    }
}
```

### Phase 3：`register_host<T>` helper

**文件**：[crates/app/src/extensions.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/extensions.rs)

新增 `IAppContextExt::register_host` —— 业务代码的唯一入口，封装 `WeakEntity<T>` → 闭包转换：

```rust
pub trait IAppContextExt {
    fn get_contribution_registry(&self) -> Arc<dyn IContributionRegistry>;

    /// Entity host 注册自身。经 `WeakEntity<T>` 构造类型擦除闭包存入 registry。
    /// 模式同 `RelayCommand::new`（command.rs:135-138）。
    fn register_host<T: IContributionHost + 'static>(&self, host_id: &str, host: WeakEntity<T>);
}

impl IAppContextExt for App {
    fn get_contribution_registry(&self) -> Arc<dyn IContributionRegistry> {
        self.get_required_service::<ContributionRegistry>() as Arc<dyn IContributionRegistry>
    }

    fn register_host<T: IContributionHost + 'static>(&self, host_id: &str, host: WeakEntity<T>) {
        let weak_add = host.clone();
        let add_fn = Box::new(move |c: Arc<dyn IContribution>, o: Option<ContributionOptions>, cx: &mut App| {
            let _ = weak_add.update(cx, |h, _| h.add(c, o));
        });
        let weak_remove = host.clone();
        let remove_fn = Box::new(move |id: &str, cx: &mut App| {
            let _ = weak_remove.update(cx, |h, _| h.remove(id));
        });
        self.get_contribution_registry().add(host_id, add_fn, remove_fn);
    }
}
```

**注意**：`WeakEntity::update` 返回 `Result`，用 `let _ =` 忽略（Entity 已销毁时静默跳过）。

### Phase 4：`#[contribute]` 宏传递 `cx`

**文件**：[crates/macros/src/contribute.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs#L308-L321)

`register_call` 宏片段新增 `cx` 参数：

```rust
let register_call = quote! {
    cx.get_contribution_registry().register(
        #host_id,
        std::sync::Arc::new(#struct_name::default()),
        Some(
            rml_core::contribution::ContributionOptions::new()
                #parent_id
                #order
                #group
                #properties_tokens,
        ),
        cx,  // ← 新增
    );
};
```

`__rml_register_xxx(cx: &mut gpui::App)` 函数签名不变（已有 `cx`）。

### Phase 5：MainWindow 直接 impl IContributionHost

**文件**：[demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

#### 移除

- `struct MainWindowHostHandle`（第 73-76 行）
- `impl IContributionHost for MainWindowHostHandle`（第 78-94 行）

#### 新增

```rust
impl IContributionHost for MainWindow {
    fn id(&self) -> &'static str {
        Self::ID
    }

    fn add(&self, contribution: Arc<dyn IContribution>, options: Option<ContributionOptions>) {
        let opts = options.unwrap_or_default();
        self.entries.write().unwrap().push((contribution, opts));
    }

    fn remove(&self, contribution_id: &str) {
        self.entries
            .write()
            .unwrap()
            .retain(|(c, _)| c.id() != contribution_id);
    }
}
```

#### 修改 `init_contribution_host`（第 146-154 行）

```rust
fn init_contribution_host(&mut self, cx: &mut Context<Self>) {
    // 注册 Entity host —— registry 存 WeakEntity 闭包，register 时经 weak.update 调 self.add
    cx.register_host(Self::ID, cx.weak_entity());
    // 触发所有 #[contribute] 生成函数 → registry.register(host_id, c, o, cx) → self.add(c, o)
    rml_app::contribution::bootstrap_host_contributions(cx, Self::ID);
    crate::cases::status_bar_case::ensure_status_ready_registered();
}
```

**对比旧代码**：
- 旧：`Arc::new(MainWindowHostHandle{...})` + `registry.add(handle)` + `bootstrap`
- 新：`cx.register_host(Self::ID, cx.weak_entity())` + `bootstrap` —— 无 adapter，无 Arc 构造

---

## 假设与决策

1. **`IContributionHost` trait 不变** —— `id`/`add`/`remove` 签名保持，满足项目记忆约束。
2. **`IContributionRegistry` trait 签名变更** —— `add`/`register`/`unregister` 签名调整。项目记忆仅约束 `IContribution`/`IVisualContribution` 签名，不约束 `IContributionRegistry`。
3. **`register` 新增 `cx` 可行** —— 唯一调用方是 `#[contribute]` 宏生成代码（[contribute.rs:353](file:///e:/GitCode/RF/rust-gpui-rml/crates/macros/src/contribute.rs#L353)），函数签名 `fn(cx: &mut App)` 已有 `cx`，只需透传。`unregister` 无实际调用方（grep 确认），签名变更无影响。
4. **`WeakEntity::update` 模式有先例** —— `RelayCommand`（[command.rs:135-138](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/command.rs#L135-L138)）用相同模式捕获 `WeakEntity<T>` + 闭包 + `cx: &mut App`。
5. **host 未注册时贡献丢弃** —— 保持原语义（不引入 pending 队列）。`MainWindow` 在 `on_loaded` 中先 `register_host` 再 `bootstrap_host_contributions`，时序保证 host 已注册。
6. **闭包 `Send + Sync`** —— 闭包仅捕获 `WeakEntity<T>`（`Send + Sync`），`&mut App` 仅作为参数不进入捕获，故闭包是 `Send + Sync`。
7. **`register_host<T>` 是泛型方法** —— 不能在 `dyn IAppContextExt` 上调用。但 `IAppContextExt` 的 impl 是 `impl IAppContextExt for App`（具体类型），非 `dyn`，故泛型方法可用。

---

## 验证步骤

1. `cargo build -p rust-rml-core` —— trait 签名编译通过
2. `cargo build -p rust-rml-app` —— `ContributionRegistry` impl + `register_host` helper 编译通过；单元测试通过
3. `cargo build -p rust-rml-macros` —— `#[contribute]` 宏生成代码编译通过
4. `cargo build -p rust-rml-demo` —— `MainWindow` 直接 impl 编译通过
5. `cargo test --workspace` —— 全部 649+ 测试通过
6. `cargo run -p rust-rml-demo` —— 验证：
   - 菜单项正常显示（File/View/Help）
   - 状态栏显示 `status.ready`
   - 案例树显示所有 case 贡献
   - 点击案例树节点打开对应 tab
   - ActivityBar 面板切换正常
   - 打开 4-5 个案例无栈溢出（8MB 栈已设置）
7. `grep -r "MainWindowHostHandle" demo/ crates/` —— 确认无残留引用
