//! RML 核心 prelude
//!
//! 使用方式：`use rml_core::prelude::*;`
//! 包含所有常用 trait、类型与 GPUI 重导出。

pub use crate::ability::{erase, query, register, ErasedAbility};
pub use crate::binding::{BindingContext, BindingPath, IBindingContext};
pub use crate::command::{CallContext, CommandAbilityExt, ICommand, RelayCommand};
pub use crate::component::IComponent;
pub use crate::contribution::{
    ContributionAbilityExt, ContributionOptions, IContribution, IContributionHost,
    IContributionRegistry, IVisualContribution, VisualAbilityExt, register_contribution_ability,
};
pub use crate::value::IValue;
pub use crate::workbench::{IWorkbench, IWorkbenchManager, IWorkbenchProvider, Uri};
pub use crate::converter::{BoolToYesNo, Currency, IConverter, LowerCase, Percent, Trim, UpperCase};
pub use crate::element_ref::ElementRef;
pub use crate::event::IEvent;
pub use crate::events::*;
pub use crate::i18n::{I18nExt, t, t_static};
pub use crate::lifecycle::ILifecycle;
pub use crate::model::{FieldMeta, IModel};
pub use crate::observable::ObservableVec;
pub use crate::two_way_binding::ITwoWayBinding;
pub use crate::validate::{IValidate, ValidResult};
pub use crate::view_model::IViewModel;
pub use crate::window::{IWindow, WindowChrome, WindowControlButtons, WindowStartupLocation, WindowState};

// 重导出 GPUI 常用类型，让 .rml.rs 文件只需 `use rml::prelude::*;`
pub use gpui::{
    App, AppContext, AsyncApp, AsyncWindowContext, Context, Entity, EventEmitter, Global, IntoElement,
    Keystroke, Modifiers, Pixels, Point, Render, SharedString, Task, WeakEntity, Window,
};

/// 文档兼容别名：旧示例代码使用 `ViewContext<T>`，实际为 `Context<T>`。
///
/// RML 文档统一用 `ViewContext<Self>` 表示视图上下文，但现代 GPUI
/// 已将 `ViewContext` 合并入 `Context`。此别名让示例代码可编译。
pub type ViewContext<'a, T> = Context<'a, T>;
