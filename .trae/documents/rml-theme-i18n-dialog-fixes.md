# Plan: RML 内置主题配色完善 + 语言切换菜单双向化 + Settings 弹窗尺寸固定

## Summary

修复三个独立问题：
1. **主题配色不完整** — 现有 light/dark 主题文件仅定义 6 个颜色变量，且与 `styles.css` 引用的变量名不一致（多处引用不存在的变量导致回退为透明黑色）。需扩充为完整的现代化亮暗配色体系。
2. **语言切换菜单单向** — View 菜单仅有 `SwitchEnCommand`（切到英文），缺少切回中文的菜单项。需添加 `SwitchZhCommand`。
3. **Settings 弹窗无固定高度** — `<dialog>` 根标签仅支持 `width` 属性，无 `height`，导致 Settings 弹窗高度随内容塌缩、视觉拥挤。需为 `<dialog>` 根标签增加 `height` 属性支持并设置合理尺寸。

---

## Current State Analysis

### Issue 1: 主题配色

**主题文件**（[dark.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/dark.css) / [light.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/light.css)）当前仅定义 6 个变量：
- `--primary-color`, `--text-color`, `--text-muted`, `--bg-color`, `--code-bg`, `--border-color`

**[styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css) 引用的变量**与主题文件命名不一致：

| styles.css 引用 | 主题文件定义 | 状态 |
|---|---|---|
| `--bg-color` | `--bg-color` | ✅ 一致 |
| `--code-bg` | `--code-bg` | ✅ 一致 |
| `--border-color` | `--border-color` | ✅ 一致 |
| `--text-muted` | `--text-muted` | ✅ 一致 |
| `--color-background` | — | ❌ 不存在 → 透明黑 |
| `--color-border` | — | ❌ 不存在 → 透明黑 |
| `--color-primary` | — | ❌ 不存在 → 透明黑 |
| `--border` | — | ❌ 不存在 → 透明黑 |
| `--text` | — | ❌ 不存在 → 透明黑 |
| `--surface` | — | ❌ 不存在 → 透明黑 |

