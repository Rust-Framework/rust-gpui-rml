# 9.1 职责归属与划分

> **本节目标**：明确 `.rml`、`.rml.rs`、Model、ViewModel、View 各自的职责边界，建立“什么代码该写在哪里”的判断准则。

## 9.1.1 为什么必须先讲职责

RML 的核心价值是**关注点分离**。如果开发者把业务逻辑塞进 `.rml` 模板、把 UI 结构写进 `.rml.rs`，RML 就退化成“带宏的 GPUI”，所有优势瞬间归零。

职责划分不是风格偏好，而是 RML 框架的**契约**：

- 编译器假设 `.rml` 只含声明式标记；
- 绑定引擎假设 ViewModel 字段是纯数据；
- 命令系统假设 `#[command]` 方法只修改状态、不直接操作 DOM；
- 样式系统假设外观完全由样式表决定。

违反契约不会立即报错，但会让热重载失效、绑定追踪失真、测试无法独立运行。

## 9.1.2 五层职责一览

```
┌──────────────────────────────────────────────────────────────┐
│                       RML 职责分层                              │
├──────────────────────────────────────────────────────────────┤
│  ① .rml          视图结构：标签、属性、指令、插值、事件绑定              │
│  ② .rml.rs       业务逻辑：ViewModel 定义、命令、生命周期、元素引用      │
│  ③ Model         纯数据：不可变结构体，无 GPUI 依赖                    │
│  ④ ViewModel     视图状态：持有 Model，暴露绑定字段与命令，自身即 Entity  │
│  ⑤ View          渲染产物：编译期为 ViewModel 生成的 Render 实现        │
└──────────────────────────────────────────────────────────────┘
```

> **关键澄清**：在 RML 中，ViewModel 与 View **不是两个独立的类型**。ViewModel 结构体（标注 `#[derive(Model)]` + `#[view]`）本身就是 GPUI 的 Entity；View 是编译器在 `OUT_DIR` 中为该 ViewModel 生成的 `impl Render` 代码块。开发者只写 ViewModel，不写 View。

## 9.1.3 各层职责详解

### ① `.rml` —— 视图结构层

**应该做的**：

- 描述 UI 的层级结构（`div`、`ul`、`button`…）
- 通过属性配置控件（`class`、`type`、`placeholder`…）
- 通过指令控制流（`if`、`each`、`model`、`show`…）
- 通过插值显示数据（`{title}`、`{user.name}`…）
- 通过 `on*` 绑定事件到命令

**禁止做的**：

- ❌ 在插值里写复杂表达式或函数调用（`{calculate_total(items)}`）
- ❌ 在模板里写业务判断逻辑（应放进计算属性）
- ❌ 在模板里直接修改状态（RML 模板是只读的）
- ❌ 在模板里硬编码业务常量（应放进 ViewModel 或资源字典）

```html
<!-- ✅ 正确：纯结构 + 绑定 -->
<div class="user-card">
  <img class="avatar" src="{user.avatar_url}" />
  <h3>{user.display_name}</h3>
  <p if={user.is_online} class="status online">在线</p>
  <button onclick={toggle_follow}>关注</button>
</div>

<!-- ❌ 错误：模板里塞业务逻辑 -->
<div class="user-card">
  <h3>{user.first_name + " " + user.last_name + " (" + user.role + ")"}</h3>
  <p if={user.posts.len() > 0 && user.is_active && !user.is_banned}>可见</p>
</div>
```

### ② `.rml.rs` —— 业务逻辑层

**应该做的**：

- 定义 ViewModel 结构体，标注 `#[derive(Model)]` + `#[view]`
- 实现 `#[command]` 命令方法
- 实现 `#[computed]` 计算属性
- 实现 `#[on_loaded]` / `#[on_unloaded]` 生命周期
- 通过 `ElementRef` 做命令式访问（仅在必要时）

**禁止做的**：

- ❌ 在命令方法里直接构造 GPUI `div()` 树
- ❌ 在 ViewModel 里持有 `ViewContext` 或 `AppContext`（应作为参数传入）
- ❌ 在 ViewModel 里写网络/文件 I/O 细节（应委托给 Service 层）
- ❌ 在命令方法里跨视图直接修改其他 ViewModel 的字段（应通过事件或 Context）

