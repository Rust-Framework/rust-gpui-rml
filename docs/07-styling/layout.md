# 7.3 布局系统

> **本节目标**：掌握 RML 的布局系统——Flexbox 布局、定位、对齐、间距。

## 7.3.1 布局的核心：Flexbox

RML 主推 Flexbox 布局，简化了布局的复杂性：

```
┌─────────────────────────────────────────┐
│              Flex 容器                   │
│  ┌──────┐ ┌──────┐ ┌──────┐ ┌──────┐   │
│  │ 子元素 │ │ 子元素 │ │ 子元素 │ │ 子元素 │   │  ← 主轴（main axis）
│  └──────┘ └──────┘ └──────┘ └──────┘   │
│                                         │
│  ← 交叉轴（cross axis）→                 │
└─────────────────────────────────────────┘
```

## 7.3.2 Flex 容器

### 创建 Flex 容器

```css
.container {
    display: flex;  /* 水平排列 */
}
```

```html
<div class="container">
    <div>子元素 1</div>
    <div>子元素 2</div>
    <div>子元素 3</div>
</div>
```

### flex-direction：主轴方向

```css
/* 水平排列（默认） */
.container { flex-direction: row; }

/* 水平反向排列 */
.container { flex-direction: row-reverse; }

/* 垂直排列 */
.container { flex-direction: column; }

/* 垂直反向排列 */
.container { flex-direction: column-reverse; }
```

```
row:                    column:
┌──┬──┬──┐              ┌──┐
│1 │2 │3 │              │1 │
└──┴──┴──┘              ├──┤
                        │2 │
                        ├──┤
                        │3 │
                        └──┘
```

### justify-content：主轴对齐

```css
/* 起点对齐 */
.container { justify-content: flex-start; }

/* 终点对齐 */
.container { justify-content: flex-end; }

/* 居中对齐 */
.container { justify-content: center; }

/* 两端对齐，子元素间距相等 */
.container { justify-content: space-between; }

/* 均匀分布，子元素周围间距相等 */
.container { justify-content: space-around; }

/* 均匀分布，子元素之间和两端间距相等 */
.container { justify-content: space-evenly; }
```

```
flex-start:     |123          |
flex-end:       |          123|
center:         |    123      |
space-between:  |1   2   3    |
space-around:   | 1   2   3   |
space-evenly:   |  1   2   3  |
```

### align-items：交叉轴对齐

```css
/* 拉伸填满（默认） */
.container { align-items: stretch; }

/* 起点对齐 */
.container { align-items: flex-start; }

/* 终点对齐 */
.container { align-items: flex-end; }

/* 居中对齐 */
.container { align-items: center; }

/* 基线对齐 */
.container { align-items: baseline; }
```

### flex-wrap：换行

```css
/* 不换行（默认） */
.container { flex-wrap: nowrap; }

/* 换行 */
.container { flex-wrap: wrap; }

/* 反向换行 */
.container { flex-wrap: wrap-reverse; }
```

### align-content：多行对齐

```css
.container {
    flex-wrap: wrap;
    align-content: flex-start;  /* 多行起点对齐 */
    align-content: center;      /* 多行居中 */
    align-content: space-between; /* 多行两端对齐 */
}
```

### gap：间距

```css
.container {
    display: flex;
    gap: 10px;              /* 行和列间距都是 10px */
    gap: 10px 20px;         /* 行间距 10px，列间距 20px */
    row-gap: 10px;          /* 行间距 */
    column-gap: 20px;       /* 列间距 */
}
```

## 7.3.3 Flex 子元素

### flex-grow：放大比例

```css
.item {
    flex-grow: 1;  /* 占据剩余空间 */
}

.item-1 { flex-grow: 1; }
.item-2 { flex-grow: 2; }  /* 占据 item-1 的两倍空间 */
```

### flex-shrink：缩小比例

```css
.item {
    flex-shrink: 0;  /* 不缩小 */
}
```

### flex-basis：初始大小

```css
.item {
    flex-basis: 200px;  /* 初始大小 200px */
    flex-basis: auto;   /* 根据内容计算 */
}
```

### flex 简写

