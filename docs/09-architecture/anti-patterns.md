# 9.6 反模式与代码异味

> **本节目标**：识别 RML 项目中的常见反模式，给出诊断信号与重构处方。

## 9.6.1 反模式 1：胖 ViewModel（God ViewModel）

### 症状

- 一个 ViewModel 超过 500 行
- 持有 20+ 字段，覆盖多个不相关领域
- 命令方法名出现领域前缀（`user_load`、`cart_add`、`settings_save`）

### 危害

- 任何字段变化触发整个视图重渲染
- 修改一处牵动测试全量
- 多人协作冲突频繁

### 处方：按职责拆分

```rust
// ❌ 胖 ViewModel
#[derive(IModel)]
pub struct AppViewModel {
    pub user: User,
    pub todos: Vec<Todo>,
    pub cart: Cart,
    pub settings: Settings,
    pub notifications: Vec<Notification>,
    // ... 30 个字段
}

// ✅ 拆分为多个 ViewModel，每个视图一个
#[derive(IModel)] pub struct ProfileViewModel { pub user: User, ... }
#[derive(IModel)] pub struct TodoListViewModel { pub todos: Vec<Todo>, ... }
#[derive(IModel)] pub struct CartViewModel { pub cart: Cart, ... }
```

跨视图共享的状态用 Context / 全局 Model，而非塞进一个 ViewModel。

## 9.6.2 反模式 2：上帝组件（God component）

### 症状

- 一个组件接受 30+ props
- 组件内含大量 `if={...}` 分支决定渲染哪种形态
- 组件被用于完全不相关的场景

### 危害

- 复用性归零：每次使用都要传一堆无关 props
- 测试组合爆炸
- 修改一处影响所有使用方

### 处方：拆分为职责单一的组件

```html
<!-- ❌ 上帝组件 -->
<Card variant="user" user="{user}" showAvatar="true" showActions="true" ... />

<!-- ✅ 拆分 -->
<UserCard user="{user}" />
<ProductCard product="{product}" />
<NotificationCard notification="{notification}" />
```

共用部分抽为更基础的 `<Card>` 容器组件，由具体组件组合使用。

## 9.6.3 反模式 3：绑定爆炸（Binding explosion）

### 症状

- 模板中出现 5 层以上的属性访问：`{user.team.leader.projects[0].name}`
- 一个插值表达式包含多个函数调用：`{format(parse(user.date), "YYYY")}`
- 计算属性依赖 10+ 字段

### 危害

- 绑定路径变长，性能下降
- 任何中间字段变化都触发重渲染
- 模板可读性极差

### 处方：在 ViewModel 中收敛

```rust
// ✅ ViewModel 提供扁平的计算属性
#[computed]
pub fn current_project_name(&self) -> SharedString {
    self.user.team.leader.projects.first()
        .map(|p| p.name.clone())
        .unwrap_or_default()
}

#[computed]
pub fn formatted_date(&self) -> SharedString {
    format_date(&self.user.date, "YYYY").into()
}
```

```html
<!-- ✅ 模板只读扁平字段 -->
<span>{current_project_name}</span>
<span>{formatted_date}</span>
```

**准则**：模板中的绑定路径不超过 2 层，复杂逻辑收敛到 `#[computed]`。

## 9.6.4 反模式 4：深嵌套（Deep nesting）

### 症状

- `.rml` 中 `div` 嵌套超过 6 层
- 每层都是为了套样式或布局

### 危害

- 可读性差，难以修改
- 性能损耗：每层都是 GPUI 元素
- 样式继承混乱

### 处方：抽组件 + 用样式组合

```html
<!-- ❌ 深嵌套 -->
<div class="card">
  <div class="card-body">
    <div class="row">
      <div class="col">
        <div class="user-info">
          <div class="user-name">{name}</div>
        </div>
      </div>
    </div>
  </div>
</div>

<!-- ✅ 抽组件 -->
<Card>
  <UserInfo user="{user}" />
</Card>
```

## 9.6.5 反模式 5：隐式状态（Implicit state）

### 症状

- 用 `bool` 组合表达状态机：`is_loading && !error && has_data`
- 状态字段散落在多个 ViewModel
- 同一逻辑状态有多种字段组合表示

### 危害

- 状态机不闭合，出现“既 loading 又 error”的非法态
- 修改时漏改某个 bool，UI 卡死

### 处方：用枚举显式建模状态

