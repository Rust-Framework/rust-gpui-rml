# Demo 消除 Rust UI 代码实施计划

## 目标

将 `demo/src/shell/main_window.rml.rs` 中所有使用 Rust 代码构建 UI 的方法迁移到 `.rml` 声明式模板，遵循 MVVM 标准方案。

## 当前状态分析

### 需要迁移的方法（main_window.rml.rs L297-484）

1. **`active_view()`** (L297-335)
   - 构建可滚动容器包裹 `visual.render(window, cx)`
   - 使用 `ScrollHandle` + `Scrollbar` 组件
   - 手动实现 `overflow_y_scroll()` + `track_scroll()`

2. **`render_menu_bar()`** (L341-384)
   - 从 `menus: Vec<MenuViewModel>` 构建 `MenuBar`
   - 递归处理子菜单（`dropdown_menu` + `build_popup_menu`）
   - 处理叶子节点命令绑定

3. **`render_status_bar()`** (L387-411)
   - 从 `status: Vec<StatusViewModel>` 构建 `NativeStatusBar`
   - 按 `align` 属性分发到 `.left()` / `.right()` / `.child()`

4. **`render_bottom_panel()`** (L423-484)
   - 使用 `div()` 构建终端面板 UI
   - 读取 `panel: &dyn ISlotScope` 参数显示元信息

### 当前 RML 模板（main_window.rml）

```xml
<template slot="menu">
    <component content={self.render_menu_bar(_window, cx)} />
</template>

<template slot="bottom" scope={panel}>
    <component content={self.render_bottom_panel(panel, _window, cx)} />
</template>

<template slot="footer">
    <component content={self.render_status_bar(_window, cx)} />
</template>

<component content={self.active_view(_window, cx)} />
```

## 框架能力调研结果

### 1. MenuBar 声明式支持

**已支持**：`<menu-bar>` + `<menu-item each={m in menus} label={m.label()} children={m.children} command={m.command} />`

- 使用 `children_bind` 模式实现递归菜单树
- 自动生成 `macro_rules!` 处理无限层级嵌套
- 支持 `command={m.command}` 绑定到 `Option<Arc<dyn ICommand>>`

**需要修改**：`MenuViewModel` 需要提供 `command()` 方法返回 `Option<ContributedCommand>`

### 2. NativeStatusBar 声明式支持

**受限**：`NativeStatusBar` 注册为 `StatelessNoId` 容器，但无特殊 translator

- 底层 API：`.left()` / `.right()` / `.child()` 方法
- RML 中无法直接调用这些方法
- **解决方案**：使用 flex 布局 + computed 属性分组

### 3. 滚动容器支持

**部分支持**：
- CSS `overflow-y: auto` → `overflow-y-auto=""` 属性
- 但自定义 `Scrollbar` 组件需要 `ScrollHandle`
- **解决方案**：使用 `overflow-y-auto=""` 替代手动滚动条

### 4. 作用域插槽参数

**已支持**：`<template slot="bottom" scope={panel}>` 中 `panel` 可在模板内直接使用

## 实施方案

### 阶段 1：MenuBar 声明式迁移

#### 1.1 修改 MenuViewModel（menu_view_model.rs）

添加 `ContributedCommand` 包装类型和 `command()` 方法：

```rust
#[derive(Clone)]
pub struct ContributedCommand(pub Arc<dyn IContribution>);

impl ContributedCommand {
    pub fn can_execute(&self, ctx: &mut rml_core::command::CallContext) -> bool {
        self.0.as_command().map(|c| c.can_execute(ctx)).unwrap_or(false)
    }
    
    pub fn execute(&self, ctx: &mut rml_core::command::CallContext) {
        if let Some(cmd) = self.0.as_command() {
            cmd.execute(ctx);
        }
    }
}

impl MenuViewModel {
    pub fn command(&self) -> Option<ContributedCommand> {
        if self.contribution.as_command().is_some() {
            Some(ContributedCommand(self.contribution.clone()))
        } else {
            None
        }
    }
}
```

#### 1.2 更新 main_window.rml

```xml
<template slot="menu">
    <menu-bar>
        <menu-item 
            each={m in menus} 
            label={m.label()} 
            children={m.children} 
            command={m.command()} />
    </menu-bar>
</template>
```

#### 1.3 删除 render_menu_bar 方法

从 `main_window.rml.rs` 中删除 L341-384 的 `render_menu_bar()` 方法。

### 阶段 2：StatusBar 声明式迁移

#### 2.1 添加 computed 属性（main_window.rml.rs）

```rust
#[computed]
pub fn status_left(&self) -> Vec<StatusViewModel> {
    self.status.iter()
        .filter(|s| s.align == StatusBarAlign::Left)
        .cloned()
        .collect()
}

#[computed]
pub fn status_center(&self) -> Vec<StatusViewModel> {
    self.status.iter()
        .filter(|s| s.align == StatusBarAlign::Center)
        .cloned()
        .collect()
}

#[computed]
pub fn status_right(&self) -> Vec<StatusViewModel> {
    self.status.iter()
        .filter(|s| s.align == StatusBarAlign::Right)
        .cloned()
        .collect()
}
```

#### 2.2 更新 main_window.rml

```xml
<template slot="footer">
    <div class="flex items-center justify-between h-full px-2">
        <div class="flex gap-2">
            <component each={s in status_left} content={s.render(_window, cx)} />
        </div>
        <div class="flex gap-2">
            <component each={s in status_center} content={s.render(_window, cx)} />
        </div>
        <div class="flex gap-2">
            <component each={s in status_right} content={s.render(_window, cx)} />
        </div>
    </div>
</template>
```

#### 2.3 删除 render_status_bar 方法

