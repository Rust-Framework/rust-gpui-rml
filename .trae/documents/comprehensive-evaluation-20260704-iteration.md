# 2026-07-04 迭代综合评估报告

> 评估范围:今日对 RML 框架的全部迭代(DescriptionList 规范化、Shell/MVVM 重构、Contribution 体系、Workbench 抽象)
> 评估维度:异味(code smells)、不和谐内容(discord)、框架高度(elevation)

---

## 一、评估摘要(直接回答三个问题)

| 问题 | 结论 | 简评 |
|---|---|---|
| 是否存在异味? | **是,3 类共 9 处** | 含 3 处严重设计妥协残留、5 处中等职责问题、1 处响应式缺口 |
| 是否存在不和谐内容? | **是,5 处** | 主要是文档与实现脱节、设计文档与代码偏差 |
| RML 是否推向新高度? | **是,显著提升** | MVVM 数据驱动到位、Shell 命令式代码彻底消除、组件规范统一度 8.5/10,但需清理 3 处设计妥协才能稳固 |

**总评**:今日迭代**方向正确、成果显著**,但存在"设计文档已收敛、代码未完全跟进"的撕裂——这是当前最大的不和谐。建议在进入下一轮功能迭代前,先用一轮"设计对齐收尾"清理 P0 项。

---

## 二、推向新高度的亮点(6 项)

### 1. Shell 命令式代码彻底消除
- `shell_chrome.rs`(原 173 行)+ `menu_shell_contribs.rs` **整体删除**(非"精简到 75 行",而是更彻底)
- 职责被三模式吸收:`ViewModel` + `RelayCommand` + `IWorkbenchManager`
- 符合用户记忆中"简化菜单命令实现逻辑,消除 shell_chrome 冗余实现"目标

### 2. 真正的 MVVM 数据驱动
- `main_window.rml.rs` L34-62:持有 `cases/menus/status/activities` 四个类型化 ViewModel 集合 + 7 个 `RelayCommand` 字段(WPF 模式)
- `#[computed] tab_bar_items`(L425-428)+ `#[command]` 装饰器(L430-467)声明式到位
- `project_entries`(L246-262)一次性投影到 4 个 ViewModel 集合,RML 模板直接消费
- 符合"业务-UI 交互全 MVVM 数据驱动显著减少代码"硬约束

### 3. Contribution 核心 trait 契约纯净
- `IContribution`/`IVisualContribution`/`IContributionHost`/`IContributionRegistry` 四 trait 定义干净([crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs))
- `IVisualContribution::render` 直接接收 `&mut Window, &mut App`(L83),符合硬约束
- `IContributionHost` 仅 `id/add/remove` 三方法(L169),纯业务自受理
- `as_visual()` **未加到 IContribution trait**,通过 `VisualAbilityExt`/`ContributionAbilityExt` 独立 extension trait 提供,符合"禁止修改 trait 方法签名"约束

