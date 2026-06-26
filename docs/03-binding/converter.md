# 3.5 值转换器

> **本节目标**：掌握 `Converter` trait 的实现与使用，在绑定路径上进行类型转换或格式化。

## 3.5.1 值转换器的定义

值转换器是绑定路径上的"过滤器"，在 ViewModel 字段值与 UI 显示值之间进行转换。

```
ViewModel 字段 (i32)  ──[Converter]──▶  UI 显示 (SharedString)
UI 输入 (SharedString) ──[Converter]──▶  ViewModel 字段 (i32)
```

典型场景：

- 数字格式化：`1500` → `"¥1,500.00"`
- 枚举显示：`Status::Loading` → `"加载中..."`
- 大小写转换：`"hello"` → `"HELLO"`
- 日期格式化：时间戳 → `"2026-06-25"`

## 3.5.2 Converter trait

```rust
pub trait Converter: Send + Sync {
    type Source;  // ViewModel 侧的类型
    type Target;  // UI 侧的类型

    /// ViewModel → UI：正向转换
    fn convert(&self, value: &Self::Source) -> Self::Target;

    /// UI → ViewModel：反向转换（双向绑定时需要）
    fn convert_back(&self, value: &Self::Target) -> Option<Self::Source>;
}
```

## 3.5.3 实现自定义转换器

### 示例：价格格式化

```rust
use rml::prelude::*;

pub struct PriceConverter;

impl Converter for PriceConverter {
    type Source = f64;
    type Target = SharedString;

    fn convert(&self, value: &f64) -> SharedString {
        format!("¥{:.2}", value).into()
    }

    fn convert_back(&self, value: &SharedString) -> Option<f64> {
        // 去掉 ¥ 前缀后解析
        value.trim_start_matches('¥').parse().ok()
    }
}
```

### 示例：状态枚举显示

```rust
pub enum Status {
    Loading,
    Success,
    Error(String),
}

pub struct StatusConverter;

impl Converter for StatusConverter {
    type Source = Status;
    type Target = SharedString;

    fn convert(&self, value: &Status) -> SharedString {
        match value {
            Status::Loading => "加载中...".into(),
            Status::Success => "成功".into(),
            Status::Error(msg) => format!("错误: {}", msg).into(),
        }
    }

    fn convert_back(&self, _: &SharedString) -> Option<Status> {
        None  // 单向转换，不支持反向
    }
}
```

## 3.5.4 在 `.rml` 中使用转换器

### 单向绑定 + 转换器

```html
<p>价格: {price | PriceConverter}</p>
<p>状态: {status | StatusConverter}</p>
```

`|` 是转换器管道符，借鉴自 shell 管道。

### 双向绑定 + 转换器

```html
<input model={price | PriceConverter} />
```

双向绑定时，`convert` 用于 ViewModel → UI，`convert_back` 用于 UI → ViewModel。

### 转换器链

多个转换器可以串联：

```html
<p>{value | TrimConverter | UpperCaseConverter}</p>
```

执行顺序：从左到右，前一个的输出作为后一个的输入。

## 3.5.5 内置转换器

RML 提供常用内置转换器：

| 转换器             | 功能               | 示例                              |
| --------------- | ---------------- | ------------------------------- |
| `Currency`      | 货币格式化            | `1500.0` → `"¥1,500.00"`        |
| `Percent`       | 百分比格式化           | `0.85` → `"85%"`                |
| `Date`          | 日期格式化            | 时间戳 → `"2026-06-25"`            |
| `DateTime`      | 日期时间格式化          | 时间戳 → `"2026-06-25 14:30:00"`   |
| `UpperCase`     | 转大写              | `"hello"` → `"HELLO"`           |
| `LowerCase`     | 转小写              | `"HELLO"` → `"hello"`           |
| `Trim`          | 去除首尾空白           | `" hello "` → `"hello"`         |
| `BoolToYesNo`   | 布尔转是/否           | `true` → `"是"`                  |

### 使用内置转换器

```html
<p>金额: {amount | Currency}</p>
<p>进度: {progress | Percent}</p>
<p>时间: {timestamp | DateTime}</p>
<p>用户名: {username | UpperCase}</p>
```

## 3.5.6 转换器与计算属性的对比

