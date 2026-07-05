# RML 框架生产级迭代计划 v2（融合方案）

## 概述

基于 `production-grade-syntax-component-runtime-plan.md`（语法/组件/运行时三大维度）与 `rml-production-grade-iteration-plan.md`（基础改进）的融合分析，结合前一个 M1 已完成的基础改进工作，制定后续迭代路线。

**设计原则**（用户明确要求）：
- 拒绝任何绕过、妥协、反模式、向下兼容
- RML 框架全新开发，无历史包袱
- 每一行代码都要求有明确的需求和必要性
- 世界级架构规范标准

**总目标**：将 RML 的语法、组件、运行时三大核心维度提升到生产水平。

---

## 已完成工作基线

### 前一个 M1（基础改进）✅
- Card/TabBar 属性映射
- 17 个组件独立 demo（Badge/Label/Separator/Tag/Progress/ProgressCircle/ButtonGroup/AvatarGroup/Card/TitleBar/NativeStatusBar/Checkbox/Switch/Input/Tree/Slider/CodeEditor）
- h2 字号修复、命名颜色扩展、flex:N 数字支持
- strict 字段、单元测试补全
- 修复 6 个 codegen 缺陷（Label/Separator/Tag/Switch/Slider/loading）

### M0'（技术债务清理）✅
- 修复 card_case dead_code warning
- 回填 Tag demo 完整 variant 演示（codegen 已修复）
- 回填 Progress loading 演示（codegen 已修复）
- 删除未消费的 hot_reload 字段（M6' 实现时再添加）

### 调研确认的现状缺口
1. **指令 codegen**：else/once/html/key 完全无效；show 语义错误（等同 if）；ref 仅生成 .id() 未注入 ElementRef
2. **半集成组件**：12 个全部未注册（Form/Kbd/List/Popover/Radio/Select/Tooltip/Notification/AlertDialog/Dialog/Icon/IconName）
3. **CSS 属性覆盖**：约 30 个属性，缺 max-w/max-h、flex-grow/shrink/basis、border 简写、position、calc()、rgb/rgba/hsl/hsla、linear-gradient、伪类、动画
4. **CSS 三层架构**：L1+L2+L3 已实现，缺 `<style>` 标签和 page_stylesheet
5. **运行时能力**：on_prop_change/on_rendered 未实现；RmlState 缺 checkbox/select/binding_stack/rendered_once；无 WindowManager
6. **解析器诊断**：RML 有 line/col 无 source_snippet；CSS 仅有字节偏移；Attribute 无 Span
7. **热重载**：完全未实现

---

## 迭代路线

### M1'：语法层缺陷修复（4 周）

**目标**：消除"已解析但 codegen 未消费"的指令缺陷，修正 show 语义，修复 ref 注入，增强错误诊断。

**核心问题**：用户写了 `else`/`once`/`html`/`key` 不报错也不生效——生产级应用不可接受。

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 1.1 实现 `else` 指令 codegen | `compiler/codegen/node.rs` | 在 if 分支后追踪同父的 else 节点，生成 `if cond { elem } else { else_elem }`；不支持 else if |
| 1.2 实现 `once` 指令 codegen | `compiler/codegen/node.rs` + `ui/src/state.rs` | RmlState 增加 `rendered_once: HashSet<&'static str>`；codegen 生成首次渲染守卫 |
| 1.3 实现 `html` 指令 codegen | `compiler/codegen/node.rs` | GPUI 无原生 HTML 渲染，降级为 `Label::new(raw)` 文本节点（明确文档说明此限制） |
| 1.4 实现 `key` 指令消费 | `compiler/codegen/node.rs` | 列表渲染时基于 key 哈希生成稳定 element_id，而非数组索引 |
| 1.5 修正 `show` 语义 | `compiler/codegen/node.rs` | 改为生成 `.when(!cond, \|d\| d.hidden())`，保留布局空间但隐藏视觉，与 if 区分 |
| 1.6 修复 `ref` ElementRef 注入 | `compiler/codegen/node.rs` + `macros/src/component.rs` + `ui/src/state.rs` | RmlState 增加 `ref_handles: HashMap<String, ElementRef>`；codegen 生成 ref_handles.insert；宏侧识别 ref 字段从 ref_handles 取值 |
| 1.7 Input 事件架构修复 | `compiler/input/event.rs` + `ui/src/components/` | Input 无 .on_change() 方法，通过 InputState EventEmitter<InputEvent> 发送事件。设计正确的 RML 事件映射架构（subscribe 模式而非方法调用） |
| 1.8 解析器错误诊断增强 | `parser/{mod.rs, tokenizer.rs}` + `css/parser.rs` | RML ParseError 增加 source_snippet；CSS ParseError 增加 line/column |
| 1.9 Attribute Span 保留 | `parser/ast.rs` + `compiler/codegen/node.rs` | Attribute::Static/Bind/Event 增加 span 字段，为 LSP 跳转预留 |
| 1.10 指令 demo | `demo/src/cases/` | else/once/html/key/show/ref 各新增 demo 验证行为 |