```css
.item {
    flex: 1;              /* flex-grow: 1, flex-shrink: 1, flex-basis: 0% */
    flex: 2;              /* flex-grow: 2 */
    flex: 1 0 200px;      /* flex-grow: 1, flex-shrink: 0, flex-basis: 200px */
    flex: auto;           /* flex: 1 1 auto */
    flex: none;           /* flex: 0 0 auto */
}
```

### align-self：单独对齐

```css
.item {
    align-self: center;  /* 单独居中对齐 */
    align-self: flex-start;
    align-self: flex-end;
}
```

### order：排序

```css
.item-1 { order: 2; }
.item-2 { order: 1; }  /* item-2 显示在 item-1 前面 */
```

## 7.3.4 常见布局模式

### 模式一：水平居中

```css
.container {
    display: flex;
    justify-content: center;  /* 水平居中 */
    align-items: center;      /* 垂直居中 */
}
```

```html
<div class="container" style="height: 200px;">
    <div>居中内容</div>
</div>
```

### 模式二：两栏布局

```css
.layout {
    display: flex;
    height: 100vh;
}

.sidebar {
    flex: 0 0 250px;  /* 固定宽度 250px */
    background: #f5f5f5;
}

.main {
    flex: 1;  /* 占据剩余空间 */
    padding: 20px;
}
```

```html
<div class="layout">
    <div class="sidebar">侧边栏</div>
    <div class="main">主内容</div>
</div>
```

### 模式三：三栏布局

```css
.layout {
    display: flex;
    height: 100vh;
}

.left {
    flex: 0 0 200px;
}

.center {
    flex: 1;
}

.right {
    flex: 0 0 250px;
}
```

### 模式四：垂直布局

```css
.vertical-layout {
    display: flex;
    flex-direction: column;
    height: 100vh;
}

.header {
    flex: 0 0 60px;  /* 固定高度 */
}

.content {
    flex: 1;  /* 占据剩余空间 */
    overflow: auto;
}

.footer {
    flex: 0 0 40px;
}
```

```html
<div class="vertical-layout">
    <div class="header">头部</div>
    <div class="content">内容</div>
    <div class="footer">页脚</div>
</div>
```

### 模式五：网格布局（用 Flexbox 模拟）

```css
.grid {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
}

.grid-item {
    flex: 0 0 calc(33.333% - 16px);  /* 3 列 */
    /* flex: 0 0 calc(25% - 16px); */ /* 4 列 */
    /* flex: 0 0 calc(50% - 16px); */ /* 2 列 */
}
```

### 模式六：圣杯布局

```css
.holy-grail {
    display: flex;
    flex-direction: column;
    min-height: 100vh;
}

.holy-grail-header,
.holy-grail-footer {
    flex: 0 0 auto;
}

.holy-grail-body {
    display: flex;
    flex: 1;
}

.holy-grail-content {
    flex: 1;
}

.holy-grail-nav,
.holy-grail-aside {
    flex: 0 0 200px;
}

.holy-grail-nav {
    order: -1;  /* 导航在左侧 */
}
```

```html
<div class="holy-grail">
    <div class="holy-grail-header">头部</div>
    <div class="holy-grail-body">
        <div class="holy-grail-content">主内容</div>
        <div class="holy-grail-nav">导航</div>
        <div class="holy-grail-aside">侧边栏</div>
    </div>
    <div class="holy-grail-footer">页脚</div>
</div>
```

## 7.3.5 定位

### position 属性

```css
/* 静态定位（默认） */
.element { position: static; }

/* 相对定位 */
.element { position: relative; }

/* 绝对定位 */
.element { position: absolute; }

/* 固定定位 */
.element { position: fixed; }

/* 粘性定位 */
.element { position: sticky; }
```

### 相对定位

```css
.element {
    position: relative;
    top: 10px;     /* 相对原位置向下 10px */
    left: 20px;    /* 相对原位置向右 20px */
}
```

### 绝对定位

```css
.parent {
    position: relative;  /* 父元素相对定位 */
    height: 200px;
}

.child {
    position: absolute;
    top: 50%;       /* 相对父元素顶部 50% */
    left: 50%;      /* 相对父元素左侧 50% */
    transform: translate(-50%, -50%);  /* 居中 */
}
```

### 固定定位

```css
.header {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 100;  /* 确保在最上层 */
}
```

