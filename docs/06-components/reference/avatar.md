# Avatar / AvatarGroup

## 概述

`Avatar` 路由到 `rml_ui::Avatar`，**StatelessNoId** 叶子组件，用于显示用户头像图片，无图片时自动回退为姓名首字母或占位图标。

`AvatarGroup` 路由到 `rml_ui::AvatarGroup`，**StatelessNoId** 容器组件，以紧凑重叠方式组合展示多个 `Avatar`。

## 基本用法

### 图片头像

```html
<Avatar src="https://example.com/avatar.jpg" large="" />
<Avatar src={user.avatar_url} />
```

### 首字母回退

未提供 `src` 时，根据 `name` 自动生成首字母并配色：

```html
<Avatar name="John Doe" large="" />
<Avatar name="Alice" />
<!-- 文本子节点等价于 name 属性 -->
<Avatar>John Doe</Avatar>
```

### 占位图标

无图片无姓名时显示默认 `User` 图标，可用 `placeholder` 替换：

```html
<Avatar large="" />
<Avatar placeholder="Building2" />
```

`placeholder` 值必须是 `IconName` 枚举名（如 `User`/`UserCircle`/`Building2`）。

### 尺寸

通过 Sizable 通用属性控制：

```html
<Avatar name="John" xsmall="" />
<Avatar name="John" small="" />
<Avatar name="John" />
<Avatar name="John" large="" />
```

## AvatarGroup

### 基础分组

子节点必须是 `<Avatar>` 元素：

```html
<AvatarGroup>
    <Avatar src="https://example.com/u1.jpg" />
    <Avatar src="https://example.com/u2.jpg" />
    <Avatar name="John Doe" />
</AvatarGroup>
```

### 限制数量 + 省略标记

```html
<AvatarGroup limit="3" ellipsis="">
    <Avatar src="https://example.com/u1.jpg" />
    <Avatar src="https://example.com/u2.jpg" />
    <Avatar src="https://example.com/u3.jpg" />
    <Avatar src="https://example.com/u4.jpg" />
    <Avatar name="John Doe" />
</AvatarGroup>
```

### 分组尺寸

Sizable 作用于所有子头像：

```html
<AvatarGroup small="" limit="5">
    <Avatar name="A" />
    <Avatar name="B" />
</AvatarGroup>
```

## 属性

### Avatar

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `src` | 字符串（URL） | `{expr}` | 图片源 |
| `name` | 字符串 | `{expr}` | 用户名（无图片时回退为首字母） |
| `placeholder` | `IconName` 枚举名 | `{expr}` | 占位图标，默认 `User` |
| `small` / `xsmall` / `large` | 布尔标志 | — | Sizable 尺寸 |
| `disabled` | 布尔 | `{expr}` | 禁用（Styled 通用） |

通用 Styled 属性（`border_*`/`shadow_*`/`rounded_*`/`bg`/`text_color` 等）可通过 `class` 或 Rust 手写应用。

### AvatarGroup

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `limit` | 整数 | `{expr}` | 最大显示数量（默认 3） |
| `ellipsis` | 布尔标志 | — | 超限时显示省略标记 |
| `small` / `xsmall` / `large` | 布尔标志 | — | 作用于所有子头像 |

## 事件

Avatar 实现 `InteractiveElement`，支持通用 `on-click`：

```html
<Avatar src={user.avatar} on-click={on_avatar_click} />
```

## 数据绑定

```html
<Avatar src={current_user.avatar_url} name={current_user.display_name} />
<AvatarGroup limit={max_visible} ellipsis="">
    <!-- each 循环渲染 -->
</AvatarGroup>
```

## 子节点 / 插槽

- **Avatar**：叶子组件，不接受元素子节点。单个文本子节点映射到 `.name()`（与显式 `name=` 互斥，显式属性优先）。
- **AvatarGroup**：容器组件，子节点**必须是 `<Avatar>` 元素**。支持 `each` 指令循环渲染。

## 完整示例

```html
<component>
    <div v_flex="" class="case-pane">
        <h2>用户资料</h2>
        <div h_flex="" gap_4="">
            <Avatar src={user.avatar} name={user.name} large="" />
            <div v_flex="">
                <span>{user.name}</span>
                <span>{user.title}</span>
            </div>
        </div>

        <h3>团队成员</h3>
        <AvatarGroup limit="4" ellipsis="">
            <Avatar src="https://example.com/alice.jpg" />
            <Avatar src="https://example.com/bob.jpg" />
            <Avatar name="Charlie Brown" />
            <Avatar name="Diana Prince" />
            <Avatar name="Eve Wilson" />
        </AvatarGroup>
    </div>
</component>
```

## 常见错误

1. **AvatarGroup 子节点非 Avatar**：`<AvatarGroup><div>...</div></AvatarGroup>` 会生成 `.child(gpui::div()...)`，但 `AvatarGroup::child` 仅接受 `Avatar` 类型，导致编译错误。
2. **placeholder 值非 IconName 枚举名**：`placeholder="foo"` 会生成 `.placeholder(rml_ui::IconName::foo)`，若 `foo` 不是合法枚举变体则编译失败。
3. **Avatar 不支持元素子节点**：`<Avatar><div /></Avatar>` 不会生成 `.child()`（Avatar 未实现 `ParentElement`）。

## 相关组件

- [badge.md](./badge.md)
- [label.md](./label.md)
- [tag.md](./tag.md)

## RML 未覆盖的 API

以下 gpui-component API 需在 Rust code-behind 中手写：

- `Avatar::with_size(px(N))` — 自定义像素尺寸（RML 仅支持枚举尺寸）
- `AvatarGroup::children(Vec<Avatar>)` — 批量添加（RML 用 `each` 指令或多个 `<Avatar>` 子节点替代）
- 自定义 Styled 样式（`border_3`/`border_color`/`shadow_sm`/`rounded(px(N))` 等）需通过 `class` 或 Rust 手写
