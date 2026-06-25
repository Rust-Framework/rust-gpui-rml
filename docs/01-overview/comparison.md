# 1.4 与原生 GPUI 的对比

> **本节目标**：用同一个 Todo 应用，横向对比原生 GPUI、gpui-rsx、RML 三种方案的代码量、可维护性与团队协作能力，建立"为什么选 RML"的直观认知。

## 1.4.1 同一需求的三种实现

需求：一个简单的计数器，显示当前值，提供增加/减少/重置三个按钮，count > 10 时显示提示。

### 方案 A：原生 GPUI

```rust
// 原生 GPUI：UI 与逻辑全部塞在一个文件
use gpui::*;
use gpui_component::Button;

pub struct Counter {
    count: i32,
}

impl Render for Counter {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .p(px(24.0))
            .child(
                div().text_xl().font_weight(FontWeight::BOLD)
                    .child(Label::new(format!("⚡ 计数器: {}", self.count)))
            )
            .when(self.count > 10, |div| {
                div.child(Label::new("🚀 超过十啦！"))
            })
            .child(
                div().flex().gap(px(8.0))
                    .child(
                        Button::new("增加")
                            .on_click(cx.listener(|this, _, cx| {
                                this.count += 1;
                                cx.notify();
                            }))
                    )
                    .when(self.count > 0, |div| {
                        div.child(
                            Button::new("减少")
                                .on_click(cx.listener(|this, _, cx| {
                                    this.count -= 1;
                                    cx.notify();
                                }))
                        )
                    })
                    .child(
                        Button::new("重置")
                            .on_click(cx.listener(|this, _, cx| {
                                this.count = 0;
                                cx.notify();
                            }))
                    )
            )
    }
}
```

### 方案 B：gpui-rsx

```rust
// gpui-rsx：JSX 风格，但仍在一个 Rust 文件
use gpui::*;
use gpui_component::Button;
use gpui_rsx::rsx;

pub struct Counter {
    count: i32,
}

impl Render for Counter {
    fn render(&mut self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        rsx! {
            <div class="flex flex-col gap-4 p-6">
                <h1 class="text-xl font-bold">⚡ 计数器: {self.count}</h1>
                {if self.count > 10 {
                    Some(rsx! { <span>🚀 超过十啦！</span> })
                } else {
                    None
                }}
                <div class="flex gap-2">
                    <Button label="增加" on_click={cx.listener(|this, _, cx| {
                        this.count += 1; cx.notify();
                    })} />
                    {if self.count > 0 {
                        Some(rsx! {
                            <Button label="减少" on_click={cx.listener(|this, _, cx| {
                                this.count -= 1; cx.notify();
                            })} />
                        })
                    } else {
                        None
                    }}
                    <Button label="重置" on_click={cx.listener(|this, _, cx| {
                        this.count = 0; cx.notify();
                    })} />
                </div>
            </div>
        }
    }
}
```

### 方案 C：RML

```html
<!-- counter.rml：纯 UI 声明 -->
<div class="counter-container">
    <h1 class="counter-title">⚡ 计数器: {count}</h1>
    <span class="counter-status" if={count > 10}>🚀 超过十啦！</span>
    <div class="counter-buttons">
        <button class="btn primary" onclick={increment}>➕ 增加</button>
        <button class="btn danger" onclick={decrement} if={count > 0}>➖ 减少</button>
        <button class="btn secondary" onclick={reset}>↺ 重置</button>
    </div>
</div>
```

```rust
// counter.rml.rs：纯业务逻辑
use rml::prelude::*;

#[derive(Model)]
#[view]
pub struct Counter {
    pub count: i32,
}

impl Counter {
    pub fn new() -> Self { Self { count: 0 } }

    #[command]
    pub fn increment(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count += 1; cx.notify();
    }

    #[command]
    pub fn decrement(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.count > 0 { self.count -= 1; cx.notify(); }
    }

    #[command]
    pub fn reset(&mut self, _: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.count = 0; cx.notify();
    }
}
```

## 1.4.2 量化对比

