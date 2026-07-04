# 6.4 组件属性

> **本节目标**：完整掌握组件属性的类型、传递方式、默认值、验证和双向绑定。

## 6.4.1 属性的类型

组件属性可以是任何 `Clone` 类型：

### 基础类型

```rust
#[derive(IModel)]
#[component]
pub struct Button {
    pub text: SharedString,         // 字符串
    pub count: i32,                 // 整数
    pub rate: f64,                  // 浮点数
    pub disabled: bool,             // 布尔
    pub size: SharedString,         // 枚举（用字符串表示）
}
```

### 集合类型

```rust
#[derive(IModel)]
#[component]
pub struct List {
    pub items: Vec<Item>,           // 列表
    pub selected_ids: HashSet<u64>, // 集合
}
```

### Option 类型

```rust
#[derive(IModel)]
#[component]
pub struct Avatar {
    pub src: SharedString,
    pub alt: Option<SharedString>,  // 可选属性
}
```

### 自定义类型

```rust
#[derive(IModel, Clone)]
pub struct User {
    pub id: u64,
    pub name: SharedString,
    pub email: SharedString,
}

#[derive(IModel)]
#[component]
pub struct UserCard {
    pub user: User,                 // 自定义类型
}
```

### 枚举类型

```rust
#[derive(Clone, PartialEq)]
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

#[derive(IModel)]
#[component]
pub struct Button {
    pub variant: ButtonVariant,     // 枚举
}
```

## 6.4.2 属性的传递

### 字面量传递

```html
<Button text="提交" variant="primary" size="large" disabled={true} />
```

### 表达式传递

```html
<Button text={button_label} disabled={is_loading} />
```

### 字段引用传递

```html
<UserCard user={current_user} />
<Avatar src={user.avatar} alt={user.name} />
```

### 布尔属性的简写

```html
<!-- 完整写法 -->
<Button disabled={true}>提交</Button>

<!-- 简写（仅 true 可省略） -->
<Button disabled>提交</Button>
```

### 复杂表达式

```html
<Button
    text={if is_loading { "加载中..." } else { "提交" }}
    disabled={is_loading || !is_form_valid}
/>
```

## 6.4.3 属性的默认值

### 通过构造函数

```rust
impl Button {
    pub fn new() -> Self {
        Self {
            text: SharedString::default(),
            variant: "primary".into(),    // 默认主要样式
            size: "medium".into(),         // 默认中等尺寸
            disabled: false,               // 默认启用
        }
    }
}
```

### 通过 `#[prop]` 属性

```rust
#[derive(IModel)]
#[component]
pub struct Button {
    pub text: SharedString,
    #[prop(default = "primary")]
    pub variant: SharedString,
    #[prop(default = "medium")]
    pub size: SharedString,
    #[prop(default = false)]
    pub disabled: bool,
}
```

### 在 `.rml` 中使用默认值

```html
<!-- 使用默认值 -->
<Button text="提交" />

<!-- 等价于 -->
<Button text="提交" variant="primary" size="medium" disabled={false} />
```

## 6.4.4 属性的响应式

当属性变化时，组件会自动重新渲染：

```html
<!-- 父视图 -->
<div>
    <p>当前用户: {user_name}</p>
    <Button text={button_text} disabled={is_loading} />
</div>
```

```rust
#[derive(IModel)]
#[component]
pub struct ParentView {
    pub user_name: SharedString,
    pub button_text: SharedString,
    pub is_loading: bool,
}

impl ParentView {
    #[command]
    pub fn submit(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_loading = true;
        self.button_text = "加载中...".into();
        cx.notify();

        cx.spawn(|this, mut cx| async move {
            // 异步操作...
            let _ = this.update(&mut cx, |this, cx| {
                this.is_loading = false;
                this.button_text = "提交".into();
                cx.notify();
            });
        }).detach();
    }
}
```

### 监听属性变化

用 `#[on_prop_change]` 监听属性变化：

