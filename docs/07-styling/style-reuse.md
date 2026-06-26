# 7.5 样式复用

> **本节目标**：掌握 RML 的样式复用策略——全局样式、组件样式、工具类、CSS 模块。

## 7.5.1 样式复用的层次

```
┌─────────────────────────────────────────┐
│              样式复用层次                   │
│                                         │
│  ┌─────────────┐  ┌─────────────┐       │
│  │  全局样式     │  │  组件样式     │       │
│  │  reset.css  │  │  button.css │       │
│  └─────────────┘  └─────────────┘       │
│                                         │
│  ┌─────────────┐  ┌─────────────┐       │
│  │  工具类       │  │  CSS 模块    │       │
│  │  flex, p-4  │  │  .button    │       │
│  └─────────────┘  └─────────────┘       │
└─────────────────────────────────────────┘
```

## 7.5.2 全局样式

### 重置样式

```css
/* src/styles/reset.css */
* {
    margin: 0;
    padding: 0;
    box-sizing: border-box;
}

body {
    font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: #333;
    background: #fff;
}

ul, ol {
    list-style: none;
}

a {
    color: inherit;
    text-decoration: none;
}

button {
    background: none;
    border: none;
    cursor: pointer;
    font-family: inherit;
}

input, textarea {
    font-family: inherit;
    font-size: inherit;
}
```

### 全局变量

```css
/* src/styles/variables.css */
:root {
    /* 颜色 */
    --color-primary: #007bff;
    --color-secondary: #6c757d;
    --color-success: #28a745;
    --color-danger: #dc3545;
    --color-warning: #ffc107;
    --color-info: #17a2b8;

    /* 文本 */
    --color-text: #333;
    --color-text-muted: #6c757d;

    /* 背景 */
    --color-bg: #fff;
    --color-bg-alt: #f8f9fa;

    /* 边框 */
    --color-border: #dee2e6;

    /* 间距 */
    --spacing-unit: 8px;

    /* 圆角 */
    --border-radius: 4px;

    /* 阴影 */
    --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
    --shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}
```

### 在 build.rs 中加载

```rust
// build.rs
fn main() {
    rml::compile_rml()
        .with_style("styles/reset.css")
        .with_style("styles/variables.css")
        .with_style("styles/utilities.css")
        .with_style("styles/components.css")
        .compile();
}
```

## 7.5.3 组件样式

### 命名约定

使用 BEM（Block Element Modifier）命名法避免冲突：

```css
/* src/styles/components/button.css */

/* Block */
.button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-unit) calc(var(--spacing-unit) * 2);
    border: 1px solid transparent;
    border-radius: var(--border-radius);
    font-size: 14px;
    cursor: pointer;
    transition: background-color 0.2s;
}

/* Modifier */
.button--primary {
    background: var(--color-primary);
    color: white;
}

.button--secondary {
    background: var(--color-secondary);
    color: white;
}

.button--danger {
    background: var(--color-danger);
    color: white;
}

.button--outline {
    background: transparent;
    border-color: var(--color-border);
    color: var(--color-text);
}

/* Size modifiers */
.button--sm {
    padding: calc(var(--spacing-unit) / 2) var(--spacing-unit);
    font-size: 12px;
}

.button--lg {
    padding: calc(var(--spacing-unit) * 1.5) calc(var(--spacing-unit) * 3);
    font-size: 16px;
}

/* State */
.button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
}
```

### 使用组件样式

```html
<button class="button button--primary button--lg">主要按钮</button>
<button class="button button--secondary">次要按钮</button>
<button class="button button--danger button--sm">删除</button>
<button class="button button--outline" disabled>禁用</button>
```

## 7.5.4 工具类

工具类是单一用途的 CSS 类，提供常用样式：

### 布局工具类

```css
/* src/styles/utilities.css */

/* Display */
.flex { display: flex; }
.inline-flex { display: inline-flex; }
.block { display: block; }
.inline-block { display: inline-block; }
.hidden { display: none; }

/* Flex direction */
.flex-row { flex-direction: row; }
.flex-col { flex-direction: column; }
.flex-row-reverse { flex-direction: row-reverse; }
.flex-col-reverse { flex-direction: column-reverse; }

/* Justify content */
.justify-start { justify-content: flex-start; }
.justify-end { justify-content: flex-end; }
.justify-center { justify-content: center; }
.justify-between { justify-content: space-between; }
.justify-around { justify-content: space-around; }

/* Align items */
.items-start { align-items: flex-start; }
.items-end { align-items: flex-end; }
.items-center { align-items: center; }
.items-stretch { align-items: stretch; }

/* Flex wrap */
.flex-wrap { flex-wrap: wrap; }
.flex-nowrap { flex-wrap: nowrap; }

/* Flex */
.flex-1 { flex: 1; }
.flex-auto { flex: auto; }
.flex-none { flex: none; }
.flex-grow { flex-grow: 1; }
.flex-shrink-0 { flex-shrink: 0; }
```

