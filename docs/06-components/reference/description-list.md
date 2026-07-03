# DescriptionList

## 概述

`descriptions` 标签路由到 `rml_ui::DescriptionList`，是 **StatelessWithItems** 直接 `.child()` 注入式容器组件，用于渲染键值对形式的只读信息列表（如用户详情、产品属性、配置摘要）。每个条目由 `description` 子项组成，可选 `separator` 分隔符。

RML **推荐使用小写语法** `<descriptions>` / `<description>` / `<separator />`，简洁干净且与 HTML 风格一致。PascalCase 写法（`<DescriptionList>` / `<DescriptionItem>`）完全兼容。`<separator>` 仅小写形式有效，避免与独立 `<Separator>` 组件命名冲突。

## 标签别名表

`canonical_tag()` 函数将所有写法统一映射到 PascalCase 标准名，供 `props_registry` 属性查询使用。`component_lookup()` 同时注册 `descriptions` 和 `DescriptionList`，`is_item_builder_tag()` 识别 `description`、`DescriptionItem`、`separator`、`DescriptionSeparator`。

| 写法 | 规范化结果 | 推荐度 | 说明 |
|------|-----------|--------|------|
| `<descriptions>` | `DescriptionList` | ✅ 推荐 | 小写，HTML 风格 |
| `<DescriptionList>` | `DescriptionList` | 兼容 | PascalCase |
| `<description>` | `DescriptionItem` | ✅ 推荐 | 小写，仅 `<descriptions>` 内上下文敏感 |
| `<DescriptionItem>` | `DescriptionItem` | 兼容 | PascalCase |
| `<separator />` | `DescriptionSeparator` | ✅ 推荐 | 仅小写；PascalCase `<Separator>` 是独立组件 |

> ⚠️ `<description>` / `<separator>` 短标签仅在 `<descriptions>` / `<DescriptionList>` 父容器内被识别（由 `is_item_builder_tag` 判断）。在顶层或其他容器内使用会报 "unknown tag" 错误。

> ⚠️ `<Separator>`（PascalCase）是独立的分隔符组件，**不是** DescriptionList 的子项。仅在 `<descriptions>` 内使用小写 `<separator />` 才会被识别为描述列表分隔符。

## 基本用法

最小示例 —— 水平布局，带边框，三列：

```html
<descriptions bordered="" columns="3" label_width="120">
    <description label="用户名" value="alice" />
    <description label="邮箱" value="alice@example.com" />
    <description label="手机号" value="13800138000" />
</descriptions>
```

- `bordered=""` 启用边框
- `columns="3"` 设置三列布局
- `label_width="120"` 设置标签列宽 120px
- `label="..."` 设置条目标签（**必填**，构造器参数）
- `value="..."` 设置条目值

## 容器属性

`<descriptions>` 容器支持的属性：

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `vertical` | 布尔标志 | — | 垂直布局（标签与值上下排列） |
| `horizontal` | 布尔标志 | — | 水平布局（标签与值左右排列，默认） |
| `bordered` | 布尔标志 | `{expr}` | 显示边框 |
| `columns` | 数字 | `{expr}` | 列数（水平布局下生效） |
| `label_width` | 像素值 | `{expr}` | 标签列宽（`gpui::Pixels`） |
| `small` / `xsmall` / `large` | 布尔标志 | — | Sizable 通用尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用（Styled 通用） |

布尔标志写法：`vertical=""` / `vertical="true"` 启用，`vertical="false"` 显式关闭。

`vertical` 与 `horizontal` 互斥，同时声明时以后者生成的 `.layout(gpui::Axis::*)` 为准。

### `label_width` 类型说明

- 静态：`label_width="120"` → `.label_width(gpui::px(120.))`
- 绑定：`label_width={width}` → `.label_width(self.width)`，要求 `width` 字段类型为 `gpui::Pixels`

## 子项属性

`<description>` 子项支持的属性：

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `label` | 字符串 | `{expr}` | 条目标签（**必填**，构造器参数，无 `.label()` setter） |
| `value` | 字符串 / 元素 | `{expr}` | 条目值 |
| `span` | 数字 | `{expr}` | 跨列数 |
| `small` / `xsmall` / `large` | 布尔标志 | — | Sizable 通用尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用 |

### `label` 必填约束

`DescriptionItem::new(label)` 构造器要求 label 在构造时传入，RML **没有** `.label()` setter。codegen 在 `<description>` 上提取 `label` 属性作为构造器首参，缺失时报 `CodegenError`。

```html
<!-- ✅ 正确：label 作为属性 -->
<description label="用户名" value="alice" />

<!-- ✅ 正确：label 绑定 -->
<description label={field_label} value={field_value} />

<!-- ❌ 错误：缺少 label -->
<description value="alice" />
```

### `value` 的三种形式

`value` 按以下优先级解析：

