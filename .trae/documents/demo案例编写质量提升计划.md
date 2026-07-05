# Demo 案例编写质量提升计划

> 参考页面:
> - 欢迎页: https://ant-design.antgroup.com/components/overview-cn (组件总览卡片网格)
> - 组件示例页: https://ant-design.antgroup.com/components/button-cn (分演示卡片)
>
> 用户反馈: 示例页面组件显示不合理,且无法正常响应鼠标、键盘操作。

## 一、Summary (总览)

本计划针对 demo 模块的两类页面进行质量提升:

1. **欢迎页 (WelcomeCase)** — 仿 Ant Design 组件总览页,按 group 分组渲染卡片网格,点击卡片打开对应 case Tab。
2. **组件示例页 (ButtonCase)** — 仿 Ant Design Button 文档页,以多张演示卡片组织内容,每张卡片聚焦一个用法主题。
3. **通用交互修复** — 修复窗口拖拽区域 (`WindowControlArea::Drag`) 拦截 title 行鼠标事件导致 Docs 按钮无响应的问题。

**改造范围**: 仅重写 `welcome_case` 与 `button_case` 两个案例;不重写其它 case。`WindowControlArea::Drag` 通用问题已通过删除无功能 Docs 按钮解决(改动 D 已完成)。

## 二、Current State Analysis (现状分析)

### 已完成改动 (从上下文继承)

| 改动 | 文件 | 状态 |
|------|------|------|
| D | `demo/src/shell/main_window.rml` | ✅ title 槽位已无 Docs Button,仅保留 `<template slot="menu">` |
| E | `demo/assets/styles.css` | ✅ 已新增 `.welcome-pane` / `.welcome-grid` / `.welcome-grid-row` / `.welcome-card-title` 样式 (行 148-201) |
| F | `demo/assets/i18n/zh-CN.json` + `en-US.json` | ✅ 已新增 `shell.welcome_title` / `shell.welcome_subtitle` |

### 待完成改动

#### 改动 A: 修订 `welcome_case.rml.rs` (ViewModel)

**问题**: 当前实现使用 `#[computed] pub fn grouped_items(&self) -> Vec<CaseNavItemGroup>`,但 RML codegen 的 `each` 指令在普通元素上总是生成 `self.{iterable}.iter()` (见 `crates/engine/src/compiler/codegen/node.rs:282-288`),不支持方法调用。若在模板写 `<div each={group in grouped_items}>`,将生成 `self.grouped_items.iter()` (字段访问,非方法调用),编译失败。

**codegen 限制验证** (节点代码生成):
- 普通元素 `each`: `self.{iterable}.iter().map(|item| { ... })` — 不处理 loop_vars,不支持方法调用
- `<component each={...} content={...} />`: 正确处理 loop_vars (行 124-138) — 必须用此模式

#### 改动 B: 重写 `welcome_case.rml` (模板)

**问题**: 当前模板仍是空占位:
```html
<component>
    <div v-flex="" class="case-pane case-empty">
        <h2>{t("shell.pick_case")}</h2>
        <p>{t("shell.pick_case_hint")}</p>
    </div>
</component>
```

需要改为 Ant Design 组件总览风格的分组卡片网格。

#### 改动 C: 重写 `button_case.rml` + `.rml.rs` (Ant Design 风格)

**问题**: 当前 `button_case.rml` 仅 3 个 demo-section(基础变体 / 状态变体 / 尺寸变体)塞在一个 Card 内,不符合 Ant Design 一卡一 demo 的清晰布局;`button_case.rml.rs` 字段命名(`button_clicks`)与命令(`on_button_demo_click`)过于笼统。

**目标**: 改为多卡片结构,每张 Card 聚焦一个主题,每张 Card 内含:标题 + 简短说明 + 演示区 + (可选)状态文本。

**codegen 限制验证** (Button 属性):
- `label` / `primary` / `ghost` / `danger` / `disabled` / `selected` / `size="small|medium|large"` / `compact` — 支持
- `icon` — 不在 `component_static_setter` 匹配表,静默丢弃 → **不演示 icon**
- `loading=""` — 生成 `.loading()` 无参调用,但 Button 方法签名是 `.loading(bool)` → **不演示 loading**
- `ButtonGroup` — 已在 tags.rs:317-318 注册为 Stateless 容器组件,可使用

## 三、Proposed Changes (具体改动)