### 间距工具类

```css
/* Padding */
.p-0 { padding: 0; }
.p-1 { padding: 4px; }
.p-2 { padding: 8px; }
.p-3 { padding: 12px; }
.p-4 { padding: 16px; }
.p-6 { padding: 24px; }
.p-8 { padding: 32px; }

.px-0 { padding-left: 0; padding-right: 0; }
.px-2 { padding-left: 8px; padding-right: 8px; }
.px-4 { padding-left: 16px; padding-right: 16px; }

.py-0 { padding-top: 0; padding-bottom: 0; }
.py-2 { padding-top: 8px; padding-bottom: 8px; }
.py-4 { padding-top: 16px; padding-bottom: 16px; }

/* Margin */
.m-0 { margin: 0; }
.m-2 { margin: 8px; }
.m-4 { margin: 16px; }
.m-auto { margin: auto; }

.mx-auto { margin-left: auto; margin-right: auto; }
.my-4 { margin-top: 16px; margin-bottom: 16px; }

/* Gap */
.gap-0 { gap: 0; }
.gap-1 { gap: 4px; }
.gap-2 { gap: 8px; }
.gap-4 { gap: 16px; }
.gap-6 { gap: 24px; }
```

### 文本工具类

```css
/* Text align */
.text-left { text-align: left; }
.text-center { text-align: center; }
.text-right { text-align: right; }

/* Font size */
.text-xs { font-size: 12px; }
.text-sm { font-size: 14px; }
.text-base { font-size: 16px; }
.text-lg { font-size: 18px; }
.text-xl { font-size: 20px; }
.text-2xl { font-size: 24px; }

/* Font weight */
.font-normal { font-weight: normal; }
.font-medium { font-weight: 500; }
.font-bold { font-weight: bold; }

/* Color */
.text-primary { color: var(--color-primary); }
.text-danger { color: var(--color-danger); }
.text-success { color: var(--color-success); }
.text-muted { color: var(--color-text-muted); }

/* Text decoration */
.no-underline { text-decoration: none; }
.underline { text-decoration: underline; }
.line-through { text-decoration: line-through; }
```

### 背景工具类

```css
.bg-primary { background: var(--color-primary); }
.bg-secondary { background: var(--color-secondary); }
.bg-success { background: var(--color-success); }
.bg-danger { background: var(--color-danger); }
.bg-white { background: white; }
.bg-transparent { background: transparent; }
```

### 边框工具类

```css
.border { border: 1px solid var(--color-border); }
.border-0 { border: none; }
.border-top { border-top: 1px solid var(--color-border); }
.border-bottom { border-bottom: 1px solid var(--color-border); }

.rounded { border-radius: var(--border-radius); }
.rounded-lg { border-radius: 8px; }
.rounded-full { border-radius: 9999px; }
```

### 使用工具类

```html
<div class="flex items-center justify-between p-4 bg-white rounded shadow">
    <h1 class="text-xl font-bold text-primary">标题</h1>
    <button class="button button--primary">按钮</button>
</div>
```

## 7.5.5 CSS 模块

CSS 模块提供作用域隔离，避免样式冲突：

### 定义 CSS 模块

```css
/* src/components/button/button.module.css */
.button {
    display: inline-flex;
    align-items: center;
    padding: 8px 16px;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
}

.primary {
    background: #007bff;
    color: white;
}

.large {
    padding: 12px 24px;
    font-size: 16px;
}
```

### 在组件中使用

```rust
// src/components/button/button.rml.rs
use rml::prelude::*;

#[component]
pub struct Button {
    pub text: SharedString,
    pub variant: SharedString,
    pub size: SharedString,
}
```

```html
<!-- src/components/button/button.rml -->
<button class="button {variant} {size}">
    {text}
</button>
```

### 在父视图中使用

```html
<div>
    <Button text="提交" variant="primary" size="large" />
    <Button text="取消" variant="secondary" size="default" />
</div>
```

## 7.5.6 样式复用的策略

### 策略一：工具类优先

