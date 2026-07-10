# Slider

## 概述

`Slider` 路由到 `rml_ui::Slider`，**Stateful** 组件，滑块输入。通过 `SliderState` Entity 管理内部状态。

## 基本用法

```html
<!-- 自动双向绑定 -->
<Slider value={volume} />

<!-- ref 模式（手动管理 SliderState） -->
<Slider ref="slider_state" />
```

## 属性

| 属性 | 类型 | 绑定 | 说明 |
|------|------|------|------|
| `value` | 数字 | `{field}` | **自动双向绑定**（Stateful StateBridge 机制） |
| `disabled` | 布尔 | `{expr}` | 禁用 |
| `label` | 字符串 | `{expr}` | 标签 |
| `small` / `large` | 布尔标志 | — | 尺寸 |
| `ref` | 字符串 | — | SliderState Entity 引用名（手动管理模式） |

## 事件

| 事件 | 说明 |
|------|------|
| `on-click` | codegen 支持，但滑块值变化由 StateBridge 自动处理 |

## 数据绑定

### `value={field}` 自动双向绑定（StateBridge）

```html
<Slider value={volume} />
```

- **正向同步**（VM→SliderState）：render 时对比字段版本号，变化则 `state.set_value(SliderValue::Single(value))`
- **反向同步**（SliderState→VM）：订阅 `SliderEvent::Change`，提取 `SliderValue::Single(v)` → 回写 `field` + `bump_version`
- **字段类型**：`f32` / `i32` / `u32` 等数值类型（中间表示为 `f32`，反向赋值时 `as <type>` 转换）

### `ref="name"` 手动模式

需在 ViewModel 中声明 `slider_state: Option<Entity<SliderState>>` 字段，在 `on_loaded` 中初始化 `SliderState::new().min(0.0).max(100.0).step(1.0).default_value(50.0)`。详见 `demo/src/cases/slider_case.rml`。

## 子节点 / 插槽

不支持。

## 完整示例

```html
<Label label="音量" />
<Slider value={volume} />
<p>当前音量：{volume}</p>
```

## 常见错误

1. **写 `onchange={handler}`** — Slider 的值变化由 StateBridge 自动处理，无需手动 `onchange`。
2. **混淆 `value` 双向绑定与 `ref` 模式** — `value={field}` 自动双向；`ref="name"` 手动管理 SliderState。

## 相关组件

- [switch.md](./switch.md)

## RML 未覆盖的 API

`.min()`、`.max()`、`.step()` 等 `SliderState` 构造器方法需在 `on_loaded` 中通过 `ref` 模式手写。
