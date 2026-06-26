# 7.4 主题与皮肤

> **本节目标**：掌握 RML 的主题系统——CSS 变量、主题切换、暗色模式、自定义主题。

## 7.4.1 主题系统的设计

RML 通过 CSS 变量实现主题系统：

```
┌─────────────────────────────────────────┐
│              主题系统                     │
│                                         │
│  ┌─────────────┐  ┌─────────────┐       │
│  │  亮色主题    │  │  暗色主题    │       │
│  │  light.css  │  │  dark.css   │       │
│  └─────────────┘  └─────────────┘       │
│         │                 │             │
│         └────────┬────────┘             │
│                  ▼                      │
│         ┌────────────────┐              │
│         │  CSS 变量       │              │
│         │  var(--color)  │              │
│         └────────────────┘              │
│                  │                      │
│                  ▼                      │
│         ┌────────────────┐              │
│         │  组件样式       │              │
│         │  使用变量       │              │
│         └────────────────┘              │
└─────────────────────────────────────────┘
```

## 7.4.2 定义主题

### 主题变量

```css
/* src/styles/themes/light.css */
:root {
    /* 颜色 */
    --color-primary: #007bff;
    --color-primary-hover: #0056b3;
    --color-secondary: #6c757d;
    --color-success: #28a745;
    --color-danger: #dc3545;
    --color-warning: #ffc107;
    --color-info: #17a2b8;

    /* 文本颜色 */
    --color-text: #212529;
    --color-text-muted: #6c757d;
    --color-text-inverse: #ffffff;

    /* 背景颜色 */
    --color-bg: #ffffff;
    --color-bg-alt: #f8f9fa;
    --color-bg-inverse: #343a40;

    /* 边框颜色 */
    --color-border: #dee2e6;
    --color-border-light: #e9ecef;

    /* 字体 */
    --font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif;
    --font-size-base: 14px;
    --font-size-sm: 12px;
    --font-size-lg: 18px;
    --font-size-xl: 24px;

    /* 间距 */
    --spacing-unit: 8px;

    /* 圆角 */
    --border-radius: 4px;
    --border-radius-lg: 8px;

    /* 阴影 */
    --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.05);
    --shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
    --shadow-lg: 0 4px 8px rgba(0, 0, 0, 0.15);
}
```

```css
/* src/styles/themes/dark.css */
:root {
    /* 颜色 */
    --color-primary: #3d8bfd;
    --color-primary-hover: #5a9dfd;
    --color-secondary: #6c757d;
    --color-success: #2dd36f;
    --color-danger: #eb445a;
    --color-warning: #ffc409;
    --color-info: #00d4ff;

    /* 文本颜色 */
    --color-text: #f4f5f8;
    --color-text-muted: #989aa2;
    --color-text-inverse: #000000;

    /* 背景颜色 */
    --color-bg: #1a1b1e;
    --color-bg-alt: #25262b;
    --color-bg-inverse: #f4f5f8;

    /* 边框颜色 */
    --color-border: #343a40;
    --color-border-light: #2c2e33;

    /* 阴影 */
    --shadow-sm: 0 1px 2px rgba(0, 0, 0, 0.3);
    --shadow: 0 2px 4px rgba(0, 0, 0, 0.4);
    --shadow-lg: 0 4px 8px rgba(0, 0, 0, 0.5);
}
```

### 在 build.rs 中加载主题

```rust
// build.rs
fn main() {
    rml::compile_rml()
        .with_style("styles/themes/light.css")  // 默认主题
        .with_style("styles/themes/dark.css")   // 暗色主题
        .with_style("styles/main.css")          // 主样式
        .compile();
}
```

## 7.4.3 使用主题变量

### 在样式中使用

```css
/* src/styles/main.css */
.button {
    background: var(--color-primary);
    color: var(--color-text-inverse);
    padding: var(--spacing-unit) calc(var(--spacing-unit) * 2);
    border: 1px solid transparent;
    border-radius: var(--border-radius);
    font-size: var(--font-size-base);
    font-family: var(--font-family);
    cursor: pointer;
    box-shadow: var(--shadow-sm);
}

.button:hover {
    background: var(--color-primary-hover);
}

.card {
    background: var(--color-bg);
    border: 1px solid var(--color-border);
    border-radius: var(--border-radius-lg);
    padding: calc(var(--spacing-unit) * 2);
    box-shadow: var(--shadow);
}

.text-muted {
    color: var(--color-text-muted);
}
```

### 在内联样式中使用

```html
<div style="background: var(--color-bg); color: var(--color-text);">
    内容
</div>
```

## 7.4.4 主题切换

### 主题管理器

```rust
// src/theme.rs
use rml::prelude::*;
use std::sync::Arc;

#[derive(Clone, PartialEq)]
pub enum Theme {
    Light,
    Dark,
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
        }
    }

    pub fn toggle(&self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }
}

#[derive(Model)]
pub struct ThemeManager {
    pub current_theme: Theme,
}

impl ThemeManager {
    pub fn new() -> Self {
        Self {
            current_theme: Theme::Light,
        }
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut ViewContext<Self>) {
        if self.current_theme != theme {
            self.current_theme = theme.clone();
            cx.set_theme(theme.name());  // 切换主题
            cx.notify();
        }
    }

    pub fn toggle_theme(&mut self, cx: &mut ViewContext<Self>) {
        let new_theme = self.current_theme.toggle();
        self.set_theme(new_theme, cx);
    }
}
```

