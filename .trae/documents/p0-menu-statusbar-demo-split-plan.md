# P0 插槽修复 + Menu/StatusBar 数据绑定 + Demo 案例独立组件化

## Context

用户三项诉求：
1. **P0 插槽修复**：`slot_title` 缺失 + `slot_status` → `slot_footer` 重命名
2. **Menu/StatusBar MVVM 数据绑定**：实现 `<menu items={...}/>` 和 `<status_bar items={...}/>` RML 标签
3. **Demo 案例拆分**：将 5 个案例从 `main_window.rml` 内联拆分为独立 `#[component]` ViewModel + `.rml` 文件

第 3 项依赖**用户组件注册表**（让 codegen 识别 `<CounterCase />` 等用户自定义 PascalCase 标签），因此需先实现组件注册表。

---

## Current State（探索验证）

### Phase 1 已完成 ✓

以下 5 个文件已完成 slot_title 新增 + slot_footer 重命名：
- `crates/engine/src/compiler/codegen/shell.rs` — `partition_slot_children` 返回 7-tuple，含 `slot_title`/`slot_footer`
- `crates/engine/src/compiler/codegen/mod.rs` — 7-tuple 解构，Modern/Tab 均调用 partition，wrapper 调用已更新
- `crates/ui/src/window/tab_window.rs` — `status_slot` → `footer_slot`
- `crates/ui/src/window/modern_window.rs` — `status_slot` → `footer_slot`
- `demo/src/shell/main_window.rml` — `<slot_status>` → `<slot_footer>`

### Phase 2 进行中（1 处编译错误）

已完成的文件：
- `crates/ui/src/components/menu.rs` — 已创建，`IMenuItem` trait + `MenuItem` + `Menu` 容器
- `crates/ui/src/components/status_bar_wrapper.rs` — 已创建，`IStatusBarItem` + `RmlStatusBar` 包装
- `crates/engine/src/tags.rs` — 已添加 `is_special_lowercase_component()` + `menu`/`status_bar` 路由入口
- `crates/engine/src/compiler/component.rs` — 已添加 `items` setter + `is_container` 排除
- `crates/engine/src/compiler/codegen/mod.rs` — 已添加 lowercase 组件路由
- `crates/ui/src/components/mod.rs` / `lib.rs` / `prelude.rs` — 已添加 re-export

**唯一剩余错误**（`menu.rs:147`）：
```rust
let mut btn = Button::new((id.clone(), ix))  // SharedString: From<usize> not satisfied
```
原因：`ElementId` 元组 `(ElementId, usize)` 与 gpui-component `Button::new` 的 `impl Into<ElementId>` 不兼容。
修复方案：参照 `activity_bar.rs:225` 的黄金模板，用 `&str` 字面量 + usize 元组：
```rust
let mut btn = Button::new(("rml-menu", ix))
```

### Phase 3 未开始（用户组件注册表）

当前 `component_lookup`（`tags.rs:213`）是硬编码 match 表，无 fallback。
`gen_component`（`component.rs:37`）在 `component_lookup` 未命中时直接报错。
用户自定义 `#[component]` 标注的结构体（如 `CounterCase`）无法作为 `<CounterCase />` 标签嵌入父视图。

### Phase 5 未开始（Demo 案例拆分）

当前 5 个案例全部内联在 `demo/src/shell/main_window.rml:32-86`：
1. `welcome` — 空状态占位
2. `binding.counter` — 计数器（`count` 字段 + `on_click` 命令）
3. `binding.two-way` — 双向绑定（`name`/`age` 字段 + `#[validate(range(min=0,max=150))]`）
4. `components.button` — 按钮变体（`button_clicks` 字段 + `on_button_demo_click` 命令）
5. `i18n.basic` — 国际化（`on_switch_en`/`on_toggle_theme` 命令）

MainWindow 当前持有 `count`/`name`/`age`/`button_clicks` 等业务状态字段，违反 MVVM 职责分离。

### 关键探索结论