### 改动 A: 修订 `demo/src/cases/welcome_case.rml.rs`

**目标**: 将 `grouped_items` 从 `#[computed]` 方法改为常规字段,新增 `render_group` (命令式构建卡片行) 与 `open_case` (委托 MainWindow 打开 Tab) 方法。

**当前结构** (要点):
```rust
pub struct WelcomeCase {
    pub items: Vec<CaseNavItem>,
}

impl WelcomeCase {
    fn refresh_items(&mut self, cx: &mut Context<Self>) { ... }

    #[computed]
    pub fn grouped_items(&self) -> Vec<CaseNavItemGroup> { ... }

    #[command]
    pub fn on_case_click(&mut self, case_id: SharedString, _: &ClickEvent, cx: &mut Context<Self>) { ... }
}
```

**修订后结构** (要点):
```rust
pub struct WelcomeCase {
    pub items: Vec<CaseNavItem>,
    pub grouped_items: Vec<CaseNavItemGroup>,  // 新增字段,在 refresh_items 中预计算
}

impl WelcomeCase {
    fn refresh_items(&mut self, cx: &mut Context<Self>) {
        // 1. 拷贝 cases → self.items
        // 2. 调用 compute_grouped_items(&self.items) → self.grouped_items
        // 3. self.__rml_bump_version("items");
        // 4. self.__rml_bump_version("grouped_items");
    }

    fn compute_grouped_items(items: &[CaseNavItem]) -> Vec<CaseNavItemGroup> {
        // 原 #[computed] 方法体移至此处的纯函数
    }

    /// 命令式构建单个分组的渲染树: h3 标题 + 卡片行
    /// 由模板 `<component each={group in grouped_items} content={self.render_group(group, _window, cx)} />` 调用
    pub fn render_group(
        &self,
        group: &CaseNavItemGroup,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{div, IntoElement, ParentElement, Styled};
        use rml_ui::Card;

        let cards: Vec<gpui::AnyElement> = group.items.iter().map(|item| {
            let case_id = item.id.clone();
            Card::new(("welcome_card", item.id.to_string()))
                .hoverable()
                .child(
                    div()
                        .class("welcome-card-title")
                        .child(item.name.clone())
                )
                .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
                    this.open_case(case_id.clone(), cx);
                }))
                .into_any_element()
        }).collect();

        div()
            .class("welcome-grid")
            .child(div().child(group.label.clone()))
            .child(div().class("welcome-grid-row").children(cards))
            .into_any_element()
    }

    /// 卡片点击 → 委托 MainWindow::open_case 打开对应 Tab
    pub fn open_case(&mut self, case_id: SharedString, cx: &mut Context<Self>) {
        if let Some(main) = cx
            .get_service::<MainWindowRef>()
            .and_then(|r| r.0.upgrade())
        {
            main.update(cx, |mw, cx| {
                mw.open_case(case_id.to_string(), cx);
            });
        }
    }
}
```

**删除的成员**:
- `#[computed] pub fn grouped_items(...)` — 改为字段 + `compute_grouped_items` 纯函数
- `#[command] pub fn on_case_click(...)` — 改为 `pub fn open_case(...)` (普通方法,由 `render_group` 内 `cx.listener` 闭包调用)

**新增 import**:
- `gpui::{div, AnyElement, IntoElement, ParentElement, Styled}` (用于 `render_group`)
- `rml_ui::Card`

**验证点**:
- `Card` 实现 `InteractiveElement` (支持 `.on_click()`) — 已确认 (`crates/ui/src/components/card.rs:137-141`)
- `Card::new(id: impl Into<ElementId>)` 接受元组 ID — 已确认 (card.rs:56-69)
- `cx.listener` 闭包签名 `|this, ev, window, cx|` — 标准 GPUI 模式

---

### 改动 B: 重写 `demo/src/cases/welcome_case.rml`

**目标**: 渲染 Ant Design 风格的分组卡片网格。

**模板**:
```html
<component>
    <div v-flex="" class="case-pane welcome-pane">
        <h2>{t("shell.welcome_title")}</h2>
        <p>{t("shell.welcome_subtitle")}</p>
        <component each={group in grouped_items} content={self.render_group(group, _window, cx)} />
    </div>
</component>
```