```rust
// ✅ 正确：命令只改状态，I/O 委托给 service
//
// 【对比传统 MVVM】在 WPF/UWP 中，ViewModel 通常要实现 INotifyPropertyChanged，
// 每个属性 setter 都要手动触发 OnPropertyChanged("IsFollowing")，字符串拼写错误
// 不会编译报错，只能在运行时发现绑定失效。RML 的优势：
//   1. #[derive(Model)] 让所有 pub 字段自动成为响应式状态，无需手写通知样板代码；
//   2. cx.notify() 一次调用即可触发对所有绑定的重新求值，由编译期生成的 Render
//      实现负责差异渲染，开发者不必关心"哪个字段变了要通知谁"；
//   3. #[command] 把方法显式标记为 UI 可调用，编译器据此生成事件分发代码，
//      避免了 WPF 中 ICommand.Execute/CanExecute 的样板实现；
//   4. cx.spawn 把异步 I/O 委托给 service，命令方法本身保持同步且可单测——
//      传统 MVVM 常常把网络请求直接写进 ViewModel，导致单测必须 mock HTTP。
#[derive(Model)]
#[view]
pub struct UserViewModel {
    pub user: User,
    pub is_following: bool,  // pub 字段自动响应式，.rml 中 {is_following} 即可绑定
}

impl UserViewModel {
    // #[command] 让此方法可被 .rml 的 onclick={toggle_follow} 直接调用
    // 编译器会生成事件分发胶水代码，无需手写 ICommand 实现
    #[command]
    pub fn toggle_follow(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        let user_id = self.user.id;
        // 乐观更新：先改本地状态，UI 立即响应
        self.is_following = !self.is_following;
        // 一次 notify 触发所有依赖 is_following 的绑定重算
        // 传统 MVVM 需要对每个变更属性单独触发通知
        cx.notify();
        // 异步 I/O 委托给 service，命令方法本身保持可单测
        cx.spawn(|this, mut cx| async move {
            let _ = cx.update(|cx| follow_service::toggle(user_id, cx)).await;
            // 失败时回滚状态由 service 通过事件通知
        }).detach();
    }
}

// ❌ 错误：命令里直接拼 GPUI 树
// 这正是传统 MVVM 在 ViewModel 中用 Code Behind 拼控件树的坏习惯，
// RML 通过编译期生成 Render 彻底杜绝了这种写法
#[command]
pub fn render_detail(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    let detail = div().child(Label::new("...")); // 不应出现在 .rml.rs
    cx.show_view(detail); // 这种 API 在 RML 中不存在
}
```

### ③ Model —— 纯数据层

**特征**：

- `#[derive(Model)]` 派生的普通结构体（可按需附加 `Clone`、`Debug`、`Serialize` 等）
- 字段是纯数据：`String`、`i32`、`Vec<T>`、`Option<T>`、嵌套 Model
- **不依赖** `gpui::*`、不持有 `ViewContext`、不实现 `Render`
- **不标注** `#[view]`（那是 ViewModel 的标记）
- 通常是 API 响应、配置、领域模型的反序列化目标

**职责**：

- 承载数据
- 提供纯函数式的派生计算（如 `full_name(&self) -> String`）
- 可被序列化 / 反序列化

```rust
// ✅ Model：纯数据，无 GPUI 依赖，不标注 #[view]
//
// 【对比传统 MVVM】在传统 WPF MVVM 中，Model 与 ViewModel 的边界经常模糊——
// 开发者为了让数据能绑定，常常让 Model 也实现 INotifyPropertyChanged，
// 导致数据层被 UI 框架污染，无法在控制台、服务端、测试用例中复用。
// RML 的设计更纯粹：
//   1. Model 只 #[derive(Model)]，不标注 #[view]，编译器不会为它生成 Render；
//   2. Model 的方法（如 full_name/is_visible）是纯函数，无副作用，可自由单测；
//   3. Model 不依赖 gpui::*，可以放进 core crate，被 CLI/服务端/桌面端共享；
//   4. 当 Model 需要被 UI 消费时，由 ViewModel 持有它并把字段暴露为绑定源，
//      数据的"纯洁性"和"可绑定性"解耦，各司其职。
#[derive(Model, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
    pub first_name: SharedString,
    pub last_name: SharedString,
    pub role: UserRole,
    pub posts: Vec<Post>,
}

impl User {
    // 纯函数式派生计算：无 &mut self、无 cx、无 I/O
    // 可在任何无 UI 环境下调用，单测只需构造 User 实例
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    // 业务规则也放在 Model，而不是塞进 .rml 模板表达式
    // 这样规则可被复用、可被测试，而不是散落在视图里
    pub fn is_visible(&self) -> bool {
        !self.posts.is_empty() && self.is_active() && !self.is_banned
    }
}
```

