# Demo 重构：文档化用例页面 + 清理冗余 + 同步框架变更

## Summary

将 demo 的 14 个 case 页面改造为「组件文档格式」（组件说明 / API / 示例代码（gpui-compnent的CodeEditor） / 演示效果），同步清理 demo 冗余逻辑，并验证框架 icon.rs / command.rs 变更在 demo 中的兼容性。不合并 case，保持现有贡献机制不变。

## Current State Analysis

### Demo 现状

* **15 个 case 文件**（[demo/src/cases/](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases)）：每个含 `.rml`（模板）+ `.rml.rs`（code-behind），均为「标题 + 单一演示」结构，无文档分节、无多场景

* **welcome\_case** 是首页占位页（无演示），其余 14 个为组件案例

* **Card 组件**（[demo/src/components/card.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/card.rml.rs)）：已定义 `slots=["header","default","footer"]`，可作文档分块容器

* **menu\_shell\_contribs.rs**（[demo/src/shell/menu\_shell\_contribs.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs)）：7 个 `impl ICommand`，其中 6 处 `try_global::<DemoShellHost>` + `upgrade` + `host.update` 样板重复约 60 行

* **main\_window\.rml.rs:177-180**（[main\_window.rml.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L177-L180)）：`mem::take` + push + 赋值，可直接 `push`

* **card.rml.rs:13-17**：空壳 `fn new()` 调用 `default()`，可删除（Default 已提供）

### 框架变更兼容性

* **icon.rs**：demo 无独立 icon 解析逻辑（委托框架 `resolve_icon`），无需同步

* **command.rs** **`CallContext`**：新增 `parameter: Option<&dyn Any>` 字段，`new()` 默认 `None`，向后兼容；demo 的 `CallContext::new(window, cx)` 调用无需修改

## Proposed Changes

### Step 1：抽取 menu\_shell\_contribs.rs 的 helper

**文件**：[demo/src/shell/menu\_shell\_contribs.rs](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_shell_contribs.rs)

**问题**：6 处 ICommand::execute 重复以下样板（约 10 行/处）：

```rust
if let Some(host) = ctx.app.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
    host.update(ctx.app, |this, cx| { ... });
}
```

**方案**：在 `DemoShellHost` 上新增 helper 方法（或在 menu\_shell\_contribs.rs 顶部新增私有 helper）：

```rust
fn with_main_window<F>(ctx: &mut CallContext, f: F)
where
    F: FnOnce(&mut MainWindow, &mut gpui::Context<MainWindow>),
{
    if let Some(host) = ctx.app.try_global::<DemoShellHost>().and_then(|h| h.0.upgrade()) {
        host.update(ctx.app, |this, cx| f(this, cx));
    }
}
```

将 6 处 execute 改写为：

```rust
impl ICommand for MenuFileNew {
    fn execute(&self, ctx: &mut CallContext) {
        with_main_window(ctx, |this, cx| this.open_case("welcome".to_string(), cx));
    }
}
```

**效果**：约 60 行样板缩减为 6 行调用，逻辑集中。

### Step 2：清理 main\_window\.rml.rs 的 mem::take