**关键点**:
- `<component each={...} content={...} />` 是 codegen 唯一支持 loop_vars 的迭代模式 (node.rs:124-138)
- `content` 表达式可引用 `self` / `_window` / `cx` (render 方法作用域内可用)
- `each={group in grouped_items}` 中 `grouped_items` 是 `self.grouped_items` 字段,codegen 生成 `self.grouped_items.iter().map(|group| self.render_group(group, _window, cx))`

**CSS 已就绪** (改动 E 已完成):
- `.welcome-pane` — align-items: stretch, gap: 16px, overflow-y: auto
- `.welcome-grid` — flex column, max-width 960px, 居中
- `.welcome-grid-row` — flex row, wrap, gap 12px
- `.welcome-grid-row > .card` — width 160px, cursor pointer

**验证点**:
- `welcome-pane` 与 `case-pane` 组合后:`case-pane` 提供 padding 24px + height 100%,`welcome-pane` 覆盖 align-items/text-align/gap/overflow
- 卡片点击 → `Card::on_click` → `cx.listener` 闭包 → `open_case` → `MainWindow::open_case` → 打开对应 Tab

---

### 改动 C: 重写 `button_case.rml` + `.rml.rs` (Ant Design 风格)

**目标**: 多卡片结构,每张 Card 一个主题,与 Ant Design Button 文档页一致。

#### C-1: `button_case.rml.rs` (ViewModel)

**修订后结构**:
```rust
#[contribute(host_id = "demo.shell", id = "components.button", kind = "case", group = "components", order = 11)]
#[component]
#[derive(Default)]
pub struct ButtonCase {
    pub basic_clicks: i32,        // 基础用法点击计数
    pub is_disabled: bool,         // 禁用状态切换
    pub is_selected: bool,         // 选中状态切换
}

impl IContribution for ButtonCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.button.title") }
}

impl ButtonCase {
    #[computed]
    pub fn basic_click_text(&self) -> String {
        format!("点击次数：{}", self.basic_clicks)
    }

    #[computed]
    pub fn disabled_status_text(&self) -> String {
        if self.is_disabled { "当前：禁用" } else { "当前：可用" }.to_string()
    }

    #[computed]
    pub fn selected_status_text(&self) -> String {
        if self.is_selected { "当前：选中" } else { "当前：未选中" }.to_string()
    }

    #[command]
    pub fn on_basic_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.basic_clicks += 1;
    }

    #[command]
    pub fn on_toggle_disabled(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_disabled = !self.is_disabled;
    }

    #[command]
    pub fn on_toggle_selected(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.is_selected = !self.is_selected;
    }
}
```

**变更点**:
- 字段重命名: `button_clicks` → `basic_clicks` (语义更清晰)
- 命令重命名: `on_button_demo_click` → `on_basic_click` / `on_toggle_disabled_click` → `on_toggle_disabled` / `on_toggle_selected_click` → `on_toggle_selected`
- 新增 computed: `disabled_status_text` / `selected_status_text` (用于演示状态反馈)
- 删除 `code_sample` computed (改为模板内联静态代码块,无需 ViewModel 字段)

#### C-2: `button_case.rml` (模板)

**Ant Design 风格结构** — 6 张 Card:

