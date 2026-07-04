# 03 属性分类

## 三大类属性

RML 属性分为三大类，由 `Attribute` enum 区分：

| 类型 | 语法 | 解析 | 示例 |
|------|------|------|------|
| Static | `name="value"` / `name=""` | `Attribute::Static` | `label="Click"` / `primary` |
| Bind | `name={expr}` | `Attribute::Bind` | `value={count}` / `items={data}` |
| Event | `on-{event}={fn}` | `Attribute::Event` | `on-click={increment}` |

## 三级分类

属性按作用域分三级，setter 查找按以下顺序：

### 1. 组件专用属性

由各组件 `setters.rs` 处理，仅对特定 tag 生效：

```rust
// description_list/setters.rs
match canonical.as_str() {
    "DescriptionList" => match name {
        "vertical" => ...,
        "bordered" => ...,
        "columns" => ...,
        "label_width" => ...,
        "items" => ...,
    },
    "DescriptionItem" => match name {
        "value" => ...,
        "span" => ...,
    },
}
```

### 2. 通用属性

由 `component.rs::component_static_setter` / `component_bind_setter` / `component_event_setter` 处理，对所有 Stateless/Stateful 组件生效。

#### 通用静态属性 (COMMON_STATIC_PROPS)

| 属性 | 说明 |
|------|------|
| `label` / `placeholder` / `tooltip` | 文本类 |
| `primary` / `secondary` / `danger` / `success` / `warning` / `info` / `ghost` / `link` / `text` | Button variant |
| `size` | Sizable 尺寸（xsmall/small/medium/large） |
| `compact` / `loading` / `disabled` / `selected` | 状态 |
| `font_thin` ... `font_black` | 字体权重 |
| `h_flex` / `v_flex` | 布局快捷方法 |

#### 通用绑定属性 (COMMON_BIND_PROPS)

| 属性 | 说明 |
|------|------|
| `content` / `value` / `disabled` / `selected` / `checked` / `label` / `size` | 通用绑定 |

#### 通用事件属性 (COMMON_EVENT_PROPS)

| 属性 | 说明 |
|------|------|
| `on_click` | 通用点击事件（声明式 `on-click`） |

### 3. 警告丢弃

未命中组件专用或通用 setter 的属性，会触发 warning 并被静默丢弃：

```rust
if crate::compiler::props_registry::is_prop_registered(tag, name) {
    eprintln!(
        "[rml warning] <{}> property `{}` is registered but has no mapping; \
         property will be silently dropped. Add a match arm in setters.rs.",
        tag, name
    );
}
```

**设计原则**：框架全新开发，**不保留兼容性设计**。已注册但无 mapping 的属性应立即补全 setter，而非静默丢弃。

## EventHandler 三种形式

事件属性 `on-{event}={...}` 的 handler 有三种形式：

| 形式 | 语法 | 生成代码 | 适用场景 |
|------|------|----------|----------|
| Ident | `on-click={method}` | `this.method(...)` | 最常见，方法名与事件同名 |
| MethodName | `on-click={handler}` | `this.handler(...)` | 方法名与事件不同名 |
| WithArgs | `on-click={method(arg1, arg2)}` | `this.method(arg1, arg2, ...)` | 需要传递额外参数 |

## 职责边界规则

### 组件专用 setter 优先

`component.rs` 的通用 setter 在调用各组件专用 setter 后，仅处理未命中的属性：

```rust
// component_static_setter 优先委托
if let Some(s) = super::description_list::setters::static_setter(name, value, tag) {
    return Some(s);
}
// 未命中才走通用 match
match name {
    "label" => ...,
    "size" => ...,
}
```

### canonical_tag 比对

组件专用 setter 内部**必须**使用 `canonical_tag(tag)` 而非裸 tag 字符串比对：

```rust
// ✅ 正确
let canonical = crate::tags::canonical_tag(tag);
match canonical.as_str() {
    "DescriptionList" => ...,
}

// ❌ 错误（多形式漏洞）
match tag {
    "DescriptionList" | "descriptions" => ...,
}
```

## 分类决策树

```
属性出现
  ├─ Static?
  │   ├─ 组件专用 static_setter 命中? → 生成 .method(value)
  │   ├─ 通用 component_static_setter 命中? → 生成 .method(value)
  │   └─ 未命中 → 检查 props_registry，已注册则 warning，否则静默丢弃
  ├─ Bind?
  │   ├─ 组件专用 bind_setter 命中? → 生成 .method(self.expr)
  │   ├─ 通用 component_bind_setter 命中? → 生成 .method(self.expr)
  │   └─ 未命中 → warning 或静默丢弃
  └─ Event?
      ├─ 组件专用 event_setter 命中? → 生成 .on_xxx(cx.listener(...))
      ├─ 通用 component_event_setter 命中? → 生成 .on_xxx(cx.listener(...))
      └─ 未命中 → warning 或静默丢弃
```
