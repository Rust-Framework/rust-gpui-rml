# 暗/亮主题色应用整改计划

## 背景与问题

当前 `crates/ui` 与 `crates/core` 中主题色应用存在三类问题，导致运行效果在暗色/亮色切换时出现缺漏：

1. **gpui-component `tokens` 未同步**：`apply_builtin_gpui_theme` 只修改了 `Theme.colors` 字段，未重建 `Theme.tokens`。gpui-component 原生组件（Button、Input 等）以及代码中 `cx.theme().tokens.*` 的取色仍停留在默认值，这是“主题色缺漏”的最主要根因。
2. **语义 token 错用**：ActivityBar 背景误用 `title_bar`、TabWindow 侧栏/底栏未与主内容区做 surface 层级区分、Table 表头/斑马纹混用 `muted`。
3. **硬编码颜色残留**：`Card` 悬浮阴影使用固定黑色半透明；Demo 全局样式与部分 case 中写死了 `#ffffff`、`#f5f5f5` 等色值，切换 dark 后不可读。

本计划目标：一次性修好框架层主题色同步与 token 使用问题，并清理 Demo 中影响主题切换的硬编码色。

## 核心决策

- **保持 RML CSS 变量与 gpui-component Theme 双轨制**，本次不改为主题单一来源；但要求两者在 `apply_builtin_gpui_theme` 中保持一致，并确保 `tokens` 重建。
- **框架代码优先使用 `cx.theme().<字段>` 取 `Hsla`**，仅在类型强制需要 `Background`/`Fill` 时使用 `cx.theme().tokens.<字段>`，减少同文件混用。
- **Card 背景保持 `theme.background`**，不改为 `popover`/`secondary`，避免浅色模式下卡片与页面融为一体；通过边框和 theme-aware 阴影建立层级。
- **CSS 颜色函数演示 case 保持原样**，因为它们的教学目的就是展示固定颜色函数；只修复非教学性质的硬编码色。

## 实施步骤

### P0：修复 gpui-component tokens 同步（根因）

**文件**：`crates/core/src/theme.rs`

在 `apply_dark_theme_config` 与 `apply_light_theme_config` 末尾添加：

```rust
t.tokens = gpui_component::theme::ThemeTokens::from(&*t);
```

使 `cx.theme().tokens.*` 与 `cx.theme().*` 反映同一套自定义 light/dark 配色。

### P1：修正核心组件语义 token

| 文件 | 当前代码 | 目标修改 | 理由 |
|---|---|---|---|
| `crates/ui/src/components/activity_bar/bar.rs:126` | `.bg(cx.theme().title_bar)` | `.bg(cx.theme().sidebar)` | ActivityBar 是侧边导航，应使用 `sidebar` token；与 gpui-component `Sidebar` 组件保持一致。 |
| `crates/ui/src/window/tab_window.rs:840` | `.bg(cx.theme().tokens.title_bar)` | `.bg(cx.theme().title_bar)` | 统一使用 `Hsla` 字段，不再依赖 token 同步；title_bar 语义正确。 |
| `crates/ui/src/window/tab_window.rs`（left/right panel） | `.bg(cx.theme().background)` | `.bg(cx.theme().sidebar)` | 左/右可调整面板应使用 sidebar surface，与主内容区形成层级。 |
| `crates/ui/src/window/tab_window.rs`（bottom panel） | `.bg(cx.theme().sidebar)` | 保持不变 | 已正确使用 sidebar。 |
| `crates/ui/src/window/tab_window.rs`（body/main） | `.bg(cx.theme().background)` | 保持不变 | 主内容区使用 background 是正确的。 |
| `crates/ui/src/components/table/table.rs:195` | `header_bg = theme.muted; stripe_bg = theme.muted` | `header_bg = theme.table_head; stripe_bg = theme.table_even` | 使用表格专用 token，表头与斑马纹不再同色。 |
| `crates/ui/src/components/card.rs:225-248` | 硬编码 `hsla(0., 0., 0., ...)` 阴影 | 使用 `theme.foreground` 为基色、按层降低透明度（约 0.08 / 0.06 / 0.04） | 亮色下为深色阴影，暗色下为浅灰 elevation，避免黑色阴影在暗色下不可见。 |
| `crates/ui/src/components/card.rs:248` | `hsla(0., 0., 0., 0.)` | `theme.transparent` | 语义化透明。 |

