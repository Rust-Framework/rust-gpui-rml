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
│  ④ ViewModel     视图状态：持有 Model，暴露绑定字段与命令                │
│  ⑤ View          渲染产物：编译期生成的 GPUI Entity，运行时不可手改        │
└──────────────────────────────────────────────────────────────┘
```

## 9.1.3 各层职责详解

### ① `.rml` —— 视图结构层

**应该做的**：

- 描述 UI 的层级结构（`div`、`ul`、`button`…）
- 通过属性配置控件（`class`、`type`、`placeholder`…）
- 通过指令控制流（`r:if`、`r:each`、`r:model`、`r:show`…）
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
  <p r:if="user.is_online" class="status online">在线</p>
  <button onclick="toggle_follow">关注</button>
</div>

<!-- ❌ 错误：模板里塞业务逻辑 -->
<div class="user-card">
  <h3>{user.first_name + " " + user.last_name + " (" + user.role + ")"}</h3>
  <p r:if="user.posts.len() > 0 && user.is_active && !user.is_banned">可见</p>
</div>
```

### ② `.rml.rs` —— 业务逻辑层

**应该做的**：

- 定义 ViewModel 结构体与 `#[derive(Model)]`
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
#[command]
pub fn toggle_follow(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    let user_id = self.user.id;
    self.is_following = !self.is_following;
    cx.notify();
    cx.spawn(|this, mut cx| async move {
        let _ = cx.update(|cx| follow_service::toggle(user_id, cx)).await;
        // 失败时回滚状态由 service 通过事件通知
    }).detach();
}

// ❌ 错误：命令里直接拼 GPUI 树
#[command]
pub fn render_detail(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    let detail = div().child(Label::new("...")); // 不应出现在 .rml.rs
    cx.show_view(detail); // 这种 API 在 RML 中不存在
}
```

### ③ Model —— 纯数据层

**特征**：

- `#[derive(Model, Clone, Debug)]` 派生的普通结构体
- 字段是纯数据：`String`、`i32`、`Vec<T>`、`Option<T>`、嵌套 Model
- **不依赖** `gpui::*`、不持有 `ViewContext`、不实现 `Render`
- 通常是 API 响应、配置、领域模型的反序列化目标

**职责**：

- 承载数据
- 提供纯函数式的派生计算（如 `full_name(&self) -> String`）
- 可被序列化 / 反序列化

```rust
// ✅ Model：纯数据，无 GPUI 依赖
#[derive(Model, Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct User {
    pub id: u64,
    pub first_name: SharedString,
    pub last_name: SharedString,
    pub role: UserRole,
    pub posts: Vec<Post>,
}

impl User {
    pub fn full_name(&self) -> String {
        format!("{} {}", self.first_name, self.last_name)
    }

    pub fn is_visible(&self) -> bool {
        !self.posts.is_empty() && self.is_active() && !self.is_banned
    }
}
```

### ④ ViewModel —— 视图状态层

**特征**：

- `#[derive(Model)]` 派生，但**持有视图专属状态**
- 通过 `#[command]` 暴露可被 UI 触发的方法
- 通过 `#[computed]` 暴露派生数据
- 通过 `#[element]` 持有对模板中元素的引用
- 实现 `#[on_loaded]` / `#[on_unloaded]`

**职责**：

- 把 Model 翻译成“视图可消费”的形态
- 管理视图本地状态（loading、error、selected_index…）
- 协调 Service 调用并把结果回写到状态
- 通过 `cx.notify()` 通知 View 重渲染

**ViewModel 不应该**：

- ❌ 持有其他 ViewModel 的强引用（用 `WeakEntity` 或 Context 事件）
- ❌ 直接渲染 UI（不实现 `Render`，由编译期生成的 View 接管）
- ❌ 跨越多个视图共享（每个视图一个 ViewModel；跨视图共享用 Context / 全局 Model）

### ⑤ View —— 渲染产物层

**特征**：

- 由 RML 编译器在 `OUT_DIR` 中生成，**开发者不手写**
- 实现 `gpui::Render`
- 持有 ViewModel 的 `Entity<Self>`
- 在 `render` 中调用绑定引擎、构建 GPUI 树

**职责**：

- 把 `.rml` 模板编译成 GPUI 调用
- 监听 ViewModel 的 `notify`，触发重渲染
- 把 UI 事件转发给 ViewModel 的命令

**开发者与 View 的关系**：

- 永远不直接构造 View
- 通过 `cx.open_view::<MyView>()` 等高层 API 打开
- 通过 `ElementRef` 在 ViewModel 中拿到元素句柄，但**不修改结构**

## 9.1.4 职责速查表

| 需求                       | 该写在哪里                          | 反例                              |
| ------------------------ | ----------------------------- | ------------------------------- |
| 改一段文字的颜色                 | `.rml` 的 `class` 或样式表         | 在命令里 `el.set_style(...)`        |
| 显示用户全名                   | ViewModel 的 `#[computed]`     | 模板里 `{first + " " + last}`       |
| 点击按钮后请求 API              | `#[command]` + Service        | 模板里 `{fetch_user()}`            |
| 列表为空时显示占位                | `.rml` 的 `r:if` + 计算属性        | 命令里手动 `show` / `hide` 元素         |
| 输入框自动聚焦                  | `#[on_loaded]` + `ElementRef` | 模板里写 `autofocus` 然后在命令里二次操作     |
| 主题切换                     | Context 事件 + CSS 变量           | 在每个 ViewModel 里监听并改字段           |
| 跨视图共享登录态                 | 全局 Model + `cx.observe`       | 把 `UserViewModel` 到处传           |
| 表单校验                     | ViewModel 的 `#[computed]`     | 模板里写 `{validate(email)}`         |

## 9.1.5 一个完整例子的分层

需求：一个登录表单，输入邮箱密码，点击登录后请求后端，失败时显示错误。

**`login.rml`** —— 只描述结构和绑定：

```html
<form class="login-form" on:submit="login">
  <input type="email" r:model="email" placeholder="邮箱" />
  <input type="password" r:model="password" placeholder="密码" />
  <p r:if="error" class="error">{error}</p>
  <button type="submit" r:attr:disabled="is_loading">
    {is_loading ? "登录中…" : "登录"}
  </p>
</form>
```

**`login.rml.rs`** —— 只处理状态和命令：

```rust
#[derive(Model)]
pub struct LoginViewModel {
    pub email: SharedString,
    pub password: SharedString,
    pub error: Option<SharedString>,
    pub is_loading: bool,
}

impl LoginViewModel {
    #[computed]
    pub fn can_submit(&self) -> bool {
        !self.email.is_empty() && !self.password.is_empty() && !self.is_loading
    }

    #[command]
    pub fn login(&mut self, _ev: &SubmitEvent, cx: &mut ViewContext<Self>) {
        self.is_loading = true;
        self.error = None;
        cx.notify();
        let email = self.email.clone();
        let password = self.password.clone();
        cx.spawn(|this, mut cx| async move {
            match auth_service::login(&email, &password, &cx).await {
                Ok(_) => cx.update(|cx| cx.dispatch(LoginSuccess)).ok(),
                Err(e) => this.update(&mut cx, |this, cx| {
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