1. **ActivityBar 黄金模板**（`activity_bar.rs`）：`ITrait` + struct + `Vec<Arc<dyn ITrait>>` + 容器组件 + `into_arc()` + builder 方法
2. **ICommand 已 object-safe**（`crates/core/src/command.rs:31`）：`RelayCommand::new(cx, |this, cx| ...)` 捕获 `WeakEntity<T>`，可用于 `MenuItem::command()`
3. **Tree Stateful 模式**（`component.rs:67-70`）：`self.case_tree_state.as_ref().expect("init TreeState in on_loaded")` 是用户组件嵌入的模板
4. **`#[component]` 宏**（`macros/src/component.rs`）：生成 `impl IComponent`（`rml_tag()` 返回结构体名）+ `include!` 注入生成代码，RML 根节点 `<component>` 已由 `tags::is_root_tag` 识别
5. **build.rs**（`build/mod.rs:262`）：按文件名 stem → PascalCase 作为 `view_struct_name`，scanner 按 `#[window]`/`#[component]` 属性识别 struct
6. **I18nState/ThemeState 均为 Global**（`core/src/i18n.rs:53`、`core/src/theme.rs:87`）：可通过 `cx.observe_global::<I18nState>` 监听语言切换
7. **`to_snake_case`** 已存在于 `build/mod.rs:464`，用于 struct_name → entity_field 转换

---

## Phase 1: P0 插槽修复（已完成，仅记录）

所有 5 个文件已修改完毕，`cargo build` 应能通过 Phase 1 改动。无需额外操作。

---

## Phase 2: Menu/StatusBar MVVM 数据绑定（修复编译错误）

### 变更文件（1 个）

**`crates/ui/src/components/menu.rs`** — 修复 `Button::new` ID 错误

L147 改动：
```rust
// 之前（错误）：
let mut btn = Button::new((id.clone(), ix))

// 之后（正确，参照 activity_bar.rs:225）：
let mut btn = Button::new(("rml-menu", ix))
```

同时移除 `RenderOnce::render` 中未使用的 `let id = self.id;`（L133），因不再需要 `id.clone()`。

### 验证

```bash
cargo build -p rust-rml-ui
```

---

## Phase 3: 用户组件注册表（Demo 拆分的前置依赖）

### 数据结构

**`crates/engine/src/compiler/mod.rs`** — CodegenCtx 新增字段 + UserComponentInfo 结构

```rust
#[derive(Debug, Clone, Default)]
pub struct UserComponentInfo {
    pub struct_name: String,      // "CounterCase"
    pub entity_field: String,     // "counter_case"（snake_case）
}

// CodegenCtx 新增：
pub user_components: HashMap<String, UserComponentInfo>,
```

### 变更文件（4 个）

**1. `crates/engine/src/build/scanner.rs`**
- `StructMetadata` 新增 `pub is_component: bool` 字段
- 扫描时记录：`is_component = has_component_attr`（`#[component]` 标注的 struct）

**2. `crates/engine/src/build/mod.rs`**
- `scan_struct_metas` 返回的 map 中，收集所有 `is_component == true` 的 struct
- 构建 `CodegenCtx` 时（L273-284），从 struct_metas 提取 user_components：
  ```rust
  let user_components: HashMap<String, UserComponentInfo> = struct_metas
      .iter()
      .filter(|(_, m)| m.is_component)
      .map(|(name, _)| (name.clone(), UserComponentInfo {
          struct_name: name.clone(),
          entity_field: to_snake_case(name),
      }))
      .collect();
  ```
- 注入 `ctx.user_components = user_components`（需把 `struct_metas` 的生命周期调整到 ctx 构建之前可用）

**3. `crates/engine/src/compiler/component.rs`** — `gen_component` 增加用户组件 fallback

L37-42 改动：
```rust
let component = match tags::component_lookup(tag) {
    Some(c) => c,
    None => {
        // 内置路由表未命中：检查用户组件注册表
        if let Some(info) = ctx.user_components.get(tag) {
            return Ok((gen_user_component(info), false));
        }
        return Err(CodegenError {
            message: format!("unknown component: <{}> (not in gpui-component routing table or user component registry)", tag),
        });
    }
};
```

新增函数：
```rust
fn gen_user_component(info: &UserComponentInfo) -> String {
    let field = &info.entity_field;
    let struct_name = &info.struct_name;
    format!(
        "self.{}.as_ref().expect(\"init {} in on_loaded\").clone()",
        field, struct_name
    )
}
```

