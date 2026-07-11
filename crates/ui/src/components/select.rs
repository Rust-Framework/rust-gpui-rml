//! Select 组件封装 —— 基于 gpui-component 的 Select
//!
//! 下拉选择器，Stateful 构造器接受 &Entity<SelectState<D>>。
//! SelectState 是泛型（D: SearchableListDelegate），RML 框架为最常见的字符串场景
//! 提供具体类型别名 `StringSelectState`（基于 SearchableVec<SharedString> 委托）。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Select ref="select_state" items={my_items} placeholder="请选择" on-change={on_select_change} />
//! ```
//!
//! `items={field}` bind 属性提供委托数据（SearchableVec<SharedString>），
//! codegen 通过 StatefulWithDelegate 机制提取 self.field.clone() 注入 state 构造器。
//!
//! on-change 事件通过 SelectEvent<SearchableVec<SharedString>>::Confirm(Option<SharedString>) 订阅，
//! 用户方法签名约定：`fn on_select_change(&mut self, value: Option<SharedString>, cx: &mut Context<Self>)`。

use gpui::SharedString;

pub use gpui_component::select::{Select, SelectEvent, SelectState};
pub use gpui_component::searchable_list::SearchableVec;
pub use gpui_component::IndexPath;

/// 字符串 Select 的具体 State 类型（SearchableVec<SharedString> 委托）
pub type StringSelectState = SelectState<SearchableVec<SharedString>>;

/// 字符串 Select 的具体 Event 类型
pub type StringSelectEvent = SelectEvent<SearchableVec<SharedString>>;
