# 10.1 性能优化

> **本节目标**：理解 RML 基于 GPUI 的真实渲染机制，掌握 `#[computed]` 缓存、`#[command(no_notify)]` 选择性 notify、element ID 稳定性等优化手段。

## 10.1.1 GPUI 渲染模型

RML 建立在 GPUI 之上，渲染流程如下：

```
cx.notify()
    │
    ▼
标记 Entity 为脏（app.notify(entity_id)）
    │
    ▼
下一次 frame：整个 render() 方法重新执行
    │
    ▼
重建整棵 element 树（所有 format!()、.child() 重新求值）
    │
    ▼
GPUI element ID intern：相同 ID 的元素复用 layout/paint 状态
    │
    ▼
GPU 合成帧
```

**关键事实**：
- `cx.notify()` 触发**整个** `render()` 重建，无法跳过子树
- `AnyElement` 是 `ArenaBox`，frame 结束时释放，**无法跨 frame 缓存 element**
- GPUI 的 element ID intern 机制会缓存 layout/paint 状态（非 render 本身），这是内置优化
- `#[computed]` 缓存的是**数据**（计算结果），不是 element

性能瓶颈通常出现在：
1. **notify 过频**：每次按键都全量重建
2. **render 内重计算**：`#[computed]` 未使用，昂贵计算重复执行
3. **element ID 不稳定**：GPUI 无法复用 layout/paint 状态

## 10.1.2 `#[computed]` 数据缓存

`#[computed]` 通过版本号追踪依赖，只在依赖字段变化时重算。这是 RML 最重要的性能优化手段。

```rust
#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
    pub items: Vec<Item>,
}

impl MainWindow {
    /// 只在 count 或 items 变化时重算
    #[computed]
    pub fn summary(&self) -> String {
        format!("共 {} 项，计数 {}", self.items.len(), self.count)
    }
}
```

**缓存机制**：
- 每个 `pub` 字段注入 `__rml_<field>_version: AtomicU64`
- `#[command]` 修改字段时自动 `bump_version`
- `#[computed]` 方法调用时检查依赖字段版本号之和，未变则返回缓存值

**优化要点**：
- 依赖范围最小化：`#[computed]` 只访问必要的字段
- 避免在 `#[computed]` 中访问整个 `Vec`（任何元素变化都触发重算）

## 10.1.3 `#[command(no_notify)]` 选择性 notify

默认情况下，`#[command]` 自动注入 `cx.notify()`。但有些场景不需要立即更新 UI：

```rust
// 默认：自动 notify（即时反馈）
#[command]
pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
    // 宏自动注入：self.__rml_bump_version("count"); cx.notify();
}

// 不自动 notify（批量操作）
#[command(no_notify)]
pub fn batch_update(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.a = 1;
    self.b = 2;
    self.c = 3;
    // 宏注入 bump_version 但不注入 notify
    cx.notify(); // 手动调用一次，而非三次
}
```

**适用场景**：
- 批量操作：多个字段修改后只需一次 notify
- 后台更新：数据加载中，UI 不需即时更新
- 条件更新：根据业务逻辑决定是否 notify

**`__rml_changed_fields()` 方法**：返回所有 observable 字段名，供手动判断：

```rust
#[command(no_notify)]
pub fn conditional_update(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
    self.count += 1;
    // 只在 count > 10 时更新 UI
    if self.count > 10 {
        cx.notify();
    }
}
```

## 10.1.4 Element ID 稳定性

GPUI 的 element ID intern 机制要求元素有稳定 ID。RML 通过 `ref` 指令提供：

```html
<div ref="list_container">
    <h1 ref="title">{title}</h1>
    <Button ref="submit_btn" onclick={on_submit} label="提交" />
</div>
```

**生成代码**：
```rust
// ref="title" → .id("rml_ref:title")
gpui::div().id("rml_ref:title").child(format!("{}", self.title))
```

**优化要点**：
- 为有状态的元素（事件监听器、动画）添加 `ref`
- 列表渲染时用 `each` 指令的 item 变量生成唯一 ID
- 避免 `r:if` 频繁切换元素类型（破坏 ID 复用）

## 10.1.5 避免过度 notify

### 反例：循环中 notify

```rust
#[command]
pub fn load_batch(&mut self, items: Vec<Item>, cx: &mut Context<Self>) {
    for item in items {
        self.items.push(item);
        // ❌ 每次 push 都触发 notify（#[command] 自动注入）
    }
}
```

### 正例：批量修改 + 单次 notify

```rust
#[command(no_notify)]
pub fn load_batch(&mut self, items: Vec<Item>, cx: &mut Context<Self>) {
    self.items.extend(items);
    cx.notify(); // ✅ 单次 notify
}
```

### 反例：高频输入 notify

```rust
// 每次 on_change 都触发全量 render 重建
#[command]
pub fn on_input(&mut self, state: &InputState, cx: &mut Context<Self>) {
    self.query = state.value();
    // #[command] 自动注入 cx.notify()
    self.refresh_results(cx); // 又一次 notify
}
```

### 正例：防抖搜索

```rust
#[command(no_notify)]
pub fn on_input(&mut self, state: &InputState, cx: &mut Context<Self>) {
    self.query = state.value();
    cx.notify(); // 更新输入框显示
    self.schedule_debounced_search(cx); // 内部防抖，延迟 notify
}
```

## 10.1.6 异步任务调度

重计算应放到后台线程，避免阻塞渲染：

```rust
#[command]
pub fn analyze(&mut self, cx: &mut Context<Self>) {
    let data = self.data.clone();
    self.is_analyzing = true;
    // #[command] 自动注入 notify

    cx.spawn(|this, mut cx| async move {
        let result = cx.background_executor()
            .spawn(async move { heavy_compute(&data) })
            .await;
        let _ = this.update(&mut cx, |this, cx| {
            this.result = result;
            this.is_analyzing = false;
            cx.notify();
        });
    }).detach();
}
```

## 10.1.7 性能优化清单

- [ ] 昂贵计算用 `#[computed]` 缓存
- [ ] 批量操作用 `#[command(no_notify)]` + 手动单次 notify
- [ ] 有状态的元素添加 `ref` 指令（稳定 element ID）
- [ ] 循环中不触发 notify
- [ ] 高频输入考虑防抖
- [ ] 重计算放到后台线程
- [ ] `#[computed]` 依赖范围最小化
- [ ] 列表渲染用 `each` + 稳定 key

## 10.1.8 性能调试

### 检查 `#[computed]` 缓存命中

在 `#[computed]` 方法中加日志：

```rust
#[computed]
pub fn summary(&self) -> String {
    eprintln!("summary recomputed"); // 版本号变化时才打印
    format!("共 {} 项", self.items.len())
}
```

如果日志打印次数多于预期，说明依赖范围过宽。

### 检查 notify 频率

```rust
#[command(no_notify)]
pub fn debug_notify(&mut self, cx: &mut Context<Self>) {
    self.count += 1;
    eprintln!("before notify");
    cx.notify();
    eprintln!("after notify");
}
```

### GPUI 内置性能工具

```sh
# 启用 GPUI 的帧调试
GPUI_FRAME_DEBUG=1 cargo run -p rust-rml-demo
```

下一节 → [10.2 调试技巧](./debugging.md)