**加载机制**（[app.rs:16-18](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs#L16-L18)）：
- `cx.set_style("styles.css")` — styles.css 无 `:root` 块，不提供基础变量
- `cx.set_theme("light")` — 从 `assets/themes/light.css` 加载所有主题变量
- 主题切换时 `set_theme("dark")` 加载 `dark.css` 覆盖

**结论**：所有颜色变量仅来自主题文件。需统一命名 + 扩充完整配色。

### Issue 2: 语言切换菜单

**[menu_commands.rs:225-252](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_commands.rs#L225-L252)**：仅注册了 `SwitchEnCommand`（id=`menu.view.lang`，label=`menu.lang_en`），调用 `apply_switch_en` → `cx.set_i18n("en-US")`。

缺少对应的 `SwitchZhCommand`，用户通过菜单切到英文后无法切回中文。

**[main_window.rml.rs:435-437](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L435-L437)**：`apply_switch_en` 方法存在，但无 `apply_switch_zh`。

**i18n 资源**：`menu.lang_en` 已有（[zh-CN.json:179](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json#L179)），缺少 `menu.lang_zh`。

### Issue 3: Settings 弹窗尺寸

**[settings_dialog.rml:1](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/settings_dialog.rml#L1)**：`<dialog title={...} width="640">` — 有 width 无 height。

**`<dialog>` 根标签 codegen**（[window.rs:96-130](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/window.rs#L96-L130)）：`gen_dialog_impl` 仅提取 `title` 和 `width` 属性，不提取 `height`。生成代码：
```rust
__rml_d.title(__rml_title).width(__rml_width).content(move |__rml_content, _, _| {
    __rml_content.child(__rml_entity.clone())
})
```

**gpui-component Dialog**（cargo 缓存 `063e55b`）：`Dialog` 仅有 `.width()` / `.max_w()`，无 `.height()`。但 `DialogContent` 实现了 `Styled` trait（支持 `.min_h()`），且通过 `.content()` 闭包接收 `__rml_content: DialogContent` 参数，可在闭包内调用 `.min_h()` 设置最小高度。

**影响范围**：仅 2 个 `<dialog>` 根标签用法（settings_dialog.rml + login_dialog.rml），添加可选 `height` 属性不影响现有用法。

---

## Proposed Changes

### Change 1: 扩充主题配色 + 统一命名

**文件**：[demo/assets/themes/light.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/light.css)

统一为短名命名（`--background` / `--text` / `--border` / `--primary` / `--surface`），扩充语义色：

```css
:root {
    /* 品牌色 */
    --primary: #007bff;

    /* 背景 / 表面 */
    --background: #f8f9fa;
    --surface: #ffffff;
    --surface-variant: #f1f3f5;
    --code-bg: #f1f3f5;

    /* 文本 */
    --text: #333333;
    --text-muted: #6b7280;

    /* 边框 */
    --border: #e5e7eb;

    /* 语义色 */
    --success: #16a34a;
    --warning: #f59e0b;
    --error: #dc2626;
    --info: #0ea5e9;
}
```

**文件**：[demo/assets/themes/dark.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/themes/dark.css)

```css
:root {
    /* 品牌色 */
    --primary: #3b82f6;

    /* 背景 / 表面 */
    --background: #1f2937;
    --surface: #283548;
    --surface-variant: #111827;
    --code-bg: #111827;

    /* 文本 */
    --text: #e5e7eb;
    --text-muted: #9ca3af;

    /* 边框 */
    --border: #374151;

    /* 语义色 */
    --success: #22c55e;
    --warning: #fbbf24;
    --error: #ef4444;
    --info: #38bdf8;
}
```

**文件**：[demo/assets/styles.css](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css)

将所有旧变量引用统一为新命名：

| 旧引用 | 新引用 | 涉及行 |
|---|---|---|
| `var(--bg-color)` | `var(--background)` | L12 |
| `var(--color-background)` | `var(--background)` | L63 |
| `var(--color-border)` | `var(--border)` | L64 |
| `var(--color-primary)` | `var(--primary)` | L79 |
| `var(--code-bg)` | 保持不变 | L315 |
| `var(--border-color)` | `var(--border)` | L316, L351, L365, L380, L471 |
| `var(--text)` | 保持不变 | L321 |
| `var(--surface)` | 保持不变 | L353 |
| `var(--text-muted)` | 保持不变 | L358, L410, L429 |
| `var(--border)` | 保持不变 | L256, L263, L269, L291 |

### Change 2: 添加切换中文菜单项

**文件**：[demo/src/shell/menu_commands.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_commands.rs)

在 `SwitchEnCommand`（L225）之后添加 `SwitchZhCommand`，结构对称：
- `#[contribute(...)]` id=`menu.view.lang.zh`，parent_id=`menu.view`，order=3，label=`menu.lang_zh`
- `IContribution::name()` → `t_static("menu.lang_zh")`
- `ICommand::execute()` → 调用 `this.apply_switch_zh(cx)`

**文件**：[demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs)

在 `apply_switch_en`（L435）旁添加 `apply_switch_zh`：
```rust
pub(crate) fn apply_switch_zh(&mut self, cx: &mut Context<Self>) {
    cx.set_i18n("zh-CN");
}
```

**文件**：[demo/assets/i18n/zh-CN.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json) + [en-US.json](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/en-US.json)

添加 i18n 键 `menu.lang_zh`：
- zh-CN: `"menu.lang_zh": "中文"`
- en-US: `"menu.lang_zh": "中文"`

**文件**：[demo/src/shell/menu_commands.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_commands.rs#L1) 头注释

将 "7 个叶子命令" 更新为 "8 个叶子命令"。

### Change 3: 为 `<dialog>` 根标签添加 height 属性 + 设置 Settings 弹窗尺寸

**文件**：[crates/engine/src/compiler/codegen/window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/window.rs#L96-L130)

在 `gen_dialog_impl` 中：
1. 提取 `height` 属性（与 `width` 同样方式：`extract_static_attr(elem, "height").and_then(|s| s.parse::<f32>().ok())`）
2. 在生成代码中，当 `height` 存在时，在 `.content()` 闭包内对 `__rml_content` 调用 `.min_h(gpui::px(HEIGHT))`，并补充 `use gpui::Styled;` 导入

生成代码变为（height 存在时）：
```rust
.content(move |__rml_content, _, _| {
    use gpui::Styled;
    __rml_content.min_h(gpui::px(HEIGHT)).child(__rml_entity.clone())
})
```

height 不存在时保持原样（不影响 login_dialog 等现有用法）。

**文件**：[demo/src/shell/settings_dialog.rml](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/settings_dialog.rml#L1)

将 `<dialog title={...} width="640">` 改为 `<dialog title={...} width="640" height="480">`。

640×480 是设置弹窗的合理大气尺寸（侧边栏 160px + 内容区 480px 宽，480px 高容纳两组设置页）。

---

## Assumptions & Decisions

1. **主题命名统一为短名**（`--background` 而非 `--bg-color`），与 styles.css 中已使用的短名（`--text`, `--border`, `--surface`）对齐。旧名不保留兼容（RML 无历史包袱）。
2. **配色值基于现有 Tailwind 风格**（#1f2937 / #374151 等深灰系用于暗色，#f8f9fa / #e5e7eb 等浅灰系用于亮色），扩展 surface 层和语义色。
3. **`height` 属性语义为最小高度**（`min_h`）而非固定高度，允许内容超出时弹窗自然增长，避免内容被裁剪。
4. **`menu.lang_zh` 的两个 locale 值均为 "中文"**（语言名用本语显示，符合 i18n 惯例）。
5. **Settings 弹窗尺寸 640×480**：宽度已为 640（保留），高度 480 适合含侧边栏的设置面板，视觉比例约 4:3，大气合理。

---

## Verification Steps

1. **主题配色** → 验证：启动 demo，切换 dark/light 主题，检查所有案例页背景、文本、边框、卡片、代码块颜色正确显示（无透明黑回退）。特别检查 `.content-box`（曾引用 `--color-background`/`--color-border`）、`.tag-list > *`（曾引用 `--color-primary`）、`.code-block`（引用 `--text`/`--code-bg`/`--border-color`）。
2. **语言切换菜单** → 验证：View 菜单同时显示 "English" 和 "中文" 两项；点击 English 切到 en-US，点击中文切回 zh-CN；界面文案随语言刷新。
3. **Settings 弹窗尺寸** → 验证：打开 Settings 弹窗，弹窗尺寸约 640×480，侧边栏 + 设置内容不再拥挤；login 弹窗不受影响（仍为 420 宽、自动高度）。
4. **编译** → 验证：`cargo build` 全部通过，无警告。
