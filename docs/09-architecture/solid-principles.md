# 9.3 SOLID 原则在 RML 中的落地

> **本节目标**：把面向对象的 SOLID 五原则翻译成 RML 的具体实践，给出可落地的代码范式。

## 9.3.1 为什么 SOLID 在 RML 中依然重要

SOLID 常被认为是 OOP 专属，但其本质是**管理依赖与变化**的工程原则。RML 虽是声明式 UI 框架，依然要面对：

- 需求变化（新增控件、新增主题、新增业务规则）
- 代码膨胀（ViewModel 越写越胖）
- 耦合扩散（改一处牵动全身）
- 测试困难（依赖 GPUI 无法单测）

SOLID 提供了诊断这些问题的语言和处方。

## 9.3.2 S —— 单一职责原则

> **一个类 / 模块只应有一个变化的理由。**

### 在 RML 中的体现

RML 的“职责”天然按文件分层：

| 文件            | 变化理由               |
| ------------- | ------------------ |
| `.rml`        | UI 结构变化            |
| `.rml.rs`     | 业务规则 / 视图状态变化      |
| 样式表           | 外观变化               |
| Service       | I/O 细节 / 第三方 API 变化 |
| Model         | 领域数据结构变化           |

### 反例：胖 ViewModel

```rust
// ❌ 这个 ViewModel 同时承担：状态、网络、缓存、序列化
#[derive(Model)]
pub struct UserViewModel {
    pub user: User,
}

impl UserViewModel {
    #[command]
    pub fn save(&mut self, cx: &mut ViewContext<Self>) {
        // 网络请求
        let resp = reqwest::blocking::post(...);
        // 缓存
        std::fs::write("cache.json", ...);
        // 序列化
        serde_json::to_string(...);
        // 状态
        self.user.saved = true;
        cx.notify();
    }
}
```

### 正例：拆分职责

```rust
// ✅ ViewModel 只管状态
#[command]
pub fn save(&mut self, cx: &mut ViewContext<Self>) {
    let user = self.user.clone();
    self.is_saving = true;
    cx.notify();
    cx.spawn(|this, mut cx| async move {
        let result = user_service::save(&user, &cx).await; // 网络在 service
        let _ = this.update(&mut cx, |this, cx| {
            this.is_saving = false;
            if result.is_ok() { this.user.saved = true; }
            cx.notify();
        });
    }).detach();
}

// user_service.rs：只管 I/O
pub async fn save(user: &User, cx: &mut AsyncApp) -> Result<()> {
    let json = serde_json::to_string(user)?; // 序列化在 service
    let resp = cx.http().post("/users").body(json).await?; // 网络在 service
    cache::write("cache.json", &json).await?; // 缓存在独立模块
    Ok(())
}
```

**判断准则**：如果一段代码同时出现 `cx.notify()` 和 `reqwest::` / `std::fs::`，职责已经混淆。

## 9.3.3 O —— 开闭原则

> **对扩展开放，对修改封闭。**

### 在 RML 中的体现

RML 提供三条扩展通道，**无需修改既有代码**即可增加新行为：

1. **组件**：把可复用片段封装成组件，新视图引用即可
2. **样式继承**：`based_on` 派生新样式，不改原样式
3. **Context 事件**：新增订阅者，不改发布者

### 示例：通过组件扩展按钮样式

```rust
// 已有：基础按钮组件
#[component(template = "button.rml")]
pub struct Button { label: SharedString, on_click: Option<Command> }

// 扩展：危险按钮，不改 Button
#[component(template = "danger_button.rml")]
pub struct DangerButton {
    #[slot] content: Slot,
    on_click: Option<Command>,
}
```

```html
<!-- danger_button.rml：复用 Button，套一层样式 -->
<button class="btn btn-danger" on:click="on_click">
  <slot name="content" />
</button>
```

新增按钮类型只需新增组件文件，**不修改** `Button` 本身。

### 反例：在 ViewModel 里堆 if-else

```rust
// ❌ 每加一种按钮都要改 render 逻辑
#[command]
pub fn render_button(&self, kind: &str) -> ... {
    match kind {
        "primary" => ...,
        "danger" => ...,
        "ghost" => ...,
        // 新增类型必须改这里
    }
}
```

## 9.3.4 L —— 里氏替换原则

> **子类型必须能替换其基类型而不破坏程序正确性。**

### 在 RML 中的体现

RML 没有类继承，但**组件组合**和**trait 实现**承担了“替换”的角色：

- 任何实现 `ButtonLike` trait 的组件，都应能被 `<Button>` 替换
- 任何 `#[component]` 都应能被其 `based_on` 的组件替换

### 示例：trait 约束的可替换性

