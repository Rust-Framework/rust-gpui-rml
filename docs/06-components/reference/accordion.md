# Accordion

## 概述

`accordion` 标签路由到 `rml_ui::Accordion`，是 **StatelessWithItems** 闭包式 builder 组件，用于渲染可折叠的内容面板列表。每个面板由 `accordion` 容器 + 多个 `item` 子项组成。

RML **推荐使用小写语法** `<accordion>` / `<item>`，简洁干净且与 HTML 风格一致。PascalCase 与 kebab-case 写法完全兼容，五种写法在 codegen 与属性校验上等价。

## 标签别名表

`canonical_tag()` 函数将所有写法统一映射到 PascalCase 标准名，供 `props_registry` 属性查询使用。`component_lookup()` 同时注册 `accordion` 和 `Accordion`，`is_item_builder_tag()` 识别 `item` 和 `AccordionItem`。

| 写法 | 规范化结果 | 推荐度 | 说明 |
|------|-----------|--------|------|
| `<accordion>` | `Accordion` | ✅ 推荐 | 小写，HTML 风格，简洁干净 |
| `<Accordion>` | `Accordion` | 兼容 | PascalCase，向后兼容 |
| `<item>` | `AccordionItem` | ✅ 推荐 | 短标签，仅 `<accordion>` 内上下文敏感 |
| `<AccordionItem>` | `AccordionItem` | 兼容 | PascalCase，向后兼容 |
| `<accordion-item>` | `AccordionItem` | 兼容 | kebab-case，由 `normalize_component_tag` 处理 |

> ⚠️ `<item>` 短标签仅在 `<accordion>` / `<Accordion>` 父容器内被识别为 `AccordionItem`（由 `is_item_builder_tag` 判断）。在顶层或其他容器内使用 `<item>` 会报 "unknown tag" 错误。

## 基本用法

最小示例 —— 单选模式（默认），带边框：

```html
<accordion bordered="">
    <item title="Section 1" open="">
        <p>第一段内容</p>
    </item>
    <item title="Section 2">
        <p>第二段内容</p>
    </item>
</accordion>
```

- `bordered=""` 启用边框
- `open=""` 设置初始展开
- `title="..."` 设置面板标题（静态字符串）

## 容器属性

`<accordion>` 容器支持的属性：

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `multiple` | 布尔标志 | `{expr}` | 多选模式（同时展开多个面板） |
| `bordered` | 布尔标志 | `{expr}` | 显示边框 |
| `on_toggle_click` | 事件 | `{fn}` | 面板切换事件 |
| `small` / `xsmall` / `large` | 布尔标志 | — | Sizable 通用尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用（Styled 通用） |

布尔标志写法：`multiple=""` / `multiple="true"` 启用，`multiple="false"` 显式关闭。

## 子项属性

`<item>` 子项支持的属性：

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `title` | 字符串 / 元素 | `{expr}` | 面板标题（接受 `impl IntoElement`，可绑定 i18n 调用） |
| `open` | 布尔标志 | `{expr}` | 初始展开状态 |
| `icon` | `IconName` 枚举名 | `{expr}` | 面板图标（如 `Settings` / `Bell`） |
| `small` / `xsmall` / `large` | 布尔标志 | — | Sizable 通用尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用该面板 |

`icon` 值必须是 `rml_ui::IconName` 枚举的合法变体名（如 `Settings`、`Bell`、`User`），codegen 生成 `.icon(rml_ui::IconName::Settings)`。非法枚举名会导致 Rust 编译失败。

## 事件

### `on_toggle_click`

面板展开/收起时触发。事件绑定到 `<accordion>` 容器，**不能**绑定到 `<item>`。

```html
<accordion bordered="" on_toggle_click={on_toggle}>
    <item title="Section 1" />
</accordion>
```

**用户方法签名**（在 code-behind 中定义）：

```rust
impl AccordionCase {
    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_ixs);
        cx.notify();
    }
}
```

