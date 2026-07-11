# RML 高级复杂度界面开发能力差距审查与迭代计划

> 审查时间：2026-07-11
> 审查范围：RML 框架达成「高级复杂度界面开发能力」的全部差距
> 方法论：基于已注册组件（37+）、三大注册表（StateBridge / StateEvent / TwoWayBinding）、指令系统、校验框架、窗口扩展 API 的全量审计

---

## 一、审查结论

**RML 已具备中高复杂度界面开发能力**，可投入实际业务开发。要达到「高级复杂度」（对标 VSCode / IntelliJ 级别），存在 4 个关键差距 + 7 个中等差距 + 4 个次要差距。

### 已具备能力（非差距，经审查确认）

| 能力 | 实现位置 | 说明 |
|------|----------|------|
| 条件渲染 | `parser/ast.rs` Directives | `if`/`else`/`else-if` 全链路 |
| 列表渲染 | `parser/ast.rs` Directives | `each`/`key` 支持 |
| 显示控制 | `parser/ast.rs` Directives | `show` 指令 |
| 单次渲染 | `parser/ast.rs` Directives | `once` 指令 |
| 原生 HTML | `parser/ast.rs` Directives | `html` 指令 |
| 引用注入 | `parser/ast.rs` Directives | `ref` 指令 |
| 双向绑定 | `compiler/twoway.rs` | Checkbox/Switch/Radio/Rating/RadioGroup/Stepper（6 组件自动双向） |
| 状态事件 | `compiler/components/state_event.rs` | ColorPicker/Calendar/DatePicker/Select/Combobox/Slider（6 组件状态回写） |
| 状态桥接 | `compiler/state_bridge.rs` | Slider（Stateful 双向绑定） |
| 校验框架 | `core/validate.rs` | `#[validate]` 规则式 + IValidate 接口式（含跨字段） |
| 上下文菜单 | `translator/menu/context_menu.rs` | `<context-menu>` 完整实现 |
| 文本域基础 | `translator/builtin/textarea.rs` | `<textarea>` 已注册（但未启用多行） |
| 命令式通知 | `ui/window/actions.rs` | `window.push_notification()` |
| 命令式对话框 | `ui/window/` | `window.open_dialog()` / `close_dialog()` / `open_sheet()` / `close_sheet()` |
| 表单容器 | `ui/components/form.rs` | Form/Field + label/description/required |
| 树数据分离 | `ui/components/tree.rs` | TreeData (Send+Sync) → TreeItem 自动转换 |

---

## 二、差距清单

### 🔴 关键差距（阻塞高级界面开发）

#### G1. Dialog 缺少 `open={bool}` 受控开关
- **现状**：`<Dialog>` 基于 trigger 自动管理打开/关闭；`<dialog>` 根节点生成 `open()`/`close()` 命令式方法。两者均不支持 `<Dialog open={is_visible}>` 受控模式，无法由 ViewModel 状态驱动对话框显示。
- **影响**：表单弹窗（新增/编辑）、确认对话框、多步对话框等典型业务场景无法声明式实现。
- **涉及文件**：
  - `crates/engine/src/compiler/components/dialog/gen.rs`
  - `crates/engine/src/compiler/components/dialog/setters.rs`
  - `crates/engine/src/compiler/components/dialog/props_registry.rs`

#### G2. 通知无法从命令/事件触发
- **现状**：`window.push_notification()` Rust API 已存在，但 RML 层无声明式入口。命令回调（`#[command]`）和事件回调内无法直接调用通知。
- **影响**：异步任务完成、错误反馈、长时间操作提示等场景必须回到 Rust 手写。
- **涉及文件**：
  - `crates/ui/src/window/actions.rs`（已存在 push_notification）
  - 需新增 RML 侧声明式 API 或命令上下文桥接

#### G3. Textarea 未启用多行模式
- **现状**：`<textarea>` 已注册为 builtin，复用 InputState，但 `InputState` 未开启 `multi_line`。
- **影响**：长文本输入、代码片段输入、评论/描述等场景只能用单行 `<input>`。
- **涉及文件**：
  - `crates/engine/src/compiler/translator/builtin/textarea.rs`
  - 可能需要调整 InputState 构造参数

