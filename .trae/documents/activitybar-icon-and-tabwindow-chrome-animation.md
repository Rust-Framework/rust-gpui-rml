# ActivityBar 图标处理优化 + TabWindow 标题栏展开/收起滑动动画

## Summary

两项独立但相邻的 UI 改进：

1. **ActivityBar 图标处理**：重构 `resolve_icon`，充分利用 `Icon` 组件的能力处理不同数据类型（命名图标 / SVG 资产路径 / 图片 URL），替代当前硬编码 16 个图标的脆弱映射。
2. **TabWindow 标题栏 chrome toggle**：为左侧窗口图标按钮增加手型光标（hover），并为 menu slot + 窗口标题的展开/收起增加左右滑动过渡动画。

---

## Current State Analysis

### 1. ActivityBar 图标处理现状

文件：[crates/ui/src/components/activity_bar/icon.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar/icon.rs)

```rust
pub fn resolve_icon(icon: Option<SharedString>, window: &Window) -> AnyElement {
    match icon.as_deref() {
        Some(s) if is_url(s) => {
            // URL → gpui::img（绕过 Icon 组件）
            img(s).flex_shrink_0().size_4().text_color(text_color).into_any_element()
        }
        Some(s) => match parse_icon_name(s) {
            Some(name) => Icon::new(name).into_any_element(),
            None => Icon::new(IconName::PanelLeft).into_any_element(), // 未匹配 → fallback
        },
        None => Icon::new(IconName::PanelLeft).into_any_element(),
    }
}

fn parse_icon_name(s: &str) -> Option<IconName> {
    match s {
        "BookOpen" => Some(IconName::BookOpen),
        "PanelLeft" => Some(IconName::PanelLeft),
        // ... 仅 16 个硬编码图标
        _ => None,
    }
}
```

**问题**：
- `parse_icon_name` 硬编码 16 个图标，未匹配时回退到 `PanelLeft`，无法使用 `gpui-component-assets` 提供的 100+ 图标
- 不支持自定义 SVG 资产路径（如 `"icons/custom.svg"`），`Icon::default().path(s)` 能力未被利用
- 未匹配字符串直接 fallback 到 `PanelLeft`，掩盖了配置错误
- `Icon` 实例未应用 `Sizable`（如 `.small()`），与 TabWindow 中 `Icon::new(app_icon).small()` 的用法不一致
- URL 一律走 `img()`：对 SVG 来说丢失了 `Icon` 的 `text_color` 主题着色能力（但 `img` 对外部 URL 是必要的，保留）

### 2. TabWindow chrome toggle 现状

文件：[crates/ui/src/window/tab_window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L316-L385)

**chrome_toggle 按钮（L316-338）**：
```rust
Button::new("tab-window-chrome-toggle")
    .text()
    .h(TITLE_BAR_HEIGHT)
    .w(TITLE_BAR_HEIGHT)
    // ... 无 .cursor_pointer()
    .child(h_flex().child(Icon::new(app_icon).small()).child(Icon::new(chevron).small()))
```

