# Demo 交互修复计划 v2

## 背景

用户报告:"示例页面的组件显示不合理,且无法正常响应鼠标、键盘操作"。

经前一轮对话已完成 welcome_case 与 button_case 重写(改动 A-F),编译通过。但运行时验证发现两类残留问题:

1. **鼠标操作失效** — demo 层 `main_window.rml.rs` 的 Scrollbar 绝对覆盖层拦截事件
2. **显示不合理** — codegen 转换 CSS class 时丢失 `flex-wrap: wrap` 属性

经用户确认,本轮范围:**仅修 A(demo Scrollbar 覆盖层)+ C(codegen flex-wrap 丢失)**,不动框架 Drag 区域(B),不实现键盘支持(D)。

## 当前状态分析

### 问题 A: Scrollbar 绝对覆盖层拦截鼠标

**位置**: [demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs#L255-L281)

```rust
return gpui::div()
    .id("active-view-container")
    .size_full()
    .relative()
    .child(
        gpui::div()
            .id("active-view-scroll-area")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&scroll_handle)
            .child(visual.render(window, cx)),
    )
    .child(  // ← 问题:绝对覆盖层
        gpui::div()
            .absolute()
            .top_0()
            .left_0()
            .right_0()
            .bottom_0()  // 覆盖整个 active-view-container
            .child(
                gpui_component::scroll::Scrollbar::vertical(&scroll_handle)
                    .id("active-view-scrollbar"),
            ),
    )
    .into_any_element();
```

**根因**: 第二个 child 是 `absolute().top_0().left_0().right_0().bottom_0()` 的全屏覆盖层,渲染在 scroll-area 之上。虽然外层 div 本身无 id(非 Stateful),但其内的 Scrollbar 组件有 id,且 Scrollbar 的命中区域可能覆盖整个父容器宽度,导致点击事件被 Scrollbar 拦截而非穿透到下层的案例视图内容。

**正确做法**: gpui-component 的 `Scrollbar::vertical(&handle)` 本身已实现绝对定位到右边缘,不需要外层全屏 absolute 包装。直接作为 `active-view-container` 的子元素即可。

### 问题 C: codegen 丢失 flex-wrap CSS 属性

**位置**: [crates/engine/src/css/mapper.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/css/mapper.rs#L60-L103)

Flexbox 区段(行 60-103)映射了 `display` / `flex-direction` / `justify-content` / `align-items` / `flex` / `min-width` / `min-height` / `gap`,但**完全没有 `flex-wrap` 映射**。

**受影响的 CSS 规则**:
- [demo/assets/styles.css:55-61](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css#L55-L61) — `.button-row { flex-wrap: wrap; ... }`
- [demo/assets/styles.css:186-191](file:///e:/GitCode/RF/rust-gpui-rml/demo/assets/styles.css#L186-L191) — `.welcome-grid-row { flex-wrap: wrap; ... }`

**现象**: 生成的 button_case.rs 中 `.button-row` 对应的 div 只有 `.flex().flex_row().gap().justify_center()`,缺少 `.flex_wrap()`,导致按钮行不换行,横向溢出。

**GPUI 对应方法**: `Styled::flex_wrap()` (无参数,对应 `flex-wrap: wrap`)

## 提议改动

### 改动 A: 移除 Scrollbar 绝对覆盖层

**文件**: `demo/src/shell/main_window.rml.rs`
**行**: 255-281 (`active_view` 方法)

**变更**: 删除外层 absolute 包装 div,让 Scrollbar 直接作为 `active-view-container` 的子元素。

**修改后结构**:
```rust
return gpui::div()
    .id("active-view-container")
    .size_full()
    .relative()
    .child(
        gpui::div()
            .id("active-view-scroll-area")
            .size_full()
            .flex()
            .flex_col()
            .overflow_y_scroll()
            .track_scroll(&scroll_handle)
            .child(visual.render(window, cx)),
    )
    .child(
        gpui_component::scroll::Scrollbar::vertical(&scroll_handle)
            .id("active-view-scrollbar"),
    )
    .into_any_element();
```

**原理**: 
- `Scrollbar::vertical(&handle)` 内部已通过 `absolute()` 自定位到父容器的右边缘,无需外层 absolute 包装
- 父容器 `active-view-container` 已有 `.relative()`,Scrollbar 的 absolute 定位会以此为定位上下文
- 移除全屏 absolute div 后,Scrollbar 仅在其自身命中区域(右边缘窄条)拦截事件,其余区域事件穿透到下层 scroll-area

**风险**: 极低。Scrollbar 组件本身设计为直接作为 relative 容器的子元素使用,这是 gpui-component 的标准模式。

### 改动 C: codegen 添加 flex-wrap 映射

**文件**: `crates/engine/src/css/mapper.rs`
**行**: Flexbox 区段(行 60-103 之间,建议在 `flex-direction` 之后)

**变更**: 添加 `flex-wrap` match arm

```rust
"flex-wrap" => match &value {
    Value::Keyword(k) if k == "wrap" => Some("flex_wrap()".into()),
    Value::Keyword(k) if k == "nowrap" => Some("flex_nowrap()".into()),
    _ => None,
},
```

**原理**:
- `wrap` → `flex_wrap()` (GPUI `Styled::flex_wrap()`)
- `nowrap` → `flex_nowrap()` (GPUI `Styled::flex_nowrap()`)
- 其他值(如 `wrap-reverse`)暂不支持,静默跳过(向前兼容)

**验证**: 修复后,生成的 button_case.rs 中 `.button-row` 对应的 div 应包含 `.flex_wrap()`,按钮行将正确换行。

**需清理构建缓存**: 修改 mapper.rs 后需 `cargo clean -p rust-rml-demo` 清除过期生成代码,再重新编译。

## 假设与决策

1. **假设**: gpui-component 的 `Scrollbar::vertical` 内部已实现 absolute 自定位。若实际不是,需保留外层 absolute 但缩窄到右边缘(如 `absolute().right_0().top_0().bottom_0().w(px(12.))`)。
2. **假设**: `flex_wrap()` / `flex_nowrap()` 是 GPUI `Styled` trait 的方法,无需额外 import。
3. **决策**: 不修改 `tab_window.rs` 的 Drag 区域(问题 B),用户已确认范围排除。
4. **决策**: 不实现键盘事件处理(问题 D),用户已确认范围排除。
5. **决策**: 不修改 CSS 文件本身,仅修复 codegen 映射。CSS 规则已正确定义,问题在 codegen 侧。

## 验证步骤

### 步骤 1: 编译验证
```powershell
cargo clean -p rust-rml-demo
cargo build -p rust-rml-demo
```
- 预期: 0 errors
- 检查点: 生成的 `button_case.rs` 中 `.button-row` 对应的 div 是否包含 `.flex_wrap()`

### 步骤 2: 运行时验证
```powershell
cargo run -p rust-rml-demo
```
- 预期: 应用启动无 panic
- 检查点:
  1. 欢迎页显示标题 + 分组卡片网格
  2. 点击欢迎页卡片 → 打开对应 case Tab(鼠标响应)
  3. Tab 切换正常(鼠标点击 Tab)
  4. Button 案例显示 7 张 Card
  5. Button 案例的按钮行在窗口窄时换行(flex-wrap 生效)
  6. 点击 Button(基础用法的 Default/Primary/Ghost/Danger)→ 计数递增(鼠标响应)
  7. 点击"切换禁用"/"切换选中" → 状态文本更新(鼠标响应)
  8. 滚动条仅出现在右边缘窄条,不覆盖内容区域

### 步骤 3: 生成代码审查
读取 `target/debug/build/rust-rml-demo-*/out/rml_generated/button_case.rs`,确认:
- `.button-row` 对应的 div 包含 `.flex_wrap()`
- 无 `.absolute().top_0().left_0().right_0().bottom_0()` 全屏覆盖层(在 main_window.rml.rs 中已移除)

## 实施顺序

1. 改动 C(mapper.rs 添加 flex-wrap 映射)→ 简单的单点修改
2. 改动 A(main_window.rml.rs 移除 Scrollbar 覆盖层)→ 单点修改
3. `cargo clean -p rust-rml-demo` → 清除过期生成代码
4. `cargo build -p rust-rml-demo` → 编译验证
5. `cargo run -p rust-rml-demo` → 运行时验证(需用户手动确认 UI 行为)