```rust
#[derive(IModel)]
#[component]
pub struct DataView {
    pub data_id: u64,
    pub data: Option<Data>,
}

impl DataView {
    #[on_prop_change(data_id)]
    pub fn on_data_id_change(&mut self, cx: &mut Context<Self>) {
        self.load_data(self.data_id, cx);
    }

    fn load_data(&mut self, id: u64, cx: &mut Context<Self>) {
        cx.spawn(|this, mut cx| async move {
            let data = fetch_data(id).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.data = Some(data);
                cx.notify();
            });
        }).detach();
    }
}
```

## 6.4.5 双向绑定属性

组件可以实现双向绑定，让父视图通过 `model` 指令绑定：

### 实现双向绑定

```rust
#[derive(IModel)]
#[component]
pub struct Counter {
    pub value: i32,
    pub min: i32,
    pub max: i32,
}

impl TwoWayBinding for Counter {
    type Value = i32;

    fn get_value(&self) -> Self::Value {
        self.value
    }

    fn set_value(&mut self, value: Self::Value, cx: &mut Context<Self>) {
        self.value = value.clamp(self.min, self.max);
        cx.notify();
    }
}
```

### 在父视图中使用

```html
<div>
    <p>当前值: {my_count}</p>
    <Counter model={my_count} min={0} max={100} />
</div>
```

```rust
#[derive(IModel)]
#[component]
pub struct MyView {
    pub my_count: i32,
}
```

### 双向绑定的数据流

```
父视图 my_count ──属性──▶ Counter value
父视图 my_count ◀──事件── Counter value
```

1. 父视图的 `my_count` 变化 → 更新 Counter 的 `value`
2. Counter 内部修改 `value` → 同步到父视图的 `my_count`

## 6.4.6 属性的验证

### 构造时验证

```rust
impl ProgressBar {
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 100.0),  // 验证范围
        }
    }
}
```

### Setter 验证

```rust
impl ProgressBar {
    pub fn set_value(&mut self, value: f64, cx: &mut Context<Self>) {
        self.value = value.clamp(0.0, 100.0);
        cx.notify();
    }
}
```

### `#[on_prop_change]` 验证

```rust
#[on_prop_change(value)]
pub fn on_value_change(&mut self, cx: &mut Context<Self>) {
    if self.value < 0.0 || self.value > 100.0 {
        self.value = self.value.clamp(0.0, 100.0);
        cx.notify();
    }
}
```

## 6.4.7 属性的设计原则

### 1. 属性应尽量少

```rust
// ✅ 精简的属性
#[component]
pub struct Button {
    pub text: SharedString,
    pub variant: SharedString,
    pub size: SharedString,
    pub disabled: bool,
}

// ❌ 过多的属性
#[component]
pub struct Button {
    pub text: SharedString,
    pub text_color: SharedString,
    pub bg_color: SharedString,
    pub border_color: SharedString,
    pub padding: f64,
    pub margin: f64,
    pub font_size: f64,
    pub font_weight: f64,
    // ... 太多了
}
```

### 2. 用枚举而非字符串

```rust
// ✅ 枚举
pub enum ButtonVariant {
    Primary,
    Secondary,
    Danger,
    Ghost,
}

// ❌ 字符串
pub variant: SharedString,  // 容易拼写错误
```

### 3. 用组合而非继承

```rust
// ✅ 组合
#[component]
pub struct UserCard {
    pub user: User,
    pub avatar_size: AvatarSize,
}

// ❌ 继承
pub struct UserCardWithAvatar : UserCard { ... }
```

### 4. 提供合理的默认值

```rust
impl Button {
    pub fn new() -> Self {
        Self {
            text: SharedString::default(),
            variant: ButtonVariant::Primary,  // 默认主要
            size: ButtonSize::Medium,          // 默认中等
            disabled: false,                   // 默认启用
        }
    }
}
```

## 6.4.8 完整示例：进度条组件