- `open_ixs: &[usize]` —— 当前展开项的索引列表（按声明顺序，0-based）
- 单选模式下长度最多为 1，多选模式下可包含多个索引

## 多选与单选

**默认单选**：展开一项自动收起其他项。

```html
<accordion bordered="">
    <item title="A" open="">...</item>
    <item title="B">...</item>
</accordion>
```

**多选模式**：`multiple=""` 启用，可同时展开多个面板。

```html
<accordion multiple="" bordered="">
    <item title="A" open="">...</item>
    <item title="B" open="">...</item>
</accordion>
```

## 尺寸

通过 Sizable 通用属性控制：

```html
<accordion small="" bordered="">
    <item title="Small">小尺寸</item>
</accordion>

<accordion large="" bordered="">
    <item title="Large">大尺寸</item>
</accordion>
```

## 图标

`<item>` 的 `icon` 属性在标题前显示图标：

```html
<accordion bordered="" on_toggle_click={on_toggle}>
    <item title="设置" icon="Settings">
        <p>设置内容</p>
    </item>
    <item title="通知" icon="Bell" disabled="true">
        <p>通知内容（已禁用）</p>
    </item>
</accordion>
```

`icon` 值必须是 `IconName` 枚举名，完整列表见 gpui-component `IconName` 文档。

## 嵌套

`<item>` 内可嵌套 `<accordion>`，实现多级折叠：

```html
<accordion bordered="">
    <item title="父级面板">
        <accordion bordered="" multiple="">
            <item title="子面板 1">
                <p>子内容 1</p>
            </item>
            <item title="子面板 2">
                <p>子内容 2</p>
            </item>
        </accordion>
    </item>
</accordion>
```

## Codegen 说明

RML 编译器将 `<accordion>` + `<item>` 转译为闭包式 builder 调用。每个 `<item>` 生成一个 `.item(|__rml_item: rml_ui::AccordionItem| ...)` 闭包。

**输入**：

```html
<accordion bordered="" multiple="">
    <item title="Section 1" open="">
        <p>Content</p>
    </item>
</accordion>
```

**生成代码**（简化示意）：

```rust
rml_ui::Accordion::new(("rml_el", 0usize))
    .bordered(true)
    .multiple(true)
    .item(|__rml_item: rml_ui::AccordionItem| {
        __rml_item.title("Section 1").open(true).child("Content")
    })
```

**`on_toggle_click` 事件生成**：

```rust
rml_ui::Accordion::new(("rml_el", 1usize))
    .on_toggle_click(cx.listener(move |this, open_ixs: &[usize], _window, cx| {
        this.on_toggle(open_ixs, cx);
    }))
```

注意 `cx.listener` 闭包接收 4 参数 `(this, open_ixs, _window, cx)`，但用户定义的 `on_toggle` 方法仅接收 3 参数 `(self, open_ixs, cx)`，`_window` 被丢弃。

## 完整示例

以下示例来自 `demo/src/cases/accordion_case.rml`，覆盖 basic / multiple / sizes / icon / nested 五种场景，全部使用小写语法：

### Basic（基础单选）

```html
<accordion bordered="">
    <item title={t("case.accordion.section1")} open="">
        <p>{t("case.accordion.content1")}</p>
    </item>
    <item title={t("case.accordion.section2")}>
        <p>{t("case.accordion.content2")}</p>
    </item>
    <item title={t("case.accordion.section3")}>
        <p>{t("case.accordion.content3")}</p>
    </item>
</accordion>
```

### Multiple（多选模式）

```html
<accordion multiple="" bordered="">
    <item title={t("case.accordion.section1")} open="">
        <p>{t("case.accordion.content1")}</p>
    </item>
    <item title={t("case.accordion.section2")} open="">
        <p>{t("case.accordion.content2")}</p>
    </item>
</accordion>
```

### Sizes（尺寸 + 事件）

```html
<accordion small="" bordered="" on_toggle_click={on_toggle}>
    <item title={t("case.accordion.small")}>
        <p>{t("case.accordion.content1")}</p>
    </item>
</accordion>
<accordion large="" bordered="" on_toggle_click={on_toggle}>
    <item title={t("case.accordion.large")}>
        <p>{t("case.accordion.content1")}</p>
    </item>
</accordion>
```

