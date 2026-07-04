# 06 CSS 定制

## 支持的选择器

RML 支持 CSS 子集，覆盖常用选择器：

| 选择器 | 语法 | 示例 | 匹配 |
|--------|------|------|------|
| Universal | `*` | `* { margin: 0 }` | 所有元素 |
| Tag | `div` | `div { padding: 8px }` | tag 名匹配 |
| Class | `.card` | `.card { bg: #fff }` | class 属性包含 |
| Id | `#main` | `#main { bg: #fff }` | id 属性匹配 |
| Compound | `.btn.primary` | `.btn.primary { bg: blue }` | 同时匹配多个 class |
| Descendant | `.container .title` | `.container .title { font-size: 24px }` | 祖先后代（任意层级） |
| Child | `.list > .item` | `.list > .item { padding: 4px }` | 直接父子 |

## 父链匹配语义

`css/matcher.rs` 通过 `ElementContext.parents` 字段实现完整父链匹配：

```rust
pub struct ElementContext<'a> {
    pub tag: &'a str,
    pub classes: Vec<&'a str>,
    pub id: Option<&'a str>,
    pub parents: Vec<ParentInfo<'a>>,  // 从根到当前元素的父链
}

pub struct ParentInfo<'a> {
    pub tag: &'a str,
    pub classes: Vec<&'a str>,
    pub id: Option<&'a str>,
}
```

### 匹配规则

- **Descendant** (`A B`): 当前元素匹配 `B`，且父链中**任意**元素匹配 `A`
- **Child** (`A > B`): 当前元素匹配 `B`，且**直接父元素**匹配 `A`

### 父链构建

codegen 在调用 `generate_styles` 时构建父链，从 codegen 上下文传递 `parents: Vec<ParentInfo>`。

## 支持的 CSS 属性

### 颜色

| 属性 | 示例 | 生成 |
|------|------|------|
| `background` / `bg` | `bg: #fff` | `.bg(gpui::rgb(0xffffff))` |
| `color` | `color: red` | `.text_color(gpui::rgb(0xff0000))` |

### 长度

| 属性 | 示例 | 生成 |
|------|------|------|
| `padding` / `p` | `p: 10px` | `.p(gpui::px(10.))` |
| `margin` / `m` | `m: 8px` | `.m(gpui::px(8.))` |
| `width` / `w` | `w: 100px` | `.w(gpui::px(100.))` |
| `height` / `h` | `h: 50px` | `.h(gpui::px(50.))` |

### 简写

| 属性 | 示例 | 生成 |
|------|------|------|
| `padding: 10px 20px` | | `.p_y(gpui::px(10.)).p_x(gpui::px(20.))` |
| `margin: 0 auto` | | `.m_y(gpui::px(0.)).m_x(gpui::px(auto))` |

### 字体

| 属性 | 示例 | 生成 |
|------|------|------|
| `font-size` | `font-size: 14px` | `.text_size(gpui::px(14.))` |
| `font-weight` | `font-weight: bold` | `.font_bold()` |

### 布局

| 属性 | 示例 | 生成 |
|------|------|------|
| `display: flex` | | `.flex()` |
| `flex-direction: column` | | `.flex_col()` |

## 主题变量

`var(--name)` 引用主题变量：

```css
.btn {
    background: var(--primary);
}
```

**生成**：颜色属性的 `var()` 生成运行时主题查询 `.bg(rml::theme::color("--primary"))`，不构建期内联。

## 样式优先级

```
全局样式表 < class 属性 < inline style
```

- **全局样式表**：`matcher.rs` 收集所有匹配规则的声明
- **class 属性**：同上，通过 `ElementContext.classes` 匹配
- **inline style**：codegen 直接处理（如 `<div style="p:10px">`）

**覆盖规则**：后出现的同名属性覆盖前者（按规则出现顺序）。

## 限制

RML CSS 子集**不实现**以下特性：

- 完整 Cascade 算法（ specificity 计算）
- `!important` 优先级
- 伪类（`:hover` / `:active` / `:focus`）
- 伪元素（`::before` / `::after`）
- 媒体查询（`@media`）
- 动画（`@keyframes` / `transition`）
- 兄弟选择器（`+` / `~`）
- 属性选择器（`[attr=value]`）

**设计理由**：GPUI 是即时模式 GUI，不支持完整的 CSS Cascade。RML CSS 旨在提供声明式样式定制能力，而非完整 CSS 实现。