| 指标            | 原生 GPUI | gpui-rsx | **RML**   |
| ------------- | ------- | -------- | --------- |
| UI 代码行数       | ~30 行   | ~20 行    | **~8 行**  |
| 逻辑代码行数        | ~10 行   | ~10 行    | **~15 行** |
| 总代码量          | 100%    | ~70%     | **~60%**  |
| UI/逻辑分离       | ❌ 同一文件  | ❌ 同一文件   | **✅ 文件级** |
| 设计师可编辑        | ❌       | ❌        | **✅**     |
| 热重载           | ❌       | ❌        | **✅**     |
| 条件渲染可读性       | `.when()` 闭包 | `if-else` 表达式 | **`if={}` 指令** |
| 事件绑定          | `.on_click(cx.listener(...))` | `on_click={...}` | **`onclick={name}`** |
| 状态更新          | `cx.notify()` | `cx.notify()` | **`cx.notify()`** |

## 1.4.3 团队协作维度

| 协作场景          | 原生 GPUI       | gpui-rsx      | **RML**        |
| ------------- | ------------- | ------------- | -------------- |
| 设计师调整 UI 布局   | ❌ 需工程师改 Rust  | ❌ 需工程师改 Rust  | **✅ 直接改 `.rml`** |
| 工程师修复业务逻辑     | ⚠️ 需在 UI 文件中定位 | ⚠️ 需在 UI 文件中定位 | **✅ 在 `.rml.rs` 中独立修改** |
| UI 变更审查       | 混在逻辑变更中       | 混在逻辑变更中       | **✅ 独立 PR**    |
| 设计资产沉淀        | ❌ 无法沉淀        | ❌ 无法沉淀        | **✅ `.rml` 即资产** |
| 新人上手          | 需学 GPUI 链式 API | 需学 JSX 语法     | **✅ 会 HTML 即可** |

## 1.4.4 性能对比

| 维度          | 原生 GPUI | gpui-rsx | **RML**     |
| ----------- | ------- | -------- | ----------- |
| 运行时开销       | 0       | 宏展开后等价   | **0（编译期生成）** |
| 编译时间        | 基准      | 略增（宏展开）  | **略增（build.rs）** |
| 运行时内存       | 基准      | 基准       | **基准**      |
| 渲染性能        | 基准      | 基准       | **基准**      |

💡 **关键结论**：RML 在运行时与原生 GPUI 完全等价，因为编译期生成的代码就是原生 GPUI 调用。代价仅是编译时间略增。

## 1.4.5 何时选 RML，何时选原生 GPUI

### 推荐 RML 的场景

- ✅ 团队有设计师参与 UI 设计
- ✅ 项目需要长期维护、UI 频繁迭代
- ✅ 团队有 Web 前端背景工程师
- ✅ 需要标准化 UI 开发流程
- ✅ 需要热重载提升开发效率

### 推荐原生 GPUI 的场景

- ✅ 小型工具脚本，UI 简单且固定
- ✅ 性能极致敏感，不愿承担任何编译时间增加
- ✅ 团队全是 Rust 老手，且无设计师参与
- ✅ 需要使用 GPUI 的所有底层能力（自定义绘制、复杂动画等）

## 1.4.6 迁移路径

如果你已有原生 GPUI 项目，可以**渐进式迁移**到 RML：

1. 先用 RML 实现新视图，旧视图保持原生
2. 在 `mod.rs` 中混合导出 RML 视图与原生视图
3. 逐步把旧视图改写为 RML

详见 [第 11 章 · 迁移指南](../11-cookbook/migration-guide.md)。

## 1.4.7 小结

RML 不是 GPUI 的替代品，而是 GPUI 之上的**工业化开发范式**。它用文件级分离、HTML 语法、编译期生成三件事，把 GPUI 的渲染能力释放给更广泛的开发者群体。在团队协作、长期维护、设计沉淀维度上，RML 显著优于原生 GPUI 和 gpui-rsx；在性能维度上，三者等价。

下一章 → [第 2 章 · RML 标记语言](../02-syntax/INDEX.md)