**验收标准**：
- 9 个指令全部行为符合文档
- `show` 与 `if` 行为可区分（show 隐藏后保留布局空间，if 不保留）
- `ref="input1"` 后 `self.input1.focus(cx)` 可调用
- Input 的 on-change 事件正确工作
- 错误消息显示 line:col + 上下文片段
- 引擎测试全绿

---

### M2'：半集成组件补全 + StatusBar 路由（6 周）

**目标**：将 12 个"已 re-export 但未注册"的组件全部推进到完整集成，按依赖分批实施。

**按依赖分批**：
- 第 1 批：Icon, Kbd（基础展示组件，无依赖）
- 第 2 批：Tooltip, Popover（依赖 Icon）
- 第 3 批：Radio + RadioGroup, Select, Form（表单组件）
- 第 4 批：Dialog, AlertDialog, Notification, List（容器/通知组件）

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 2.1 Icon 集成 | `tags.rs`、`props_registry.rs`、`component.rs` | Stateless，支持 name/size/color |
| 2.2 Kbd 集成 | 同上 | Stateless，支持 key/size |
| 2.3 Tooltip 集成 | 同上 + `compiler/tooltip/` | Stateless，支持 label/placement/trigger |
| 2.4 Popover 集成 | 同上 + `compiler/popover/` | StatelessWithItems，支持 trigger/placement/content |
| 2.5 Radio + RadioGroup 集成 | 同上 + `compiler/radio/` + `ui/src/state.rs` | RadioGroup 持有 selected_value；RmlState 增加 radio_states |
| 2.6 Select 集成 | 同上 + `compiler/select/` + `ui/src/state.rs` | Stateless，支持 items/value/on-change；RmlState 增加 select_states |
| 2.7 Form + FormItem 集成 | 同上 + `compiler/form/` | Form 容器，FormItem 支持 label/required/validate |
| 2.8 Dialog 完整集成 | `tags.rs` + `compiler/dialog/` | 扩展属性映射 + title/open/on-close |
| 2.9 AlertDialog 集成 | 同上 + `compiler/alert_dialog/` | 复用 Dialog，增加 cancel-text/confirm-text/on-confirm |
| 2.10 List 集成 | 同上 + `compiler/list/` | StatelessWithItems，支持 items/render/on-select |
| 2.11 Notification 集成 | 同上 + `compiler/notification/` | 与 Root 集成，支持 title/description/type/duration |
| 2.12 StatusBar 路由修复 | `tags.rs` | 在 component_lookup 注册 StatusBar/status-bar 路由 |
| 2.13 各组件 demo | `demo/src/cases/` | 12 个新 demo |
| 2.14 组件集成回归测试 | `compiler/component.rs` | 每个新组件 gen_component 测试 + props_registry 对齐测试 |

**验收标准**：
- 12 个新组件在 .rml 中可用且通过 demo 验证
- StatusBar 路由一致（文档与实现匹配）
- props_registry 对齐测试通过
- 引擎测试全绿

---

### M3'：CSS 完善 + 属性扩展（5 周）

**目标**：实现 `<style>` 标签和 page_stylesheet，扩展 CSS 屇性至 70% 覆盖率，补齐颜色函数和 calc()。