### P2：清理 Demo 硬编码色

**仅处理影响主题切换可读性的非教学色值**：

| 文件 | 修改 |
|---|---|
| `demo/assets/styles.css:80` | `.tag-list > * { color: #ffffff; }` → `color: var(--primary-foreground);` |
| `demo/src/cases/content_binding_case.rml.rs:21-22,122-123` | 两个 badge 的 `.bg(gpui::rgb(...)).text_color(gpui::rgb(0xffffff))` 改为 `.bg(cx.theme().primary).text_color(cx.theme().primary_foreground)` 与 `.bg(cx.theme().success).text_color(cx.theme().success_foreground)`；需引入 `gpui_component::ActiveTheme`。 |
| `demo/src/cases/focus_event_case.rml:23-24` | 内联 `background="#f5f5f5"` → `background="var(--surface)"`；`border="1px solid #d9d9d9"` → `border="1px solid var(--border)"`。 |
| `demo/src/cases/hover_card_case.rml:78` | 写死黄色背景改为 `background="var(--surface)" border="1px solid var(--border)"`。 |
| `demo/src/cases/popover_case.rml:73` | 同上。 |
| `demo/src/cases/resizable_case.rml` | 各面板内联背景色改为 `var(--surface)` / `var(--surface-variant)`，保留布局演示意图。 |
| `demo/src/cases/title_bar_case.rml` | 写死背景色改为 `var(--title-bar)`（若不存在则新增 `--title-bar` CSS 变量并映射到 `theme.title_bar`）。 |
| `demo/src/cases/native_status_bar_case.rml` | 写死背景色改为 `var(--status-bar)`（若不存在则新增 `--status-bar` CSS 变量并映射到 `theme.status_bar`）。 |

**以下文件保持原样**（教学演示 CSS 颜色函数，非主题 bug）：
- `demo/src/cases/css_functions_case.rml` / `.css`
- `demo/src/cases/css_priority_case.rml` / `.css`
- `demo/src/cases/overflow_test_case.rml`
- `demo/src/cases/virtual_list_case.rml`
- `demo/src/cases/color_picker_case.rml.rs` 中的 `hsla(...)` 字符串（颜色选择器输出示例）

### P3：可选的低风险补全

| 文件 | 检查/修改 |
|---|---|
| `crates/ui/src/window/modern_window.rs` | 确认 `gpui_component::TitleBar` 在 dark 下背景/前景是否正确；若仍异常，在外层容器补 `.bg(cx.theme().title_bar).text_color(cx.theme().foreground)`。 |
| `crates/ui/src/components/menu.rs` | 若 MenuBar 放在非主题背景上文字不可读，给容器补 `.bg(cx.theme().background).text_color(cx.theme().foreground)`。 |

这两项以“运行验证时发现异常再改”为原则，避免过度修改。

## 新增/同步的 CSS 变量

若 demo case 需要引用标题栏/状态栏色，建议在 `crates/core/src/theme.rs` 内置色表与 `demo/assets/themes/{light,dark}.css` 中新增：

| CSS 变量 | light | dark | 对应 gpui-component 字段 |
|---|---|---|---|
| `--title-bar` | `#f3f4f6` | `#1a1b1d` | `title_bar` |
| `--status-bar` | `#f3f4f6` | `#1a1b1d` | `status_bar` |
| `--primary-foreground` | `#ffffff` | `#ffffff` | `primary_foreground` |

并在 `apply_builtin_gpui_theme` 中保持三者一致。

## 主题色使用约定（后续代码应遵循）

1. **取色默认使用 `cx.theme().<字段>`**（返回 `Hsla`，适用于 `.bg`/`.text_color`/`.border_color` 等）。
2. **仅在类型要求 `Background`/`Fill` 时使用 `cx.theme().tokens.<字段>`**，例如某些泛型参数无法自动转换时。
3. **修改 `ThemeColor` 后必须重建 `tokens`**：任何手动赋值 `t.xxx = ...` 后执行 `t.tokens = ThemeTokens::from(&*t);`。
4. **组件语义优先**：
   - 侧边栏/可调整侧栏 → `sidebar`
   - 标题栏 → `title_bar`
   - 状态栏 → `status_bar`
   - 表格头/斑马纹 → `table_head` / `table_even`
   - 卡片/浮层面板背景 → 保持 `background`，通过边框/阴影区分