### ④ ViewModel —— 视图状态层

**特征**：

- `#[derive(Model)]` 派生，并标注 `#[view]`（标记为 RML 视图的 Code-Behind）
- **持有视图专属状态**
- 通过 `#[command]` 暴露可被 UI 触发的方法
- 通过 `#[computed]` 暴露派生数据
- 通过 `#[element]` 持有对模板中元素的引用
- 实现 `#[on_loaded]` / `#[on_unloaded]`

**职责**：

- 把 Model 翻译成"视图可消费"的形态
- 管理视图本地状态（loading、error、selected_index…）
- 协调 Service 调用并把结果回写到状态
- 通过 `cx.notify()` 通知重渲染（触发编译期生成的 `Render` 实现）

**ViewModel 不应该**：

- ❌ 持有其他 ViewModel 的强引用（用 `WeakEntity` 或 Context 事件）
- ❌ 手写 `impl Render`（由编译器根据 `.rml` 自动生成，开发者只需标注 `#[view]`）
- ❌ 跨越多个视图共享（每个视图一个 ViewModel；跨视图共享用 Context / 全局 Model）

### ⑤ View —— 渲染产物层

**特征**：

- 由 RML 编译器在 `OUT_DIR` 中生成，**开发者不手写**
- 为 ViewModel 生成 `impl RmlView`（声明 `.rml` 模板路径）和 `impl Render`（构建 GPUI 元素树）
- **不是独立的类型**——ViewModel 结构体自身即是 GPUI Entity，View 只是附加在它身上的 `Render` 实现
- 在 `render` 中调用绑定引擎、构建 GPUI 树

**职责**：

- 把 `.rml` 模板编译成 GPUI 调用
- 监听 ViewModel 的 `notify`，触发重渲染
- 把 UI 事件转发给 ViewModel 的命令

**开发者与 View 的关系**：

- 永远不直接构造 View，也不手写 `impl Render`
- 通过 `RmlApplication::new().run::<MyViewModel>()` 启动根视图
- 通过 `ElementRef` 在 ViewModel 中拿到元素句柄，但**不修改结构**

## 9.1.4 职责速查表

| 需求                       | 该写在哪里                          | 反例                              |
| ------------------------ | ----------------------------- | ------------------------------- |
| 改一段文字的颜色                 | `.rml` 的 `class` 或样式表         | 在命令里 `el.set_style(...)`        |
| 显示用户全名                   | ViewModel 的 `#[computed]`     | 模板里 `{first + " " + last}`       |
| 点击按钮后请求 API              | `#[command]` + Service        | 模板里 `{fetch_user()}`            |
| 列表为空时显示占位                | `.rml` 的 `if` + 计算属性         | 命令里手动 `show` / `hide` 元素         |
| 输入框自动聚焦                  | `#[on_loaded]` + `ElementRef` | 模板里写 `autofocus` 然后在命令里二次操作     |
| 主题切换                     | Context 事件 + CSS 变量           | 在每个 ViewModel 里监听并改字段           |
| 跨视图共享登录态                 | 全局 Model + `cx.observe`       | 把 `UserViewModel` 到处传           |
| 表单校验                     | ViewModel 的 `#[computed]`     | 模板里写 `{validate(email)}`         |

## 9.1.5 一个完整例子的分层

需求：一个登录表单，输入邮箱密码，点击登录后请求后端，失败时显示错误。

**`login.rml`** —— 只描述结构和绑定：