### z-index：层级

```css
.modal-overlay {
    position: fixed;
    z-index: 1000;
}

.modal {
    position: fixed;
    z-index: 1001;  /* 比遮罩层高 */
}
```

## 7.3.6 溢出处理

### overflow 属性

```css
/* 可见（默认） */
.element { overflow: visible; }

/* 隐藏 */
.element { overflow: hidden; }

/* 滚动 */
.element { overflow: scroll; }

/* 自动滚动 */
.element { overflow: auto; }

/* 分别设置 x 和 y */
.element {
    overflow-x: hidden;
    overflow-y: auto;
}
```

### 文本溢出

```css
.text-ellipsis {
    white-space: nowrap;      /* 不换行 */
    overflow: hidden;         /* 隐藏溢出 */
    text-overflow: ellipsis;  /* 显示省略号 */
}
```

## 7.3.7 响应式布局

RML 支持简单的响应式布局，通过窗口尺寸调整：

### 媒体查询（部分支持）

```css
/* 默认样式（小窗口） */
.grid-item {
    flex: 0 0 100%;
}

/* 中等窗口 */
@media (min-width: 768px) {
    .grid-item {
        flex: 0 0 50%;
    }
}

/* 大窗口 */
@media (min-width: 1200px) {
    .grid-item {
        flex: 0 0 33.333%;
    }
}
```

### 通过数据绑定响应

```html
<div class="grid">
    <div
        each={item in items}
        class={is_compact ? "grid-item-compact" : "grid-item"}
    >
        {item.name}
    </div>
</div>
```

```rust
#[computed]
pub fn is_compact(&self) -> bool {
    self.window_width < 768.0
}
```

## 7.3.8 完整示例：仪表盘布局

```html
<!-- views/dashboard.rml -->
<div class="dashboard">
    <!-- 顶部导航 -->
    <header class="dashboard-header">
        <div class="header-left">
            <h1 class="header-title">仪表盘</h1>
        </div>
        <div class="header-right">
            <button class="btn-icon" on-click={toggle_notifications}>
                🔔
            </button>
            <Avatar src={user.avatar} />
        </div>
    </header>

    <!-- 主体内容 -->
    <div class="dashboard-body">
        <!-- 侧边栏 -->
        <aside class="dashboard-sidebar">
            <nav class="sidebar-nav">
                <ul class="nav-list">
                    <li class="nav-item active">
                        <a on-click={show_overview}>概览</a>
                    </li>
                    <li class="nav-item">
                        <a on-click={show_analytics}>分析</a>
                    </li>
                    <li class="nav-item">
                        <a on-click={show_settings}>设置</a>
                    </li>
                </ul>
            </nav>
        </aside>

        <!-- 主内容区 -->
        <main class="dashboard-main">
            <!-- 统计卡片 -->
            <div class="stats-grid">
                <div class="stat-card">
                    <div class="stat-icon">📊</div>
                    <div class="stat-info">
                        <div class="stat-label">总用户</div>
                        <div class="stat-value">{total_users}</div>
                    </div>
                </div>
                <div class="stat-card">
                    <div class="stat-icon">💰</div>
                    <div class="stat-info">
                        <div class="stat-label">总收入</div>
                        <div class="stat-value">{total_revenue}</div>
                    </div>
                </div>
                <div class="stat-card">
                    <div class="stat-icon">📦</div>
                    <div class="stat-info">
                        <div class="stat-label">订单数</div>
                        <div class="stat-value">{total_orders}</div>
                    </div>
                </div>
                <div class="stat-card">
                    <div class="stat-icon">📈</div>
                    <div class="stat-info">
                        <div class="stat-label">增长率</div>
                        <div class="stat-value">{growth_rate}%</div>
                    </div>
                </div>
            </div>

            <!-- 图表区域 -->
            <div class="chart-area">
                <div class="chart-card">
                    <h2 class="chart-title">销售趋势</h2>
                    <div class="chart-container">
                        <!-- 图表内容 -->
                    </div>
                </div>
            </div>

            <!-- 列表区域 -->
            <div class="list-area">
                <div class="list-card">
                    <h2 class="list-title">最近订单</h2>
                    <ul class="order-list">
                        <li each={order in recent_orders} key={order.id} class="order-item">
                            <span class="order-id">#{order.id}</span>
                            <span class="order-customer">{order.customer_name}</span>
                            <span class="order-amount">¥{order.amount}</span>
                            <span class="order-status {order.status_class}">{order.status}</span>
                        </li>
                    </ul>
                </div>
            </div>
        </main>
    </div>
</div>
```

