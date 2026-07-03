# Demo Case 文档格式改造（续）

## Summary

延续上一轮 plan（`demo-refactor-doc-format.md`），Steps 1-4 已完成，button_case 与 counter_case 已改造为文档格式范例。本 plan 聚焦**剩余 11 个 case 的文档格式改造 + 最终构建验证**，不重复已完成工作。

## Current State（已确认）

### 已完成（无需重做）

| Step | 内容 | 验证 |
|------|------|------|
| 1 | `menu_shell_contribs.rs` 抽取 `with_main_window` helper，8 处调用 | grep 确认 8 处 `with_main_window(ctx, ...)` |
| 2 | `main_window.rml.rs` `mem::take` → 直接 `push`/`iter_mut` | grep 确认 `open_tabs.push(tab)` + `apply_switch_en` |
| 3 | `demo/src/components/card.rml.rs` 已删除 | Glob 确认 components 目录无 .rml.rs |
| 4 | button_case 文档格式范例 + counter_case 文档格式 | 已读取确认 4 分节 Card 布局 |
| 样式 | styles.css 已含 `.doc-pane`/`.api-table`/`.api-row`/`.api-prop-name`/`.api-prop-type`/`.api-header`/`.code-block`/`.demo-section`/`.demo-section h3` | grep 确认 15 处样式定义 |

### 文档格式模板（已在 button_case/counter_case 验证）

```html
<component>
    <div v_flex="" class="case-pane doc-pane">
        <h2>{t("case.xxx.title")}</h2>

        <Card title="组件说明">
            <p>...组件用途、路由类型、标签说明...</p>
        </Card>

        <Card title="API">
            <div class="api-table">
                <div class="api-row api-header"><span class="api-prop-name">属性</span><span class="api-prop-type">类型</span><span>说明</span></div>
                <div class="api-row"><span class="api-prop-name">...</span><span class="api-prop-type">...</span><span>...</span></div>
            </div>
        </Card>

        <Card title="示例代码">
            <div class="code-block">{code_sample}</div>
        </Card>

        <Card title="演示效果">
            <div class="demo-section">
                <h3>场景名</h3>
                <p>{状态文本}</p>
                <!-- 实际演示组件 -->
            </div>
            <!-- 多场景 -->
        </Card>
    </div>
</component>
```

**`.rml.rs` 模式**：新增 `#[computed] pub fn code_sample(&self) -> String` 返回示例代码字符串（RML 源码原文）；按需新增演示场景的状态字段和 `#[command]` 方法。

**关键约束**（来自上轮探索）：
- CSS 不支持 `>` 子选择器、`:nth-child`、`!important`、逗号分隔值
- 示例代码用 `<div class="code-block">{code_sample}</div>`，不能用 CodeEditor（Entity<InputState> 不兼容 `#[derive(Default)]`）
- `#[command]` 方法的 dead_code warning 为预存在问题，不修复
- 文档内容（说明/API/示例代码/场景标题）用硬编码中文，不走 i18n；仅 case 标题走 i18n

## Proposed Changes

### Step 5：改造 11 个剩余 case

每个 case 按「.rml.rs 加 code_sample + .rml 替换为 4 分节 Card 布局」模式改造。下面列出每个 case 的具体演示场景规划。

#### 5.1 two_way_case（binding.two-way）

**文件**：
- [demo/src/cases/two_way_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml.rs)
- [demo/src/cases/two_way_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml)

**现有**：`name: String`、`age: i32`（带 `#[validate(range(min=0,max=150))]`）、`profile_summary` computed

**新增**：`code_sample` computed

**演示场景**（3 个）：
1. 文本双向绑定：`<input model={name} />` + 显示 name
2. 数值双向绑定：`<input model={age} />` + 显示 age
3. 验证失败场景：输入超范围值（如 200），观察红框 + tooltip（框架自动处理）

#### 5.2 accordion_case（components.accordion）

**文件**：
- [demo/src/cases/accordion_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml.rs)
- [demo/src/cases/accordion_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml)

**现有**：`last_open: String`、`status_text` computed、`on_toggle` command；.rml 已有 5 个演示场景（basic/multiple/sizes/with_icon/nested）

**新增**：`code_sample` computed

**演示场景**（沿用现有 5 个，组织进 demo-section）：
1. 单展开模式（默认，同时只能展开一项）
2. 多展开模式（`multiple=""`）
3. 尺寸变体（`small=""` / `large=""`）
4. 带图标项（`icon="Settings"`）
5. 嵌套手风琴