### With Icon（图标 + 禁用）

```html
<accordion bordered="" on_toggle_click={on_toggle}>
    <item title={t("case.accordion.settings")} icon="Settings">
        <p>{t("case.accordion.content1")}</p>
    </item>
    <item title={t("case.accordion.disabled")} icon="Bell" disabled="true">
        <p>{t("case.accordion.content2")}</p>
    </item>
</accordion>
```

### Nested（嵌套）

```html
<accordion bordered="">
    <item title={t("case.accordion.parent")}>
        <accordion bordered="" multiple="">
            <item title={t("case.accordion.child1")}>
                <p>{t("case.accordion.content1")}</p>
            </item>
            <item title={t("case.accordion.child2")}>
                <p>{t("case.accordion.content2")}</p>
            </item>
        </accordion>
    </item>
</accordion>
```

### Code-behind（Rust 侧）

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.accordion",
    kind = "case",
    group = "components",
    order = 10,
)]
#[component]
#[derive(Default)]
pub struct AccordionCase {
    pub last_open: String,
}

impl IContribution for AccordionCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.accordion.title").into() }
}

impl ILifecycle for AccordionCase {}

impl AccordionCase {
    #[computed]
    pub fn status_text(&self) -> String {
        if self.last_open.is_empty() {
            "尚未切换任何项".to_string()
        } else {
            format!("上次展开项索引：{}", self.last_open)
        }
    }

    #[command]
    pub fn on_toggle(&mut self, open_ixs: &[usize], cx: &mut Context<Self>) {
        self.last_open = format!("{:?}", open_ixs);
        cx.notify();
    }
}
```

## 常见错误

1. **`<item>` 在 `<accordion>` 外使用** —— 顶层使用 `<item>` 报 "unknown tag" 错误。`<item>` 短标签仅在 `<accordion>` / `<Accordion>` 父容器内被 `is_item_builder_tag` 识别。

2. **`<accordion>` 包含非 `<item>` 子节点** —— `<accordion><div /></accordion>` 报错：`<accordion> 仅支持 <item> 或 <AccordionItem> 子节点，得到 <div>`。

3. **`<accordion>` 包含文本子节点** —— 文本子节点会输出 `[rml warning] <Accordion> 不支持文本子节点` 并被忽略。文本应放在 `<item>` 内部。

4. **`icon` 值非 `IconName` 枚举名** —— `icon="foo"` 生成 `.icon(rml_ui::IconName::foo)`，若 `foo` 不是合法枚举变体则 Rust 编译失败。

5. **`on_toggle_click` 绑定到 `<item>`** —— `on_toggle_click` 是容器事件，仅在 `<accordion>` 上有效。绑定到 `<item>` 不会生成任何代码。

6. **混合写法不一致** —— `<accordion><AccordionItem /></accordion>` 可正常工作（别名等价），但建议同一文件内保持写法一致（推荐全小写）。

## 相关组件

- [组件参考目录](./INDEX.md) —— 所有已注册组件
- [属性映射参考](./props-mapping.md) —— 组件属性 ↔ builder 方法对照表
- [标签映射 §2.2.9](../../02-syntax/tags-mapping.md) —— kebab-case 与小写别名规范

## RML 未覆盖的 API

以下 gpui-component Accordion API 需在 Rust code-behind 中手写：

- `Accordion::default_open(ixs)` —— 通过编程方式设置默认展开项（RML 仅支持 `<item open="">` 静态声明）
- 动态控制展开状态（运行时增删 item、修改 open 状态）—— 需通过 `ref` 获取 `Accordion` 实例后操作
- 自定义面板渲染（替换默认标题/内容布局）—— 需扩展 `AccordionItem` 或手写 builder
- `on_toggle_click` 之外的细粒度事件（单项展开/收起回调）—— gpui-component 当前仅提供 `on_toggle_click`