生成代码示例：`<CounterCase />` → `self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()`
返回 `Entity<CounterCase>`，因 `CounterCase: Render`（由 `#[component]` 生成），`Entity<T: Render>: IntoElement`。

**4. `crates/engine/src/compiler/codegen/mod.rs`** — `gen_element` 路由保持不变

`is_component(tag)` 已覆盖所有 PascalCase 标签（包括用户组件），`gen_component` 内部 fallback 到 user_components。
无需修改 `gen_element`，仅 `gen_component` 内部增加 fallback。

### 验证

新增集成测试 `crates/engine/tests/codegen_user_component_test.rs`：
- `<CounterCase />` 生成 `self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()`

---

## Phase 5: Demo 案例独立组件化

### 新增案例组件文件（5 个案例 × 2 文件 = 10 个新文件）

| 文件 | struct | 状态字段 | 命令 | 说明 |
|------|--------|---------|------|------|
| `demo/src/cases/welcome_case.rml` + `.rml.rs` | `WelcomeCase` | 无 | 无 | 纯展示组件 |
| `demo/src/cases/counter_case.rml` + `.rml.rs` | `CounterCase` | `count: i32` | `on_click` | 计数器 |
| `demo/src/cases/two_way_case.rml` + `.rml.rs` | `TwoWayCase` | `pub name: String`, `pub age: i32`（带 `#[validate(range(min=0,max=150))]`） | 无 | 双向绑定 |
| `demo/src/cases/button_case.rml` + `.rml.rs` | `ButtonCase` | `button_clicks: i32` | `on_button_demo_click` | 按钮变体 |
| `demo/src/cases/i18n_case.rml` + `.rml.rs` | `I18nCase` | 无（观察全局 I18nState） | `on_switch_en`, `on_toggle_theme` | i18n + 主题切换 |

### case .rml 模板（根节点 `<component>`）

**`welcome_case.rml`**：
```xml
<component>
    <div class="case-pane case-empty">
        <h2 class="case-title">{t("shell.pick_case")}</h2>
        <p class="case-hint">{t("shell.pick_case_hint")}</p>
    </div>
</component>
```

**`counter_case.rml`**：
```xml
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.counter.title")}</h2>
        <p class="count">{counter_text}</p>
        <Button ref="click_btn" label={t("demo.click_btn")} primary="" onclick={on_click} />
    </div>
</component>
```

**`two_way_case.rml`**：
```xml
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.two_way.title")}</h2>
        <div class="form">
            <input model={name} placeholder={t("demo.name_placeholder")} />
            <input model={age} placeholder={t("demo.age_placeholder")} />
            <p class="profile">{profile_summary}</p>
        </div>
    </div>
</component>
```

**`button_case.rml`**：
```xml
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.button.title")}</h2>
        <p class="count">{button_demo_text}</p>
        <div class="button-row">
            <Button label={t("case.button.primary")} primary="" onclick={on_button_demo_click} />
            <Button label={t("case.button.ghost")} ghost="" onclick={on_button_demo_click} />
            <Button label={t("case.button.danger")} danger="" onclick={on_button_demo_click} />
        </div>
    </div>
</component>
```

**`i18n_case.rml`**：
```xml
<component>
    <div class="case-pane">
        <h2 class="case-title">{t("case.i18n.title")}</h2>
        <p>{t("demo.hello")}</p>
        <Button label={t("menu.lang_en")} onclick={on_switch_en} />
        <Button label={t("menu.theme_toggle")} onclick={on_toggle_theme} />
    </div>
</component>
```

### case .rml.rs 模板

**`welcome_case.rml.rs`**：
```rust
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct WelcomeCase {}
```

**`counter_case.rml.rs`**：
```rust
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct CounterCase {
    count: i32,
}

impl CounterCase {
    #[computed]
    pub fn counter_text(&self) -> String {
        format!("点击次数：{}", self.count)
    }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
    }
}
```

**`two_way_case.rml.rs`**：
```rust
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct TwoWayCase {
    pub name: String,
    #[validate(range(min = 0, max = 150))]
    pub age: i32,
}

impl TwoWayCase {
    #[computed]
    pub fn profile_summary(&self) -> String {
        if self.name.is_empty() {
            format!("请输入姓名（年龄：{}）", self.age)
        } else {
            format!("你好，{}（{}岁）", self.name, self.age)
        }
    }
}
```