```html
<component>
    <div v-flex="" class="case-pane doc-pane">
        <h2>{t("case.button.title")}</h2>

        <Card title="组件说明">
            <p>Button 是 gpui-component 按钮的 RML 封装，标签为 PascalCase。路由表注册为 Stateless 组件，codegen 生成 rml_ui::Button::new(id) 链式调用。</p>
            <p>支持 label / primary / ghost / danger / disabled / selected / size / compact 等属性。小写 button 属于内置 HTML 轨，映射为 gpui::div()。</p>
        </Card>

        <Card title="基础用法">
            <p>点击按钮累计计数，演示 on-click 事件绑定。</p>
            <p>{basic_click_text}</p>
            <div class="button-row">
                <Button label="Default" on-click={on_basic_click} />
                <Button label="Primary" primary="" on-click={on_basic_click} />
                <Button label="Ghost" ghost="" on-click={on_basic_click} />
                <Button label="Danger" danger="" on-click={on_basic_click} />
            </div>
        </Card>

        <Card title="按钮尺寸">
            <p>size 属性控制按钮大小：small / medium / large。</p>
            <div class="button-row">
                <Button label="Small" size="small" primary="" />
                <Button label="Medium" size="medium" primary="" />
                <Button label="Large" size="large" primary="" />
            </div>
        </Card>

        <Card title="不可用状态">
            <p>disabled 属性禁用按钮交互，点击切换查看效果。</p>
            <p>{disabled_status_text}</p>
            <div class="button-row">
                <Button label="切换禁用" on-click={on_toggle_disabled} />
                <Button label="Disabled Button" disabled={is_disabled} primary="" />
            </div>
        </Card>

        <Card title="选中状态">
            <p>selected 属性切换按钮选中态，点击切换查看效果。</p>
            <p>{selected_status_text}</p>
            <div class="button-row">
                <Button label="切换选中" on-click={on_toggle_selected} />
                <Button label="Selected Button" selected={is_selected} />
            </div>
        </Card>

        <Card title="按钮组 ButtonGroup">
            <p>ButtonGroup 容器组合多个按钮，常用于操作分组。</p>
            <ButtonGroup>
                <Button label="上一页" />
                <Button label="下一页" />
            </ButtonGroup>
        </Card>

        <Card title="API">
            <div class="api-table">
                <div class="api-row api-header"><span class="api-prop-name">属性</span><span class="api-prop-type">类型</span><span>说明</span></div>
                <div class="api-row"><span class="api-prop-name">label</span><span class="api-prop-type">字符串</span><span>按钮文字</span></div>
                <div class="api-row"><span class="api-prop-name">primary / ghost / danger</span><span class="api-prop-type">布尔标志</span><span>变体</span></div>
                <div class="api-row"><span class="api-prop-name">disabled</span><span class="api-prop-type">布尔</span><span>禁用</span></div>
                <div class="api-row"><span class="api-prop-name">selected</span><span class="api-prop-type">布尔</span><span>选中态</span></div>
                <div class="api-row"><span class="api-prop-name">size</span><span class="api-prop-type">small/medium/large</span><span>尺寸</span></div>
                <div class="api-row"><span class="api-prop-name">compact</span><span class="api-prop-type">布尔标志</span><span>紧凑模式</span></div>
                <div class="api-row"><span class="api-prop-name">on-click</span><span class="api-prop-type">事件</span><span>点击回调</span></div>
            </div>
        </Card>
    </div>
</component>
```

**变更点**:
- 拆分原单一"演示效果"Card 为 5 张独立 Card (基础用法 / 按钮尺寸 / 不可用状态 / 选中状态 / 按钮组)
- 删除原"示例代码"Card (Ant Design 在线 demo 才需要;桌面 demo 直接看演示即可)
- 保留"组件说明"与"API"两张 Card 作为文档信息
- 命令处理器名与 ViewModel 同步重命名

**CSS 已就绪** (无新增):
- `.case-pane.doc-pane` — align-items: stretch, gap: 16px, overflow-y: auto
- `.doc-pane .card` — max-width: 960px, margin: 0 auto
- `.button-row` — flex row, gap 12px, wrap, justify-content: center (在 doc-pane 下被覆盖为 flex-start)
- `.api-table` / `.api-row` / `.api-prop-name` / `.api-prop-type` — 已有

**验证点**:
- `ButtonGroup` 已注册 (tags.rs:317-318, Stateless 容器组件, container: true)
- `ButtonGroup` 接受子 `<Button>` 作为 children (container: true 路由)
- `size="small" / "medium" / "large"` 生成 `.with_size(rml_ui::Size::Small|Medium|Large)` (component.rs:299-307)
- `disabled={is_disabled}` 生成 `.disabled(self.is_disabled)` (component.rs:423-425)
- `selected={is_selected}` 生成 `.selected(self.is_selected)` (component.rs:427-429)

## 四、Assumptions & Decisions (假设与决策)

### 假设
1. `MainWindowRef` 服务已注册 (从上下文确认,与 activity_panel.rml.rs 同一模式)
2. `Card::on_click` 通过 `InteractiveElement` trait 提供 (card.rs:137-141 已确认)
3. `cx.listener` 闭包在 `render_group` 中可正常捕获 `case_id` (标准 GPUI 模式)
4. `ButtonGroup` 容器接受 `<Button>` 子元素 (container: true 路由,与 accordion item 模式一致)

