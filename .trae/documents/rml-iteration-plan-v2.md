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
| 1.2 实现 `once` 指令 codegen | `compiler/codegen/node.rs` + `compiler/expr.rs` + `compiler/codegen/once.rs` + `ui/src/state.rs` | RmlState 增加 `once_cache: HashMap<&'static str, Box<dyn Any + Send + Sync>>` + `once_get_or_init<T>()` 方法；codegen 收集元素字段引用，首次渲染快照数据，后续渲染用快照重建元素（AnyElement 不可跨帧缓存，采用数据快照方案） |
| 1.3 实现 `html` 指令 codegen | `compiler/codegen/node.rs` | GPUI 无原生 HTML 渲染，降级为 `rml_ui::Label::new(raw)` 文本节点；支持与 if/show/each 组合（条件/迭代渲染 Label） |
| 1.4 实现 `key` 指令消费 | `compiler/codegen/node.rs` + `crates/core/src/element_id.rs` + `compiler/codegen/once.rs` | 新增 `rml_core::element_id::from_key<T: Hash>()` 运行时支持；codegen 在 id 生成处按 ref > key > 事件处理器优先级消费 key 表达式，生成 `.id(("rml_key", rml_core::element_id::from_key(&expr)))` NamedInteger id；key 表达式在 each 作用域内求值，引用循环变量（如 `item.id`）而非 `self.item.id`；once.rs `collect_element_fields` 添加 Key 指令字段收集（用 effective_refs 跳过循环变量） |
| 1.5 修正 `show` 语义 | `compiler/codegen/node.rs` | `show={cond}` 改为生成 `.when(!cond, \|d\| d.invisible())`（GPUI `Visibility::Hidden`，CSS `visibility:hidden`），始终渲染元素但按条件隐藏视觉，保留布局空间 —— 与 `if`（`Display::None` 不占空间）明确区分。`if` 优先于 `show`（同时存在时 `show` 被忽略）。重构 `each` 包装到 `if`/`show` 之后，使条件按 item 逐项应用（修复 `each + if`/`each + show` 条件被忽略的预存缺陷）。同时修复 `each` iterable 引用外层循环变量时误加 `self.` 前缀的缺陷 |
| 1.6 修复 `ref` ElementRef 注入 ✅ | `compiler/component.rs` + `macros/src/component.rs` + `ui/src/state.rs` + `compiler/codegen/render.rs` + `compiler/{tree,code_editor}/gen.rs` + `tags.rs` | **完成**。RmlState 增加 `ref_entities: HashMap<String, Box<dyn Any + Send + Sync>>` + `get_or_init_ref<T: 'static>()` 方法（Entity<T> 自身 Send + Sync 不依赖 T，故可类型擦除存储）；Stateful 组件 codegen 在 `ref="name"` 时生成 `get_or_init_ref("name", w, c, ctor_expr)` 调用，通过 `state_ctor` 闭包表达式适配不同构造函数签名（InputState::new(w,c) / SliderState::new() / TreeState::new(c)）；宏侧 `gen_populate_refs_impl` 扫描 `ElementRef<T>` 字段，生成 `__rml_populate_refs()` 方法从 `ref_entities` 取出 `Entity<T>` 并 `.set()` 到字段；render.rs 在渲染后调用 `self.__rml_populate_refs()`。新增 6 个宏侧测试 + 3 个 codegen 测试。input_case demo 已迁移至 `ElementRef<InputState>` + `ref="input_state"` 模式 |
| 1.7 Input 事件架构修复 ✅ | `compiler/input/event.rs` + `compiler/component.rs` + `compiler/code_editor/gen.rs` + `ui/src/state.rs` | **完成**。Input element 无 `.on_change()`/`.on_enter()` 等方法（gpui-component 设计），事件通过 `InputState: EventEmitter<InputEvent>` 发送，用户通过 `cx.subscribe` 订阅。codegen 设计：①`component_event_setter` 对 Input 事件（on_change/on_enter/on_focus/on_blur）返回 None，跳过 setter 链；②`gen_component` Stateful 分支收集 Input 事件处理器，生成 block 表达式 `({ let __rml_entity = <entity>; <subscribe...>; Input::new(&__rml_entity) })`，subscription 句柄用 `detach()` 让其随 entity 生命周期自动销毁；③`RmlState` 增加 `subscribed_events: Mutex<HashSet<String>>` 字段 + `is_event_subscribed`/`mark_event_subscribed` 方法，防止 render 时重复 subscribe（每次 render 都会重新评估事件订阅代码）；④ref_key 作为 subscribe 标识键：`ref="name"` 优先，回退到 state_field 名。CodeEditor 路径同步重构。新增 8 个测试（6 个 event.rs + 2 个 gen_component） |
| 1.8 解析器错误诊断增强 ✅ | `parser/{mod.rs, tokenizer.rs}` + `css/parser.rs` | **完成**。RML ParseError 增加 `source_snippet: Option<String>` 字段 + `with_source(source)` 方法，由 `parse()` 在返回错误前根据 `line` 从源码提取对应行内容；Display 渲染源码上下文块 `  \| <源码行>\n  \|   ^`。修复 `parse_each_expr` 的 `line:0/column:0` 占位缺陷：`RawAttribute` 增加 `line/column` 字段，`each` 属性位置透传到 `parse_each_expr` 签名。CSS ParseError 增加 `line/column` 字段，由 `Parser::pos_to_line_col(pos)` 根据 `pos` 计算（O(n) 遍历，错误罕见可接受，避免每次 advance 维护行列）。Display 格式 `CSS parse error at line:col (pos): msg`。新增 6 个测试（3 RML + 3 CSS） |
| 1.9 Attribute Span 保留 ✅ | `parser/ast.rs` + `parser/tokenizer.rs` + `parser/mod.rs` + 全编译器模块 | **完成**。`Attribute::Static/Bind/Event` 三变体增加 `span: Span` 字段（半开字节区间 `[start, end)`），由 tokenizer 在构造 `RawAttribute` 时填充，`build_element` 透传到 AST。26 个编译器文件解构模式批量更新（match arm 添加 `..` 忽略 span，构造点添加 `span: Span::empty()` 或 `span: attr.span`）。13 个测试模块添加 `use crate::parser::Span;`。LSP 跳转预留接口就位 |
| 1.10 指令 demo ✅ | `demo/src/cases/` | **完成**。新增 6 个指令专项 demo：else_case（if/else 双分支）、once_case（首次渲染快照 vs 实时对比）、html_case（降级为 Label 文本）、key_case（each + key 稳定 ElementId）、show_case（Visibility::Hidden 保留布局 vs if Display::None）、ref_case（ElementRef 字段绑定 + with_mut 命令式访问）。i18n 添加 6 个 case.{name}.title。修复 once codegen 单元素快照的多余括号 warning（`(expr,)` → `expr,`，由外层 `({snap})` 形成 1-tuple）。修复 html codegen 对 SharedString 字段的 move 问题（demo 中用 `html={field.clone()}` 表达式） |

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
| 2.1 Icon 集成 ✅ | `tags.rs`、`props_registry.rs`、`component.rs`、`compiler/icon.rs` | **完成**。Icon 注册为 StatelessNoId（RenderOnce 无 ElementId），由专属 `compiler/icon.rs` 模块处理 name/path 属性提取：`name="Settings"` → `Icon::new(IconName::Settings)`，`path="..."` → `Icon::empty().path(...)`，name 优先于 path。size 走通用 Sizable setter 链。新增 6 个 icon codegen 测试 + icon_case demo（静态 IconName + computed 动态切换 + 自定义 path） |
| 2.2 Kbd 集成 ✅ | `tags.rs`、`props_registry.rs`、`component.rs`、`compiler/kbd.rs` | **完成**。Kbd 注册为 StatelessNoId，由专属 `compiler/kbd.rs` 处理 key 属性：`key="cmd-a"` → `Kbd::new(Keystroke::parse("cmd-a").expect("valid keystroke"))`。专用 setter：outline/appearance。`key` 必填（缺失返回 CodegenError）。新增 6 个 kbd codegen 测试 + kbd_case demo（基础用法 + 样式变体） |
| 2.3 Tooltip 集成 ✅ | `compiler/tooltip.rs` | **完成**。Tooltip 作为通用属性（非独立组件），由 `compiler/tooltip.rs` 提供 `static_setter`/`bind_setter`/`supports_tooltip`。组件白名单：Button/IconButton/DropdownButton/Toggle/Checkbox/Clipboard/Radio/Switch。`tooltip="text"` → `.tooltip("text")`，`tooltip={expr}` → `.tooltip(self.expr)`（bind 需先经 `component_bind_rust_expr` 转换）。移除 component.rs match 中的旧 tooltip setter，统一由 tooltip 模块处理。新增 6 个 tooltip 测试 + tooltip_case demo |
| 2.4 Popover 集成 ✅ | `tags.rs`、`props_registry.rs`、`component.rs`、`compiler/popover.rs` | **完成**。Popover 注册为 StatelessWithItems，由专属 `compiler/popover.rs` 处理：构造 `Popover::new(id)` + 专用 setter + 子节点路由。子节点通过 `slot="trigger"` 路由到 `.trigger()`（trigger 元素需实现 Selectable + IntoElement），其余子节点作为 content 注入 `.child()` / `.children()`。专用静态 setter：anchor（8 个枚举值，转 `gpui::Anchor::X`）/mouse_button（left/right/middle，转 `gpui::MouseButton::X`）/appearance/overlay_closable/default_open。bind setter：default_open。受控模式（`open` + `on_open_change`）因回调签名特殊（`Fn(&bool, &mut Window, &mut App)` 非标准 component event）暂不暴露，待需求出现时再添加。新增 15 个 popover 测试 + popover_case demo（基础用法 + 锚点定位 + 默认展开 + API 表） |
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