### 4. 框架与业务解耦良好
- `MenuBar` 刻意不定义 `IMenuItem` 数据结构(WPF 风格,业务自定义 ViewModel)—— [menu.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/components/menu.rs) 顶部注释明示
- `MenuViewModel` 仅在 demo 层([demo/src/shell/menu_view_model.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/menu_view_model.rs)),框架零耦合
- `Tabs` 数据为 `Vec<Arc<dyn IValue>>`([tab_window.rs L147](file:///e:/GitCode/RF/rust-gpui-rml/crates/ui/src/window/tab_window.rs#L147)),容纳多样业务数据

### 5. main_window 由框架管理
- [app.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/app.rs) 仅 17 行配置 style/i18n/theme
- [main.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/main.rs) L19-21:`RmlApplication::new().main_window::<shell::MainWindow>().run::<app::Startup>()`
- 硬约束"主窗口生命周期不应在 app.rs"已落实

### 6. 组件规范统一度高(8.5/10)
- 24 个组件文件,kebab-case 声明式 + snake_case 内部双层模型执行严格
- `DescriptionList` 完全合规:`<descriptions items={desitems} bordered="" columns="2" label-width="100" />`
- `tags.rs` + `props_registry.rs` + `setters.rs` 三处同步协议清晰
- SKILL.md 覆盖 7 维度,与实现高度一致

---

## 三、异味清单(按严重度分级)

### P0 严重——设计妥协残留(3 处)

> 这三处是设计文档已要求移除/对齐,但代码未跟进的"撕裂点",是当前最大的不和谐。

#### P0-1 `EntityHostHandle` 残留,违反"Host 直接 impl trait"目标
- **位置**:[crates/app/src/contribution/host_handle.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/host_handle.rs) L36 `EntityHostHandle<T>` + `HostOp` channel + `install_entity_host`/`drain_host_ops`
- **问题**:用户记忆明确"HostHandle 是不必要的;contributions 应直接交付给 IContributionHost 实现"。当前仍走 channel 桥接,`#[contributehost]` 宏(`contributehost.rs` L113-118)强制生成 `__rml_install_host` 调用
- **影响**:设计目标与实现背道而驰,host 仍需框架侧 handle 中转

#### P0-2 `VisualEntityCache` 换名存留,违反"EntityCache 移除"意图
- **位置**:[crates/app/src/contribution/entity_cache.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/contribution/entity_cache.rs) L20 `VisualEntityCache`
- **问题**:用户记忆明确"ComponentEntityCache 是不必要的;框架不存储此内容"。当前宏生成的 `IVisualContribution::render` 仍经 `get_or_create_entity::<T>(cx)` 复用 Entity,即换名存留
- **影响**:框架仍在存储视觉实体,与"不存储缓存"设计冲突

#### P0-3 `get_contribution_registry()` 未按设计实现
- **位置**:[crates/app/src/extensions.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/app/src/extensions.rs) L25-45 `IAppContextExt`
- **问题**:设计要求 `get_contribution_registry() -> &dyn IContributionRegistry`,实际为 `contribution_registry(&self) -> Arc<ContributionRegistry>`(返回具体 `Arc`,非 `&dyn`)。方法名与返回类型均偏离
- **影响**:暴露内部具体类型 `ContributionRegistry`,违反接口隔离

### P1 中等——职责/可维护性(5 处)

#### P1-1 `Default` 手写 30+ 个 `__rml_*` 字段,宏仪式泄漏
- **位置**:[demo/src/shell/main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) L67-123
- **问题**:`__rml_*_version` / `__rml_computed_cache` / `__rml_input_states` 等框架内部字段裸露在业务 struct
- **影响**:业务侧心智负担重,应通过 `#[window]` 宏自动生成 Default 或隐藏字段

#### P1-2 `on_loaded` 100 行做 9 件事,职责过载
- **位置**:[main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) L143-240
- **问题**:host install / cmd init / ability reg / project / service / LSP / manager / welcome / ActivityBar 9 件事平铺
- **影响**:可读性差,建议拆 `init_commands`/`init_manager`/`init_activity_bar` 子方法

#### P1-3 `DemoWorkbench` 枚举为补 trait 缺口而生
- **位置**:[demo/src/shell/workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) L152-186
- **问题**:`IWorkbench` trait 缺 `render`/`id`/`uri`,业务被迫用枚举 `Case/Lsp` 桥接 render 分发
- **影响**:框架 trait 设计缺口倒逼应用层 workaround,应在框架层补 `IWorkbench::id()` 或引入 `IVisualWorkbench`

#### P1-4 `CaseWorkbenchProvider` 数据双写一致性风险
- **位置**:[workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) L194-215
- **问题**:provider 持有 `RwLock<HashMap<String, CaseViewModel>>` 副本,`MainWindow.cases` 与 provider 内 cases 数据双写(L193 `manager.sync_cases(self.cases.clone())`)
- **影响**:一致性风险,应让 provider 直接引用 MainWindow 的 cases 或经 service 查询

#### P1-5 `panic!` 错误处理粗暴
- **位置**:[workbench.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/workbench.rs) L226 `panic!("case not found: {case_id}")`、L383 `panic!("unknown workbench schema")`
- **问题**:业务错误用 panic,应返回 `Option`/`Result`
- **影响**:运行时崩溃风险

### P2 轻微——响应式缺口(1 处)

#### P2-1 `apply_switch_en` 手工重建 status,非响应式
- **位置**:[main_window.rml.rs](file:///e:/GitCode/RF/rust-gpui-rml/demo/src/shell/main_window.rml.rs) L483-486
- **问题**:切 locale 后需手动 re-project status 集合
- **影响**:反映 RML 缺少响应式集合(与 project_memory.md L45"运行时订阅模型结构性缺口"一致)

---

## 四、不和谐内容清单(5 处)

### D-1 文档 `onclick` vs 实现 `on-click`(6 份文档落后)
- **位置**:`docs/06-components/reference/` 下 `button.md:34` `input.md:34` `menu-items.md:31` `menu-bar.md:12-14` 等
- **问题**:文档用 `onclick`(单单词),实际 `.rml` 文件统一用 `on-click`(已检查 30 处)
- **影响**:SKILL.md 反模式明确"onclick 应为 on-click",文档自相矛盾

### D-2 `menu-bar.md` 误导性 `items` 绑定示例
- **位置**:`docs/06-components/reference/menu-bar.md:22`
- **问题**:文档写 `<menu items={menu_items} />`,但 [props_registry.rs:77](file:///e:/GitCode/RF/rust-gpui-rml/crates/engine/src/compiler/props_registry.rs) 注释明确"MenuBar / StatusBar 不支持 items 绑定"
- **影响**:误导用户,框架刻意不定义 IMenuItem(WPF 风格)

### D-3 `<slot_menu>` typo
- **位置**:`docs/06-components/reference/menu-bar.md:5`
- **问题**:文本中出现 `<slot_menu>`(snake_case),违反 kebab-case 命名规范
- **影响**:文档 typo

### D-4 架构文档仍引用已废弃类型
- **位置**:`docs/09-architecture/contribution-system.md`
- **问题**:仍引用 `contribution_entries`/`HostHandle`/`ContributedEntry`,实际代码已移除(或换名)
- **影响**:文档与代码脱节,新人困惑

### D-5 Registry 方法名偏差
- **位置**:[crates/core/src/contribution.rs](file:///e:/GitCode/RF/rust-gpui-rml/crates/core/src/contribution.rs) L188 `IContributionRegistry`
- **问题**:设计要求 `add(host)/remove(host_id)`,实际为 `add_host(host)/remove_host(host_id)`
- **影响**:命名偏差,语义无歧义但与设计文档不一致

---

## 五、改进建议与优先级

### P0 必须处理(设计对齐收尾,建议本轮迭代内完成)
1. **EntityHostHandle 移除评估**:确认 host 直接 impl `IContributionHost` 的可行性,移除 `host_handle.rs` + `__rml_install_host` 宏生成,或明确评估"channel 桥接是必要妥协"并更新设计文档
2. **VisualEntityCache 评估**:确认视觉 Entity 复用是否真有必要;若必要则更新设计文档承认其存在,若不必要则移除并让 `render` 每次重建
3. **`get_contribution_registry()` 签名对齐**:改为返回 `&dyn IContributionRegistry` 或 `Arc<dyn IContributionRegistry>`,隐藏具体类型

### P1 重要(下一轮功能迭代前处理)
4. **`Default` 宏仪式隐藏**:`#[window]` 宏自动生成 `__rml_*` 字段的 Default,业务 struct 仅声明业务字段
5. **`on_loaded` 拆分**:拆为 `init_commands`/`init_manager`/`init_activity_bar` 子方法
6. **`IWorkbench::id()` 补全**:框架 trait 补 `id()`/`uri()`,消除 `DemoWorkbench` 枚举 workaround
7. **`CaseWorkbenchProvider` 数据双写消除**:provider 经 service 引用 MainWindow.cases,不再 clone 副本
8. **`panic!` 转 `Result`**:L226/L383 改为 `Option::ok_or`/`Result`

### P2 改善(随功能迭代逐步推进)
9. **响应式集合规划**:RML 框架层规划响应式集合订阅模型,消除 `apply_switch_en` 手工 re-project
10. **文档同步**:`docs/06-components/reference/` 下 onclick→on-click、移除 menu-bar items 绑定示例、修复 `<slot_menu>` typo
11. **架构文档更新**:`docs/09-architecture/contribution-system.md` 同步移除 `contribution_entries`/`HostHandle`/`ContributedEntry` 引用
12. **Registry 方法名对齐**:`add_host/remove_host` → `add/remove`(或更新设计文档承认 `add_host` 命名)

---

## 六、验证步骤

完成上述改进后,通过以下方式验证:

1. **编译验证**:`cargo build --workspace` 全通过
2. **运行验证**:`cargo run -p demo` 启动后:
   - 菜单/状态栏/案例树/Tab 全部正常渲染
   - 切换 locale(en↔zh)后 status 自动响应(验证响应式集合)
   - 打开/关闭 Tab 无 panic(验证错误处理)
3. **规范验证**:
   - `grep -r "onclick" docs/` 无结果
   - `grep -r "contribution_entries\|HostHandle\|ContributedEntry" docs/` 无结果
   - `grep -r "EntityHostHandle\|VisualEntityCache" crates/` 按预期清空或保留(取决于 P0 评估结论)
4. **设计对齐验证**:逐项核对 `project_memory.md` 硬约束,确认无违规

---

## 七、结论

今日迭代**成功将 RML 框架推向新高度**:Shell 命令式代码彻底消除、MVVM 数据驱动到位、组件规范统一度 8.5/10、Contribution 核心 trait 契约纯净。

但存在**3 处 P0 设计妥协残留**(EntityHostHandle/VisualEntityCache/get_contribution_registry 签名),导致"设计文档已收敛、代码未跟进"的撕裂——这是当前最大的不和谐。建议在进入下一轮功能迭代前,先用一轮"设计对齐收尾"清理 P0 项,让框架稳固地停在新高度上,再继续向上攀登。
