# 主题色完整应用实施计划

> 目标：在已完成的第一轮整改基础上，进一步消除 `cx.theme().tokens.*` 与硬编码颜色，确保内置暗/亮主题色在框架组件与 Demo 案例中和谐、完整、一致地应用。

---

## 一、当前状态

1. **主题同步已修复**：`crates/core/src/theme.rs` 的 `apply_dark_theme_config` / `apply_light_theme_config` 在修改 `Theme.colors` 后执行 `t.tokens = ThemeTokens::from(&*t)`，保证 `cx.theme().*` 与 `cx.theme().tokens.*` 同步。
2. **部分组件已修正**：
   - `ActivityBar` 背景由 `title_bar` 改为 `sidebar`。
   - `TabWindowShell` 标题栏/左/右/底面板已使用语义 token。
   - `Table` 表头/斑马纹已使用 `table_head` / `table_even`。
   - `Card` 悬浮阴影已使用 `theme.foreground` 派生。
3. **仍存在的两类问题**：
   - **框架组件风格不一致**：`tab.rs` / `tabs.rs` 仍使用 `cx.theme().tokens.*`，应统一为 `cx.theme().*`。
   - **Demo 中硬编码颜色**：`css_functions_case`、`css_priority_case`、`overflow_test_case`、`virtual_list_case` 等仍大量写死 hex/rgb，暗色模式下视觉断裂。

---

## 二、问题清单与根因

| 优先级 | 位置 | 问题 | 影响 |
|--------|------|------|------|
| P0 | `crates/ui/src/components/tab/tab.rs` | 多处 `cx.theme().tokens.*` | 风格不一致；依赖 tokens 同步，一旦同步遗漏即出错 |
| P0 | `crates/ui/src/components/tab/tabs.rs` | 多处 `cx.theme().tokens.*` | 同上 |
| P0 | `crates/core/src/theme.rs` | 未设置 `tab_bar_segmented` | 去掉 `tokens.tab_bar_segmented` 后可能失去语义色 |
| P1 | `demo/src/cases/css_functions_case.css` | 大量 hex/rgb 硬编码 | 暗色模式不协调 |
| P1 | `demo/src/cases/css_functions_case.rml` | 大量 hex/rgb 硬编码 | 暗色模式不协调 |
| P1 | `demo/src/cases/css_priority_case.css` | 多处 hex 硬编码 | 暗色模式不协调 |
| P1 | `demo/src/cases/css_priority_case.rml` | 多处 hex 硬编码 | 暗色模式不协调 |
| P2 | `demo/src/cases/overflow_test_case.rml` | `#e0e0e0` 边框 | 暗色模式下边框过亮 |
| P2 | `demo/src/cases/virtual_list_case.rml` | `#e8e8e8` 边框 | 暗色模式下边框过亮 |

---

## 三、实施方案

### 3.1 主题同步补全

**文件**：[`crates/core/src/theme.rs`](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/theme.rs)

在 `apply_dark_theme_config` 与 `apply_light_theme_config` 中补充 `tab_bar_segmented`，使其与 `tab_bar` 使用同一语义色：

```rust
// dark
t.tab_bar_segmented = gpui::rgb(0x1a1b1d).into();

// light
t.tab_bar_segmented = gpui::rgb(0xf3f4f6).into();
```

> 注：`tab_bar_segmented` 在 gpui-component 中 fallback 到 `secondary`，为保持 Tab 栏视觉统一，与 `tab_bar` 同色。

---

### 3.2 Tab 组件消除 `tokens.*`

#### 3.2.1 `crates/ui/src/components/tab/tab.rs`

将所有 `cx.theme().tokens.<X>` 替换为 `cx.theme().<X>`：

| 行号范围 | 当前代码 | 替换后 |
|----------|----------|--------|
| ~237 | `cx.theme().tokens.secondary_hover` | `cx.theme().secondary_hover` |
| ~244 | `cx.theme().tokens.secondary` | `cx.theme().secondary` |
| ~251, 304, 375 | `cx.theme().tokens.background` | `cx.theme().background` |
| ~275, 286, 326, 345 | `cx.theme().tokens.tab_active` | `cx.theme().tab_active` |
| ~298 | `cx.theme().tokens.primary` | `cx.theme().primary` |
| ~360, 365 | 已是 `cx.theme().primary_*` | 无需改动 |
| ~991 | `cx.theme().tokens.secondary_hover` | `cx.theme().secondary_hover` |

验证点：
- `TabStyle.bg` / `inner_bg` 类型为 `Background`，`Hsla` 可通过 `.into()` 转换，类型兼容。
- 编译后 `cargo check -p rust-rml-ui` 无错误。

