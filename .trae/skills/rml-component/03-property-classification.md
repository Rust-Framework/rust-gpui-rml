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

> **已废弃**：`h_flex` / `v_flex` 已迁移到归一化样式属性，详见下方"归一化样式属性"小节。

#### 通用绑定属性 (COMMON_BIND_PROPS)

| 属性 | 说明 |
|------|------|
| `content` / `value` / `disabled` / `selected` / `checked` / `label` / `size` | 通用绑定 |

#### 通用事件属性 (COMMON_EVENT_PROPS)

| 属性 | 说明 |
|------|------|
| `on_click` | 通用点击事件（声明式 `on-click`） |

### 3. 归一化样式属性

由 [style_attr.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs) `apply_style_attr` 处理，**同时支持原生元素与扩展组件**（gpui-component 实现 `Styled` trait）。

#### 特征

- **属性名**：CSS kebab-case（声明式 `flex-direction="column"` / `padding-top="8px"`），normalize 后内部 snake_case 匹配
- **入口**：`style_attr::apply_style_attr(name, value)` 单一入口
- **映射源**：复用 [css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) `map_declaration` 单一映射源，避免双轨制
- **静态形式**：`width="100px"` / `display="flex"` / `gap="8px"`
- **bind 形式不支持**：`width={computed}` 输出 warning 并丢弃（运行时动态样式走 `class=` + 主题切换）
- **优先级**：在组件专用 setter 之前命中（保证所有扩展组件共享样式属性能力）

#### 属性清单（对齐 [STYLE_ATTR_PROPS](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L64-L81)）

| 类别 | 属性 | 数量 |
|------|------|------|
| 盒模型 | `width` / `height` / `padding` / `padding-{top,right,bottom,left}` / `margin` / `margin-{top,right,bottom,left}` / `border-radius` / `border` / `border-color` / `border-{top,right,bottom,left}` | 19 |
| 文本 | `font-size` / `font-weight` / `font-family` / `text-align` / `line-height` / `white-space` / `color` / `background` / `background-color` | 9 |
| Flexbox | `display` / `flex-direction` / `flex-wrap` / `justify-content` / `align-items` / `flex` / `gap` / `min-width` / `max-width` / `min-height` / `max-height` | 11 |
| 视觉效果 | `opacity` / `overflow` / `overflow-x` / `overflow-y` | 4 |

#### 语义快捷词

| 快捷词 | 等价值 | 生成代码 |
|--------|--------|----------|
| `width="full"` / `height="full"` | `width="100%"` / `height="100%"` | `.w_full()` / `.h_full()` |
| `min-width="0"` / `min-height="0"` | （CSS 0 关键字） | `.min_w_0()` / `.min_h_0()` |

#### CSS 变量与运行时主题查询

`color` / `background` / `border-color` 等颜色属性支持 `var(--name)` 形式，生成运行时主题查询调用 `rml::theme::color("--name")`，主题切换即时生效。

详见 [07-size-layout-conventions.md "归一化样式属性" 章节](file:///d:/GitCode/RF/rust-gpui-rml/.trae/skills/rml-component/07-size-layout-conventions.md)。

### 4. 警告丢弃

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
  │   ├─ 归一化样式属性命中? → 生成 .gpui_method(...)   ← style_attr::apply_style_attr
  │   ├─ 通用 component_static_setter 命中? → 生成 .method(value)
  │   └─ 未命中 → 检查 props_registry，已注册则 warning，否则静默丢弃
  ├─ Bind?
  │   ├─ 组件专用 bind_setter 命中? → 生成 .method(self.expr)
  │   ├─ 通用 component_bind_setter 命中? → 生成 .method(self.expr)
  │   ├─ 归一化样式属性? → warning + 丢弃（bind 形式不支持）
  │   └─ 未命中 → warning 或静默丢弃
  └─ Event?
      ├─ 组件专用 event_setter 命中? → 生成 .on_xxx(cx.listener(...))
      ├─ 通用 component_event_setter 命中? → 生成 .on_xxx(cx.listener(...))
      └─ 未命中 → warning 或静默丢弃
```

## 引擎代码生成路径与通用/特例分离原则

RML 引擎在样式属性归一化中遵循"高内聚低耦合"原则：通用路径共享单一入口与单一映射源，特例路径明确隔离为独立分支。本节刻画 5 条代码生成路径的边界。

### 通用路径（高内聚，单一映射源）

| 路径 | 入口 | 处理范围 | 单一映射源 |
|------|------|---------|-----------|
| **A. 原生元素通用样式** | [attribute.rs `apply_static_attr`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L20) → [style_attr.rs `apply_style_attr`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs#L52) | `<div>`/`<span>` 等原生元素的归一化样式属性 | `css::map_declarations` |
| **B. 扩展组件通用样式** | [component.rs `component_static_setter`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/component.rs#L364) → [style_attr.rs `apply_style_attr`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/style_attr.rs#L52) | `<Button>`/`<Card>` 等扩展组件的归一化样式属性（gpui-component 实现 `Styled` trait） | `css::map_declarations` |

两条路径共用同一入口 `style_attr::apply_style_attr` 与同一映射源 `css::map_declarations`，避免双轨制。

### 特例路径（低耦合，明确隔离）

| 路径 | 入口 | 处理范围 | 行为 |
|------|------|---------|------|
| **C. 原生元素特例（deprecation）** | [attribute.rs:27-36](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L27-L36) match 臂 | `h_flex`/`v_flex`/`h_full`/`w_full`/`min_w_0`/`min_h_0` | warning + 丢弃 |
| **D. bind 形式特例（不支持）** | [attribute.rs `apply_bind_attr` _ 分支](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/attribute.rs#L137-L145) | `is_style_attr(name)` 为真 | warning + 丢弃（运行时动态样式走 `class=` + 主题切换） |
| **E. CodeEditor 默认高度特例** | [gen.rs `height_chain`](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/code_editor/gen.rs#L137-L140) | CodeEditor 通过 `if !has("height") && !has("h")` 守卫应用默认 `.h(gpui::px(360.))`；用户写 `height="full"` 时跳过默认，由 `component_static_setter` → `apply_style_attr` 生成 `.h_full()` 追加到 setter 链 | has 守卫跳过默认 |

### 单一映射源验证

所有"通用样式"路径（A/B/E）均复用 [css/mapper.rs](file:///d:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs) `map_declaration`，无独立映射逻辑。

### 通用与特例的边界

- **通用**：43 个 CSS 属性（盒模型 19 + 文本 9 + Flexbox 11 + 视觉效果 4）经 `style_attr::apply_style_attr` 处理
- **特例**：6 个废弃 Tailwind 式属性（路径 C）+ bind 形式不支持（路径 D）+ CodeEditor 默认高度（路径 E）

特例路径**不引入新映射逻辑**，仅做 warning 或依赖通用路径（路径 E 复用 A/B 的 `.h_full()` 生成）。