### 在应用中使用

```rust
// src/app.rml.rs
use rml::prelude::*;
use crate::theme::{ThemeManager, Theme};

#[derive(Model)]
#[component]
pub struct App {
    pub theme_manager: Entity<ThemeManager>,
}

impl App {
    pub fn new(cx: &mut AppContext) -> Self {
        Self {
            theme_manager: cx.new_model(|_| ThemeManager::new()),
        }
    }

    #[command]
    pub fn toggle_theme(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.theme_manager.update(cx, |manager, cx| {
            manager.toggle_theme(cx);
        });
    }
}
```

```html
<!-- src/app.rml -->
<div class="app">
    <header class="app-header">
        <h1>我的应用</h1>
        <button onclick={toggle_theme}>
            {theme_manager.current_theme == Theme::Light ? "🌙" : "☀️"}
        </button>
    </header>

    <main class="app-content">
        <!-- 内容 -->
    </main>
</div>
```

## 7.4.5 暗色模式

### 自动跟随系统

```rust
use rml::prelude::*;

impl ThemeManager {
    pub fn new(cx: &mut AppContext) -> Self {
        let system_theme = cx.system_theme();  // 获取系统主题
        Self {
            current_theme: system_theme,
        }
    }

    pub fn follow_system(&mut self, cx: &mut ViewContext<Self>) {
        cx.on_system_theme_change(|theme, cx| {
            cx.set_theme(theme.name());
            cx.notify();
        });
    }
}
```

### 手动切换

```rust
#[command]
pub fn toggle_theme(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    let new_theme = match self.current_theme {
        Theme::Light => Theme::Dark,
        Theme::Dark => Theme::Light,
    };
    self.set_theme(new_theme, cx);
}
```

### 主题持久化

```rust
impl ThemeManager {
    pub fn load_saved_theme(&mut self, cx: &mut ViewContext<Self>) {
        if let Some(saved) = cx.local_storage().get("theme") {
            let theme = match saved.as_str() {
                "dark" => Theme::Dark,
                _ => Theme::Light,
            };
            self.set_theme(theme, cx);
        }
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut ViewContext<Self>) {
        if self.current_theme != theme {
            self.current_theme = theme.clone();
            cx.set_theme(theme.name());
            cx.local_storage().set("theme", theme.name());  // 持久化
            cx.notify();
        }
    }
}
```

## 7.4.6 自定义主题

### 用户自定义颜色

```rust
#[derive(Model)]
pub struct ThemeManager {
    pub current_theme: Theme,
    pub custom_primary_color: Option<SharedString>,
}

impl ThemeManager {
    pub fn set_custom_primary_color(&mut self, color: SharedString, cx: &mut ViewContext<Self>) {
        self.custom_primary_color = Some(color.clone());
        cx.set_css_var("--color-primary", &color);
        cx.notify();
    }

    pub fn reset_primary_color(&mut self, cx: &mut ViewContext<Self>) {
        self.custom_primary_color = None;
        cx.remove_css_var("--color-primary");
        cx.notify();
    }
}
```

```html
<div class="theme-settings">
    <h2>主题设置</h2>

    <div class="setting-group">
        <label>主色调</label>
        <div class="color-options">
            <button
                class="color-option"
                style="background: #007bff;"
                onclick={set_primary_color, '#007bff'}
            ></button>
            <button
                class="color-option"
                style="background: #28a745;"
                onclick={set_primary_color, '#28a745'}
            ></button>
            <button
                class="color-option"
                style="background: #dc3545;"
                onclick={set_primary_color, '#dc3545'}
            ></button>
            <button
                class="color-option"
                style="background: #ffc107;"
                onclick={set_primary_color, '#ffc107'}
            ></button>
        </div>
    </div>

    <button onclick={reset_primary_color}>重置</button>
</div>
```

## 7.4.7 主题的作用域

### 全局主题

```rust
// 应用全局主题
cx.set_theme("dark");
```

### 局部主题

```css
/* 局部主题 */
.dark-section {
    --color-bg: #1a1b1e;
    --color-text: #f4f5f8;
    --color-border: #343a40;
}

.light-section {
    --color-bg: #ffffff;
    --color-text: #212529;
    --color-border: #dee2e6;
}
```

```html
<div class="container">
    <div class="dark-section">
        <!-- 这部分使用暗色主题 -->
        <p>暗色区域</p>
    </div>

    <div class="light-section">
        <!-- 这部分使用亮色主题 -->
        <p>亮色区域</p>
    </div>
</div>
```

## 7.4.8 主题的设计原则

### 1. 语义化命名

```css
/* ✅ 语义化命名 */
:root {
    --color-primary: #007bff;
    --color-text: #333;
    --color-bg: #fff;
}

/* ❌ 颜色值命名 */
:root {
    --blue: #007bff;
    --dark-gray: #333;
    --white: #fff;
}
```