```rust
// components/progress_bar.rml.rs
use rml::prelude::*;

#[derive(Clone, PartialEq)]
pub enum ProgressVariant {
    Primary,
    Success,
    Warning,
    Danger,
}

impl Default for ProgressVariant {
    fn default() -> Self {
        Self::Primary
    }
}

#[derive(IModel)]
#[component]
pub struct ProgressBar {
    pub value: f64,
    pub max: f64,
    pub variant: ProgressVariant,
    pub show_label: bool,
    pub label_format: SharedString,
}

impl ProgressBar {
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 100.0),
            max: 100.0,
            variant: ProgressVariant::default(),
            show_label: true,
            label_format: "{value}%".into(),
        }
    }

    #[computed]
    pub fn percentage(&self) -> f64 {
        (self.value / self.max * 100.0).clamp(0.0, 100.0)
    }

    #[computed]
    pub fn label(&self) -> SharedString {
        self.label_format
            .replace("{value}", &format!("{:.1}", self.percentage))
            .into()
    }

    #[computed]
    pub fn variant_class(&self) -> &'static str {
        match self.variant {
            ProgressVariant::Primary => "progress-primary",
            ProgressVariant::Success => "progress-success",
            ProgressVariant::Warning => "progress-warning",
            ProgressVariant::Danger => "progress-danger",
        }
    }
}

impl TwoWayBinding for ProgressBar {
    type Value = f64;

    fn get_value(&self) -> Self::Value {
        self.value
    }

    fn set_value(&mut self, value: Self::Value, cx: &mut Context<Self>) {
        self.value = value.clamp(0.0, self.max);
        cx.notify();
    }
}
```

```html
<!-- components/progress_bar.rml -->
<div class="progress-container">
    <div class="progress-bar {variant_class}">
        <div class="progress-fill" style="width: {percentage}%"></div>
    </div>
    <span if={show_label} class="progress-label">{label}</span>
</div>
```

### 使用进度条

```html
<!-- views/upload_view.rml -->
<div class="upload-view">
    <h1>文件上传</h1>

    <input type="file" onchange={handle_file_select} />

    <div if={is_uploading}>
        <ProgressBar
            value={upload_progress}
            variant={upload_variant}
            show_label={true}
        />
        <button on-click={cancel_upload}>取消</button>
    </div>

    <div if={upload_complete}>
        <p>上传完成！</p>
        <ProgressBar value={100} variant="success" />
    </div>
</div>
```

```rust
// views/upload_view.rml.rs
use rml::prelude::*;
use crate::components::progress_bar::{ProgressBar, ProgressVariant};

#[derive(IModel)]
#[component]
pub struct UploadView {
    pub upload_progress: f64,
    pub is_uploading: bool,
    pub upload_complete: bool,
    pub upload_variant: ProgressVariant,
}

impl UploadView {
    pub fn new() -> Self {
        Self {
            upload_progress: 0.0,
            is_uploading: false,
            upload_complete: false,
            upload_variant: ProgressVariant::Primary,
        }
    }

    #[command]
    pub fn handle_file_select(&mut self, ev: &ChangeEvent, cx: &mut Context<Self>) {
        self.is_uploading = true;
        self.upload_progress = 0.0;
        self.upload_complete = false;
        self.upload_variant = ProgressVariant::Primary;
        cx.notify();

        // 模拟上传
        cx.spawn(|this, mut cx| async move {
            for i in 1..=100 {
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(50))
                    .await;

                let _ = this.update(&mut cx, |this, cx| {
                    this.upload_progress = i as f64;
                    cx.notify();
                });
            }

            let _ = this.update(&mut cx, |this, cx| {
                this.is_uploading = false;
                this.upload_complete = true;
                this.upload_variant = ProgressVariant::Success;
                cx.notify();
            });
        }).detach();
    }

    #[command]
    pub fn cancel_upload(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_uploading = false;
        self.upload_progress = 0.0;
        self.upload_variant = ProgressVariant::Danger;
        cx.notify();
    }
}
```

## 6.4.9 小结

组件属性是组件的输入接口：

- **类型**：基础类型、集合、Option、自定义类型、枚举
- **传递**：字面量、表达式、字段引用、布尔简写
- **默认值**：构造函数或 `#[prop(default = ...)]`
- **响应式**：属性变化自动重新渲染
- **双向绑定**：实现 `TwoWayBinding` trait
- **验证**：构造时、Setter、`#[on_prop_change]`

下一节 → [6.5 组件组合](./composition.md)