#### 3.2.2 `crates/ui/src/components/tab/tabs.rs`

| 行号范围 | 当前代码 | 替换后 |
|----------|----------|--------|
| ~296, 851 | `cx.theme().tokens.background` | `cx.theme().background` |
| ~303, 313 | `cx.theme().tokens.primary` | `cx.theme().primary` |
| ~429, 434, 443 | `cx.theme().tokens.tab_bar` | `cx.theme().tab_bar` |
| ~461 | `cx.theme().tokens.tab_bar_segmented` | `cx.theme().tab_bar_segmented` |

验证点：
- `bg` 变量类型为 `Background`，替换后保持 `.into()` 调用。
- 同步补全 `tab_bar_segmented` 后方可替换，避免颜色回退不一致。

---

### 3.3 Demo 案例主题化

#### 3.3.1 `demo/src/cases/css_functions_case.css`

当前 `:root` 变量（如 `--brand-rgb`）可保留作为“CSS 函数演示”，但所有直接使用 hex/rgb/hsl 的 class 应改用 `var(--*)` 变量，并在 `:root` 中新增一组语义变量。修改策略：

1. 在 `:root` 中新增语义变量，映射到现有主题变量概念：

```css
:root {
    --brand-rgb: rgb(0, 102, 204);
    --brand-hsl: hsl(280, 100%, 50%);
    --accent-rgba: rgba(255, 140, 0, 0.8);
    --danger-rgba: rgba(220, 53, 69, 0.9);

    --demo-surface: var(--surface);
    --demo-surface-variant: var(--surface-variant);
    --demo-border: var(--border);
    --demo-text: var(--text);
    --demo-text-muted: var(--text-muted);
    --demo-primary-fg: var(--primary-foreground);
    --demo-info: var(--info);
    --demo-warning: var(--warning);
    --demo-error: var(--error);
    --demo-success: var(--success);
}
```

2. 将所有 `rgb(...)` / `hsl(...)` 字面量替换为 `var(--demo-*)`：

| class | 修改项 | 建议变量 |
|-------|--------|----------|
| `.rgb-card` | bg/border | `--demo-surface-variant` / `--demo-border` |
| `.rgba-overlay` | bg | `--demo-info` 透明度 0.3（或保留 rgba 函数演示） |
| `.hsl-card` | bg/border | `--demo-success` 透明度 / `--demo-success` |
| `.hsla-gradient` | bg | `--demo-warning` 透明度 0.4 |
| `.em-rem-box` | bg/border | `--demo-surface` / `--demo-warning` |
| `.vw-vh-box` | bg/border | `--demo-surface` / `--demo-info` |
| `.var-color-box` / `.var-hsl-box` | color | `--demo-primary-fg` |
| `.theme-light` / `.theme-dark` | 改为 `.theme-box` 使用 `--demo-surface` / `--demo-text` / `--demo-border` |
| `.nested-container` | bg | `--demo-surface-variant` |

> 保留 `--brand-rgb`、`--brand-hsl`、`--accent-rgba`、`--danger-rgba` 等 CSS 函数变量作为“函数演示”，但应用这些变量的容器前景色需跟随主题。

#### 3.3.2 `demo/src/cases/css_functions_case.rml`

将内联 `background="rgb(...)"` / `style="background: hsla(...)"` 替换为 `background="var(--demo-*)"` 或 `style="background: var(--demo-*)"`。同时更新说明文字，避免继续展示硬编码色值。

涉及位置：
- `background="rgb(200, 230, 200)"` → `background="var(--demo-success-soft)"`
- `background="rgb(245,245,255)"` → `background="var(--demo-surface)"`
- `background="rgb(255,248,225)"` → `background="var(--demo-warning-soft)"`
- `style="background: hsla(120, 100%, 50%, 0.1)"` → `style="background: var(--demo-success-soft)"`

#### 3.3.3 `demo/src/cases/css_priority_case.css`

```css
:root {
    --accent-color: var(--primary);
    --card-padding: 16px;
    --spacing-sm: 8px;
    --spacing-md: 16px;
    --spacing-lg: 24px;
}

.style-override {
    padding: 8px;
    background: var(--surface-variant);
    border: 1px solid var(--border);
    border-radius: 4px;
}

.var-demo {
    padding: var(--spacing-md);
    background: var(--accent-color);
    color: var(--primary-foreground);
    border-radius: 4px;
}

.nested-container {
    ...
    background: var(--surface);
    ...
}

.dynamic-active {
    background: var(--success-soft);
    border: 2px solid var(--success);
}

.dynamic-inactive {
    background: var(--error-soft);
    border: 2px solid var(--error);
}
```