```html
<!-- ✅ 工具类组合 -->
<div class="flex items-center gap-4 p-4 bg-white rounded shadow">
    <img src={avatar} class="w-12 h-12 rounded-full" />
    <div class="flex-1">
        <h3 class="text-lg font-bold">{name}</h3>
        <p class="text-sm text-muted">{email}</p>
    </div>
</div>

<!-- ❌ 自定义类名 -->
<div class="user-card">
    <img src={avatar} class="user-avatar" />
    <div class="user-info">
        <h3 class="user-name">{name}</h3>
        <p class="user-email">{email}</p>
    </div>
</div>
```

### 策略二：组件样式用于复杂组件

```css
/* 复杂组件用 BEM 命名 */
.user-card {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 16px;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.user-card__avatar {
    width: 48px;
    height: 48px;
    border-radius: 50%;
}

.user-card__name {
    font-size: 18px;
    font-weight: bold;
}

.user-card__email {
    font-size: 14px;
    color: #6c757d;
}

.user-card--highlighted {
    border: 2px solid #007bff;
}
```

### 策略三：CSS 变量实现主题化

```css
/* 用 CSS 变量定义主题 */
:root {
    --user-card-bg: white;
    --user-card-text: #333;
    --user-card-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.user-card {
    background: var(--user-card-bg);
    color: var(--user-card-text);
    box-shadow: var(--user-card-shadow);
}
```

## 7.5.7 样式复用的最佳实践

### 1. DRY 原则

```css
/* ✅ 提取公共样式 */
.button-base {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 8px 16px;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
}

.button-primary {
    /* 继承 button-base 的样式 */
    background: #007bff;
    color: white;
}

/* ❌ 重复样式 */
.button-primary {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: 8px 16px;
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    background: #007bff;
    color: white;
}
```

### 2. 单一职责

```css
/* ✅ 单一职责 */
.text-center { text-align: center; }
.text-red { color: red; }
.p-4 { padding: 16px; }

/* ❌ 混合职责 */
.centered-red-box {
    text-align: center;
    color: red;
    padding: 16px;
}
```

### 3. 命名清晰

```css
/* ✅ 清晰命名 */
.button-primary { ... }
.card-header { ... }
.text-muted { ... }

/* ❌ 模糊命名 */
.style1 { ... }
.div1 { ... }
.red { ... }
```

### 4. 避免过度嵌套

```css
/* ✅ 扁平结构 */
.card { ... }
.card-header { ... }
.card-body { ... }

/* ❌ 过度嵌套 */
.card .header .title .text { ... }
```

## 7.5.8 完整示例：样式系统组织

```
src/
├── styles/
│   ├── reset.css           # 重置样式
│   ├── variables.css       # CSS 变量
│   ├── utilities.css       # 工具类
│   ├── components/
│   │   ├── button.css      # 按钮样式
│   │   ├── card.css        # 卡片样式
│   │   ├── form.css        # 表单样式
│   │   ├── table.css       # 表格样式
│   │   └── modal.css       # 模态框样式
│   └── themes/
│       ├── light.css       # 亮色主题
│       └── dark.css        # 暗色主题
├── components/
│   ├── button/
│   │   ├── button.rml
│   │   └── button.rml.rs
│   └── card/
│       ├── card.rml
│       └── card.rml.rs
└── views/
    ├── home/
    │   ├── home.rml
    │   └── home.rml.rs
    └── user/
        ├── user.rml
        └── user.rml.rs
```

```rust
// build.rs
fn main() {
    rml::compile_rml()
        // 基础样式
        .with_style("styles/reset.css")
        .with_style("styles/variables.css")
        // 主题
        .with_style("styles/themes/light.css")
        .with_style("styles/themes/dark.css")
        // 工具类
        .with_style("styles/utilities.css")
        // 组件样式
        .with_style("styles/components/button.css")
        .with_style("styles/components/card.css")
        .with_style("styles/components/form.css")
        .with_style("styles/components/table.css")
        .with_style("styles/components/modal.css")
        // 编译
        .compile();
}
```

## 7.5.9 小结

RML 的样式复用策略：

- **全局样式**：重置样式、全局变量
- **组件样式**：BEM 命名，避免冲突
- **工具类**：单一职责，灵活组合
- **CSS 模块**：作用域隔离
- **CSS 变量**：主题化

最佳实践：

- 工具类优先，组件样式用于复杂组件
- DRY 原则，提取公共样式
- 单一职责，清晰命名
- 避免过度嵌套

下一章 → [第 8 章 · 生命周期管理](../08-lifecycle/INDEX.md)