```rust
pub trait Clickable {
    fn set_on_click(&mut self, cmd: Command);
    fn is_disabled(&self) -> bool;
}

#[component(template = "button.rml")]
pub struct Button { ... }
impl Clickable for Button { ... }

#[component(template = "link_button.rml")]
pub struct LinkButton { ... }
impl Clickable for LinkButton { ... }
```

调用方代码：

```rust
fn bind_action<C: Clickable>(c: &mut C, action: Command) {
    if !c.is_disabled() { c.set_on_click(action); }
}
```

`Button` 和 `LinkButton` 可互相替换，调用方无感知。**违反 LSP 的信号**：子组件需要调用方传入额外参数、或抛出基组件不会抛的错。

## 9.3.5 I —— 接口隔离原则

> **不应强迫客户依赖它不使用的方法。**

### 在 RML 中的体现

ViewModel 对 View 暴露的字段 / 命令应当**精确**：只暴露该视图真正需要的，而不是把所有内部状态都设为 `pub`。

### 反例：上帝 ViewModel

```rust
// ❌ 一个 ViewModel 服务所有视图，暴露所有字段
#[derive(Model)]
pub struct AppViewModel {
    pub user: User,
    pub todos: Vec<Todo>,
    pub settings: Settings,
    pub notifications: Vec<Notification>,
    pub cart: Cart,
    // ... 50 个字段
}
```

每个视图都依赖这个巨型结构，但每个视图只用其中 3 个字段——改任何一个字段都会触发所有视图重编译。

### 正例：按视图拆分

```rust
// ✅ 每个视图一个精简 ViewModel
#[derive(Model)]
pub struct ProfileViewModel { pub user: User, pub is_editing: bool }

#[derive(Model)]
pub struct TodoListViewModel { pub todos: Vec<Todo>, pub filter: Filter }

#[derive(Model)]
pub struct CartViewModel { pub items: Vec<CartItem>, pub total: f64 }
```

跨视图共享的状态通过 Context / 全局 Model 提供，而非塞进一个 ViewModel。

## 9.3.6 D —— 依赖倒置原则

> **高层模块不应依赖低层模块，二者都应依赖抽象。**

### 在 RML 中的体现

ViewModel 不应直接依赖具体的 Service 实现，而应依赖 trait；具体实现通过 Context 注入。

### 反例：硬编码依赖

```rust
#[command]
pub fn load(&mut self, cx: &mut ViewContext<Self>) {
    let users = reqwest::get("/api/users").await...; // 直接依赖 reqwest
    self.users = users;
    cx.notify();
}
```

无法测试：单测时不能真的发 HTTP 请求。

### 正例：依赖 trait + 注入

```rust
// 抽象
pub trait UserRepo: 'static {
    fn fetch_all(&self, cx: &mut AsyncApp) -> Task<Result<Vec<User>>>;
}

// 实现
pub struct HttpUserRepo;
impl UserRepo for HttpUserRepo { ... }

pub struct MockUserRepo;
impl UserRepo for MockUserRepo { ... }

// ViewModel 依赖抽象
#[derive(Model)]
pub struct UserListViewModel {
    pub users: Vec<User>,
    repo: Arc<dyn UserRepo>, // 注入
}

#[command]
pub fn load(&mut self, cx: &mut ViewContext<Self>) {
    let repo = self.repo.clone();
    cx.spawn(|this, mut cx| async move {
        let users = repo.fetch_all(&mut cx).await;
        let _ = this.update(&mut cx, |this, cx| {
            if let Ok(u) = users { this.users = u; cx.notify(); }
        });
    }).detach();
}
```

测试时注入 `MockUserRepo`，生产时注入 `HttpUserRepo`，ViewModel 代码不变。

## 9.3.7 SOLID 速查表

| 原则   | RML 落地                              | 反模式信号              |
| ---- | ----------------------------------- | ------------------- |
| SRP  | 每个文件 / ViewModel / Service 单一职责      | ViewModel 同时做网络和缓存  |
| OCP  | 通过组件、样式继承、事件扩展                      | match 分支随需求增长       |
| LSP  | trait 约束的组件可互换                      | 子组件要求额外参数           |
| ISP  | ViewModel 只暴露视图需要的字段                | 一个 ViewModel 服务所有视图 |
| DIP  | ViewModel 依赖 trait，实现由 Context 注入    | 命令里直接 `reqwest::`   |

## 9.3.8 何时停止应用 SOLID

SOLID 是工具，不是教条。下列场景**不必强行套用**：

- **原型阶段**：快速验证想法时，胖 ViewModel 可接受，后续重构
- **一次性脚本**：没有复用需求，抽象反而增加成本
- **稳定领域**：如果某模块三年没变，给它加抽象是浪费

原则的价值在于**当变化发生时降低成本**。如果变化不会发生，提前抽象就是负债。

下一节 → [9.4 项目结构规范](./project-structure.md)
