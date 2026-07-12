# 11.4 常见问题 FAQ

> **本节目标**：给出高频问题的标准答案，省去反复踩坑的时间。

## 基础概念

### Q1：RML 和 GPUI 是什么关系？

RML 是 GPUI 之上的声明式 UI 框架。GPUI 是 Zed 团队的 Rust UI 框架，RML 在编译期把 `.rml` 模板转换为原生 GPUI 代码，运行时零开销。RML 不替代 GPUI，而是为 GPUI 提供更高层的开发体验。

### Q2：RML 是解释执行还是编译？

编译。`.rml` 在 `build.rs` 中被编译为 Rust 代码，再由 cargo 编译为机器码。运行时没有模板解析、没有反射、没有 JIT。

### Q3：RML 有运行时依赖吗？

有，但很轻量。`rml-runtime` crate 提供绑定引擎、热重载 IPC、ElementRef 等，约几十 KB。核心渲染由 GPUI 完成。

### Q4：RML 必须用 .rml 文件吗？能在 Rust 代码里写吗？

RML 的核心价值在于文件级分离。虽然技术上可以用宏在 Rust 中内联模板，但不推荐——会失去热重载、设计师协作等优势。命令式 GPUI 调用仍可在 `.rml.rs` 中通过 `ElementRef` 使用，但应限于少数场景。

## 绑定与状态

### Q5：为什么我修改了字段，UI 没更新？

最常见原因：忘记调用 `cx.notify()`。RML 不会自动追踪字段修改，必须显式通知。检查清单：

1. 命令方法中是否调用了 `cx.notify()`
2. 是否在后台线程修改状态（必须通过 `this.update`）
3. 异步任务完成后是否在 `this.update` 闭包中 notify

### Q6：计算属性不更新怎么办？

`#[computed]` 通过依赖追踪工作。如果计算属性依赖的字段没变化，它不会重算。检查：

1. 依赖的字段是否真的变化了
2. 字段变化时是否调用了 `cx.notify()`
3. 计算属性是否被模板正确引用（函数名而非字段名）

### Q7：双向绑定和单向绑定怎么选？

- 单向 `{value}`：ViewModel → View，只显示
- 双向 `value={field}`：View ↔ ViewModel，用户可编辑（input、textarea、select）

表单输入用双向，纯展示用单向。

### Q8：能在插值里调用函数吗？

简单函数可以：`{format_date(created_at)}`。但复杂逻辑应收敛到 `#[computed]`，原因：

- 计算属性有依赖追踪，函数调用没有
- 计算属性可单测，模板函数不可
- 模板可读性更好

### Q9：如何深度监听嵌套对象的变化？

RML 的依赖追踪是字段级的。如果 `user.address.city` 变化，需要：

1. 整体替换 `user` 字段（推荐）
2. 或把 `address` 拆为独立字段

不要原地修改嵌套对象的字段，依赖追踪会失效。

## 组件

### Q10：组件和视图有什么区别？

- **视图**：对应一个路由 / 窗口 / 页面，有自己的 ViewModel
- **组件**：可复用的 UI 单元，接受 props，触发事件，通常无独立状态

视图是“页面级”，组件是“控件级”。

### Q11：组件能有自己的状态吗？

可以，但不推荐。组件应当尽量无状态，状态由父视图通过 props 传入。若组件确实需要内部状态（如展开 / 折叠），可用私有字段 + `cx.notify()`，但不要让父视图依赖这个状态。

### Q12：如何在组件中触发父视图的事件？

通过组件的自定义事件：

```html
<!-- 父视图 -->
<TodoItem todo="{$item}" on:toggle="toggle" on:remove="remove" />
```

```rust
// 组件内部
#[command]
pub fn on_click(&mut self, cx: &mut ViewContext<Self>) {
    cx.emit(ToggleEvent { id: self.todo.id });
}
```

### Q13：插槽和 props 有什么区别？

- **props**：数据，传给组件
- **插槽**：内容（UI 片段），传给组件渲染

需要传 UI 结构时用插槽，传数据时用 props。

## 样式

### Q14：RML 支持完整 CSS 吗？

不支持。RML 实现了 CSS 子集，覆盖布局（Flexbox）、盒模型、颜色、字体、选择器（类、ID、后代）。不支持 CSS 动画的 `@keyframes` 之外的高级特性。详见第 7 章。

### Q15：如何做主题切换？

用 CSS 变量 + Context 事件：

```css
:root { --color-bg: #fff; }
:root.dark { --color-bg: #000; }
```

```rust
cx.dispatch(ThemeChange::Dark);
// 根视图监听后切换根元素的 class
```