**当前状态**：L1+L2+L3 已实现，缺 `<style>` 标签和 page_stylesheet。CSS 属性约 30 个，缺大量关键属性。

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 3.1 `<style>` 标签解析 | `parser/`、`parser/ast.rs` | 识别 `<style>` 元素，支持 source 属性 + 文本子节点 |
| 3.2 CodegenCtx 持有页面样式 | `compiler/mod.rs` | 新增 page_stylesheet 字段 |
| 3.3 多级样式表查询 | `css/matcher.rs` | collect_matching_declarations 先查 page 后查 global |
| 3.4 `<style source="..."/>` 文件加载 | `build/mod.rs` | 构建期读取外部 CSS 注入 page_stylesheet |
| 3.5 CSS P0 属性扩展 | `css/mapper.rs` | max-width/max-height、flex-grow/shrink/basis、align-self、align-content、overflow-x/y |
| 3.6 CSS P1 属性扩展 | `css/mapper.rs` | border 简写、border-width/color/style、outline、cursor、letter-spacing、font-style、font-family、position/top/right/bottom/left/z-index |
| 3.7 CSS 颜色函数完善 | `css/mapper.rs` | rgb()/rgba()/hsl()/hsla() 函数值映射（parser 已解析 Value::Function，mapper 需消费） |
| 3.8 属性选择器支持 | `css/parser.rs`、`matcher.rs` | [type="text"]、[disabled] 选择器 |
| 3.9 calc() 函数支持 | `css/mapper.rs` | 解析 calc(100% - 20px) 表达式，编译期求值或生成 GPUI 表达式 |
| 3.10 linear-gradient/radial-gradient | `css/mapper.rs` | 消费 Value::Function，映射到 GPUI 渐变 |
| 3.11 局部 CSS 变量作用域 | `css/parser.rs`、`matcher.rs` | 支持 .card { --color: red } 局部变量，子元素 var() 优先查局部 |
| 3.12 CSS 分层 demo | `demo/src/cases/css_layering_case.rml` | 演示三层 CSS 叠加 + calc + 属性选择器 |

**验收标准**：
- `<style source="/button.css"/>` 在 .rml 中可用且作用域正确
- CSS 属性覆盖率从 30 提升至 60+（70% 目标）
- 属性选择器 [disabled] 匹配测试通过
- calc(100% - 20px) 在 .rml 中生效
- rgb()/rgba()/hsl()/hsla() 颜色函数生效
- 引擎测试全绿

---

### M4'：CSS 伪类 + 动画（4 周）

**目标**：实现 :hover/:focus/:active 伪类支持，提供 transition/animation 基础能力。

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 4.1 伪类 CSS 解析 | `css/{parser.rs, ast.rs}` | 扩展 Selector 支持 :hover/:focus/:active |
| 4.2 伪类匹配器 | `css/matcher.rs` | 区分基础声明与伪类声明，收集到 pseudo_declarations |
| 4.3 伪类事件挂接 | `compiler/codegen/node.rs` | codegen 为匹配伪类规则的元素生成 .on_hover/.on_focus/.on_active 事件监听 |
| 4.4 伪类运行时状态 | `ui/src/state.rs` | RmlState 增加 hovered_elements/focused_elements 临时状态 |
| 4.5 transition 支持 | `css/mapper.rs` + `compiler/codegen/node.rs` | 解析 transition: property duration timing-function，映射到 GPUI Animation |
| 4.6 @keyframes 解析 | `css/parser.rs`、`ast.rs` | 新增 AtRule::Keyframes 节点 |
| 4.7 animation 支持 | `css/mapper.rs` + `compiler/codegen/node.rs` | 解析 animation: name duration timing-function iteration-count |
| 4.8 伪类与动画 demo | `demo/src/cases/css_pseudo_animation_case.rml` | 演示 :hover 颜色过渡 + @keyframes 入场动画 |

**验收标准**：
- :hover 在 demo 中生效
- :focus 在 input 上生效
- transition: background-color 0.3s 产生平滑过渡
- @keyframes fadeIn + animation: fadeIn 0.5s 生效
- 引擎测试全绿

---

### M5'：运行时关键能力（5 周）

**目标**：补齐 on_prop_change、多窗口管理、循环防护、切换防抖、on_rendered。

**注意**：checkbox/radio/select 状态管理已移到 M2' 配合组件集成。

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 5.1 on_prop_change 生命周期 | `core/src/lifecycle.rs` + `macros/src/component.rs` + `ui/src/state.rs` | ILifecycle 增加 on_prop_change 默认空实现；宏扫描 fn on_prop_change_* 方法，bump_version 后注入调用 |
| 5.2 on_rendered 生命周期 | `core/src/lifecycle.rs` + `macros/src/component.rs` | ILifecycle 增加 on_rendered 默认空；宏检测 fn on_rendered 并在 render 后注入 |
| 5.3 多窗口管理器 | `app/src/window_manager.rs` + `app/src/application.rs` | 新增 WindowManager 持有 HashMap<WindowId, WindowHandle>；RmlApplication 增加 .window::<W>() |
| 5.4 窗口关闭确认 | `core/src/window.rs` + `app/src/window_manager.rs` | IWindow trait 增加 on_closing() -> bool 默认 true |
| 5.5 绑定循环防护 | `ui/src/state.rs` | RmlState 增加 binding_stack；bump_version 时检测循环 |
| 5.6 主题/i18n 切换防抖 | `core/src/{theme.rs, i18n.rs}` | 切换方法增加 100ms 防抖 |
| 5.7 非颜色 CSS 变量支持 | `core/src/theme.rs` + `css/mapper.rs` | ThemeState 支持 --spacing: 8px 等非颜色变量；var() 按类型分发 |
| 5.8 运行时能力 demo | `demo/src/cases/` | on_prop_change_case、multi_window_case |