### 2. 层次化设计

```css
:root {
    /* 基础颜色 */
    --color-primary: #007bff;
    --color-primary-hover: #0056b3;
    --color-primary-active: #004085;
    --color-primary-light: #e7f1ff;

    /* 文本颜色 */
    --color-text: #212529;
    --color-text-secondary: #6c757d;
    --color-text-disabled: #adb5bd;

    /* 背景颜色 */
    --color-bg: #ffffff;
    --color-bg-alt: #f8f9fa;
    --color-bg-disabled: #e9ecef;
}
```

### 3. 一致性

```css
:root {
    /* 间距统一使用 8px 倍数 */
    --spacing-xs: 4px;
    --spacing-sm: 8px;
    --spacing-md: 16px;
    --spacing-lg: 24px;
    --spacing-xl: 32px;

    /* 圆角统一 */
    --radius-sm: 2px;
    --radius-md: 4px;
    --radius-lg: 8px;
    --radius-full: 9999px;

    /* 字体大小统一 */
    --text-xs: 12px;
    --text-sm: 14px;
    --text-base: 16px;
    --text-lg: 18px;
    --text-xl: 20px;
    --text-2xl: 24px;
}
```

## 7.4.9 完整示例：主题系统

```rust
// src/theme.rs
use rml::prelude::*;

#[derive(Clone, PartialEq)]
pub enum Theme {
    Light,
    Dark,
    Auto,
}

impl Theme {
    pub fn name(&self) -> &'static str {
        match self {
            Theme::Light => "light",
            Theme::Dark => "dark",
            Theme::Auto => "auto",
        }
    }

    pub fn from_name(name: &str) -> Self {
        match name {
            "dark" => Theme::Dark,
            "auto" => Theme::Auto,
            _ => Theme::Light,
        }
    }
}

#[derive(Model)]
pub struct ThemeManager {
    pub current_theme: Theme,
    pub is_following_system: bool,
}

impl ThemeManager {
    pub fn new(cx: &mut AppContext) -> Self {
        let saved_theme = cx
            .local_storage()
            .get("theme")
            .map(|s| Theme::from_name(&s))
            .unwrap_or(Theme::Light);

        Self {
            current_theme: saved_theme.clone(),
            is_following_system: saved_theme == Theme::Auto,
        }
    }

    pub fn set_theme(&mut self, theme: Theme, cx: &mut ViewContext<Self>) {
        self.current_theme = theme.clone();
        self.is_following_system = theme == Theme::Auto;

        let actual_theme = if theme == Theme::Auto {
            Theme::from_name(cx.system_theme().name())
        } else {
            theme.clone()
        };

        cx.set_theme(actual_theme.name());
        cx.local_storage().set("theme", theme.name());
        cx.notify();
    }

    #[command]
    pub fn toggle_theme(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        let new_theme = match self.current_theme {
            Theme::Light => Theme::Dark,
            Theme::Dark | Theme::Auto => Theme::Light,
        };
        self.set_theme(new_theme, cx);
    }

    pub fn theme_icon(&self) -> &'static str {
        match self.current_theme {
            Theme::Light => "☀️",
            Theme::Dark => "🌙",
            Theme::Auto => "🖥️",
        }
    }

    pub fn theme_label(&self) -> &'static str {
        match self.current_theme {
            Theme::Light => "亮色",
            Theme::Dark => "暗色",
            Theme::Auto => "跟随系统",
        }
    }
}
```

```html
<!-- src/views/settings.rml -->
<div class="settings-page">
    <h1>设置</h1>

    <div class="setting-section">
        <h2>外观</h2>

        <div class="setting-item">
            <label>主题</label>
            <div class="theme-options">
                <button
                    class="theme-option {current_theme == Theme::Light ? 'active' : ''}"
                    onclick={set_theme, 'light'}
                >
                    ☀️ 亮色
                </button>
                <button
                    class="theme-option {current_theme == Theme::Dark ? 'active' : ''}"
                    onclick={set_theme, 'dark'}
                >
                    🌙 暗色
                </button>
                <button
                    class="theme-option {current_theme == Theme::Auto ? 'active' : ''}"
                    onclick={set_theme, 'auto'}
                >
                    🖥️ 跟随系统
                </button>
            </div>
        </div>
    </div>
</div>
```

```rust
#[command]
pub fn set_theme(&mut self, theme_name: SharedString, _: &ClickEvent, cx: &mut ViewContext<Self>) {
    let theme = Theme::from_name(&theme_name);
    self.theme_manager.update(cx, |manager, cx| {
        manager.set_theme(theme, cx);
    });
}
```

## 7.4.10 小结

RML 的主题系统：

- **CSS 变量**：通过 `var(--name)` 实现主题
- **主题切换**：`cx.set_theme(name)` 切换主题
- **暗色模式**：支持手动切换和跟随系统
- **持久化**：通过 `local_storage` 保存主题偏好
- **自定义**：运行时修改 CSS 变量
- **作用域**：全局主题和局部主题

下一节 → [7.5 样式复用](./style-reuse.md)