**`button_case.rml.rs`**：
```rust
use rml::prelude::*;

#[component]
#[derive(Default)]
pub struct ButtonCase {
    button_clicks: i32,
}

impl ButtonCase {
    #[computed]
    pub fn button_demo_text(&self) -> String {
        format!("按钮点击：{}", self.button_clicks)
    }

    #[command]
    pub fn on_button_demo_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.button_clicks += 1;
    }
}
```

**`i18n_case.rml.rs`**：
```rust
use gpui::AppContext;
use rml::prelude::*;
use rml_core::i18n::I18nExt;
use rml_core::theme::ThemeExt;

#[component]
#[derive(Default)]
pub struct I18nCase {}

impl ILifecycle for I18nCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        // 监听全局 I18nState 变化，语言切换时自动刷新
        cx.observe_global::<rml_core::i18n::I18nState>(|_this, cx| {
            cx.notify();
        }).detach();
    }
}

impl I18nCase {
    #[command]
    pub fn on_switch_en(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        cx.set_i18n("en-US");
    }

    #[command]
    pub fn on_toggle_theme(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        let next = if cx.current_theme() == "dark" { "light" } else { "dark" };
        cx.set_theme(next);
    }
}
```

### MainWindow 改造

**`demo/src/shell/main_window.rml.rs`** — 重构

移除（迁移到各 case 组件）：
- `count`/`name`/`age`/`button_clicks` 字段
- `counter_text`/`button_demo_text`/`profile_summary` computed
- `on_click`/`on_button_demo_click` 命令

新增：
- 5 个 case entity 字段：
  ```rust
  welcome_case: Option<Entity<WelcomeCase>>,
  counter_case: Option<Entity<CounterCase>>,
  two_way_case: Option<Entity<TwoWayCase>>,
  button_case: Option<Entity<ButtonCase>>,
  i18n_case: Option<Entity<I18nCase>>,
  ```
- `on_loaded` 中初始化各 case entity：
  ```rust
  if self.welcome_case.is_none() {
      self.welcome_case = Some(cx.new(|_| WelcomeCase::default()));
  }
  // ... 其余 4 个同理
  ```
- `on_loaded` 中观察全局 I18nState，语言切换时刷新 tab 标题：
  ```rust
  cx.observe_global::<rml_core::i18n::I18nState>(|this, cx| {
      this.i18n_version = this.i18n_version.wrapping_add(1);
      this.open_tabs.iter_mut().for_each(|tab| {
          tab.title = cx.t(cases::case_title_key(&tab.id)).to_string();
      });
      cx.notify();
  }).detach();
  ```
- `menu_items` computed（返回 `MenuItems`，绑定命令）：
  ```rust
  #[computed]
  pub fn menu_items(&self) -> MenuItems {
      let _ = self.i18n_version;
      vec![
          MenuItem::new("切换主题").command(...).into_arc(),
          MenuItem::new("English").command(...).into_arc(),
      ]
  }
  ```
  命令通过 `RelayCommand::new(cx, |this, cx| this.on_toggle_theme(...))` 在 `on_loaded` 中创建并缓存为 `Arc<dyn ICommand>` 字段（`theme_cmd`/`lang_cmd`）。
  
  **注**：`#[computed]` 不能调用 `RelayCommand::new`（需 `cx`），因此命令对象在 `on_loaded` 中创建存为字段，`menu_items` computed 引用字段。

- `status_items` computed（返回 `StatusBarItems`）：
  ```rust
  #[computed]
  pub fn status_items(&self) -> StatusBarItems {
      vec![
          StatusBarItem::new("Ready").align(StatusBarAlign::Left).into_arc(),
      ]
  }
  ```

保留：
- `on_switch_en`/`on_toggle_theme` 命令（供 menu_items 的命令调用）
- `on_panel_change`/`open_case`/`on_tab_click`/`on_chrome_toggle` 命令
- `tab_bar_items` computed

**`demo/src/shell/main_window.rml`** — 重构

