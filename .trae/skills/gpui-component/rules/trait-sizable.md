---
title: Implement Sizable with Size Enum
impact: HIGH
tags: trait, sizable, size
---

## Sizable Trait

Use `Size` enum and `Sizable` trait for consistent component sizing.

### Size Enum

```rust
#[derive(Clone, Default, Copy, PartialEq, Eq)]
pub enum Size {
    Size(Pixels),  // Custom pixel size
    XSmall,
    Small,
    #[default]
    Medium,
    Large,
}

impl From<Pixels> for Size {
    fn from(pixels: Pixels) -> Self {
        Size::Size(pixels)
    }
}
```

### Sizable Trait

```rust
pub trait Sizable: Sized {
    fn with_size(self, size: impl Into<Size>) -> Self;

    fn xsmall(self) -> Self { self.with_size(Size::XSmall) }
    fn small(self) -> Self { self.with_size(Size::Small) }
    fn medium(self) -> Self { self.with_size(Size::Medium) }
    fn large(self) -> Self { self.with_size(Size::Large) }
}
```

### Implementation

```rust
#[derive(IntoElement)]
pub struct Button {
    size: Size,
    // ...
}

impl Sizable for Button {
    fn with_size(mut self, size: impl Into<Size>) -> Self {
        self.size = size.into();
        self
    }
}
```

### StyleSized Trait

Apply size-based styles to elements:

```rust
pub trait StyleSized<T: Styled> {
    fn input_text_size(self, size: Size) -> Self;
    fn input_size(self, size: Size) -> Self;  // px + py + h
    fn input_h(self, size: Size) -> Self;
    fn button_text_size(self, size: Size) -> Self;
}

impl<T: Styled> StyleSized<T> for T {
    fn input_h(self, size: Size) -> Self {
        match size {
            Size::Large => self.h_11(),
            Size::Medium => self.h_8(),
            Size::Small => self.h_6(),
            Size::XSmall => self.h_5(),
            Size::Size(px) => self.h(px),
        }
    }

    fn input_text_size(self, size: Size) -> Self {
        match size {
            Size::XSmall => self.text_xs(),
            Size::Small => self.text_sm(),
            Size::Medium => self.text_sm(),
            Size::Large => self.text_base(),
            Size::Size(size) => self.text_size(size * 0.875),
        }
    }
}
```

### Usage in Render

```rust
impl RenderOnce for Button {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        div()
            .input_size(self.size)      // Apply height + padding
            .input_text_size(self.size) // Apply text size
            .child(self.label)
    }
}
```

### Usage by Users

```rust
Button::new("btn").label("Click").small()
Button::new("btn").label("Click").large()
Button::new("btn").label("Click").with_size(px(48.))
// Medium is the default — no .with_size() call needed
Button::new("btn").label("Medium") // default
```

### RML Integration

RML 的 `size` 属性映射到 `Sizable::with_size()`。`Medium` 是 `Size` enum 的 `#[default]`，即组件原生默认，遵循原生写法不生成冗余调用：

| RML 写法 | 生成代码 | 说明 |
|----------|----------|------|
| `size="xsmall"` | `.with_size(rml_ui::Size::XSmall)` | 超小 |
| `size="small"` | `.with_size(rml_ui::Size::Small)` | 小 |
| `size="large"` | `.with_size(rml_ui::Size::Large)` | 大 |
| `size="medium"` / `size="default"` | **无调用** | 原生默认 |
| 不写 `size` | **无调用** | 同 medium/default |
| `size={field}` | `.with_size(self.field)` | 动态绑定（字段须为 `Size` 或 `Into<Size>`） |

`size` 位于 `COMMON_STATIC_PROPS`，对所有实现 `Sizable` 的组件生效。gpui-component 中实现 `Sizable` 且已在 RML 注册的组件包括：Button、ButtonGroup、Badge、Checkbox、Switch、Input、CodeEditor、Tag、Alert、Accordion、AccordionItem、Avatar、AvatarGroup、Icon、Spinner、Tab、Tabs、TabBar、Table、DescriptionList、Pagination、Radio、Progress、ProgressCircle。