> 若当前 `:root` 变量集未定义 `--success-soft` / `--error-soft` / `--warning-soft` 等，则在 `demo/assets/styles.css` 或案例 CSS 的 `:root` 中追加，避免使用不存在的变量。

#### 3.3.4 `demo/src/cases/css_priority_case.rml`

将所有内联硬编码颜色改为 `var(--*)`：

- `style="padding: 20px; background: #f0f0f0; border-radius: 6px;"` → `style="padding: 20px; background: var(--surface-variant); border-radius: 6px;"`
- `background="#e8f4fd"` → `background="var(--surface)"`
- `background="#ffffff"` → `background="var(--background)"`
- `style="padding: 12px; background: #f5f5f5;"` → `style="padding: 12px; background: var(--surface-variant);"`
- `background="#fff8e1"` → `background="var(--warning-soft)"`

#### 3.3.5 `demo/src/cases/overflow_test_case.rml`

- `border: 1px solid #e0e0e0` → `border: 1px solid var(--border)`

#### 3.3.6 `demo/src/cases/virtual_list_case.rml`

- `border-bottom: 1px solid #e8e8e8` → `border-bottom: 1px solid var(--border)`
- `border-right: 1px solid #e8e8e8` → `border-right: 1px solid var(--border)`

---

### 3.4 新增 Demo 语义变量（若不存在）

**文件**：[`demo/assets/styles.css`](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css)

在文件顶部或 `:root` 中追加：

```css
:root {
    --success-soft: color-mix(in srgb, var(--success) 15%, var(--background));
    --warning-soft: color-mix(in srgb, var(--warning) 15%, var(--background));
    --error-soft: color-mix(in srgb, var(--error) 15%, var(--background));
    --info-soft: color-mix(in srgb, var(--info) 15%, var(--background));
}
```

> 若 RML CSS 解析器暂不支持 `color-mix`，则改为使用已有变量，例如 `--surface-variant` 或 `--code-bg`，不再引入新的半透明颜色。

---

## 四、执行顺序

1. **补全主题同步**（`theme.rs`）
   - 添加 `tab_bar_segmented` 赋值。
   - 验证：`cargo check -p rust-rml-core`。

2. **替换 Tab 组件 tokens 用法**（`tab.rs`、`tabs.rs`）
   - 批量替换 `cx.theme().tokens.*` → `cx.theme().*`。
   - 验证：`cargo check -p rust-rml-ui`。

3. **Demo CSS/RML 主题化**
   - 按 3.3 节逐项修改。
   - 验证：`cargo check -p rust-rml-demo`。

4. **全量编译与测试**
   - `cargo check --workspace`
   - `cargo test -p rust-rml-core`（主题相关单测）
   - `cargo test -p rust-rml-ui`（如存在）

5. **视觉验收**
   - 启动 Demo，分别在 light / dark 主题下检查：
     - Tab 组件各变体（Tab/Flat/Outline/Pill/Segmented/Underline）颜色是否协调。
     - CSS Functions / CSS Priority 案例在暗色下无刺眼硬编码色。
     - Overflow / VirtualList 案例边框颜色自然。

---

## 五、验收标准

- [ ] `grep -n "theme().tokens\." crates/ui/src` 无任何命中（除注释/文档外）。
- [ ] `grep -n "#\\d\\{3,6\\}\|rgb(\|hsl(" demo/src/cases/css_functions_case.* demo/src/cases/css_priority_case.*` 无业务颜色硬编码（允许 `:root` 变量定义中的函数演示值）。
- [ ] `cargo check --workspace` 通过。
- [ ] `cargo test -p rust-rml-core` 通过。
- [ ] Demo 在 light/dark 主题切换后，所有整改页面无明显色彩断层、过亮/过暗区域。

---

## 六、风险与回退

1. **`color-mix` 不支持**：若 RML CSS 引擎未实现 `color-mix`，则改用现有 `--surface-variant` / `--code-bg` 等变量，不引入新语法。
2. **`tab_bar_segmented` 语义选择**：若与 `tab_bar` 同色导致 segmented 变体视觉层次不足，可调整为与 `secondary` 同色，回退到 gpui-component 默认 fallback 行为。
3. **CSS 函数演示的“函数”属性被削弱**：保留 `:root` 中的 `--brand-rgb` / `--brand-hsl` 等变量，确保 rgb()/hsl() 语法仍被展示。