```html
<!--
  ✅ 模板只描述"界面长什么样"，完全不关心数据从哪来、怎么校验、怎么提交。
  对比传统 MVVM：WPF 的 XAML 常常混入 Converter 链、Trigger、Style 重写，
  最终变成"XAML 里写逻辑"。RML 的 .rml 强制保持声明式，业务判断全部上推到
  ViewModel 的 #[computed]，模板可读性接近纯 HTML，设计师可直接编辑。
-->
<form class="login-form" onsubmit={login}>
  <!-- model={email} 是双向绑定：输入回写 ViewModel，ViewModel 变更同步到 value -->
  <input type="email" model={email} placeholder="邮箱" />
  <input type="password" model={password} placeholder="密码" />
  <!-- if={error} 条件渲染：error 为 None 时此节点不存在，而非 display:none -->
  <p if={error} class="error">{error}</p>
  <!-- disabled={is_loading} 是单向绑定：ViewModel 状态驱动 UI 禁用态 -->
  <button type="submit" disabled={is_loading}>
    {is_loading ? "登录中…" : "登录"}
  </button>
</form>
```

**`login.rml.rs`** —— 只处理状态和命令：

```rust
// 【对比传统 MVVM】这个登录示例集中体现了 RML 相对 WPF MVVM 的优雅：
//   1. 无样板：不需要 INotifyPropertyChanged、ICommand、RelayCommand、DependencyProperty
//      这一整套基础设施，#[derive(Model)] + #[command] + #[computed] 三个宏全搞定；
//   2. 可单测：can_submit 是纯函数，login 命令把 I/O 委托给 auth_service，
//      单测时注入 mock service 即可，无需启动 GPUI 窗口；
//   3. 异步安全：cx.spawn 闭包持有 weak entity 引用，视图卸载时自动取消，
//      不会出现"ViewModel 已销毁但网络回调还在写状态"的悬空访问——
//      这是传统 MVVM 异步编程的高发 bug 区；
//   4. 状态机清晰：is_loading/error/email/password 四个字段就是完整状态空间，
//      UI 的所有外观都由这四个字段推导，不存在隐式的"控件可见性"等命令式状态。
#[derive(Model)]
#[view]
pub struct LoginViewModel {
    pub email: SharedString,       // 双向绑定：input ↔ ViewModel
    pub password: SharedString,    // 双向绑定：input ↔ ViewModel
    pub error: Option<SharedString>, // 单向绑定：ViewModel → <p if={error}>
    pub is_loading: bool,          // 单向绑定：ViewModel → button disabled + 文案
}

impl LoginViewModel {
    // #[computed] 自动追踪依赖：email/password/is_loading 任一变化都触发重算
    // 传统 MVVM 需要手动在 setter 里调用 CanExecuteChanged 事件
    #[computed]
    pub fn can_submit(&self) -> bool {
        !self.email.is_empty() && !self.password.is_empty() && !self.is_loading
    }

    #[command]
    pub fn login(&mut self, _ev: &SubmitEvent, cx: &mut ViewContext<Self>) {
        // 进入 loading 态：UI 立即禁用按钮、文案变为"登录中…"
        self.is_loading = true;
        self.error = None;
        cx.notify();
        // 克隆数据进闭包，避免 &self 跨 await 生命周期问题（Rust 所有权保障）
        let email = self.email.clone();
        let password = self.password.clone();
        // cx.spawn 持有 weak entity，视图卸载时自动中止，无悬空回调
        cx.spawn(|this, mut cx| async move {
            match auth_service::login(&email, &password, &cx).await {
                Ok(_) => cx.update(|cx| cx.dispatch(LoginSuccess)).ok(),
                Err(e) => this.update(&mut cx, |this, cx| {
                    // 失败回滚：error 字段驱动 <p if={error}> 显示错误文案
                    this.error = Some(e.to_string().into());
                    this.is_loading = false;
                    cx.notify();
                }).err(),
            }
        }).detach();
    }
}
```

**`auth_service.rs`** —— 真正的 I/O：

```rust
pub async fn login(email: &str, password: &str, cx: &mut AsyncApp) -> Result<Token> { ... }
```

三层各司其职：模板可被设计师改、ViewModel 可被工程师单测、Service 可被集成测试。

## 9.1.6 判断准则

当你在写代码时，问自己三个问题：

1. **这段代码描述的是 UI 结构吗？** → 写进 `.rml`
2. **这段代码会修改视图状态吗？** → 写进 ViewModel 的 `#[command]` 或 `#[computed]`
3. **这段代码做的是 I/O 或领域计算吗？** → 写进 Service / Model

如果答案是“都沾点边”，说明职责正在混淆——停下来拆分。

下一节 → [9.2 MVVM 模式实践](./mvvm-practice.md)