```css
/* src/styles/dashboard.css */

.dashboard {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: #f5f5f5;
}

/* 顶部导航 */
.dashboard-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 24px;
    height: 60px;
    background: white;
    border-bottom: 1px solid #e0e0e0;
    flex-shrink: 0;
}

.header-title {
    font-size: 20px;
    font-weight: bold;
    color: #333;
}

.header-right {
    display: flex;
    align-items: center;
    gap: 16px;
}

/* 主体内容 */
.dashboard-body {
    display: flex;
    flex: 1;
    overflow: hidden;
}

/* 侧边栏 */
.dashboard-sidebar {
    flex: 0 0 240px;
    background: #2c3e50;
    color: white;
    overflow-y: auto;
}

.sidebar-nav {
    padding: 16px 0;
}

.nav-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.nav-item {
    padding: 0;
}

.nav-item a {
    display: block;
    padding: 12px 24px;
    color: #ecf0f1;
    text-decoration: none;
    cursor: pointer;
}

.nav-item.active a {
    background: #34495e;
    border-left: 3px solid #3498db;
}

.nav-item a:hover {
    background: #34495e;
}

/* 主内容区 */
.dashboard-main {
    flex: 1;
    padding: 24px;
    overflow-y: auto;
}

/* 统计卡片网格 */
.stats-grid {
    display: flex;
    flex-wrap: wrap;
    gap: 16px;
    margin-bottom: 24px;
}

.stat-card {
    flex: 1 1 calc(25% - 16px);
    min-width: 200px;
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 20px;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.stat-icon {
    font-size: 32px;
}

.stat-label {
    font-size: 14px;
    color: #666;
    margin-bottom: 4px;
}

.stat-value {
    font-size: 24px;
    font-weight: bold;
    color: #333;
}

/* 图表区域 */
.chart-area {
    margin-bottom: 24px;
}

.chart-card {
    padding: 20px;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.chart-title {
    font-size: 18px;
    font-weight: bold;
    margin-bottom: 16px;
    color: #333;
}

.chart-container {
    height: 300px;
    background: #f9f9f9;
    border-radius: 4px;
}

/* 列表区域 */
.list-card {
    padding: 20px;
    background: white;
    border-radius: 8px;
    box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
}

.list-title {
    font-size: 18px;
    font-weight: bold;
    margin-bottom: 16px;
    color: #333;
}

.order-list {
    list-style: none;
    padding: 0;
    margin: 0;
}

.order-item {
    display: flex;
    align-items: center;
    gap: 16px;
    padding: 12px 0;
    border-bottom: 1px solid #eee;
}

.order-item:last-child {
    border-bottom: none;
}

.order-id {
    flex: 0 0 80px;
    font-weight: bold;
    color: #333;
}

.order-customer {
    flex: 1;
    color: #666;
}

.order-amount {
    flex: 0 0 100px;
    text-align: right;
    font-weight: bold;
    color: #333;
}

.order-status {
    flex: 0 0 80px;
    text-align: center;
    padding: 4px 8px;
    border-radius: 4px;
    font-size: 12px;
}

.order-status.pending {
    background: #fff3cd;
    color: #856404;
}

.order-status.completed {
    background: #d4edda;
    color: #155724;
}

.order-status.cancelled {
    background: #f8d7da;
    color: #721c24;
}
```

## 7.3.9 小结

RML 的布局系统：

- **Flexbox**：主推布局方式，简化布局
- **容器属性**：`flex-direction`、`justify-content`、`align-items`、`flex-wrap`、`gap`
- **子元素属性**：`flex-grow`、`flex-shrink`、`flex-basis`、`order`、`align-self`
- **定位**：`position`、`top`、`right`、`bottom`、`left`、`z-index`
- **溢出**：`overflow`、`text-overflow`
- **响应式**：媒体查询（部分）、数据绑定

下一节 → [7.4 主题与皮肤](./theming.md)