```xml
<tab_window ...>
    <slot_left>
        <ContributionView host="shell.activity-bar" active_id={active_panel_id} on_active_change="on_panel_change" />
    </slot_left>

    <slot_menu>
        <menu items={menu_items} />
    </slot_menu>

    <slot_footer>
        <status_bar items={status_items} />
    </slot_footer>

    <div class="case-host">
        <div if={active_case_id == "welcome" || active_case_id == ""}>
            <WelcomeCase />
        </div>
        <div if={active_case_id == "binding.counter"}>
            <CounterCase />
        </div>
        <div if={active_case_id == "binding.two-way"}>
            <TwoWayCase />
        </div>
        <div if={active_case_id == "components.button"}>
            <ButtonCase />
        </div>
        <div if={active_case_id == "i18n.basic"}>
            <I18nCase />
        </div>
    </div>
</tab_window>
```

**`demo/src/cases/mod.rs`** — 新增模块声明

```rust
pub mod catalog;
#[path = "welcome_case.rml.rs"] pub mod welcome_case;
#[path = "counter_case.rml.rs"] pub mod counter_case;
#[path = "two_way_case.rml.rs"] pub mod two_way_case;
#[path = "button_case.rml.rs"] pub mod button_case;
#[path = "i18n_case.rml.rs"] pub mod i18n_case;

pub use catalog::{case_title_key, init_tree_state, refresh_tree_state, OpenTab};
pub use button_case::ButtonCase;
pub use counter_case::CounterCase;
pub use i18n_case::I18nCase;
pub use two_way_case::TwoWayCase;
pub use welcome_case::WelcomeCase;
```

---

## 实现顺序

```
Phase 1: P0 插槽修复                    — 已完成 ✓
Phase 2: menu/status_bar 编译错误修复    — 1 文件，~5 行
Phase 3: 用户组件注册表                  — 4 修改文件，~80 行
Phase 5: Demo 案例拆分                   — 10 新文件 + 3 修改，~400 行
Phase 6: 验证                            — cargo build/test
```

Phase 3 必须在 Phase 5 之前完成（Phase 5 的 `<CounterCase />` 依赖 Phase 3 的用户组件注册表）。

---

## 验证步骤

1. `cargo build -p rust-rml-ui` — Phase 2 修复后 ui crate 编译通过
2. `cargo build --workspace` — 全工作区编译通过
3. `cargo test --workspace` — 现有测试不回归
4. 新增集成测试 `crates/engine/tests/codegen_user_component_test.rs`：
   - `<CounterCase />` 生成 `self.counter_case.as_ref().expect("init CounterCase in on_loaded").clone()`
5. 运行 demo `cargo run -p rust-rml-demo`，验证：
   - 菜单栏 `<menu items={menu_items} />` 显示数据绑定项，点击执行命令
   - 状态栏 `<status_bar items={status_items} />` 显示数据绑定项
   - Tab 切换显示各独立案例组件，状态独立
   - I18nCase 中切换语言/主题后，MainWindow tab 标题同步刷新

---

## Assumptions & Decisions

1. **5 个案例全部拆分**：用户明确要求"5 个案例"，包括无状态的 welcome（`WelcomeCase` 为空 struct）
2. **I18nCase 自持语言/主题切换**：直接调用 `cx.set_i18n`/`cx.set_theme`（全局操作），MainWindow 通过 `cx.observe_global::<I18nState>` 监听并刷新 tab 标题
3. **menu_items 命令缓存为字段**：`#[computed]` 无法访问 `cx` 创建 `RelayCommand`，因此 `theme_cmd`/`lang_cmd` 在 `on_loaded` 中创建为 `Arc<dyn ICommand>` 字段，computed 引用字段
4. **用户组件嵌入用 `Option<Entity<T>>`**：惰性初始化，`on_loaded` 中 `cx.new(|_| T::default())` 创建
5. **status_items 简化**：demo 仅展示静态 "Ready" 文本，通过 `StatusBarItems` 数据绑定（可扩展为动态状态）
6. **case .rml.rs 的 `#[component]` 宏无需改动**：已生成 `impl IComponent` + `include!`，RML 根节点 `<component>` 已支持
7. **welcome_case 无 ILifecycle**：空 struct 无需 `on_loaded`，`#[derive(Default)]` 足够
8. **TwoWayCase 的 `#[validate]` 迁移**：`age` 字段的 `#[validate(range(min=0,max=150))]` 随字段迁移到 TwoWayCase，codegen 自动生成校验代码