1. **`value` 属性**（最高优先级）
   - 静态：`value="alice"` → `.value("alice")`
   - 绑定：`value={user_name}` → `.value(self.user_name.clone())`

2. **文本子节点**（无 `value` 属性时）
   ```html
   <description label="邮箱">alice@example.com</description>
   ```
   生成 `.value("alice@example.com")`

3. **元素子节点**（无 `value` 属性时）
   ```html
   <description label="角色">
       <Badge primary="">{role}</Badge>
   </description>
   ```
   生成 `.value(<Badge 元素代码>)`

   多个元素子节点用 `gpui::div()` 包裹：
   ```html
   <description label="标签">
       <Badge>管理员</Badge>
       <Badge>活跃</Badge>
   </description>
   ```
   生成 `.value(gpui::div().child(<Badge1>).child(<Badge2>))`

## separator 分隔符

`<separator />` 在条目之间插入分隔线：

```html
<descriptions bordered="" columns="2">
    <description label="产品" value="RML 框架" />
    <description label="版本" value="1.0.0" />
    <separator />
    <description label="作者" value="Rust 社区" />
    <description label="许可证" value="MIT" />
</descriptions>
```

`<separator />` 无属性，codegen 生成容器 `.separator()` 方法调用。

## 布局方向

### 水平布局（默认）

标签与值左右排列，`columns` 控制列数：

```html
<descriptions bordered="" columns="3" label_width="120">
    <description label="姓名" value="张三" />
    <description label="年龄" value="28" />
    <description label="邮箱" value="zhangsan@example.com" />
</descriptions>
```

### 垂直布局

`vertical=""` 切换为标签在上、值在下的纵向排列：

```html
<descriptions vertical="" bordered="">
    <description label="姓名" value="张三" />
    <description label="年龄" value="28" />
    <description label="邮箱" value="zhangsan@example.com" />
</descriptions>
```

## 跨列（span）

`span` 属性让条目占据多列：

```html
<descriptions bordered="" columns="3">
    <description label="用户名" value="alice" />
    <description label="邮箱" value="alice@example.com" />
    <description label="手机号" value="13800138000" />
    <description label="角色" value="管理员" />
    <description label="状态" value="活跃" span="2" />
</descriptions>
```

`span="2"` 使"状态"条目占据 2 列宽度。

## 数据绑定

所有支持绑定的属性均可使用 `{field}` 表达式：

```html
<descriptions bordered="" columns="2" label_width={width}>
    <description label="用户名" value={user_name} />
    <description label="邮箱" value={user_email} />
    <description label="角色" value={role} span="2" />
</descriptions>
```

**Code-behind 字段类型要求**：

| 属性 | 绑定字段类型 | 说明 |
|------|-------------|------|
| `label_width` | `gpui::Pixels` | `.label_width(self.width)` 直接传入 |
| `bordered` | `bool` | `.bordered(self.flag)` |
| `columns` | `usize` | `.columns(self.count)` |
| `value` | `String` / `SharedString` | `.value(self.field.clone())`（需 Clone） |
| `span` | `usize` | `.span(self.span)` |
| `label` | `String` / `SharedString` | `DescriptionItem::new(self.field.clone())` |

## Codegen 说明

RML 编译器将 `<descriptions>` + `<description>` 转译为直接 `.child()` 注入式 builder 调用。

**输入**：

```html
<descriptions bordered="" columns="3" label_width="120">
    <description label="用户名" value="alice" />
    <separator />
    <description label="角色" span="2">
        <Badge primary="">管理员</Badge>
    </description>
</descriptions>
```

**生成代码**（简化示意）：

```rust
rml_ui::DescriptionList::new()
    .bordered(true)
    .columns(3)
    .label_width(gpui::px(120.))
    .child(rml_ui::DescriptionItem::new("用户名").value("alice"))
    .separator()
    .child(
        rml_ui::DescriptionItem::new("角色")
            .span(2)
            .value(rml_ui::Badge::new(("rml_el", 0usize)).primary())
    )
```

**关键点**：

- `DescriptionList::new()` **不接收 ElementId**（与 TitleBar 一致），`ref` 指令被静默忽略
- 每个 `<description>` 生成 `DescriptionItem::new(label)` 表达式，通过容器 `.child()` 注入
- `<separator />` 生成容器 `.separator()` 调用
- `label` 作为构造器首参提取，缺失时报 `CodegenError`

## 完整示例

以下示例来自 `demo/src/cases/description_list_case.rml`，覆盖水平布局、垂直布局、小写标签、bind 绑定、元素子节点五种场景：

### 1. 水平布局 + bordered + columns

```html
<DescriptionList bordered="" columns="3" label_width="120">
    <DescriptionItem label="用户名" value="alice" />
    <DescriptionItem label="邮箱" value="alice@example.com" />
    <DescriptionItem label="手机号" value="13800138000" />
    <DescriptionItem label="角色" value="管理员" />
    <DescriptionItem label="状态" value="活跃" span="2" />
</DescriptionList>
```