#### 5.3 avatar_case（components.avatar）

**文件**：
- [demo/src/cases/avatar_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml.rs)
- [demo/src/cases/avatar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml)

**现有**：无字段无命令；.rml 已有 4 个演示场景（image/initials/placeholder/group）

**新增**：`code_sample` computed（无需新增字段/命令）

**演示场景**（沿用现有 4 个）：
1. 图片头像（`src` + `large`/默认/`small`）
2. 首字母头像（`name`）
3. 占位头像（`placeholder="icon-name"`）
4. 头像分组（`AvatarGroup` + `limit` + `ellipsis`）

#### 5.4 slot_case（components.slot）

**文件**：
- [demo/src/cases/slot_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml.rs)
- [demo/src/cases/slot_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml)

**现有**：无字段无命令，使用框架 `Card title=... hoverable=""`

**新增**：`code_sample` computed

**演示场景**（3 个）：
1. 标题插槽：`<Card title="...">` 基础用法
2. 悬浮效果：`hoverable=""` 卡片
3. 内容组合：Card 内含 `<p>` + `<Button>`

#### 5.5 i18n_case（i18n.basic）

**文件**：
- [demo/src/cases/i18n_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/i18n_case.rml.rs)
- [demo/src/cases/i18n_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/i18n_case.rml)

**现有**：`on_loaded` 观察 I18nState、`on_switch_en`、`on_toggle_theme` commands

**新增**：`code_sample` computed、`current_locale` computed（返回 `cx` 当前 locale 不易，改为展示静态说明）、`current_theme` computed（同前）

**简化决策**：不新增 current_locale/current_theme computed（避免在 computed 中访问 cx 的复杂度），演示场景直接用按钮触发，状态通过页面文案变化体现。

**演示场景**（3 个）：
1. 静态翻译：显示 `{t("demo.hello")}` 当前语言文案
2. 切换语言：`on_switch_en` 切到 en-US，观察文案变化
3. 切换主题：`on_toggle_theme` 在 dark/light 间切换，观察整体外观变化

#### 5.6 status_bar_case（components.status_bar）

**文件**：
- [demo/src/cases/status_bar_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml.rs)
- [demo/src/cases/status_bar_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml)

**现有**：仅 hint 文本 + `StatusReady` 贡献 struct

**新增**：`code_sample` computed

**演示场景**（2 个）：
1. 状态栏贡献说明：解释 `#[contribute(kind="status")]` 机制，指向窗口底部状态栏的 `StatusReady` 实例
2. 贡献点代码示例：展示 StatusReady 的 contribute 属性

#### 5.7 menu_context_case（components.menu.context）

**文件**：
- [demo/src/cases/menu_context_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_context_case.rml.rs)
- [demo/src/cases/menu_context_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_context_case.rml)

**现有**：`last_action: String`、`context_status` computed、7 个 commands（open/copy/cut/paste/new_file/new_folder/delete）

**新增**：`code_sample` computed

**演示场景**（3 个）：
1. 基础右键菜单：`context-menu` 包裹目标区域，右键触发
2. 子菜单：`<menu-item label="New">` 内嵌 menu-item
3. 图标与分隔符：`icon="..."` + `<menu-separator />`

#### 5.8 menu_dropdown_case（components.menu.dropdown）

**文件**：
- [demo/src/cases/menu_dropdown_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_dropdown_case.rml.rs)
- [demo/src/cases/menu_dropdown_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_dropdown_case.rml)

**现有**：`last_action: String`、`dropdown_status` computed、3 个 commands（custom/standard/exit）

**新增**：`code_sample` computed

**演示场景**（2 个）：
1. 基础下拉：`dropdown-menu` + `anchor="TopRight"` + Button 触发
2. 分隔符与图标：`menu-separator` + `icon="..."`

#### 5.9 menu_editor_case（components.menu.editor）

**文件**：
- [demo/src/cases/menu_editor_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_editor_case.rml.rs)
- [demo/src/cases/menu_editor_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_editor_case.rml)

**现有**：`word_wrap: bool`、`last_action: String`、`editor_status` computed、5 个 commands（save/save_as/find/replace/toggle_wrap）

**新增**：`code_sample` computed

