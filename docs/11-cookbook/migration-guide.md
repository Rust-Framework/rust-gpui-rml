# 11.3 迁移指南

> **本节目标**：帮助从原生 GPUI、gpui-rsx、WPF、Vue 迁移到 RML 的开发者快速建立映射。

## 11.3.1 从原生 GPUI 迁移

### 心智模型转变

原生 GPUI 是“命令式链式调用”，RML 是“声明式标记”。核心转变：

| 原生 GPUI                          | RML                          |
| --------------------------------- | ---------------------------- |
| `div().flex().child(...)`         | `<div class="flex">...</div>` |
| `cx.listener(\|this, ev, cx\| ...)` | `on:click="method"`          |
| `Entity<Self>` 持状态                | ViewModel 持状态，View 自动生成      |
| `Render::render` 手写               | `.rml` 模板自动生成 Render         |

### 迁移步骤

1. **识别视图边界**：每个 `impl Render` 对应一个 RML 视图
2. **提取状态到 ViewModel**：把 `render` 中读写的字段移到 ViewModel
3. **把链式调用翻译为模板**：`div().class("x")` → `<div class="x">`
4. **把监听器翻译为命令**：`on_click(cx.listener(...))` → `on:click="cmd"`
5. **保留复杂逻辑**：实在难翻译的部分可暂时用 `ElementRef` 命令式访问

### 代码对照

**原生 GPUI**：

```rust
fn render(&mut self, _window: &mut Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
    div()
        .flex()
        .flex_col()
        .gap(px(16.0))
        .child(Label::new(format!("Count: {}", self.count)))
        .child(
            Button::new("Click me")
                .on_click(cx.listener(|this, _ev, cx| {
                    this.count += 1;
                    cx.notify();
                }))
        )
}
```

**RML**：

```html
<div class="flex flex-col gap-4">
  <p>Count: {count}</p>
  <button on:click="increment">Click me</button>
</div>
```

```rust
#[command]
pub fn increment(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.count += 1;
    cx.notify();
}
```

## 11.3.2 从 gpui-rsx 迁移

gpui-rsx 是 JSX 风格的宏，与 RML 的差异主要在“文件分离”和“绑定系统”。

| gpui-rsx                          | RML                              |
| --------------------------------- | -------------------------------- |
| `rsx! { div { ... } }` 在 Rust 内  | 独立 `.rml` 文件                   |
| 闭包捕获变量                            | 绑定到 ViewModel 字段                |
| 无双向绑定                             | `r:model` 双向绑定                   |
| 无计算属性                             | `#[computed]`                    |
| 无热重载                              | 支持热重载                            |

迁移要点：

- 把 `rsx!` 块整体移到 `.rml` 文件
- 闭包捕获的变量改为 ViewModel 字段
- 手动的 `format!` 改为插值
- 手动的事件处理改为 `#[command]`

## 11.3.3 从 WPF 迁移

WPF 是 RML 的主要灵感来源，迁移最自然。

| WPF                               | RML                              |
| --------------------------------- | -------------------------------- |
| `.xaml` + `.xaml.cs`              | `.rml` + `.rml.rs`               |
| `{Binding Path=Name}`             | `{name}` 或 `r:bind="name"`       |
| `Mode=TwoWay`                     | `r:model="name"`                 |
| `ICommand`                        | `#[command]`                     |
| `INotifyPropertyChanged`          | `cx.notify()`                    |
| `StaticResource`                  | 资源字典                            |
| `DataTemplate`                    | 组件 + 插槽                         |
| `Style BasedOn`                   | `based_on`                       |
| `x:Name`                          | `ref`                            |

迁移要点：

- XAML 标签大多可直接对应 RML 标签
- `INotifyPropertyChanged` 的属性改为 ViewModel 字段
- `ICommand` 实现改为 `#[command]` 方法
- `ValueConverter` 改为实现 `Converter` trait

## 11.3.4 从 Vue 迁移

Vue 与 RML 在声明式理念上高度相似。

| Vue                               | RML                              |
| --------------------------------- | -------------------------------- |
| `<template>` SFC                  | `.rml` 文件                        |
| `ref()` / `reactive()`            | `#[derive(Model)]` 字段            |
| `v-if`                            | `r:if`                           |
| `v-for`                           | `r:each`                         |
| `v-model`                         | `r:model`                        |
| `computed`                        | `#[computed]`                    |
| `methods`                         | `#[command]`                     |
| `mounted` / `unmounted`           | `#[on_loaded]` / `#[on_unloaded]` |
| `props` + `emit`                  | 组件 props + 事件                   |
| `slot`                            | `<slot>`                         |
| `provide` / `inject`              | Context                          |

### 主要差异

1. **类型系统**：RML 基于 Rust，类型严格；Vue 基于 JS，运行时检查
2. **响应式**：Vue 是 Proxy 自动追踪；RML 是编译期依赖分析 + `cx.notify()`
3. **状态管理**：Vue 用 Pinia / Vuex；RML 用 Context + 全局 Model
4. **样式作用域**：Vue 的 `scoped`；RML 的组件级样式表

### 迁移示例

**Vue**：

```vue
<template>
  <div>
    <p>{{ count }}</p>
    <button @click="increment">+1</button>
  </div>
</template>

<script setup>
import { ref } from 'vue';
const count = ref(0);
const increment = () => count.value++;
</script>
```

**RML**：

```html
<div>
  <p>{count}</p>
  <button on:click="increment">+1</button>
</div>
```

```rust
#[derive(Model)]
pub struct CounterVM { pub count: i32 }

impl CounterVM {
    #[command]
    pub fn increment(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count += 1;
        cx.notify();
    }
}
```

## 11.3.5 迁移策略

### 渐进式迁移（推荐）

不要一次性重写。推荐顺序：

1. **新功能用 RML**：新增视图用 RML 写
2. **叶子视图先迁**：从无依赖的简单视图开始
3. **共享状态抽 Context**：迁移过程中用 Context 桥接新旧代码
4. **逐步替换**：每迁一个视图，跑全量测试

### 一次性迁移

适用于小型项目或重写项目。步骤：

1. 梳理所有视图，列出迁移清单
2. 设计 Model / Service 层（与框架无关，可先做）
3. 逐个视图翻译
4. 集成测试

### 不可迁移的部分

- 高度定制的渲染逻辑（如自绘 canvas）：保留为 GPUI 自定义元素，在 RML 中通过 `<Custom>` 标签嵌入
- 性能极敏感的动画层：保留原生 GPUI

## 11.3.6 迁移常见陷阱

### 陷阱 1：把 Vue 的“魔法”带进 RML

Vue 的响应式是自动的，RML 需要显式 `cx.notify()`。忘记 notify 是最常见 bug。

### 陷阱 2：在模板里写 JS 表达式

Vue 模板支持复杂 JS 表达式，RML 模板只支持简单绑定。复杂逻辑收敛到 `#[computed]`。

### 陷阱 3：用 ref 直接操作 DOM

Vue 的 `ref` 是 DOM 引用，RML 的 `ElementRef` 是 GPUI 元素句柄，能力不同。优先用状态驱动，`ElementRef` 只用于聚焦等少数场景。

### 陷阱 4：忽略类型

Vue 是动态类型，RML 是静态类型。迁移时要补全类型，否则编译失败。

下一节 → [11.4 常见问题 FAQ](./faq.md)