5. **文字必须成对出现**：`primary` 配 `primary_foreground`，`popover` 配 `popover_foreground`，`sidebar` 配 `sidebar_foreground`。
6. **框架代码禁止硬编码 `hsla(0.,0.,0.,...)`、`gpui::rgb(0xffffff)` 等色值**；Demo 教学示例除外，但需明确标注。
7. **RML CSS 变量与 gpui-component 颜色保持一致**：新增 `--xxx` 变量时，应在 `apply_builtin_gpui_theme` 中映射对应字段，并在主题 CSS 文件中给出覆盖值。
8. **`is_dark()` 仅用于无法用语义 token 表达的细节**（如阴影强度），不要用它选择整套颜色。

## 验证方案

### 编译与测试

```powershell
# 1. 全工作区语法检查
cargo check --workspace

# 2. core 主题相关单测
cargo test -p rust-rml-core --lib theme

# 3. UI crate 编译
cargo check -p rust-rml-ui

# 4. Demo 可构建
cargo check -p demo
```

### 运行期视觉验收

启动 demo，在 light / dark 两种主题下逐项检查：

| 场景 | light 预期 | dark 预期 |
|---|---|---|
| **ActivityBar** | 图标栏背景为浅灰 sidebar 色，与主内容区有边界；激活按钮高亮可见。 | 图标栏为深色 sidebar，激活高亮可见，文字可读。 |
| **TabWindow** | 标题栏、左/右侧边面板、主内容区三层背景分明；Tabs 文字/背景正确。 | 标题栏、侧栏、主内容区层次清晰；Tabs 选中态可见。 |
| **Table（带 stripe）** | 表头与斑马纹颜色不同；文字可读。 | 同上，暗色下斑马纹不突兀。 |
| **Card** | 卡片边框清晰；悬浮时阴影可见；文字可读。 | 卡片边框清晰；悬浮时可见浅灰 elevation；文字可读。 |
| **Button/Input/Tabs 等原生组件** | 主题色正确，无 token 缺漏导致的异常灰/白色。 | 主题色正确，暗色下无 token 缺漏。 |
| **Demo 示例页** | 无 `#f5f5f5` 等浅色块导致文字不可读；Tag 文字在 primary 背景上可读。 | 所有示例区域颜色随主题切换，无刺眼固定浅色。 |

### 静态检查（防回归）

整改完成后，可在 `crates/ui/src` 中再次搜索：

```powershell
# 不应再出现框架代码中的硬编码黑阴影或 gpui::rgb(0x...)
Select-String -Path crates/ui/src -Pattern 'hsla\(0\.\s*,\s*0\.\s*,\s*0\.' -Recurse
Select-String -Path crates/ui/src -Pattern 'gpui::rgb\(0x' -Recurse
```

## 风险与回退

1. **Card 阴影视觉取舍**：使用 `foreground` 作基色在 dark 下呈浅灰 elevation，可能与传统黑色阴影心理模型不同。若验收不自然，可回退为 `theme.border` 基色或新增 `--shadow` 变量。
2. **侧栏改为 `sidebar` 后的对比度**：若 `sidebar` 与 `background` 在某一主题下过于接近，可微调 `apply_builtin_gpui_theme` 中的 `sidebar` 值。
3. **Token 同步时机**：`apply_builtin_gpui_theme` 中重建 `tokens` 可能会覆盖 gpui-component 之前自行维护的状态。由于 `ThemeTokens::from(&*t)` 完全由 `colors` 派生，风险较低。
4. **Demo CSS 函数教学示例**：本次明确保留，不影响 `css_functions_case` 与 `css_priority_case` 的教学意图。

## 关键修改文件清单

- `crates/core/src/theme.rs`
- `crates/ui/src/components/activity_bar/bar.rs`
- `crates/ui/src/window/tab_window.rs`
- `crates/ui/src/components/table/table.rs`
- `crates/ui/src/components/card.rs`
- `demo/assets/styles.css`
- `demo/src/cases/content_binding_case.rml.rs`
- `demo/src/cases/focus_event_case.rml`
- `demo/src/cases/hover_card_case.rml`
- `demo/src/cases/popover_case.rml`
- `demo/src/cases/resizable_case.rml`
- `demo/src/cases/title_bar_case.rml`
- `demo/src/cases/native_status_bar_case.rml`
- `demo/assets/themes/light.css`
- `demo/assets/themes/dark.css`