#### G4. Table 缺少行内编辑
- **现状**：Table 支持数据展示、列定义、分页等，但不支持单元格点击进入编辑模式。
- **影响**：数据管理后台的核心场景（可直接编辑表格内容）缺失。
- **涉及文件**：
  - `crates/engine/src/compiler/translator/component/table_delegate.rs`
  - `crates/engine/src/compiler/translator/component/table.rs`

### 🟡 中等差距（影响高级体验）

#### G5. 缺少声明式主题切换
- **现状**：Theme Rust API 已存在，但 RML 层无 `<theme name="dark">` 或 `window.set_theme()` 声明式入口。
- **影响**：用户偏好主题切换、跟随系统主题等场景需手写 Rust。

#### G6. 缺少键盘快捷键绑定
- **现状**：无 `<KeyBinding key="Ctrl+S" command="save" />` 或 `on-key` 指令。
- **影响**：高级用户效率场景（快捷键操作）缺失，对标 VSCode 的 keybinding 系统无法实现。

#### G7. 缺少 Grid 布局组件
- **现状**：有 Flex（div + fl()/gp()）和 Stack，但无声明式 `<Grid>` / `<GridItem>`。
- **影响**：仪表盘、复杂表单等网格布局场景需要手写 CSS。

#### G8. 缺少动态组件切换
- **现状**：`<SwitchCase>` / `<component is={name}>` 缺失，只能用 `if`/`else-if` 链式判断。
- **影响**：多视图路由、Tab 内容动态加载等场景代码冗长。

#### G9. 缺少 TimePicker / DateRangePicker
- **现状**：有 Calendar、DatePicker，但 TimePicker 和 DateRangePicker 未实现。
- **影响**：预约系统、时间筛选、报表查询等场景不完整。

#### G10. 缺少 FileUpload 组件
- **现状**：无 `<FileUpload>` 或 `<input type="file">` 声明式支持。
- **影响**：文件上传场景（头像、附件、导入）无法声明式实现。

#### G11. 缺少拖拽（Drag-and-Drop）
- **现状**：Tree 不支持拖拽排序、跨树移动；列表不支持拖拽。
- **影响**：看板、可排序列表、树形结构重组等场景缺失。

### 🟢 次要差距（锦上添花）

#### G12. 缺少动画指令
- **现状**：无 `<transition>` / `animate` 指令。
- **影响**：过渡效果需手写 GPUI 动画 API。

#### G13. 缺少虚拟树
- **现状**：Tree 使用完整渲染，大数据量（>1000 节点）会卡顿。
- **影响**：超大型文件树、目录浏览等场景性能受限。

#### G14. 缺少 RichText 组件
- **现状**：无富文本编辑组件。
- **影响**：内容编辑场景需自行集成第三方。

#### G15. CodeEditor 缺少语言支持
- **现状**：CodeEditor 基础能力具备，但语言高亮、LSP 集成不完整。
- **影响**：代码编辑场景体验不完整。

---

## 三、迭代计划

### Phase 1：关键差距（优先级最高，解锁业务开发）

> 目标：消除阻塞高级界面开发的 4 个关键差距
> 预计：4 个工作项

#### 任务 1.1：Dialog 受控开关（G1）

**改动范围**：
- `crates/engine/src/compiler/components/dialog/gen.rs`：在 codegen 中识别 `open` bind 属性，生成 `.open(self.is_visible.clone())` 调用
- `crates/engine/src/compiler/components/dialog/setters.rs`：新增 `open` setter（读取 bool bind 表达式）
- `crates/engine/src/compiler/components/dialog/props_registry.rs`：登记 `open` 属性为 bind-bool

**验证标准**：
- `<Dialog open={show_dialog}>` 生成 `.open(self.show_dialog.clone())` 代码
- ViewModel 修改 `show_dialog = true` 后对话框自动打开
- 用户点击关闭按钮后 `show_dialog` 自动回写为 `false`（双向）
- 新增 demo：`dialog_controlled_case.rml`

#### 任务 1.2：通知命令桥接（G2）