### 决策
1. **不演示 icon 与 loading** — codegen 不支持 (icon 静默丢弃,loading 生成无参调用导致编译错误)
2. **不使用 `<Button>` 内文本子节点** — 保持 `label` 属性风格一致性
3. **删除"示例代码"Card** — Ant Design 在线 demo 的代码块在桌面 demo 中价值有限,直接看演示更直观
4. **`grouped_items` 改为字段而非 computed** — 规避 codegen 限制 (`#[computed]` 不能用于 `each` iterable)
5. **`render_group` 命令式构建** — 利用 `<component content={...} />` 透明容器特性注入 AnyElement
6. **不重写其它 case** — 用户明确限定改造范围为 "Button + 修复通用交互问题"

## 五、Verification Steps (验证步骤)

### 1. 编译验证
```powershell
cargo build -p rml-demo
```
**预期**: 0 errors, 0 warnings (新增代码)。

### 2. 启动应用
```powershell
cargo run -p rml-demo
```
**预期**: 窗口启动,默认 Tab 为 "欢迎" (WelcomeCase)。

### 3. 欢迎页验证
- ✅ 显示标题 "RML 组件总览" + 副标题 "点击下方卡片快速跳转到对应组件示例"
- ✅ 按 group 分组 (绑定 / 组件 / 国际化 / 菜单),每组 h3 标题 + 卡片行
- ✅ 卡片宽度 160px,hover 时 shadow 提升
- ✅ 点击任一卡片 → 打开对应 case Tab (与左侧案例树点击行为一致)
- ✅ 切换 locale (中/英) → 标题、副标题、分组名、卡片名全部刷新

### 4. Button 案例验证
- ✅ 在左侧案例树点击 "按钮样式" → 打开 ButtonCase Tab
- ✅ 显示 7 张 Card: 组件说明 / 基础用法 / 按钮尺寸 / 不可用状态 / 选中状态 / 按钮组 / API
- ✅ "基础用法": 点击 Default/Primary/Ghost/Danger 任一按钮 → `{basic_click_text}` 计数 +1
- ✅ "按钮尺寸": 三种尺寸按钮视觉差异明显
- ✅ "不可用状态": 点击"切换禁用" → "Disabled Button" 在可用/禁用间切换,禁用态不响应点击
- ✅ "选中状态": 点击"切换选中" → "Selected Button" 在选中/未选中间切换,选中态视觉高亮
- ✅ "按钮组 ButtonGroup": 两个按钮横向排列为一个整体
- ✅ "API": 表格展示 7 行属性说明

### 5. 通用交互回归
- ✅ Tab 切换、关闭正常
- ✅ 左侧案例树点击正常
- ✅ 菜单栏 (File/View/Help) 点击正常 (改动 D 已修复)
- ✅ 状态栏显示正常

### 6. locale 切换回归
- ✅ 切换到 English → 所有 case 标题、按钮文字、状态文本使用 en-US 翻译
- ✅ 切换回中文 → 恢复 zh-CN

## 六、Implementation Order (实施顺序)

1. **改动 A**: 修订 `welcome_case.rml.rs` (字段化 grouped_items + 新增 render_group + open_case)
2. **改动 B**: 重写 `welcome_case.rml` (模板使用 `<component each content>` 模式)
3. **改动 C-1**: 重写 `button_case.rml.rs` (字段重命名 + 命令重命名 + 新增 computed)
4. **改动 C-2**: 重写 `button_case.rml` (7 张 Card 多卡片结构)
5. **编译验证**: `cargo build -p rml-demo`
6. **运行验证**: `cargo run -p rml-demo` 走完上述 5 个验证步骤

## 七、Files Touched (涉及文件)

| 文件 | 操作 | 说明 |
|------|------|------|
| `demo/src/cases/welcome_case.rml.rs` | 修订 | 改动 A: 字段化 + render_group + open_case |
| `demo/src/cases/welcome_case.rml` | 重写 | 改动 B: 分组卡片网格模板 |
| `demo/src/cases/button_case.rml.rs` | 重写 | 改动 C-1: 字段重命名 + 新增 computed |
| `demo/src/cases/button_case.rml` | 重写 | 改动 C-2: 7 张 Card Ant Design 风格 |

**不修改文件** (已完成或不在范围内):
- `demo/src/shell/main_window.rml` (改动 D 已完成)
- `demo/assets/styles.css` (改动 E 已完成)
- `demo/assets/i18n/zh-CN.json` + `en-US.json` (改动 F 已完成)
- 其它 `*_case.rml` / `*_case.rml.rs` (不在改造范围内)
