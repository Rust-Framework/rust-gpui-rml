# 10.1 性能优化

> **本节目标**：理解 RML 的渲染机制，掌握增量渲染、key 策略、避免过度 notify 等优化手段。

## 10.1.1 RML 渲染机制回顾

RML 的渲染基于 GPUI 的保留模式：

```
状态变化 (cx.notify)
    │
    ▼
绑定引擎重新计算依赖该状态的绑定
    │
    ▼
生成的 Render 实现重新构建受影响的子树
    │
    ▼
GPUI diff 新旧树，只重绘变化部分
    │
    ▼
GPU 合成帧
```

性能瓶颈通常出现在三个环节：

1. **notify 过频**：触发太多重渲染
2. **绑定路径过深**：单次重渲染计算量大
3. **列表无 key**：GPUI 无法复用元素，全量重建

## 10.1.2 增量渲染：只重绘变化的部分

GPUI 的 diff 算法基于元素标识与属性比对。RML 编译器为每个 `.rml` 元素生成稳定的元素 ID，GPUI 据此复用旧节点。

```html
<!-- 每个元素在编译期获得稳定 ID，GPUI 据此 diff -->
<div class="list">
  <li r:each="items" r:key="id">{title}</li>
</div>
```

**优化点**：

- 避免在 `r:if` / `r:each` 之间频繁切换不同类型元素（破坏 diff 复用）
- 同一位置尽量保持元素类型稳定

## 10.1.3 key 策略：列表复用的关键

`r:each` 必须配合 `r:key` 使用。key 的作用是让 GPUI 在列表变化时复用已有元素，而非全量重建。

### 反例：无 key 或用 index 作 key

```html
<!-- ❌ 无 key：列表任何变化都全量重建 -->
<li r:each="items">{title}</li>

<!-- ❌ 用 index 作 key：插入/删除时错位 -->
<li r:each="items" r:key="{$index}">{title}</li>
```

### 正例：用稳定唯一字段作 key

```html
<!-- ✅ 用业务唯一 ID -->
<li r:each="items" r:key="id">{title}</li>
```

### key 的选择准则

| 场景          | 推荐 key                     | 不推荐              |
| ----------- | -------------------------- | ---------------- |
| 数据来自数据库     | 主键 ID                      | 数组索引             |
| 用户可编辑的列表    | 客户端生成的 UUID                | 创建时间戳（可能重复）      |
| 静态选项        | 选项的 value                  | 显示文本             |
| 临时项（如新建未保存） | 临时 UUID                    | `temp` 字符串       |

## 10.1.4 避免过度 notify

`cx.notify()` 会触发当前 ViewModel 的所有绑定重新计算。频繁 notify 是性能杀手。

### 反例：循环中 notify

```rust
#[command]
pub fn load_batch(&mut self, items: Vec<Item>, cx: &mut ViewContext<Self>) {
    for item in items {
        self.items.push(item);
        cx.notify(); // ❌ 每次都 notify
    }
}
```

### 正例：批量修改后单次 notify

```rust
#[command]
pub fn load_batch(&mut self, items: Vec<Item>, cx: &mut ViewContext<Self>) {
    self.items.extend(items); // ✅ 批量修改
    cx.notify();              // ✅ 单次 notify
}
```

### 反例：高频输入 notify

```rust
#[command]
pub fn on_input(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
    self.query = ev.value.clone();
    cx.notify(); // 每次按键都触发搜索结果重算
    self.refresh_results(cx); // 又一次 notify
}
```

### 正例：防抖 + 合并 notify

```rust
#[command]
pub fn on_input(&mut self, ev: &ChangeEvent, cx: &mut ViewContext<Self>) {
    self.query = ev.value.clone();
    cx.notify();
    self.schedule_debounced_search(cx); // 内部防抖，不立即 notify
}
```

## 10.1.5 计算属性的依赖追踪

`#[computed]` 通过依赖追踪只在依赖变化时重算。但若依赖过宽，性能会退化。

### 反例：计算属性依赖整个列表

```rust
#[computed]
pub fn first_active_title(&self) -> Option<SharedString> {
    self.items.iter().find(|i| i.active).map(|i| i.title.clone())
    // 任何 item 任何字段变化都触发重算
}
```

### 正例：拆分状态，缩小依赖

```rust
#[derive(Model)]
pub struct VM {
    pub items: Vec<Item>,
    pub active_index: Option<usize>, // 显式追踪
}

impl VM {
    #[computed]
    pub fn first_active_title(&self) -> Option<SharedString> {
        self.active_index.and_then(|i| self.items.get(i)).map(|i| i.title.clone())
    }
}
```

## 10.1.6 大列表的虚拟化

当列表项超过 1000 时，全量渲染会卡顿。RML 提供虚拟列表组件：

```html
<VirtualList items="{items}" item-height="40" height="600">
  <template r:slot="item">
    <div class="row">{title}</div>
  </template>
</VirtualList>
```

`VirtualList` 只渲染可视区域 + 少量缓冲区的项，滚动时动态替换。

## 10.1.7 异步任务的调度

避免在主线程做重计算。用 `cx.background_executor()` 把任务丢到后台：

```rust
#[command]
pub fn analyze(&mut self, cx: &mut ViewContext<Self>) {
    let data = self.data.clone();
    self.is_analyzing = true;
    cx.notify();
    cx.spawn(|this, mut cx| async move {
        // 在后台线程计算
        let result = cx.background_executor().spawn(async move {
            heavy_compute(&data)
        }).await;
        let _ = this.update(&mut cx, |this, cx| {
            this.result = result;
            this.is_analyzing = false;
            cx.notify();
        });
    }).detach();
}
```

## 10.1.8 性能测量

### 渲染耗时

```rust
#[on_loaded]
pub fn on_loaded(&mut self, cx: &mut ViewContext<Self>) {
    cx.observe_render_time(|duration| {
        if duration > Duration::from_millis(16) {
            log::warn!("渲染耗时 {:?}，掉帧风险", duration);
        }
    });
}
```

### notify 计数

```rust
// 在测试中统计 notify 次数
let counter = cx.notify_counter();
vm.load_batch(items, &mut cx);
assert_eq!(counter.get(), 1, "应当只 notify 一次");
```

### 绑定追踪

```sh
RML_TRACE_BINDING=1 cargo run
```

环境变量打开后，每次绑定重算都会打印日志，用于定位过度 notify。

## 10.1.9 性能优化清单

- [ ] 所有 `r:each` 都有稳定的 `r:key`
- [ ] 循环中不调用 `cx.notify()`
- [ ] 高频输入有防抖
- [ ] 计算属性依赖范围最小化
- [ ] 大列表使用 `VirtualList`
- [ ] 重计算在后台线程
- [ ] 无 5 层以上的绑定路径
- [ ] 渲染耗时 < 16ms（60fps）

下一节 → [10.2 调试技巧](./debugging.md)
