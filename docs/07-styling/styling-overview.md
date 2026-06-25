# 7.1 样式系统总览

> **本节目标**：了解 RML 样式系统的全貌——样式来源、加载机制、作用域、与 GPUI 的关系。

## 7.1.1 样式的来源

RML 的样式来自三个层次：

```
┌─────────────────────────────────────────────┐
│           RML 样式系统                        │
│                                             │
│  ┌─────────────────┐  ┌─────────────────┐   │
│  │  内联样式        │  │  外部样式表       │   │
│  │  style="..."    │  │  styles.css     │   │
│  └─────────────────┘  └─────────────────┘   │
│                                             │
│  ┌─────────────────┐  ┌─────────────────┐   │
│  │  工具类          │  │  主题变量        │   │
│  │  class="flex"   │  │  var(--color)   │   │
│  └─────────────────┘  └─────────────────┘   │
└─────────────────────────────────────────────┘
```

### 内联样式

通过 `style` 属性直接写在元素上：

```html
<div style="background: red; padding: 10px;">
    红色背景
</div>
```

### 外部样式表

通过 `.css` 文件定义样式，在 `build.rs` 中引入：

```css
/* src/styles/main.css */
.container {
    padding: 20px;
    background: #f5f5f5;
}

.title {
    font-size: 24px;
    font-weight: bold;
}
```

```rust
// build.rs
rml::compile_rml()
    .with_style("styles/main.css")
    .compile()
```

### 工具类

预定义的类名，提供常用样式：

```html
<div class="flex flex-col gap-4 p-6">
    <h1 class="text-xl font-bold">标题</h1>
</div>
```

### 主题变量

通过 CSS 变量实现主题切换：

```css
:root {
    --primary-color: #007bff;
    --text-color: #333;
}

.button {
    background: var(--primary-color);
    color: var(--text-color);
}
```

## 7.1.2 样式的加载机制

### 编译时加载

RML 在编译时加载所有样式表，生成高效的样式代码：

```rust
// build.rs
fn main() {
    rml::compile_rml()
        .with_style("styles/reset.css")       // 重置样式
        .with_style("styles/variables.css")   // 主题变量
        .with_style("styles/utilities.css")   // 工具类
        .with_style("styles/components.css")  // 组件样式
        .compile();
}
```

### 加载顺序

样式按声明顺序加载，后加载的样式优先级更高：

```
1. reset.css       ← 基础重置
2. variables.css   ← 主题变量
3. utilities.css   ← 工具类
4. components.css  ← 组件样式
```

### 热重载

开发模式下，修改 `.css` 文件会自动触发热重载：

```bash
# 修改 styles/main.css 后，应用自动刷新
```

## 7.1.3 样式的作用域

### 全局样式

在 `.css` 文件中定义的样式是全局的：

```css
/* 全局样式，所有 .button 都会应用 */
.button {
    padding: 8px 16px;
    border-radius: 4px;
}
```

### 组件样式

通过命名约定避免冲突：

```css
/* 组件样式，使用组件名作为前缀 */
.user-card {
    /* ... */
}

.user-card-header {
    /* ... */
}

.user-card-body {
    /* ... */
}
```

### 内联样式

内联样式优先级最高，只作用于当前元素：

```html
<div style="color: red;">红色文字</div>
```

## 7.1.4 样式的优先级

RML 样式优先级从低到高：

```
1. 全局样式表（按加载顺序）
2. 组件样式表
3. class 属性
4. style 属性（内联样式）
```

### 优先级示例

```css
/* 全局样式 */
.button {
    color: black;
    background: white;
}

/* 组件样式 */
.primary-button {
    background: blue;
}
```

```html
<!-- class 优先级高于全局样式 -->
<button class="button primary-button">按钮</button>
<!-- 颜色：black（全局）
     背景：blue（组件样式覆盖全局） -->

<!-- style 优先级最高 -->
<button class="button primary-button" style="background: red;">按钮</button>
<!-- 背景：red（内联样式覆盖一切） -->
```

## 7.1.5 样式与 GPUI 的关系

RML 的样式系统建立在 GPUI 之上：

```
RML 样式（CSS 子集）
       ↓ 编译
GPUI 样式方法（.bg()、.p()、.text_xl() 等）
       ↓ 渲染
GPU 渲染
```

### 编译示例

```html
<div class="container" style="padding: 10px;">
    <h1 class="title">标题</h1>
</div>
```

编译后等价于：

```rust
div()
    .class("container")  // 应用 .container 的样式
    .p(px(10.0))         // 应用内联样式
    .child(
        div()
            .class("title")  // 应用 .title 的样式
            .child(Label::new("标题"))
    )
```

## 7.1.6 样式的限制

RML 的样式系统是 CSS 子集，有一些限制：

### 不支持的 CSS 特性

| 特性              | 说明                  |
| --------------- | ------------------- |
| `@media` 媒体查询   | 部分支持，详见 7.4         |
| `@keyframes` 动画 | 通过 GPUI 动画系统实现      |
| `:hover` 等伪类    | 通过事件绑定实现            |
| `::before` 伪元素  | 不支持                 |
| `calc()` 函数     | 部分支持                |
| CSS Grid        | 不支持，使用 Flexbox 替代   |

### 替代方案

| CSS 特性          | RML 替代方案              |
| ---------------- | --------------------- |
| `:hover`         | `onmouseenter`/`onmouseleave` 事件 |
| `:active`        | `onmousedown`/`onmouseup` 事件     |
| `:focus`         | `onfocus`/`onblur` 事件             |
| 动画               | GPUI 动画 API          |
| CSS Grid         | Flexbox 嵌套            |

## 7.1.7 样式的调试

### 开发者工具

RML 应用支持开发者工具，可以查看元素的样式：

```bash
# 启动应用时启用开发者工具
RML_DEV_TOOLS=1 cargo run
```

### 样式日志

在代码中打印样式信息：

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.log_styles();  // 打印当前视图的所有样式
}
```

## 7.1.8 样式系统的设计理念

RML 样式系统的设计理念：

### 1. 熟悉优先

借鉴 CSS 语法，降低学习成本：

```css
/* 与 CSS 几乎相同 */
.container {
    padding: 20px;
    background: #f5f5f5;
    border-radius: 8px;
}
```

### 2. 性能优先

编译时处理样式，运行时高效：

- 样式在编译时解析为 GPUI 方法调用
- 避免运行时样式计算
- 支持样式缓存

### 3. 桌面优先

针对桌面应用优化：

- 使用像素（px）作为主要单位
- 不支持响应式布局（桌面窗口大小固定）
- 支持高 DPI 显示

### 4. 简洁优先

提供工具类简化常用样式：

```html
<!-- 工具类 -->
<div class="flex flex-col gap-4 p-6">

<!-- 等价于 -->
<div style="display: flex; flex-direction: column; gap: 16px; padding: 24px;">
```

## 7.1.9 小结

RML 的样式系统：

- **来源**：内联样式、外部样式表、工具类、主题变量
- **加载**：编译时加载，支持热重载
- **作用域**：全局、组件、内联
- **优先级**：全局 < 组件 < class < style
- **与 GPUI**：样式编译为 GPUI 方法调用
- **限制**：CSS 子集，部分特性不支持

下一节 → [7.2 CSS 子集与扩展](./css-subset.md)