```rust
// ❌ 隐式状态
#[derive(IModel)]
pub struct BadVM {
    pub is_loading: bool,
    pub is_error: bool,
    pub is_empty: bool,
    pub data: Vec<Item>,
}

// ✅ 显式状态机
#[derive(Model, Clone)]
pub enum LoadState {
    Idle,
    Loading,
    Loaded(Vec<Item>),
    Error(SharedString),
}

#[derive(IModel)]
pub struct GoodVM {
    pub state: LoadState,
}

impl GoodVM {
    #[computed]
    pub fn is_loading(&self) -> bool { matches!(self.state, LoadState::Loading) }
    #[computed]
    pub fn data(&self) -> &[Item] {
        match &self.state { LoadState::Loaded(d) => d, _ => &[] }
    }
}
```

## 9.6.6 反模式 6：命令里构造 UI

### 症状

```rust
#[command]
pub fn show_detail(&mut self, cx: &mut ViewContext<Self>) {
    let panel = div().child(Label::new("...")); // ❌ 在 .rml.rs 构造 GPUI 树
    cx.add_child(panel);
}
```

### 危害

- 破坏关注点分离
- 热重载失效
- 无法被设计师修改

### 处方：用状态控制模板

```rust
#[command]
pub fn show_detail(&mut self, cx: &mut ViewContext<Self>) {
    self.is_detail_visible = true; // ✅ 只改状态
    cx.notify();
}
```

```html
<!-- 模板根据状态渲染 -->
<Detail if={is_detail_visible} data={selected} />
```

## 9.6.7 反模式 7：跨视图直接修改

### 症状

```rust
// 视图 A 的命令里直接改视图 B 的 ViewModel
#[command]
pub fn on_login(&mut self, cx: &mut ViewContext<Self>) {
    let dashboard = cx.get_view::<DashboardViewModel>(); // ❌
    dashboard.update(cx, |d, cx| { d.user = self.user.clone(); cx.notify(); });
}
```

### 危害

- 视图耦合，无法独立测试
- 重构时牵一发动全身
- 生命周期难以管理

### 处方：通过 Context 事件解耦

```rust
// 视图 A 派发事件
#[command]
pub fn on_login(&mut self, cx: &mut ViewContext<Self>) {
    cx.dispatch(LoginSuccess(self.user.clone())); // ✅
}

// 视图 B 订阅事件
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.subscribe::<LoginSuccess>(|this, ev, cx| {
        this.user = ev.0.clone();
        cx.notify();
    });
}
```

## 9.6.8 反模式 8：模板里写业务逻辑

### 症状

```html
<!-- ❌ 模板里塞业务判断 -->
<p if={items.len() > 0 && user.role == 'admin' && !settings.readonly}>
  {items.iter().filter(|i| i.active).count()} 个活跃项
</p>
```

### 危害

- 模板不可单测
- 设计师无法理解
- 业务规则散落各处

### 处方：收敛到计算属性

```rust
#[computed]
pub fn should_show_active_count(&self) -> bool {
    !self.items.is_empty()
        && self.user.role == UserRole::Admin
        && !self.settings.readonly
}

#[computed]
pub fn active_count(&self) -> usize {
    self.items.iter().filter(|i| i.active).count()
}
```

```html
<p if={should_show_active_count}>{active_count} 个活跃项</p>
```

## 9.6.9 反模式速查表

| 反模式       | 信号                       | 处方                |
| --------- | ------------------------ | ----------------- |
| 胖 ViewModel | 字段 > 20，跨多个领域            | 按职责拆分             |
| 上帝组件      | props > 10，多形态分支         | 按场景拆分             |
| 绑定爆炸      | 绑定路径 > 2 层，多函数调用         | 收敛到计算属性           |
| 深嵌套       | div 嵌套 > 6 层             | 抽组件               |
| 隐式状态      | bool 组合表达状态机             | 用枚举               |
| 命令构造 UI   | `.rml.rs` 出现 `div()`     | 改用状态控制模板          |
| 跨视图直接修改   | `cx.get_view::<Other>()` | 用 Context 事件      |
| 模板写业务逻辑   | 模板里多 `&&` / 函数调用         | 收敛到计算属性           |

## 9.6.10 重构的节奏

反模式不是一天形成的，重构也不必一天完成。推荐节奏：

1. **识别最痛的反模式**：选影响最大的一个
2. **加测试覆盖现状**：重构前先有安全网
3. **小步重构**：每次只改一个 ViewModel / 组件
4. **保持测试通过**：每步后跑测试
5. **重复**：直到反模式消除

不要追求“完美架构”，追求“持续可维护”。架构是演进的，不是一次定型的。

下一章 → [第 10 章 · 高级技巧与工具链](../10-advanced/INDEX.md)
