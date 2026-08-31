# 11.1 常见 UI 模式

> **本节目标**：把绑定、组件、样式组合成可复用的 UI 解决方案，覆盖表单、列表、对话框、抽屉、Tab、向导六大模式。

## 11.1.1 表单模式

### 受控表单

每个字段双向绑定到 ViewModel，提交时统一校验。

```html
<div class="form">
  <label>
    邮箱
    <input type="email" value={email} />
  </label>
  <label>
    密码
    <input type="password" value={password} />
  </label>
  <p if={errors.email} class="error">{errors.email}</p>
  <p if={errors.password} class="error">{errors.password}</p>
  <Button label="提交" on-click={on_submit} disabled={!can_submit} />
</div>
```

```rust
#[derive(IModel)]
pub struct FormViewModel {
    pub email: SharedString,
    pub password: SharedString,
    pub errors: FormErrors,
    pub is_submitting: bool,
}

#[derive(Model, Default)]
pub struct FormErrors {
    pub email: Option<SharedString>,
    pub password: Option<SharedString>,
}

impl FormViewModel {
    #[computed]
    pub fn can_submit(&self) -> bool {
        !self.is_submitting
            && !self.email.is_empty()
            && !self.password.is_empty()
            && self.errors.email.is_none()
            && self.errors.password.is_none()
    }

    #[command]
    pub fn on_submit(&mut self, _ev: &SubmitEvent, cx: &mut ViewContext<Self>) {
        self.validate();
        if self.errors.email.is_some() || self.errors.password.is_some() {
            cx.notify();
            return;
        }
        self.is_submitting = true;
        cx.notify();
        // 提交逻辑...
    }

    fn validate(&mut self) {
        self.errors.email = if self.email.is_empty() { Some("必填".into()) }
            else if !self.email.contains('@') { Some("格式错误".into()) }
            else { None };
        self.errors.password = if self.password.len() < 8 { Some("至少 8 位".into()) }
            else { None };
    }
}
```

**要点**：

- 校验在 ViewModel，不在模板
- `can_submit` 计算属性控制按钮 disabled
- 错误信息作为状态字段，模板只读

### 动态表单

字段数量动态变化（如“添加更多”按钮）：

```html
<div class="form">
  <div each={field in fields} key={field.id} class="field-row">
    <input value={field.value} on-change={on_field_change} />
    <Button label="删除" on-click={remove_field} />
  </div>
  <Button label="添加字段" on-click={add_field} />
</div>
```

```rust
#[command]
pub fn add_field(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.fields.push(Field { id: uuid(), value: "".into() });
    cx.notify();
}

#[command]
pub fn remove_field(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    let id = ev.data::<u64>();
    self.fields.retain(|f| f.id != id);
    cx.notify();
}
```

## 11.1.2 列表模式

### 可筛选列表

```html
<div class="list-view">
  <input value={filter} placeholder="筛选…" on-change={on_filter} />
  <p if={filtered_items.is_empty()}>无匹配项</p>
  <ul>
    <li each={item in filtered_items} key={item.id} on-click={select}>
      <span>{item.title}</span>
      <span if={item.id == selected_id} class="check">✓</span>
    </li>
  </ul>
</div>
```

```rust
#[derive(IModel)]
pub struct ListViewModel {
    pub items: Vec<Item>,
    pub filter: SharedString,
    pub selected_id: Option<u64>,
}

impl ListViewModel {
    #[computed]
    pub fn filtered_items(&self) -> Vec<Item> {
        let f = self.filter.to_lowercase();
        self.items.iter()
            .filter(|i| i.title.to_lowercase().contains(&f))
            .cloned()
            .collect()
    }

    #[command]
    pub fn select(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        self.selected_id = ev.data::<u64>();
        cx.notify();
    }
}
```

### 分页列表

```rust
#[derive(IModel)]
pub struct PagedViewModel {
    pub items: Vec<Item>,
    pub page: usize,
    pub page_size: usize,
}

impl PagedViewModel {
    #[computed]
    pub fn current_page_items(&self) -> &[Item] {
        let start = self.page * self.page_size;
        let end = (start + self.page_size).min(self.items.len());
        &self.items[start..end]
    }

    #[computed]
    pub fn total_pages(&self) -> usize {
        (self.items.len() + self.page_size - 1) / self.page_size
    }

    #[command]
    pub fn next_page(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.page + 1 < self.total_pages() {
            self.page += 1;
            cx.notify();
        }
    }
}
```

### 虚拟滚动大列表

```html
<VirtualList items={items} item-height="40" height="600">
  <template slot="item">
    <div class="row">{title}</div>
  </template>
</VirtualList>
```

## 11.1.3 对话框模式

### 受控对话框

对话框的显隐由父视图状态控制，自身只管内容。