**改动范围**：
- `crates/ui/src/window/actions.rs`：确认 `push_notification` 签名对 RML 友好
- 新增 RML 侧桥接：在命令上下文（`CommandContext`）或事件上下文中暴露 `cx.push_notification(...)`
- 可能新增 `notification` 命令式 API（类似 `window.open_dialog()`）

**验证标准**：
- 命令回调内可调用 `cx.push_notification(level, title, message)`
- 事件回调（如 on_click）内可调用同上
- 新增 demo：在按钮点击后弹出通知

#### 任务 1.3：Textarea 多行启用（G3）

**改动范围**：
- `crates/engine/src/compiler/translator/builtin/textarea.rs`：构造 InputState 时启用 `multi_line(true)` 或调用 `InputState::new_multi_line()`
- 验证 gpui-component 的 InputState 多行 API

**验证标准**：
- `<textarea>` 渲染为多行输入框
- 支持 `rows="4"` 等属性
- 新增 demo：`textarea_case.rml`

#### 任务 1.4：Table 行内编辑（G4）

**改动范围**：
- `crates/engine/src/compiler/translator/component/table_delegate.rs`：在 delegate 中增加 `editable` 标记 + 编辑回调
- `crates/engine/src/compiler/translator/component/table.rs`：生成 `.on_cell_edit(...)` 调用
- 可能需要扩展 TableDelegate trait

**验证标准**：
- `<Column editable on-edit={handle_edit} />` 支持单元格点击进入编辑
- 编辑完成后回调 `handle_edit(row, col, new_value, cx)`
- 新增 demo：可编辑表格

---

### Phase 2：中等差距（提升高级体验）

> 目标：补齐 7 个中等差距，达到对标 VSCode 的体验
> 预计：7 个工作项

#### 任务 2.1：声明式主题切换（G5）

**改动范围**：
- 在 `crates/engine/src/compiler/translator/` 下新增 `theme.rs` 或在 window codegen 中识别 `theme` 属性
- 生成 `window.set_theme(name)` 调用

**验证标准**：
- `<window theme="dark">` 或 `<ThemeSwitcher value={current_theme} />` 可切换主题
- 新增 demo：主题切换案例

#### 任务 2.2：键盘快捷键（G6）

**改动范围**：
- 新增 `crates/engine/src/compiler/translator/key_binding.rs`
- 注册 `<KeyBinding key="Ctrl+S" on-press={handle_save} />` 组件
- 在窗口层注册全局键盘监听

**验证标准**：
- `<KeyBinding key="Ctrl+S" on-press={save} />` 触发 save 命令
- 支持 `when` 条件（如 `when={editor_focused}`）
- 新增 demo：快捷键案例

#### 任务 2.3：Grid 布局（G7）

**改动范围**：
- 新增 `crates/ui/src/components/grid.rs`：Grid / GridItem 包装
- 在 `tags.rs` 注册 `<Grid>` / `<GridItem>`
- 新增 translator

**验证标准**：
- `<Grid columns="3" gap="8px">` + `<GridItem span="2">` 正确布局
- 新增 demo：仪表盘网格布局

#### 任务 2.4：动态组件（G8）

**改动范围**：
- 新增 `crates/engine/src/compiler/translator/switch_case.rs`
- 支持 `<Switch on={view_type}>` + `<Case value="settings"><SettingsView /></Case>`

**验证标准**：
- 根据 ViewModel 字段动态切换子组件
- 新增 demo：多视图路由

#### 任务 2.5：TimePicker / DateRangePicker（G9）

**改动范围**：
- 新增 `crates/ui/src/components/time_picker.rs`：包装 gpui-component
- 新增 `crates/ui/src/components/date_range_picker.rs`
- 在 `tags.rs` 注册，新增 translator

**验证标准**：
- `<TimePicker value={time} />` 可选择时间
- `<DateRangePicker start={start} end={end} />` 可选择日期范围
- 新增 demo：时间筛选案例

#### 任务 2.6：FileUpload（G10）

**改动范围**：
- 新增 `crates/ui/src/components/file_upload.rs`：文件选择 + 拖拽上传
- 在 `tags.rs` 注册 `<FileUpload>`

**验证标准**：
- `<FileUpload on-change={handle_files} multiple />` 支持文件选择
- 新增 demo：文件上传案例