**文件**：[demo/src/shell/main\_window.rml.rs#L177-L180](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L177-L180)

**变更**：

```rust
// Before
let mut tabs = std::mem::take(&mut self.open_tabs);
tabs.push(tab);
self.open_tabs = tabs;

// After
self.open_tabs.push(tab);
```

### Step 3：删除 card.rml.rs 的空壳 new()

**文件**：[demo/src/components/card.rml.rs#L13-L17](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/components/card.rml.rs#L13-L17)

删除 `impl Card { pub fn new() -> Self { Self::default() } }` 整块（Default 已提供同等能力）。若 RML codegen 依赖 `Card::new()`，则保留——需在实施时验证 codegen 输出。

### Step 4：建立 case 文档格式模板

**目标**：将每个 case 的 `.rml` 改造为文档布局，使用 Card 组件分块，包含 4 个分节：

1. **组件说明**（Card header="组件说明"）：组件用途、路由类型（Stateless/StatelessWithItems/Stateful）、标签名
2. **API**（Card header="API"）：属性表 + 事件表（用 div 表格或列表呈现）
3. **示例代码**（Card header="示例代码"）：RML 代码片段（用 `<pre>` 或 div + 等宽字体展示）
4. **演示效果**（Card header="演示效果"）：实际交互演示，多场景

**模板范例**（button\_case.rml）：

```html
<component>
    <div v_flex="" class="case-pane">
        <h2>{t("case.button.title")}</h2>
        
        <Card>
            <template slot="header">组件说明</template>
            <template slot="default">
                <p>Button 是 gpui-component 按钮的 RML 封装，标签为 PascalCase &lt;Button&gt;。路由表注册为 Stateless 组件，codegen 生成 rml_ui::Button::new(id) 链式调用。</p>
            </template>
        </Card>
        
        <Card>
            <template slot="header">API</template>
            <template slot="default">
                <div class="api-table">
                    <div class="api-row"><span>label</span><span>字符串</span><span>按钮文字</span></div>
                    <div class="api-row"><span>primary/ghost/danger</span><span>布尔标志</span><span>变体</span></div>
                    <div class="api-row"><span>onclick</span><span>事件</span><span>点击回调</span></div>
                </div>
            </template>
        </Card>
        
        <Card>
            <template slot="header">示例代码</template>
            <template slot="default">
                <pre class="code-block">&lt;Button label="提交" primary="" onclick={on_submit} /&gt;</pre>
            </template>
        </Card>
        
        <Card>
            <template slot="header">演示效果</template>
            <template slot="default">
                <p>{button_demo_text}</p>
                <div class="button-row">
                    <Button label={t("case.button.primary")} primary="" onclick={on_button_demo_click} />
                    <Button label={t("case.button.ghost")} ghost="" onclick={on_button_demo_click} />
                    <Button label={t("case.button.danger")} danger="" onclick={on_button_demo_click} />
                </div>
            </template>
        </Card>
    </div>
</component>
```

**演示场景全面性要求**：每个 case 的「演示效果」分节需覆盖：

* **基础场景**：组件最常用法

* **进阶场景**：属性组合、绑定、状态变化

* **边界场景**：禁用/空值/极端值/错误处理（视组件特性而定）

### Step 5：批量改造 14 个 case

按以下顺序改造（welcome\_case 保持不变，是首页占位页）：

| Case                 | 文件                                                                                                    | 演示场景扩展方向                           |
| -------------------- | ----------------------------------------------------------------------------------------------------- | ---------------------------------- |
| button\_case         | [button\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/button_case.rml)                | 基础变体 + disabled/selected + loading |
| counter\_case        | [counter\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/counter_case.rml)              | 基础+/- + 步长 + min/max 边界            |
| two\_way\_case       | [two\_way\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/two_way_case.rml)             | 文本双向 + 数值双向 + 验证失败                 |
| accordion\_case      | [accordion\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/accordion_case.rml)          | 单展开 + 多展开 + 带图标                    |
| avatar\_case         | [avatar\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/avatar_case.rml)                | 基础 + 尺寸 + 形状                       |
| slot\_case           | [slot\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/slot_case.rml)                    | 默认插槽 + 命名插槽 + 条件插槽                 |
| i18n\_case           | [i18n\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/i18n_case.rml)                    | 静态 + 动态 + 切换语言                     |
| menu\_context\_case  | [menu\_context\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_context_case.rml)   | 基础右键 + 禁用项 + 分隔符                   |
| menu\_dropdown\_case | [menu\_dropdown\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_dropdown_case.rml) | 基础下拉 + 多级 + 触发方式                   |
| menu\_editor\_case   | [menu\_editor\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_editor_case.rml)     | 编辑器菜单 + 快捷键 + 状态                   |
| menu\_features\_case | [menu\_features\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_features_case.rml) | 综合特性演示                             |
| menu\_custom\_case   | [menu\_custom\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/menu_custom_case.rml)     | 自定义菜单项 + 图标 + 主题                   |
| status\_bar\_case    | [status\_bar\_case.rml](file:///d:/GitCode/RF/rust-gpui-rml/demo/src/cases/status_bar_case.rml)       | 基础状态 + 多区域 + 动态更新                  |

每个 case 改造内容：

1. `.rml`：替换为文档格式模板（4 分节 Card 布局）
2. `.rml.rs`：如需新增演示场景的状态字段/命令方法，相应补充

### Step 6：i18n key 补充

若新增演示场景需要新的 i18n 文案，在 demo 的 i18n 资源中补充对应 key。优先复用现有 key，避免过度新增。

### Step 7：样式补充

在 demo 全局样式（`main_window.rml` 或样式表）中补充文档格式所需样式：

* `.api-table` / `.api-row`：API 表格样式

* `.code-block`：代码块等宽字体 + 背景

* Card 间距调整

### Step 8：构建与测试验证

```bash
cargo build -p rust-rml-demo
cargo test -p rust-rml-demo
cargo clippy -p rust-rml-demo -- -D warnings
```

验证点：

1. demo 编译通过（CallContext 变更兼容性）
2. 每个 case 页面渲染 4 分节文档布局
3. 演示场景交互正常
4. 无 clippy 警告（除已存在的）

## Assumptions & Decisions

1. **不合并 case**：用户明确要求保持现有 case 数量，仅给每个 case 加文档格式
2. **不新建框架组件**：复用 demo 现有 Card 组件作文档分块容器，不在框架层新增 DocPage 组件
3. **文档内容写在 .rml 模板中**：组件说明/API/示例代码作为静态文本直接写在 `.rml` 中，不走 i18n（避免 i18n 资源膨胀）；仅标题与演示场景文案走 i18n
4. **演示场景扩展**：每个 case 需扩展为多场景，状态字段在 `.rml.rs` 中补充
5. **CallContext 兼容性**：`new()` 默认 `parameter=None`，demo 无需修改调用点
6. **Card::new() 删除前提**：实施时需验证 RML codegen 是否依赖 `Card::new()`，若依赖则保留

## Verification

* [ ] `cargo build -p rust-rml-demo` 通过

* [ ] `cargo test -p rust-rml-demo` 通过

* [ ] `cargo clippy -p rust-rml-demo -- -D warnings` 无新增警告

* [ ] 每个 case 渲染 4 分节文档布局（组件说明/API/示例代码/演示效果）

* [ ] menu\_shell\_contribs.rs 6 处样板缩减为 helper 调用

* [ ] main\_window\.rml.rs 的 mem::take 已简化为 push

* [ ] card.rml.rs 空壳 new() 已删除（或确认 codegen 依赖后保留）

## Implementation Order

1. Step 1-3：清理冗余（menu\_shell\_contribs helper / mem::take / Card::new）→ 验证编译
2. Step 4：建立 button\_case 文档格式范例 → 验证渲染
3. Step 5：批量改造剩余 13 个 case
4. Step 6-7：i18n + 样式补充
5. Step 8：全量构建/测试/clippy 验证

