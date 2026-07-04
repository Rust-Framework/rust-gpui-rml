# 计划：ActivityBar 图标能力增强 + TabWindow 标题栏展开/收起动画

> 本计划承接上一会话（已失上下）的成果。经 Phase 1 探索确认：**两项代码改动均已落地**，
> 本计划聚焦于"完成态确认 + 编译验证 + 视觉验证 + 已知边缘问题记录"。

---

## 一、当前状态分析（Phase 1 探索结论）

### Task 1：ActivityBar icon 充分利用 Icon 组件能力 —— ✅ 代码已落地

**文件**：`crates/ui/src/components/activity_bar/icon.rs`

`resolve_icon` 已重构为 5 级优先级解析，充分利用 `Icon` 组件能力：

| 优先级 | 输入 | 输出 | 使用的 Icon 能力 |
|--------|------|------|------------------|
| 1 | URL（`file:`/`http:`/`https:`/`://`） | `gpui::img(s)` | Icon 无法加载 URL，回退 img |
| 2 | 命名图标（如 `"BookOpen"`） | `Icon::new(name).small()` | `IconName` 枚举 + `Sizable` |
| 3 | SVG 资产路径（含 `/` 或 `.svg`） | `Icon::default().path(s).small()` | `path()` 自定义资产 |
| 4 | 其他未匹配字符串 | `Icon::new(PanelLeft).small()` | fallback |
| 5 | `None` | `Icon::new(PanelLeft).small()` | fallback |

- `parse_icon_name` 覆盖约 90 个图标（gpui-component-assets 全部 SVG）
- 所有 Icon 实例统一 `.small()`，与 TabWindow 标题栏图标尺寸一致
- `is_url` / `is_asset_path` 辅助函数边界清晰

### Task 2：TabWindow 标题栏手型 + 展开/收起滑动动画 —— ✅ 代码已落地

**文件**：`crates/ui/src/window/tab_window.rs`

**改动点 1：手型光标**（L321）
```rust
Button::new("tab-window-chrome-toggle")
    .text()
    .cursor_pointer()  // ← 新增
    ...
```
- 利用 Button 实现 `Styled` trait，`.cursor_pointer()` 通过 `StyleRefinement`
  在 Button 内部 `cursor_default()` 之后经 `.refine_style(&self.style)` 应用，覆盖默认。

**改动点 2：chevron 图标切换**（L312-316）
```rust
let chevron = if show_chrome { IconName::ChevronLeft } else { IconName::ChevronRight };
```
- 展开时显示 `ChevronLeft`（指向左，提示可收起）
- 收起时显示 `ChevronRight`（指向右，提示可展开）

**改动点 3：menu slot + title 随 show_chrome 展开/收起 + 滑动动画**（L357-435）
- 始终构建 `prefix_parts`（menu_slot + title），不再按 `show_chrome` 闸门
- 用 `h_flex().overflow_hidden()` 包裹作为动画容器
- 动画机制：
  - `window.use_keyed_state("tab-window-chrome-anim", cx, |_, _| self.show_chrome)`
    跟踪上一次提交的 `show_chrome` 值（init 仅首次渲染调用）
  - `chrome_changed = prev_chrome != self.show_chrome` 检测变更
  - 变更时：`with_animation("tab-window-chrome-slide", anim, |this, delta| {...})`
    - `progress = if target_chrome { delta } else { 1.0 - delta }`
    - `this.max_w(px(800.0) * progress).opacity(progress)` 实现左右滑动 + 淡入淡出
  - 动画时长 0.25s，缓动 `cubic_bezier(0.4, 0., 0.2, 1.)`
  - 动画结束后 `cx.spawn` 延迟 0.25s 更新 `chrome_state` → 触发重渲使 `chrome_changed = false`
- 静止态：展开时正常显示；收起时 `w_0().opacity_0()`

**数据绑定链路**（已验证）：
- `demo/src/shell/main_window.rml` L10-11：`show-chrome={show_chrome}` + `on-chrome-toggle="on_chrome_toggle"`
- `main_window.rml.rs` L215-217：`on_chrome_toggle` 命令翻转 `self.show_chrome`
- `main_window.rml.rs` L123：`on_loaded` 初始化 `self.show_chrome = true`

---

## 二、待执行事项

### 步骤 1：编译验证（首要）
```bash
cargo build -p rust-rml-ui
```
**预期风险点**（需在编译时关注）：
1. `Icon::default().path(s)` —— 确认 `Icon` 实现 `Default` 且 `path()` 接受 `&str`/`SharedString`
2. `use_keyed_state` 闭包签名 —— `|_, _| self.show_chrome` 需匹配 `FnOnce(&mut Window, &mut App) -> bool`
3. `chrome_state.read(cx)` 返回类型 —— 需可解引用为 `bool`
4. `state.update(cx, |s, _| *s = target_chrome)` —— 确认 `Entity::update` 签名
5. `with_animation` 闭包捕获 `target_chrome`（`bool` Copy）—— 应无借用问题
6. `SmallVec::new()` —— 已 `use smallvec::SmallVec`

若编译失败，按错误信息定点修复（不扩大改动范围）。

### 步骤 2：Demo 编译验证
```bash
cargo build -p rust-rml-demo
```
确保 demo 也能编译通过（验证 RML 模板与 Rust 代码的绑定一致）。

### 步骤 3：视觉验证（可选但推荐）
```bash
cargo run -p rust-rml-demo
```
**验证清单**：
- [ ] 鼠标移入左上角窗口图标按钮，光标变为手型
- [ ] 点击按钮，chevron 从 `ChevronLeft` 切换为 `ChevronRight`（或反之）
- [ ] 切换时 menu slot 和窗口标题随动画左右滑动收起/展开（0.25s 平滑过渡）
- [ ] 收起状态下，menu slot 和标题完全隐藏（宽度 0、透明度 0）
- [ ] 展开状态下，menu slot 和标题正常显示
- [ ] ActivityBar 图标正确显示（如 `BookOpen` 命名图标）

---

## 三、已知边缘问题（记录，不在本次范围内修复）

**快速连续点击的动画状态竞争**：
- 用户在 0.25s 动画未完成时再次点击 toggle，可能出现短暂的视觉跳变
- 原因：`chrome_state` 更新有 0.25s 延迟，期间 `prev_chrome` 仍为旧值，
  若 `show_chrome` 翻转回原值，`chrome_changed` 误判为 `false`，渲染静止态
- 系统会自纠（spawned task 触发重渲后恢复正确状态），但存在短暂闪烁
- **决策**：当前实现满足用户"切换过程需要左右滑动过渡动画效果"的核心诉求，
  边缘竞争问题留待后续如需精进时处理（可考虑用动画 id 计数器或取消上次 spawn task）

---

## 四、假设与决策

1. **假设**：上一会话的代码改动已保存到磁盘（Phase 1 探索已确认）
2. **假设**：`use_keyed_state` API 签名与 gpui-component text_view.rs 用法一致（已验证）
3. **决策**：不重新实现已完成的代码，仅做编译与视觉验证
4. **决策**：若编译失败，仅做最小修复，不重构现有逻辑
5. **决策**：动画边缘竞争问题不在本次范围内处理

---

## 五、验证完成标准

- [ ] `cargo build -p rust-rml-ui` 成功
- [ ] `cargo build -p rust-rml-demo` 成功
- [ ] （可选）`cargo run -p rust-rml-demo` 视觉验证通过上述清单