```html
<!-- 父视图 -->
<div>
  <Button label="打开" on-click={open_dialog} />
  <Dialog open={is_dialog_open} title="确认删除？" on-confirm={confirm_delete} on-cancel={close_dialog} />
</div>
```

```rust
#[command]
pub fn open_dialog(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.is_dialog_open = true;
    cx.notify();
}

#[command]
pub fn confirm_delete(&mut self, _ev: &ConfirmEvent, cx: &mut ViewContext<Self>) {
    self.is_dialog_open = false;
    self.delete_item(cx);
}

#[command]
pub fn close_dialog(&mut self, _ev: &CancelEvent, cx: &mut ViewContext<Self>) {
    self.is_dialog_open = false;
    cx.notify();
}
```

### 全局对话框服务

跨视图复用的对话框通过 Context 服务：

```rust
pub struct DialogService { current: Option<DialogConfig> }

impl DialogService {
    pub fn show(&mut self, config: DialogConfig, cx: &mut AppContext) {
        self.current = Some(config);
        cx.notify_global();
    }
}
```

任何视图调用 `cx.global::<DialogService>().show(...)` 即可弹窗。

## 11.1.4 抽屉模式

```html
<div class="drawer-container">
  <div class="content">{children}</div>
  <aside class="drawer" class:open={is_drawer_open}>
    <header>
      <h2>{drawer_title}</h2>
      <Button label="✕" on-click={close_drawer} />
    </header>
    <div class="drawer-body">
      <template slot="drawer-content" />
    </div>
  </aside>
</div>
```

抽屉组件接受 `is_open`、`title` props 和 `drawer-content` 插槽，父视图控制状态。

## 11.1.5 Tab 模式

```html
<div class="tabs">
  <div class="tab-headers">
    <Button each={tab in tabs} key={tab.id}
            class:active={tab.id == active_tab}
            on-click={switch_tab}
            label={tab.label} />
  </div>
  <div class="tab-content">
    <div if={active_tab == 'profile'}><ProfileTab /></div>
    <div if={active_tab == 'settings'}><SettingsTab /></div>
  </div>
</div>
```

```rust
#[command]
pub fn switch_tab(&mut self, ev: &ClickEvent, cx: &mut ViewContext<Self>) {
    self.active_tab = ev.data::<SharedString>();
    cx.notify();
}
```

**优化**：用枚举而非字符串表示 tab，避免拼写错误：

```rust
#[derive(Model, Clone, PartialEq)]
pub enum Tab { Profile, Settings }

#[derive(IModel)]
pub struct TabsVM { pub active: Tab }
```

## 11.1.6 向导模式（多步表单）

```rust
#[derive(IModel)]
pub struct WizardViewModel {
    pub step: usize,
    pub total_steps: usize,
    pub data: WizardData,
}

impl WizardViewModel {
    #[computed]
    pub fn can_next(&self) -> bool {
        match self.step {
            0 => !self.data.name.is_empty(),
            1 => self.data.email.contains('@'),
            2 => !self.data.password.is_empty(),
            _ => false,
        }
    }

    #[command]
    pub fn next(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.can_next() && self.step < self.total_steps - 1 {
            self.step += 1;
            cx.notify();
        }
    }

    #[command]
    pub fn prev(&mut self, _ev: &ClickEvent, cx: &mut ViewContext<Self>) {
        if self.step > 0 {
            self.step -= 1;
            cx.notify();
        }
    }
}
```

```html
<div class="wizard">
  <div class="steps">
    <span each={i in step_indices} key={i} class:active={i == step} />
  </div>
  <div class="step-content">
    <div if={step == 0}><input value={data.name} placeholder="姓名" /></div>
    <div if={step == 1}><input value={data.email} placeholder="邮箱" /></div>
    <div if={step == 2}><input value={data.password} type="password" /></div>
  </div>
  <div class="actions">
    <Button if={step > 0} label="上一步" on-click={prev} />
    <Button if={step < total_steps - 1} label="下一步" on-click={next} disabled={!can_next} />
    <Button if={step == total_steps - 1} label="完成" on-click={finish} />
  </div>
</div>
```

## 11.1.7 模式速查

| 需求          | 推荐模式                  |
| ----------- | --------------------- |
| 用户输入并提交     | 受控表单 + 计算属性校验         |
| 大量数据展示      | 虚拟列表 + 分页             |
| 临时确认        | 受控对话框 / 全局对话框服务       |
| 侧边详情        | 抽屉组件 + 插槽             |
| 多视图切换       | Tab + 枚举状态            |
| 多步流程        | 向导 + 步骤枚举             |
| 实时搜索        | 防抖 input + 计算属性筛选     |
| 拖拽排序        | `each` + 拖拽事件 + 重排命令 |

下一节 → [11.2 案例研究：Todo 应用](./case-study.md)