#### 任务 2.7：拖拽支持（G11）

**改动范围**：
- `crates/engine/src/compiler/translator/component/tree.rs`：增加 `draggable` 属性 + `on-drop` 回调
- `crates/ui/src/components/tree.rs`：TreeState 增加拖拽处理
- 可能扩展到列表组件

**验证标准**：
- `<Tree draggable on-drop={handle_drop} />` 支持节点拖拽
- 新增 demo：可拖拽树

---

### Phase 3：次要差距（锦上添花）

> 目标：补齐 4 个次要差距，达到极致体验
> 预计：4 个工作项

#### 任务 3.1：动画指令（G12）

**改动范围**：
- 新增 `crates/engine/src/parser/ast.rs` 中的 `Transition` 指令
- 新增 translator 生成 GPUI 动画 API 调用

**验证标准**：
- `<transition name="fade">` 包裹元素实现淡入淡出
- 新增 demo：动画案例

#### 任务 3.2：虚拟树（G13）

**改动范围**：
- `crates/ui/src/components/tree.rs`：基于 virtual_list 实现虚拟化
- TreeState 增加虚拟化开关

**验证标准**：
- 10000 节点树流畅渲染
- 新增 demo：大型树性能测试

#### 任务 3.3：RichText（G14）

**改动范围**：
- 新增 `crates/ui/src/components/rich_text.rs`：集成第三方富文本（或自研轻量版）
- 在 `tags.rs` 注册

**验证标准**：
- `<RichText value={content} />` 支持富文本编辑
- 新增 demo：富文本编辑案例

#### 任务 3.4：CodeEditor 语言支持（G15）

**改动范围**：
- 扩展 CodeEditor 支持 language 配置
- 集成语法高亮（可能基于 syntect 或 tree-sitter）

**验证标准**：
- `<CodeEditor language="rust" />` 支持 Rust 语法高亮
- 新增 demo：多语言代码编辑案例

---

## 四、执行原则

1. **严格遵循设计规范**：所有改动遵循 CLAUDE.md 三原则（简洁、外科手术式改动、目标驱动）
2. **无兼容性设计**：RML 是全新框架，不保留旧 API 兼容
3. **属性命名规范**：RML 属性一律 kebab-case（parser 自动转 snake_case），禁止下划线
4. **单文件单职责**：每个新组件一个 rs 文件
5. **每任务配套 demo**：每个任务必须新增可运行 demo 验证
6. **测试优先**：改动后运行 `cargo test -p rml-engine` 确保 1256+ 测试全通过
7. **按优先级推进**：Phase 1 → Phase 2 → Phase 3，不跨阶段并行

---

## 五、验收标准

### Phase 1 完成（关键差距消除）
- [ ] `<Dialog open={field}>` 受控开关可用
- [ ] 命令/事件回调内可触发通知
- [ ] `<textarea>` 多行输入可用
- [ ] Table 单元格可编辑
- [ ] 所有 demo 编译运行通过
- [ ] 引擎测试全通过

### Phase 2 完成（高级体验达标）
- [ ] 声明式主题切换可用
- [ ] 键盘快捷键可用
- [ ] Grid 布局可用
- [ ] 动态组件切换可用
- [ ] TimePicker / DateRangePicker 可用
- [ ] FileUpload 可用
- [ ] 树拖拽可用
- [ ] 所有 demo 编译运行通过

### Phase 3 完成（极致体验）
- [ ] 动画指令可用
- [ ] 虚拟树支持 10000+ 节点
- [ ] RichText 可用
- [ ] CodeEditor 支持多语言高亮
- [ ] 所有 demo 编译运行通过

---

## 六、关键约束

- **不新增宏**：Phase C 已被否决，不新增 macro 特性
- **不引入 contribution_entries**：业务代码不出现 contribution_entries
- **IContributionHost 设计**：仅 id/add/remove，业务自行实现 acceptance
- **IContribution / IVisualContribution trait 签名不可修改**
- **IValue: Send + Sync + Any**：所有 contribute 结构体必须满足
- **ComponentEntityCache / ContributedEntry / HostHandle 已废弃**：不重新引入
