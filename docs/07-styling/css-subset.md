# 7.2 CSS 子集与扩展

> **本节目标**：完整掌握 RML 支持的 CSS 属性、单位、选择器和函数。

## 7.2.1 支持的 CSS 属性

### 盒模型

| 属性            | 说明           | 示例                              |
| ------------- | ------------ | ------------------------------- |
| `width`       | 宽度           | `width: 200px;`                 |
| `height`      | 高度           | `height: 100px;`                |
| `padding`     | 内边距          | `padding: 10px;`                |
| `padding-top` | 上内边距         | `padding-top: 10px;`            |
| `margin`      | 外边距          | `margin: 10px;`                 |
| `margin-top`  | 上外边距         | `margin-top: 10px;`             |
| `border`      | 边框           | `border: 1px solid #ccc;`       |
| `border-radius` | 圆角           | `border-radius: 4px;`           |
| `box-sizing`  | 盒模型计算方式      | `box-sizing: border-box;`       |

### 背景

| 属性                  | 说明           | 示例                              |
| ------------------- | ------------ | ------------------------------- |
| `background`        | 背景（简写）       | `background: #f5f5f5;`          |
| `background-color`  | 背景色          | `background-color: red;`        |
| `background-image`  | 背景图          | `background-image: url('bg.png');` |
| `background-size`   | 背景大小         | `background-size: cover;`       |
| `background-position` | 背景位置         | `background-position: center;`  |

### 文本

| 属性                | 说明           | 示例                              |
| ----------------- | ------------ | ------------------------------- |
| `color`           | 文字颜色         | `color: #333;`                  |
| `font-size`       | 字体大小         | `font-size: 14px;`              |
| `font-weight`     | 字体粗细         | `font-weight: bold;`            |
| `font-family`     | 字体族          | `font-family: 'Arial';`         |
| `text-align`      | 文本对齐         | `text-align: center;`           |
| `text-decoration` | 文本装饰         | `text-decoration: underline;`   |
| `line-height`     | 行高           | `line-height: 1.5;`             |
| `letter-spacing`  | 字间距          | `letter-spacing: 0.5px;`        |

### Flexbox 布局

| 属性                | 说明           | 示例                              |
| ----------------- | ------------ | ------------------------------- |
| `display`         | 显示类型         | `display: flex;`                |
| `flex-direction`  | 主轴方向         | `flex-direction: column;`       |
| `justify-content` | 主轴对齐         | `justify-content: center;`      |
| `align-items`     | 交叉轴对齐        | `align-items: center;`          |
| `flex-wrap`       | 换行           | `flex-wrap: wrap;`              |
| `flex`            | flex 简写      | `flex: 1;`                      |
| `gap`             | 间距           | `gap: 10px;`                    |

### 定位

| 属性        | 说明     | 示例                  |
| --------- | ------ | ------------------- |
| `position`| 定位类型   | `position: absolute;` |
| `top`     | 上偏移    | `top: 10px;`        |
| `right`   | 右偏移    | `right: 10px;`      |
| `bottom`  | 下偏移    | `bottom: 10px;`     |
| `left`    | 左偏移    | `left: 10px;`       |
| `z-index` | 层级     | `z-index: 100;`     |

### 视觉效果

| 属性                | 说明           | 示例                              |
| ----------------- | ------------ | ------------------------------- |
| `opacity`         | 透明度          | `opacity: 0.5;`                 |
| `box-shadow`      | 阴影           | `box-shadow: 0 2px 4px rgba(0,0,0,0.1);` |
| `overflow`        | 溢出处理         | `overflow: hidden;`             |