| 特性     | 转换器                    | 计算属性              |
| ------ | ---------------------- | ----------------- |
| 复用性    | ✅ 可在多个绑定中复用            | ❌ 与特定 ViewModel 绑定 |
| 参数化    | ❌ 无参数（通过类型参数化）         | ❌ 无参数             |
| 缓存     | ❌ 每次绑定都执行              | ✅ 自动缓存            |
| 适用场景   | 通用类型转换、格式化             | 特定 ViewModel 的派生值 |
| 定义位置   | 独立 struct，实现 Converter | ViewModel 的方法     |

### 选择建议

- **通用转换逻辑**（如货币格式化）：用转换器，可在多个 ViewModel 中复用
- **特定派生值**（如"已完成任务数"）：用计算属性
- **简单一次性计算**：直接在插值中写表达式

## 3.5.7 转换器的注册

转换器需要在 ViewModel 中注册，RML 编译器才能识别：

```rust
#[derive(Model)]
#[component]
pub struct MyView {
    pub price: f64,
    pub status: Status,
}

impl MyView {
    pub fn new() -> Self {
        let mut view = Self {
            price: 0.0,
            status: Status::Loading,
        };

        // 注册转换器
        view.register_converter("PriceConverter", PriceConverter);
        view.register_converter("StatusConverter", StatusConverter);

        view
    }
}
```

💡 **提示**：内置转换器无需注册，可直接使用。

## 3.5.8 转换器的错误处理

`convert_back` 返回 `Option`，表示反向转换可能失败：

```rust
impl Converter for PriceConverter {
    fn convert_back(&self, value: &SharedString) -> Option<f64> {
        value.trim_start_matches('¥').parse().ok()
    }
}
```

如果反向转换失败，RML 会：

1. 保持 ViewModel 字段不变
2. 在 UI 上显示转换失败提示（可选）

### 自定义错误提示

```html
<input
    model={price | PriceConverter}
    on_convert_error={handle_convert_error}
/>
<p if={convert_error.is_some()} class="error">{convert_error}</p>
```

```rust
#[command]
pub fn handle_convert_error(&mut self, error: &ConvertError, cx: &mut ViewContext<Self>) {
    self.convert_error = Some(error.message.clone());
    cx.notify();
}
```

## 3.5.9 转换器的性能

转换器在每次绑定时执行，没有缓存。因此：

- ✅ 适合轻量级转换（格式化、大小写）
- ❌ 不适合重量级计算（复杂数学运算）

重量级计算应该用计算属性，享受缓存。

## 3.5.10 完整示例：金额输入

需求：用户输入金额字符串，ViewModel 存储为 `f64`，显示时格式化为 `"¥1,500.00"`。

```rust
// converter.rs
use rml::prelude::*;

pub struct CurrencyConverter;

impl Converter for CurrencyConverter {
    type Source = f64;
    type Target = SharedString;

    fn convert(&self, value: &f64) -> SharedString {
        format!("¥{:.2}", value).into()
    }

    fn convert_back(&self, value: &SharedString) -> Option<f64> {
        let cleaned = value
            .trim_start_matches('¥')
            .replace(',', "")
            .trim()
            .to_string();
        cleaned.parse().ok()
    }
}
```

```rust
// view.rml.rs
#[derive(Model)]
#[component]
pub struct PaymentView {
    pub amount: f64,
    pub error: Option<SharedString>,
}

impl PaymentView {
    pub fn new() -> Self {
        let mut view = Self {
            amount: 0.0,
            error: None,
        };
        view.register_converter("CurrencyConverter", CurrencyConverter);
        view
    }

    #[command]
    pub fn submit(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.amount <= 0.0 {
            self.error = Some("金额必须大于 0".into());
            cx.notify();
            return;
        }
        // 提交逻辑...
    }
}
```

```html
<!-- view.rml -->
<div class="payment-form">
    <label>金额：</label>
    <input model={amount | CurrencyConverter} placeholder="¥0.00" />
    <p if={error.is_some()} class="error">{error}</p>
    <button onclick={submit}>提交</button>

    <p>当前金额: {amount | CurrencyConverter}</p>
</div>
```

## 3.5.11 小结

值转换器是绑定路径上的"过滤器"：

- **定义**：实现 `Converter` trait，提供 `convert` 和 `convert_back`
- **使用**：在绑定中用 `|` 管道符，如 `{value | Converter}`
- **复用**：转换器是独立 struct，可在多个 ViewModel 中复用
- **缓存**：转换器无缓存，适合轻量级转换

选择建议：通用格式化用转换器，特定派生值用计算属性。

下一节 → [3.6 绑定引擎原理](./binding-engine.md)