### Q16：能用 Tailwind 吗？

可以。RML 支持类名形式的样式，Tailwind 风格的 utility class 可直接使用。也可通过工具类映射到 GPUI 样式 API。详见 7.5 节。

## 生命周期

### Q17：on_loaded 何时触发？

视图首次渲染完成后触发。此时可以安全地访问 ElementRef、启动定时器、订阅事件。注意：`on_loaded` 只触发一次，状态变化导致的重渲染不会再触发。

### Q18：如何取消异步任务？

保存任务的 `Task` 句柄，在 `on_unloaded` 中 abort：

```rust
#[on_unloaded]
pub fn on_unloaded(&mut self, _cx: &mut ViewContext<Self>) {
    if let Some(t) = self.load_task.take() { t.abort(); }
}
```

### Q19：内存泄漏怎么排查？

常见原因：

1. 循环引用：ViewModel 持有子 ViewModel 的强引用，子又持有父
2. 未取消的订阅：`cx.subscribe` 返回的 `Subscription` 未保存，被 drop 后订阅本应取消——但若保存不当会泄漏
3. 未 abort 的任务：任务持有 `this` 的弱引用，通常安全；但若持有强引用则泄漏

用 `WeakEntity` 替代强引用，用 `cargo run --features=leak-detector` 排查。

## 性能

### Q20：列表滚动卡顿怎么办？

1. 确认 `each` 列表元素有稳定的 `key={item.id}`
2. 列表项超过 1000 时用 `VirtualList`
3. 检查列表项的 `#[computed]` 依赖是否过宽
4. 用 `RML_TRACE_BINDING=1` 看绑定重算频率

### Q21：渲染慢怎么定位？

1. `cx.observe_render_time` 测量
2. `RML_TRACE_BINDING=1` 看绑定
3. `cargo rml-expand` 看生成代码是否异常
4. 检查是否有深嵌套或大列表

## 工具链

### Q22：热重载不生效？

1. 检查 `rml` 是否启用 `hot-reload` feature
2. 检查是否在 dev profile
3. 检查文件是否在被监听目录内
4. 检查 IDE 是否真的写盘

### Q23：build.rs 报错“找不到 .rml 文件”？

检查 `scan_dir` 路径。默认扫描 `src`，若模板在其他目录需显式配置。

### Q24：LSP 补全不出现？

1. 等待索引完成（首次 10-30s）
2. 检查 ViewModel 是否有 `#[derive(IModel)]`
3. 检查字段是否 `pub`
4. 重启 LSP：`Ctrl+Shift+P` → `RML: Restart Server`

## 架构

### Q25：ViewModel 多大算太大？

经验法则：

- 字段超过 15 个：考虑拆分
- 命令超过 10 个：考虑拆分
- 行数超过 300 行：审视职责

不是硬性限制，而是审视信号。

### Q26：跨视图如何共享状态？

用 Context / 全局 Model：

```rust
cx.set_global(AuthState { user: Some(user) });
let user = cx.global::<AuthState>().user.clone();
```

不要把一个 ViewModel 传给多个视图。

### Q27：Service 必须是 async 吗？

推荐 async。I/O 通常是异步的，async Service 可在主线程不阻塞。纯计算 Service 可同步。

## 其他

### Q28：RML 支持移动端吗？

目前 RML 基于 GPUI，GPUI 主要面向桌面。移动端支持取决于 GPUI 的演进。

### Q29：RML 支持服务端渲染吗？

不直接支持。RML 的 Model 层可独立于 GPUI，理论上可在服务端复用数据层，但 UI 渲染依赖 GPUI 的 GPU 能力。

### Q30：如何为 RML 项目做国际化？

推荐方案：

1. 在 `assets/i18n/` 放置 `zh-CN.json`、`en-US.json` 等资源文件
2. 在 `on_launch` 或视图 `on_loaded` 中调用 `cx.use_i18n("zh-CN")`
3. RML 模板使用 `{t("menu.file")}`，codegen 生成 `cx.t(...)`
4. 切换语言时调用 `cx.set_i18n("en-US")` 并 `cx.notify()`

```rust
impl IAppLifecycle for MyApp {
    fn on_launch(&mut self, cx: &mut App) {
        cx.use_i18n("zh-CN");
        MainWindow::default().open(cx);
    }
}
```

```html
<button label={t("app.save")} />
```

构建期可用 `rml::build().extract_i18n("assets/i18n/zh-CN.json")` 扫描模板并补全缺失 key。

下一节 → [11.5 避坑清单](./pitfall-checklist.md)