RML 元素还可直接写布尔属性 `overflow-y-auto=""`、`overflow-x-auto=""` 等（见 [layout.md](./layout.md#rml-布尔属性推荐)）。

| `cursor`          | 鼠标指针         | `cursor: pointer;`              |
| `visibility`      | 可见性          | `visibility: hidden;`           |

## 7.2.2 支持的单位

### 长度单位

| 单位    | 说明           | 示例              |
| ----- | ------------ | --------------- |
| `px`  | 像素（主要单位）     | `width: 100px;` |
| `pt`  | 点（1pt = 1.333px） | `font-size: 12pt;` |
| `em`  | 相对父元素字体大小    | `font-size: 1.2em;` |
| `rem` | 相对根元素字体大小    | `font-size: 1.2rem;` |
| `%`   | 百分比          | `width: 50%;`   |
| `vw`  | 视口宽度百分比      | `width: 50vw;`  |
| `vh`  | 视口高度百分比      | `height: 50vh;` |

### 颜色单位

| 格式            | 示例                              |
| ------------- | ------------------------------- |
| 十六进制          | `#ff0000`、`#f00`                |
| RGB           | `rgb(255, 0, 0)`                |
| RGBA          | `rgba(255, 0, 0, 0.5)`          |
| 颜色名           | `red`、`blue`、`transparent`      |
| CSS 变量        | `var(--primary-color)`          |

### 角度单位

| 单位    | 说明     | 示例              |
| ----- | ------ | --------------- |
| `deg` | 度      | `transform: rotate(45deg);` |

### 时间单位

| 单位    | 说明     | 示例              |
| ----- | ------ | --------------- |
| `s`   | 秒      | `transition: 0.3s;` |
| `ms`  | 毫秒     | `transition: 300ms;` |

## 7.2.3 支持的选择器

### 基础选择器

```css
/* 标签选择器 */
div {
    padding: 10px;
}

/* 类选择器 */
.container {
    max-width: 1200px;
}

/* ID 选择器 */
#main-content {
    background: white;
}
```

### 组合选择器

```css
/* 后代选择器 */
.container .title {
    font-size: 24px;
}

/* 子选择器 */
.list > .item {
    border-bottom: 1px solid #ccc;
}

/* 相邻兄弟选择器 */
.item + .item {
    margin-top: 10px;
}
```

### 多重选择器

```css
/* 分组选择器 */
h1, h2, h3 {
    font-weight: bold;
}

/* 交集选择器 */
.button.primary {
    background: blue;
    color: white;
}
```

### 属性选择器

```css
/* 有 type 属性的元素 */
[type] {
    padding: 4px;
}

/* type="text" 的元素 */
[type="text"] {
    border: 1px solid #ccc;
}
```

### 伪类（部分支持）

```css
/* 鼠标悬停（通过事件实现） */
.button:hover {
    background: darker(blue);
}

/* 获得焦点 */
input:focus {
    border-color: blue;
}
```

⚠️ **注意**：RML 的伪类通过事件绑定实现，详见 [7.1.6 样式的限制](./styling-overview.md#716-样式的限制)。

## 7.2.4 支持的函数

### 颜色函数

```css
/* rgba() */
background: rgba(255, 0, 0, 0.5);

/* hsl() */
color: hsl(120, 100%, 50%);

/* var() - CSS 变量 */
color: var(--text-color);
background: var(--primary-color, #007bff);  /* 带默认值 */
```

### 计算函数

```css
/* calc() - 部分支持 */
width: calc(100% - 20px);
height: calc(100vh - 60px);
```

### 渐变函数

```css
/* 线性渐变 */
background: linear-gradient(to right, red, blue);
background: linear-gradient(45deg, #ff0000, #0000ff);

/* 径向渐变 */
background: radial-gradient(circle, red, blue);
```

## 7.2.5 CSS 变量

### 定义变量

```css
:root {
    --primary-color: #007bff;
    --secondary-color: #6c757d;
    --text-color: #333;
    --bg-color: #fff;
    --border-color: #ddd;
    --font-size-base: 14px;
    --spacing-unit: 8px;
}
```

### 使用变量

```css
.button {
    background: var(--primary-color);
    color: white;
    padding: var(--spacing-unit) calc(var(--spacing-unit) * 2);
    font-size: var(--font-size-base);
    border: 1px solid var(--border-color);
    border-radius: 4px;
}
```

### 变量的作用域

```css
:root {
    --color: blue;  /* 全局 */
}

.card {
    --color: red;   /* 局部，只在 .card 内有效 */
    background: var(--color);
}
```

### 变量的动态修改

通过 JavaScript... 不，通过 RML 的主题系统修改变量，详见 [7.4 主题与皮肤](./theming.md)。

## 7.2.6 样式的简写

### 盒模型简写

```css
/* padding 简写 */
padding: 10px;              /* 上下左右 */
padding: 10px 20px;         /* 上下 左右 */
padding: 10px 20px 30px;    /* 上 左右 下 */
padding: 10px 20px 30px 40px; /* 上 右 下 左 */

/* margin 简写 */
margin: 10px 20px;

/* border 简写 */
border: 1px solid #ccc;
border-top: 2px dashed red;
```

### 背景简写

```css
/* background 简写 */
background: #f5f5f5 url('bg.png') no-repeat center/cover;

/* 等价于 */
background-color: #f5f5f5;
background-image: url('bg.png');
background-repeat: no-repeat;
background-position: center;
background-size: cover;
```

### 字体简写

```css
/* font 简写 */
font: bold 14px/1.5 'Arial', sans-serif;

/* 等价于 */
font-weight: bold;
font-size: 14px;
line-height: 1.5;
font-family: 'Arial', sans-serif;
```

## 7.2.7 完整示例

```css
/* src/styles/main.css */

/* 重置样式 */
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
    background: #f5f5f5;
}

/* 主题变量 */
:root {
    --primary-color: #007bff;
    --primary-hover: #0056b3;
    --danger-color: #dc3545;
    --success-color: #28a745;
    --text-color: #333;
    --text-muted: #6c757d;
    --bg-color: #fff;
    --border-color: #ddd;
    --spacing-unit: 8px;
    --border-radius: 4px;
}

/* 通用样式 */
.container {
    max-width: 1200px;
    margin: 0 auto;
    padding: calc(var(--spacing-unit) * 3);
}

/* 卡片样式 */
.card {
    background: var(--bg-color);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    padding: calc(var(--spacing-unit) * 2);
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.card-header {
    border-bottom: 1px solid var(--border-color);
    padding-bottom: var(--spacing-unit);
    margin-bottom: calc(var(--spacing-unit) * 2);
}

.card-title {
    font-size: 18px;
    font-weight: bold;
    color: var(--text-color);
}

.card-body {
    color: var(--text-color);
}

/* 按钮样式 */
.btn {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    padding: var(--spacing-unit) calc(var(--spacing-unit) * 2);
    border: 1px solid transparent;
    border-radius: var(--border-radius);
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: background-color 0.2s;
}

.btn-primary {
    background: var(--primary-color);
    color: white;
}

.btn-primary:hover {
    background: var(--primary-hover);
}

.btn-danger {
    background: var(--danger-color);
    color: white;
}

.btn-success {
    background: var(--success-color);
    color: white;
}

.btn-outline {
    background: transparent;
    border-color: var(--border-color);
    color: var(--text-color);
}

/* 表单样式 */
.form-group {
    margin-bottom: calc(var(--spacing-unit) * 2);
}

.form-label {
    display: block;
    margin-bottom: var(--spacing-unit);
    font-weight: 500;
    color: var(--text-color);
}

.form-input {
    width: 100%;
    padding: var(--spacing-unit);
    border: 1px solid var(--border-color);
    border-radius: var(--border-radius);
    font-size: 14px;
    color: var(--text-color);
    background: var(--bg-color);
}

.form-input:focus {
    border-color: var(--primary-color);
    outline: none;
    box-shadow: 0 0 0 2px rgba(0, 123, 255, 0.25);
}

/* 列表样式 */
.list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.list-item {
    padding: var(--spacing-unit);
    border-bottom: 1px solid var(--border-color);
}

.list-item:last-child {
    border-bottom: none;
}

/* 工具类 */
.flex {
    display: flex;
}

.flex-col {
    flex-direction: column;
}

.items-center {
    align-items: center;
}

.justify-center {
    justify-content: center;
}

.justify-between {
    justify-content: space-between;
}

.gap-2 {
    gap: calc(var(--spacing-unit) / 2);
}

.gap-4 {
    gap: var(--spacing-unit);
}

.p-4 {
    padding: var(--spacing-unit);
}

.text-center {
    text-align: center;
}

.text-muted {
    color: var(--text-muted);
}
```

## 7.2.8 小结

RML 支持的 CSS 子集：

- **属性**：盒模型、背景、文本、Flexbox、定位、视觉效果
- **单位**：px、pt、em、rem、%、vw、vh、颜色、角度、时间
- **选择器**：标签、类、ID、后代、子、相邻兄弟、分组、交集、属性、伪类（部分）
- **函数**：rgba、hsl、var、calc、linear-gradient、radial-gradient
- **变量**：`:root` 定义，`var()` 使用
- **简写**：padding、margin、border、background、font

下一节 → [7.3 布局系统](./layout.md)