**验收标准**：
- fn on_prop_change_count 在 count 字段变化时被调用
- 多窗口应用可打开/关闭次要窗口
- 窗口关闭前可拦截（on_closing 返回 false）
- 连续切换主题 10 次只触发 1 次 refresh_windows
- 引擎测试全绿

---

### M6'：热重载（6 周）

**目标**：实现 .rml/.css 模板热重载，保留 ViewModel 状态，秒级反馈。

| 任务 | 涉及文件 | 说明 |
|------|---------|------|
| 6.1 file watcher 模块 | `crates/engine/src/watch/` | 基于 notify crate 监听 src/**/*.rml 与 **/*.css |
| 6.2 增量编译触发 | `crates/engine/src/watch/compiler.rs` | 文件变化时单文件重编译，输出到内存缓冲 |
| 6.3 IPC 通信层 | `crates/engine/src/watch/ipc.rs` | 使用 interprocess crate 跨平台 IPC |
| 6.4 运行时 Render 替换 | `crates/core/src/runtime.rs` | 应用侧监听 IPC，收到通知后替换当前 View 的 Render 实现 |
| 6.5 状态保留机制 | `ui/src/state.rs` | 热重载时保留 RmlState 完整字段 |
| 6.6 热重载错误处理 | `crates/engine/src/watch/` + `core/src/runtime.rs` | 失败时不崩溃，保持上一个有效状态，显示错误提示 |
| 6.7 Builder.hot_reload 启用 | `crates/engine/src/build/mod.rs` | 重新添加 hot_reload 字段，启动 watch 线程 |
| 6.8 Cargo feature | `crates/engine/Cargo.toml` + `crates/core/Cargo.toml` | 新增 hot-reload feature，仅 dev profile 启用 |
| 6.9 热重载 demo | `demo/src/cases/hot_reload_demo.rml` | 演示修改 .rml 后 UI 实时更新 |
| 6.10 热重载集成测试 | `crates/engine/tests/hot_reload.rs` | 端到端测试 |

**验收标准**：
- 修改 .rml 文件后 1 秒内看到 UI 更新
- ViewModel 状态在热重载后保留
- 热重载失败时不崩溃，显示错误提示
- 引擎测试全绿

---

## 调整原则

1. **优先级排序**：先修"静默失败"缺陷（M1'）→ 补半集成组件（M2'）→ 扩 CSS（M3'）→ 补运行时（M4'-M5'）→ 热重载（M6'）
2. **依赖驱动**：Icon 是 Tooltip/Popover/Notification 的基础，M2' 按依赖分批集成
3. **配合集成**：checkbox/radio/select 状态管理移到 M2' 配合组件集成，不分散到 M5'
4. **拒绝妥协**：不简化 demo、不绕过 codegen bug、不保留未消费的字段
5. **世界级标准**：每个 codegen 路径有测试，每个组件有 demo，每个指令有验证

## 风险与缓解

| 风险 | 影响 | 缓解 |
|------|------|------|
| ref 注入需宏侧改造 | 中 | M1' 先做 codegen 侧 + RmlState 字段，宏侧配合实施 |
| Input 事件架构变更 | 中 | M1' 设计 subscribe 模式，替代错误的 .on_change() 方法调用 |
| GPUI Animation API 限制 | 中 | M4' 动画实现前先验证 GPUI Animation 能力 |
| 伪类事件监听性能 | 中 | 仅对匹配伪类规则的元素挂接事件 |
| 热重载状态保留复杂度 | 高 | M6' 限定范围，先支持简单状态 |
| 多窗口管理器与 GPUI 窗口模型差异 | 中 | M5' 初期先调研 GPUI 多窗口能力 |
| on_prop_change 宏扫描误识别 | 低 | 严格匹配 fn on_prop_change_<field> 模式 |

## 总周期

| 里程碑 | 周期 | 重点 |
|--------|------|------|
| M0' | 1 周（已完成） | 技术债务清理 |
| M1' | 4 周 | 语法层缺陷修复 |
| M2' | 6 周 | 半集成组件补全 + StatusBar 路由 |
| M3' | 5 周 | CSS 完善 + 属性扩展 |
| M4' | 4 周 | CSS 伪类 + 动画 |
| M5' | 5 周 | 运行时关键能力 |
| M6' | 6 周 | 热重载 |

**总周期**：31 周（约 7.75 个月）