### 2. 垂直布局

```html
<DescriptionList vertical="" bordered="">
    <DescriptionItem label="姓名" value="张三" />
    <DescriptionItem label="年龄" value="28" />
    <DescriptionItem label="邮箱" value="zhangsan@example.com" />
</DescriptionList>
```

### 3. 小写标签 + separator

```html
<descriptions bordered="" columns="2">
    <description label="产品名称" value="RML 框架" />
    <description label="版本" value="1.0.0" />
    <separator />
    <description label="作者" value="Rust 社区" />
    <description label="许可证" value="MIT" />
</descriptions>
```

### 4. bind 绑定

```html
<DescriptionList bordered="" columns="2" label_width={width}>
    <DescriptionItem label="用户名" value={user_name} />
    <DescriptionItem label="邮箱" value={user_email} />
    <DescriptionItem label="角色" value={role} span="2" />
</DescriptionList>
```

### 5. 元素子节点作为 value

```html
<DescriptionList bordered="" columns="2">
    <DescriptionItem label="角色">
        <Badge primary="">{role}</Badge>
    </DescriptionItem>
    <DescriptionItem label="状态">
        <Badge success="">活跃</Badge>
    </DescriptionItem>
</DescriptionList>
```

### Code-behind（Rust 侧）

```rust
use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::t_static;

#[contribute(
    host_id = "demo.activity",
    id = "components.description_list",
    kind = "case",
    group = "components",
    order = 16,
)]
#[component]
#[derive(Default)]
pub struct DescriptionListCase {
    pub user_name: String,
    pub user_email: String,
    pub role: String,
    pub width: gpui::Pixels,
}

impl IContribution for DescriptionListCase {
    fn id(&self) -> &str { Self::CONTRIBUTION_ID }
    fn name(&self) -> SharedString { t_static("case.description_list.title").into() }
}

impl ILifecycle for DescriptionListCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {
        self.user_name = "alice".into();
        self.user_email = "alice@example.com".into();
        self.role = "管理员".into();
        self.width = gpui::px(120.0);
    }
}

impl DescriptionListCase {
    #[computed]
    pub fn code_sample(&self) -> String {
        r#"..."#.to_string()
    }
}
```

## 常见错误

1. **`<description>` 缺少 `label` 属性** —— `label` 是构造器必填参数，缺失报 `CodegenError: <description> 缺少必填属性 label`。

2. **`<description>` / `<separator>` 在 `<descriptions>` 外使用** —— 顶层使用报 "unknown tag" 错误。这两个短标签仅在 `<descriptions>` / `<DescriptionList>` 父容器内被 `is_item_builder_tag` 识别。

3. **`<descriptions>` 包含非 `<description>` / `<separator>` 子节点** —— `<descriptions><div /></descriptions>` 报错：`<descriptions> 仅支持 <description> 或 <separator> 子节点`。

4. **`<Separator>`（PascalCase）误用为描述列表分隔符** —— `<Separator>` 是独立组件，不会被识别为 DescriptionList 子项。必须使用小写 `<separator />`。

5. **`label_width` 绑定字段类型不匹配** —— 静态 `label_width="120"` 生成 `.label_width(gpui::px(120.))`，绑定 `label_width={width}` 生成 `.label_width(self.width)`，要求 `width: gpui::Pixels`。若字段为 `f32` 会导致 Rust 编译失败。

6. **`value` 属性与子节点同时存在** —— `value` 属性优先级最高，子节点被忽略。不会报错但可能造成困惑，建议二选一。

7. **`ref` 指令无效** —— `DescriptionList::new()` 不接收 ElementId，`ref="name"` 会被静默忽略（不报错，但不生成稳定 ID）。如需引用，请在 code-behind 中通过其他方式管理状态。

## 相关组件

- [组件参考目录](./INDEX.md) —— 所有已注册组件
- [属性映射参考](./props-mapping.md) —— 组件属性 ↔ builder 方法对照表
- [标签映射 §2.2.9](../../02-syntax/tags-mapping.md) —— kebab-case 与小写别名规范
- [Separator](./separator.md) —— 独立分隔符组件（与 `<separator />` 区分）

## RML 未覆盖的 API

以下 gpui-component DescriptionList API 需在 Rust code-behind 中手写：

- 动态增删条目 —— RML 仅支持静态声明 `<description>` 子项，运行时增删需通过编程方式操作
- 自定义条目渲染（替换默认 label/value 布局）—— 需扩展 `DescriptionItem` 或手写 builder
- `DescriptionText` 的复杂类型转换 —— RML 的 `value` 绑定生成 `.value(self.field.clone())`，若字段类型非 `String`/`SharedString` 需手动实现 `Into<DescriptionText>`