**演示场景**（3 个）：
1. 编辑器菜单基础：`dropdown-menu check_side="Right"`
2. 勾选状态：`on_toggle_wrap` 切换 word_wrap，菜单项显示勾选
3. 分隔符与命令分组：save/save_as 一组，find/replace 一组

#### 5.10 menu_features_case（components.menu.features）

**文件**：
- [demo/src/cases/menu_features_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml.rs)
- [demo/src/cases/menu_features_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml)

**现有**：`is_checked: bool`、`last_action: String`、`features_status` computed、5 个 commands（available/disabled/toggle_check/nested_a/nested_b）

**新增**：`code_sample` computed

**演示场景**（4 个）：
1. 可用项与禁用项：`disabled=""` 对比
2. 勾选项：`checked={is_checked}` + `on_toggle_check`
3. 链接项：`href="..."` + `icon="Info"`
4. 子菜单与可滚动：`<menu-item>` 内嵌 + `scrollable="" max_h="280"`

#### 5.11 menu_custom_case（components.menu.custom）

**文件**：
- [demo/src/cases/menu_custom_case.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_custom_case.rml.rs)
- [demo/src/cases/menu_custom_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_custom_case.rml)

**现有**：`dark_mode: bool`、`last_action: String`、`dark_mode_label`/`custom_status` computed、2 个 commands（toggle_dark/sign_out）

**新增**：`code_sample` computed

**演示场景**（3 个）：
1. 自定义菜单项：`header=""` 分组标题
2. 主题切换：`on_toggle_dark` 切换 dark_mode，显示当前状态
3. 链接项与登出：`href` 外链 + `on_sign_out`

### Step 6：i18n key 检查

**预期**：无需新增 i18n key。所有 case 标题已存在（`case.xxx.title`），文档内容（说明/API/示例代码/场景标题）用硬编码中文，与 button_case/counter_case 范例一致。

**验证**：改造后若 cargo build 报缺少 i18n key，按需补充。

### Step 7：样式补充

**预期**：无需新增样式。styles.css 已含 `.doc-pane`/`.api-table`/`.api-row`/`.api-prop-name`/`.api-prop-type`/`.api-header`/`.code-block`/`.demo-section`/`.demo-section h3` 全套样式。

**验证**：若新 case 的演示场景需要额外布局（如 avatar 的 `h_flex gap_4`），沿用现有 utility class。

### Step 8：构建验证

```bash
cargo build -p rust-rml-demo
cargo clippy -p rust-rml-demo -- -D warnings
```

**验证点**：
1. 编译通过（CallContext 兼容性、code_sample computed 类型正确）
2. 无新增 clippy 警告（预存在的 `#[command]` dead_code warning 可忽略）
3. 抽样运行 demo，确认若干 case 页面渲染 4 分节文档布局

## Assumptions & Decisions

1. **沿用上轮 plan 决策**：不合并 case、不新建框架组件、文档内容硬编码中文不走 i18n、示例代码用 `<div class="code-block">{code_sample}</div>`（非 CodeEditor）
2. **code_sample 内容**：每个 case 的示例代码展示本组件典型 RML 用法，3-5 行，直接硬编码字符串
3. **演示场景来源**：优先沿用现有 .rml 已有的演示场景（accordion 5 个、avatar 4 个），组织进 demo-section；menu 系列 case 演示场景较简单，按特性分组扩展
4. **i18n_case 简化**：不新增 current_locale/current_theme computed（computed 中访问 cx 较复杂），通过文案/外观变化体现效果
5. **status_bar_case**：该 case 本身无交互演示，文档格式以说明 + 贡献点代码示例为主，演示效果分节指向窗口底部实际状态栏

## Verification

- [ ] 11 个 case 的 .rml 均替换为 4 分节 Card 文档布局（class="case-pane doc-pane"）
- [ ] 11 个 case 的 .rml.rs 均含 `#[computed] pub fn code_sample(&self) -> String`
- [ ] `cargo build -p rust-rml-demo` 通过
- [ ] `cargo clippy -p rust-rml-demo -- -D warnings` 无新增警告
- [ ] 每个 case 的「演示效果」分节含 2-5 个场景，覆盖基础/进阶/边界

## Implementation Order

1. 按顺序改造 5.1 → 5.11（two_way → menu_custom），每改 2-3 个 case 后跑一次 `cargo build -p rust-rml-demo` 早发现问题
2. Step 8 最终构建 + clippy 验证
3. 期间若发现 i18n/样式缺口，补充后继续