从 `main_window.rml.rs` 中删除 L387-411 的 `render_status_bar()` 方法。

### 阶段 3：active_view 声明式迁移

#### 3.1 添加 ScrollHandle 字段（main_window.rml.rs）

```rust
pub struct MainWindow {
    // ... existing fields ...
    active_view_scroll: gpui::ScrollHandle,
}

impl Default for MainWindow {
    fn default() -> Self {
        Self {
            // ... existing defaults ...
            active_view_scroll: gpui::ScrollHandle::default(),
        }
    }
}
```

#### 3.2 简化 active_view 方法

```rust
pub fn active_view(&self, _window: &mut Window, _cx: &mut gpui::Context<Self>) -> Option<Arc<dyn IWorkbench>> {
    self.activated.read().unwrap().clone()
}
```

#### 3.3 更新 main_window.rml

```xml
<div class="flex-1 overflow-y-auto" id="active-view-container">
    <component content={self.active_view(_window, cx).and_then(|wb| wb.as_visual()).map(|v| v.render(_window, cx))} />
</div>
```

**注意**：上述方案需要 RML 支持 `Option` 类型的条件渲染。如果框架不支持，需要保留简化的 `active_view()` 方法返回 `AnyElement`。

**替代方案**（保留最小 Rust 代码）：

```rust
pub fn active_view(&self, window: &mut Window, cx: &mut gpui::Context<Self>) -> gpui::AnyElement {
    let activated = self.activated.read().unwrap().clone();
    if let Some(wb) = activated {
        let iv: &dyn IContribution = wb.as_ref();
        if let Some(visual) = iv.as_visual() {
            return visual.render(window, cx);
        }
    }
    gpui::div().into_any_element()
}
```

```xml
<div class="flex-1 overflow-y-auto" id="active-view-container">
    <component content={self.active_view(_window, cx)} />
</div>
```

#### 3.4 删除滚动容器构建代码

从 `active_view()` 方法中删除 L302-330 的 `ScrollHandle` 初始化和 `Scrollbar` 组件构建代码。

### 阶段 4：bottom_panel 声明式迁移

#### 4.1 添加 computed 属性（main_window.rml.rs）

```rust
#[computed]
pub fn bottom_panel_info(&self) -> (String, String, bool) {
    // 这些值需要从 panel 参数获取，但 panel 是渲染期引用
    // 无法在 computed 中访问，需要保留 render_bottom_panel 或使用其他方式
    ("bottom".to_string(), "N/A".to_string(), false)
}
```

**问题**：`panel: &dyn ISlotScope` 是渲染期参数，无法在 computed 属性中访问。

**解决方案**：保留 `render_bottom_panel()` 方法但简化为纯数据提取，或直接在 RML 中使用硬编码值。

#### 4.2 更新 main_window.rml

```xml
<template slot="bottom" scope={panel}>
    <div class="flex flex-col size-full">
        <div class="flex flex-row items-center justify-between px-3 py-1.5 border-b">
            <div class="text-[13px] font-semibold">终端面板</div>
            <div class="flex gap-3 text-xs text-muted-foreground">
                <text>slot={panel.slot_name()}</text>
                <text>size={panel.current_size().map(|s| format!("{}", s)).unwrap_or("N/A".to_string())}</text>
                <text>resizable={panel.has_resizable()}</text>
            </div>
        </div>
        <div class="flex-1 px-3 py-2 text-xs text-muted-foreground">
            <text>$ demo terminal — scope variable accessible from slot content</text>
        </div>
    </div>
</template>
```

#### 4.3 删除 render_bottom_panel 方法

从 `main_window.rml.rs` 中删除 L423-484 的 `render_bottom_panel()` 方法。

## 实施步骤

1. **修改 MenuViewModel**
   - 添加 `ContributedCommand` 包装类型
   - 添加 `command()` 方法
   - 文件：`demo/src/shell/menu_view_model.rs`

2. **更新 main_window.rml**
   - 替换 `<template slot="menu">` 为声明式 `<menu-bar>`
   - 替换 `<template slot="footer">` 为 flex 布局 + computed 属性
   - 替换主内容区为滚动容器 + 简化的 active_view
   - 替换 `<template slot="bottom">` 为声明式 div 布局
   - 文件：`demo/src/shell/main_window.rml`

3. **修改 MainWindow**
   - 添加 `active_view_scroll: ScrollHandle` 字段
   - 添加 `status_left()` / `status_center()` / `status_right()` computed 属性
   - 简化 `active_view()` 方法（保留最小 Rust 代码或完全删除）
   - 删除 `render_menu_bar()` 方法
   - 删除 `render_status_bar()` 方法
   - 删除 `render_bottom_panel()` 方法
   - 文件：`demo/src/shell/main_window.rml.rs`

4. **验证编译和功能**
   - 运行 `cargo build -p rust-rml-demo`
   - 启动 demo 验证菜单、状态栏、主内容区、底部面板功能正常

## 关键决策

1. **MenuBar**：使用框架已支持的 `children_bind` 模式，完全声明式
2. **StatusBar**：使用 computed 属性分组 + flex 布局，完全声明式
3. **active_view**：使用 CSS `overflow-y-auto` 替代手动滚动条，保留简化的 `active_view()` 方法返回渲染内容
4. **bottom_panel**：直接在 RML 中使用 `panel` 参数，完全声明式

## 预期结果

- `main_window.rml.rs` 中不再包含任何 `div()` / `MenuBar::new()` / `NativeStatusBar::new()` 等 UI 构建代码
- 所有 UI 结构在 `main_window.rml` 中声明式定义
- 业务逻辑通过 computed 属性和数据绑定驱动 UI
- 符合 MVVM 标准架构