- `Button` 内部 `render` 调用 `self.base.cursor_default()`（[button.rs L471/L476](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/button/button.rs#L471)），仅 link 变体切换 `cursor_pointer()`
- `.text()` 变体不是 link，故光标为 default（箭头），不是手型
- `Button` 实现 `Styled`，用户调用 `.cursor_pointer()` 会经 `.refine_style(&self.style)`（L555）覆盖内部 `cursor_default()` — **可行**

**prefix 展开/收起（L355-385）**：
```rust
if show_chrome {
    let mut prefix_parts: SmallVec<[AnyElement; 2]> = SmallVec::new();
    if let Some(menu) = self.menu_slot { prefix_parts.push(...); }
    if let Some(title) = self.title { prefix_parts.push(...); }
    if !prefix_parts.is_empty() {
        tab_bar = tab_bar.prefix(h_flex()...children(prefix_parts));
    }
}
```

- `show_chrome=false` 时 prefix 完全不渲染（瞬间消失/出现，无动画）
- chevron 方向已根据 `show_chrome` 切换 `ChevronLeft`/`ChevronRight`（L310-314） — **无需改动**

**动画基础设施**：
- `TabWindowShell` 是 `RenderOnce` 组件（无状态），但 `window.use_keyed_state` 在 `RenderOnce` 中可用（参考 [button.rs L450](file:///C:/Users/lusid/.cargo/git/checkouts/gpui-component-95ce574d8a0da8b8/063e55b/crates/ui/src/button/button.rs#L450) 用法）
- `with_animation` 由 `AnimationExt` trait 提供，可作用于 `div()`/`h_flex()`
- `use_keyed_state` 语义（[gpui window.rs L3346](file:///C:/Users/lusid/.cargo/git/checkouts/zed-a70e2ad075855582/3f5705b/crates/gpui/src/window.rs#L3346)）：`init` 为 `FnOnce`，仅首次渲染调用；后续渲染返回持久化 `Entity<S>`；`entity.update()` 触发 `cx.notify()` 重渲
- 状态变更检测 + 延迟同步模式参考 gpui-component rules 的 checkbox 示例

---

## Proposed Changes

### Task 1: 重构 `resolve_icon` — 充分利用 Icon 组件能力

**文件**：[crates/ui/src/components/activity_bar/icon.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/activity_bar/icon.rs)

#### 1.1 数据分类与分发

将 `IContribution::icon()` 返回的 `Option<SharedString>` 按以下优先级解析：

| 输入示例 | 检测条件 | 处理方式 |
|---------|---------|---------|
| `"file://..."`, `"http://..."`, `"https://..."` | `is_url(s)` | `img(s)` — 外部图片，Icon 无法加载 URL |
| `"BookOpen"`, `"Settings"`, `"PanelLeft"` 等 | `parse_icon_name(s)` 匹配 | `Icon::new(name).small()` — 命名图标 |
| `"icons/custom.svg"`, `"my-icon.svg"` | 非 URL + 含 `.` 或 `/` + 未匹配命名 | `Icon::default().path(s).small()` — **新增**：SVG 资产路径 |
| `"UnknownName"`（无 `.`/`/`） | 非 URL + 未匹配命名 + 不像路径 | `Icon::new(IconName::PanelLeft).small()` — fallback（保留，避免静默失败） |
| `None` | 无图标 | `Icon::new(IconName::PanelLeft).small()` — fallback |

**关键改动**：新增第三类「SVG 资产路径」分支，利用 `Icon::default().path(s)` 能力。判定规则：非 URL + 未匹配 `parse_icon_name` + 字符串含 `/` 或以 `.svg` 结尾。

#### 1.2 统一 Sizable 尺寸

所有 `Icon` 实例追加 `.small()`（对应 `size_3p5()` ≈ 14px），与 `tab_window.rs` 中 `Icon::new(app_icon).small()` 一致，确保 ActivityBar 图标与标题栏图标视觉尺寸统一。`img()` 分支保留 `size_4()`（img 无 Sizable）。

#### 1.3 扩展 `parse_icon_name` 映射

补充代码库中已使用 + 常用图标（基于 `gpui-component-assets` SVG 文件名 PascalCase 化）：

新增（在现有 16 个基础上）：
- `ArrowLeft`, `ArrowRight`, `ArrowUp`, `ArrowDown`
- `ChevronLeft`, `ChevronRight`, `ChevronUp`, `ChevronDown`
- `X`, `Check`, `Plus`, `Minus`, `Search`
- `Edit`, `Trash`, `Copy`, `Save`
- `Play`, `Pause`, `RefreshCw`
- `Code`, `Terminal`, `GitBranch`
- `Home`, `LayoutGrid`, `Columns`
- `Eye`, `Filter`, `SortAsc`
- `Sun`, `Moon`, `Monitor`
- `Wrench`, `Package`, `Boxes`
- `ChevronsLeft`, `ChevronsRight`

（完整列表在实现时按 `gpui-component-assets` 实际 SVG 文件名补全；遗漏项会走资产路径分支或 fallback，不会崩溃）

#### 1.4 保持返回类型 `AnyElement`

不引入新 enum 包装类型。`resolve_icon` 仍返回 `AnyElement`，因为：
- 调用方 `bar.rs` 仅将图标作为 `Button::child()` 使用，不需要链式定制
- `Icon` 和 `img` 渲染机制不同，统一 enum 会增加无谓抽象
- 改动最小化，聚焦于「充分利用 Icon 能力」本身

### Task 2: TabWindow chrome toggle 手型光标 + 滑动动画

**文件**：[crates/ui/src/window/tab_window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs)

#### 2.1 新增 imports

```rust
use std::time::Duration;
use gpui::{Animation, AnimationExt as _, ...};  // 追加 Animation, AnimationExt
use gpui_component::{..., animation::cubic_bezier, ...};  // 追加 cubic_bezier
```

#### 2.2 chrome_toggle 按钮增加手型光标

L317-337 处，`Button::new("tab-window-chrome-toggle")` 链上追加 `.cursor_pointer()`：

```rust
Button::new("tab-window-chrome-toggle")
    .text()
    .cursor_pointer()        // ← 新增
    .h(TITLE_BAR_HEIGHT)
    .w(TITLE_BAR_HEIGHT)
    // ... 其余不变
```

原理：`Button::render` 在 L555 调用 `.refine_style(&self.style)`，将用户通过 `Styled::style()` 设置的 `cursor: Pointer` 覆盖内部 L471/L476 的 `cursor_default()`。

#### 2.3 prefix 展开/收起滑动动画

**重构 L354-385**：移除 `if show_chrome { ... }` 闸门，改为始终构建 `prefix_parts`，再用动画容器包裹。

```rust
// 始终构建 prefix_parts（不再 gate on show_chrome）
let mut prefix_parts: SmallVec<[AnyElement; 2]> = SmallVec::new();
if let Some(menu) = self.menu_slot {
    prefix_parts.push(div().h_full().flex_shrink_0().child(menu).into_any_element());
}
if let Some(title) = self.title {
    prefix_parts.push(div().px_2().flex_shrink_0().child(title).into_any_element());
}

if !prefix_parts.is_empty() {
    // 1. 用 use_keyed_state 跟踪上一次的 show_chrome（init 仅首次调用）
    let chrome_state = window.use_keyed_state(
        "tab-window-chrome-anim",
        cx,
        |_, _| self.show_chrome,
    );
    let prev_chrome = *chrome_state.read(cx);
    let chrome_changed = prev_chrome != self.show_chrome;
    let target_chrome = self.show_chrome;

    // 2. 状态变更时，动画结束后同步 keyed_state → 触发一次重渲使 chrome_changed 归 false
    if chrome_changed {
        let state = chrome_state.clone();
        cx.spawn(async move |cx| {
            cx.background_executor()
                .timer(Duration::from_secs_f64(0.25))
                .await;
            _ = state.update(cx, |s, _| *s = target_chrome);
        })
        .detach();
    }

    // 3. 动画参数
    let anim = Animation::new(Duration::from_secs_f64(0.25))
        .with_easing(cubic_bezier(0.4, 0., 0.2, 1.));

    // 4. 构建动画容器
    let prefix = h_flex()
        .h_full()
        .items_center()
        .flex_shrink_0()
        .gap_1()
        .overflow_hidden()
        .when(chrome_changed, |this| {
            this.with_animation(
                "tab-window-chrome-slide",
                anim,
                move |this, delta| {
                    // 展开：delta 0→1 对应 progress 0→1
                    // 收起：delta 0→1 对应 progress 1→0
                    let progress = if target_chrome { delta } else { 1.0 - delta };
                    this.max_w(px(800.0) * progress).opacity(progress)
                },
            )
        })
        .when(!chrome_changed, |this| {
            // 静止态：展开时正常，收起时 0 宽 + 0 透明度
            if self.show_chrome { this } else { this.w_0().opacity_0() }
        })
        .children(prefix_parts);

    tab_bar = tab_bar.prefix(prefix);
}
```

**动画语义**：
- `max_w(px(800.0) * progress)` + `overflow_hidden()`：内容随容器宽度收缩被裁剪，形成左右滑动视效
- `opacity(progress)`：叠加淡入/淡出，过渡更柔和
- `progress = delta`（展开）或 `1.0 - delta`（收起）：保证动画从旧状态滑向新状态
- `px(800.0)` 为上限，覆盖 menu + title 常见宽度；超出部分由 `overflow_hidden` 裁剪
- `use_keyed_state` 确保 `with_animation` 仅在 `show_chrome` 真正变更时触发，避免每次重渲重启动画

**状态同步时序**：
1. 用户点击 → `on_chrome_toggle` 翻转 `show_chrome` → `MainWindow` `cx.notify()` → TabWindowShell 重渲
2. 重渲时 `prev_chrome != show_chrome` → 应用 `with_animation`，同时 `cx.spawn` 0.25s 后 `state.update`
3. 0.25s 内：动画播放（gpui 同 id 动画跨重渲延续，不重启）
4. 0.25s 后：`state.update` → `cx.notify` → 重渲 → `prev_chrome == show_chrome` → 不再动画，渲染静止态

#### 2.4 不变部分

- chevron 方向逻辑（L310-314）不变：`show_chrome=true` → `ChevronLeft`，`false` → `ChevronRight`
- `chrome_toggle` 按钮的 `app_icon + chevron` 子元素结构不变
- `on_chrome_toggle` 回调签名与 MainWindow 的 `#[command] on_chrome_toggle` 不变

---

## Assumptions & Decisions

1. **`Icon::default().path(s)` 用于 SVG 资产路径**：`s` 是相对资产根的路径（如 `"icons/foo.svg"`），由 gpui 资产系统解析。非资产路径会渲染空白，但不会崩溃。
2. **URL 一律走 `img()`**：`Icon` 的 `path()` 接受资产路径，不接受 `file://`/`http://` URL。对 SVG URL 保留 `img()` 以维持一致性，牺牲 `Icon` 的 `text_color` 着色（URL 图片通常自带颜色）。
3. **`parse_icon_name` 仍手工维护**：`IconName` 由 `icon_named!` 宏从 SVG 文件名生成，无 `FromStr` impl。宏改造成本高且影响上游，不在本次范围。补充常用图标 + 资产路径 fallback 已能满足「处理不同数据」诉求。
4. **动画 `max_w` 上限 800px**：menu-bar + 标题在 demo 中远小于 800px。若业务场景出现超长标题，可调大常量或改用测量宽度（本次不做）。
5. **`use_keyed_state` key 固定 `"tab-window-chrome-anim"`**：假设单窗口单 TabWindowShell。多实例场景需引入 `id` 字段，本次不做。
6. **`cx.spawn` 在 `RenderOnce::render` 中可用**：`render(self, window: &mut Window, cx: &mut App)`，`App::spawn` 返回 `Task`，`.detach()` 后台执行。参考 button.rs L450 在 RenderOnce 中使用 `use_keyed_state` 的先例。
7. **不引入 Entity 化 TabWindowShell**：保持 `RenderOnce` 无状态架构，动画状态由 `use_keyed_state`（窗口级 element state）承载，符合 gpui-component 模式。

---

## Verification Steps

### Task 1 验证

1. **编译**：`cargo build -p rust-rml-ui` 通过
2. **命名图标**：demo 运行后 ActivityBar 各面板图标正常显示（BookOpen 等）
3. **新增图标名**：临时在某面板 `icon()` 返回 `"ArrowLeft"` / `"Settings"` 等新增项，确认显示
4. **资产路径**（可选验证）：在某面板 `icon()` 返回 `"icons/custom.svg"`（需准备测试 SVG 放到资产目录），确认显示
5. **URL 图片**：在某面板 `icon()` 返回 `"https://example.com/icon.png"`，确认 img 加载
6. **fallback**：返回 `"NonExistentIcon"`，确认显示 `PanelLeft`（fallback）
7. **尺寸一致**：ActivityBar 图标与 TabWindow 标题栏图标视觉尺寸一致（均 `.small()`）

### Task 2 验证

1. **编译**：`cargo build -p rust-rml-ui` 通过
2. **手型光标**：demo 运行后，鼠标移入左侧窗口图标按钮，光标变为手型
3. **展开→收起**：点击 chevron，menu + title 向左滑动消失，chevron 从 `ChevronLeft` 变 `ChevronRight`
4. **收起→展开**：再次点击，menu + title 从左滑入出现，chevron 从 `ChevronRight` 变 `ChevronLeft`
5. **动画时长**：过渡约 0.25s，平滑无闪烁
6. **静止态**：动画结束后，展开态 menu+title 正常显示，收起态完全不占空间（`w_0`）
7. **无副作用重渲**：切换 tab、resize 窗口等不触发 chrome 滑动动画（因 `use_keyed_state` 检测到 `prev == current`）
8. **快速连击**：展开过程中点击收起，动画方向反转无卡顿（gpui 同 id 动画延续）

---

## 实施顺序

1. Task 1：修改 `crates/ui/src/components/activity_bar/icon.rs`（独立，无依赖）
2. Task 2：修改 `crates/ui/src/window/tab_window.rs`（独立，无依赖）
3. `cargo build -p rust-rml-ui` + `cargo run -p rust-rml-demo` 联合验证

两项任务互不依赖，可顺序或并行实施。
