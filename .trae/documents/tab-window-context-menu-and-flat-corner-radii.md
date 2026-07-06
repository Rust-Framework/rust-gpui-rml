# TabWindow 右键菜单补齐三项 + 删除 Flat 圆角特殊配置

## Context

用户反馈两个问题：

1. **Tab 标签的"上圆下方"圆角配置冗余**。当前 `crates/ui/src/components/tab/tab.rs` 的 `TabVariant::corner_radii` 方法对 `TabVariant::Flat + selected` 做了非对称圆角特殊处理（上两角 `cx.theme().radius`、下两角 `0`），模拟浏览器 tab"圆顶方底接 body"的视觉。用户偏好"避免过度追求圆角和卡片"，要求删除这个配置，让 Flat 走默认分支 `_ => Corners::all(self.radius(size, cx))`（即四角统一为 `radius()` 返回值，对 Flat 而言是 `px(0.)`，即四角直角）。

2. **TabWindow 右键菜单只显示"关闭"一项**。根因不是被禁用，而是**没注册**：TabBar 已经完整实现了 Close / Close Others / Close All 三项菜单逻辑（[tab\_bar.rs:686-757](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab_bar.rs#L686-L757)），i18n key（`rml.tab.close` / `rml.tab.close_all` / `rml.tab.close_others`）在 `demo/assets/i18n/zh-CN.json` 与 `en-US.json` 已就绪。但 TabWindow 只透传了 `on_tab_close` → `tab_bar.on_close(...)`（[tab\_window.rs:517-519](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L517-L519)），**完全没有** `on_tab_close_all` / `on_tab_close_others` 字段或 setter，导致 TabBar 的 `self.on_close_all` / `self.on_close_others` 始终为 `None`，对应菜单项从未注册（不是 disabled）。RML 引擎 codegen `shell.rs` 也只为 `on_tab_close` 生成了 `.on_tab_close(...)` 调用，未建立两个新事件的属性映射；`props_registry.rs` 的 `tab-window` 属性列表也只列了 `on_tab_close`。

memory 中 [project\_memory.md](file:///c:/Users/lusid/.trae-cn/memory/projects/-e-GitCode-RF-rust-gpui-rml/project_memory.md) 明确要求 "TabWindow must natively include 'Close/Close All/Close Others' menu items with business callback extension points"，[user\_profile.md](file:///c:/Users/lusid/.trae-cn/memory/user_profile.md) 也有同样偏好。当前实现违反此约束。

**目标**：删除 Flat 圆角特殊配置；让 TabWindow 暴露 `on_tab_close_all` / `on_tab_close_others` 两个回调接口，并在 demo 层对接好业务逻辑，使右键菜单原生显示三项且可工作。

## 改动方案

### Step 1 — 删除 Flat 圆角特殊分支

**文件**：[crates/ui/src/components/tab/tab.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs#L416-L427)

把 `corner_radii` 方法简化为单一默认分支：

```rust
fn corner_radii(&self, size: Size, selected: bool, disabled: bool, cx: &App) -> Corners<Pixels> {
    let _ = (selected, disabled); // 保留参数签名以备未来变体使用
    Corners::all(self.radius(size, cx))
}
```

或者更简洁地直接去掉 `selected` / `disabled` 参数（但需同步更新 [tab.rs:785-787](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs#L785-L787) 调用方）。

**采用最小改动版本**：保留参数签名（避免改调用方），仅删除 `TabVariant::Flat if selected && !disabled => Corners { ... }` 分支与上方注释 `// Selected Flat tab: rounded top, square bottom to meet the body.`。

**视觉影响**：TabWindow（用 `.flat()`）选中 Tab 顶部失去圆角，变为纯矩形色块，符合用户偏好。其他 5 个 variant（Tab/Outline/Pill/Segmented/Underline）不受影响。无测试/demo 断言依赖此分支。

### Step 2 — TabWindow 暴露 close\_all / close\_others 接口

**文件**：[crates/ui/src/window/tab\_window.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs)

参照现有 `on_tab_close`（[line 156, 259-265, 517-519](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L156)）模式：

1. **结构体字段**（在 [line 158](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L158) [`on_tab_close`](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L158) [后](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L158) 增加两行）：

   ```rust
   on_tab_close: Option<TabClickHandler>,
   on_tab_close_all: Option<ChromeToggleHandler>,        // 新增：Fn(&mut Window, &mut App)
   on_tab_close_others: Option<TabClickHandler>,          // 新增：Fn(usize, &mut Window, &mut App)
   ```

   类型别名 `TabClickHandler` / `ChromeToggleHandler` 已在 [line 35-36](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L35-L36) 定义，签名与 TabBar 的 `on_close_all` / `on_close_others` 完全对齐。

2. **`new()`** **初始化**（[line 182](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L182) [`on_tab_close: None,`](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L182) [后](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L182)）追加 `on_tab_close_all: None, on_tab_close_others: None,`。

3. **builder 方法**（在 [line 265](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L265) [`on_tab_close`](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L265) [闭合](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L265) [`}`](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L265) [后](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L265) 追加，紧贴 `on_tab_close` 保持语义聚合）：

   ```rust
   /// "关闭全部"菜单项触发时调用（TabWindow 透传到 TabBar::on_close_all）。
   pub fn on_tab_close_all(
       mut self,
       f: impl Fn(&mut Window, &mut App) + 'static,
   ) -> Self {
       self.on_tab_close_all = Some(Rc::new(f));
       self
   }

   /// "关闭其他"菜单项触发时调用，参数为保留 tab 的索引
   /// （TabWindow 透传到 TabBar::on_close_others）。
   pub fn on_tab_close_others(
       mut self,
       f: impl Fn(usize, &mut Window, &mut App) + 'static,
   ) -> Self {
       self.on_tab_close_others = Some(Rc::new(f));
       self
   }
   ```

4. **RenderOnce 透传**（在 [line 517-519](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L517-L519) [`on_tab_close`](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L517-L519) [透传块后](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L517-L519) 追加）：

   ```rust
   if let Some(on_close_all) = self.on_tab_close_all {
       tab_bar = tab_bar.on_close_all(move |window, cx| on_close_all(window, cx));
   }
   if let Some(on_close_others) = self.on_tab_close_others {
       tab_bar = tab_bar.on_close_others(move |ix, window, cx| on_close_others(*ix, window, cx));
   }
   ```

   注意 `*ix`：TabBar 的 `on_close_others` 签名是 `Fn(&usize, ...)`（[tab\_bar.rs:211-217](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab_bar.rs#L211-L217)），需解引用。

### Step 3 — RML codegen 注册两个事件属性

**文件**：[crates/engine/src/compiler/codegen/shell.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L355-L384)

参照 [line 355-369](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L355-L369) [`on_tab_close`](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L355-L369) [分支](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L355-L369) 模式，在 `on_tab_close` 分支后、`on_chrome_toggle` 分支前插入两个新分支：

* `on_tab_close_all` → 闭包签名 `move |_window: &mut gpui::Window, app: &mut gpui::App|`（无 index 参数，参照 [line 370-384](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L370-L384) [`on_chrome_toggle`](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L370-L384)）

* `on_tab_close_others` → 闭包签名 `move |index: usize, _window: &mut gpui::Window, app: &mut gpui::App|`（参照 `on_tab_close`）

### Step 4 — props\_registry 注册属性名

**文件**：[crates/engine/src/compiler/props\_registry.rs:188-193](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs#L188-L193)

在 `tab-window` 属性列表 `"on_tab_close"` 后追加 `"on_tab_close_all", "on_tab_close_others"`：

```rust
("tab-window", &[
    "title", "width", "height", "startup", "icon",
    "tabs", "selected_index", "show_chrome",
    "left_size", "right_size", "bottom_size",
    "on_tab_click", "on_tab_close",
    "on_tab_close_all", "on_tab_close_others",   // 新增
    "on_chrome_toggle", "tab_item_template",
]),
```

未注册时 [shell.rs:329-337](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L329-L337) 会发 warning 并 silently drop。

### Step 5 — demo RML 绑定两个事件

**文件**：[demo/src/shell/main\_window.rml:9](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml#L9)

在 `on-tab-close="on_tab_close"` 行后追加：

```xml
on-tab-close-all="on_tab_close_all"
on-tab-close-others="on_tab_close_others"
```

### Step 6 — demo 实现 close\_all / close\_others 命令

**文件**：[demo/src/shell/main\_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L386-L398)

在 `on_tab_close` 命令后追加两个 `#[command]` 方法。复用 `ObservableVec::clear()`（[observable.rs:65](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/observable.rs#L65)）与 `push()`，**不修改** `IWorkbenchManager` trait（保持 surgical change）。

参照现有 `on_tab_close` 的 `IWorkbenchManager::close` 实现（[line 488-509](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L488-L509)）中的 activated 重算逻辑：

```rust
/// 关闭全部 workbench：清空后激活项置 None，bump `activated` 触发 selected_tab computed 失效。
#[command]
pub fn on_tab_close_all(&mut self, cx: &mut Context<Self>) {
    if self.workbenches.is_empty() {
        return;
    }
    self.workbenches.clear();
    *self.activated.write().unwrap() = None;
    self.__rml_bump_version("activated");
    cx.notify();
}

/// 关闭其他 workbench：仅保留 index 对应项。clear + 重 push 保留项，
/// 避免 remove_where 仅移除首个导致的循环；activated 切到保留项。
#[command]
pub fn on_tab_close_others(&mut self, index: usize, cx: &mut Context<Self>) {
    let snapshot = self.workbenches.snapshot();
    let keep = match snapshot.get(index).cloned() {
        Some(wb) => wb,
        None => return,
    };
    self.workbenches.clear();
    self.workbenches.push(keep.clone());
    *self.activated.write().unwrap() = Some(keep);
    self.__rml_bump_version("activated");
    cx.notify();
}
```

**注意点**：

* `clear()` 与 `push()` 各触发一次 ObservableVec 内部版本递增 + flume send，背景任务会 `cx.notify()` 两次，无害。

* `__rml_bump_version("activated")` 必须手动调用（与 `on_tab_close` 一致），因为 workbenches 的 bump 不会触发 `selected_tab` computed 失效（computed 同时依赖 `activated` 版本）。

* `on_tab_close_others` 不调用 `IWorkbenchManager::close` N 次，因为 close 内部的 activated 跳转逻辑（关激活项→切 N-1）会与"保留 index 项"语义冲突。

## 不修改的部分

* **TabBar**：三项菜单构造逻辑（[tab\_bar.rs:686-757](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab_bar.rs#L686-L757)）、`on_close_all` / `on_close_others` setter（[tab\_bar.rs:199-217](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab_bar.rs#L199-L217)）、disabled 判定（`closable_count` / `other_closable_count`）、i18n key 全部已就绪，**无需修改**。

* **Tab**：`context_menu_provider` 透传机制（[tab.rs:489-491, 688-695, 1055-1059](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/tab/tab.rs)）已完备，**无需修改**。

* **i18n 文件**：`zh-CN.json` / `en-US.json` 的 `rml.tab.close` / `rml.tab.close_all` / `rml.tab.close_others` 已存在（[zh-CN.json:3,5,7](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/i18n/zh-CN.json#L3)），**无需修改**。

* **IWorkbenchManager trait**：不新增 `close_all` / `close_others` 方法，demo 直接操作 `ObservableVec` + `activated`，保持 trait 最小化。

## 验证

### 编译验证

```powershell
cargo check -p rust-rml-ui
cargo check -p rust-rml-engine
cargo check
```

### 功能验证（手动）

1. 启动 demo：`cargo run -p demo`
2. 在 TabWindow 标签上**右键** → 应看到三个菜单项：「关闭」「关闭其他」「关闭全部」（中文 i18n 生效）
3. 测试用例：

   * 单 tab 时右键 → "关闭其他"和"关闭全部"应 disabled（`other_closable_count=0` / `closable_count=1`）

   * 多 tab 时右键第 2 个 → "关闭"disabled（因 demo `<Tab closable />` 全部可关，应可点）；点"关闭其他"→ 只剩第 2 个；点"关闭全部"→ tab 全空

   * 关闭后激活项切换符合预期：close\_all → 无激活；close\_others → 激活保留项
4. 圆角视觉验证：选中 Tab 顶部不再有圆角，与未选中 Tab 视觉一致（仅 bg/fg 区分），符合"避免过度圆角"偏好

### 单元测试

* TabBar 现有测试应全部通过（无修改）：

  ```powershell
  cargo test --lib -p rust-rml-engine code_editor
  cargo test --lib -p rust-rml-ui tab
  ```

* 若 shell.rs 有 codegen 测试覆盖 `on_tab_close`，可考虑加 `on_tab_close_all` / `on_tab_close_others` 的快照测试（可选，参考 [shell.rs:547](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L547) [`gen_tab_window_wrapper_with_slot_tabs`](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/codegen/shell.rs#L547)）。

## 关键文件清单

| 文件                                             | 改动类型                       |
| ---------------------------------------------- | -------------------------- |
| `crates/ui/src/components/tab/tab.rs`          | 删除 Flat 圆角分支（Step 1）       |
| `crates/ui/src/window/tab_window.rs`           | 增字段/setter/透传（Step 2）      |
| `crates/engine/src/compiler/codegen/shell.rs`  | 增 2 个 Event 分支（Step 3）     |
| `crates/engine/src/compiler/props_registry.rs` | 注册 2 个属性（Step 4）           |
| `demo/src/shell/main_window.rml`               | 增 2 个事件绑定（Step 5）          |
| `demo/src/shell/main_window.rml.rs`            | 增 2 个 `#[command]`（Step 6） |

