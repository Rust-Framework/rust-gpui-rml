//! 集中能力扩展 —— 所有 studio 子 trait 的能力查询与注册。
//!
//! 对齐 rml_core 模式:每个子 trait 配套 AbilityExt + register 函数,
//! 经 `rml_core::ability::query`/`register`/`erase`/`restore` 实现类型擦除能力查询。
//!
//! 所有 studio crate 的 `*AbilityExt` trait 与 `register_*_ability` 函数集中定义在此文件,
//! 避免分散到多个模块。

use std::any::Any;

use rml_core::value::IValue;

use crate::command::IEditorCommand;
use crate::component::{IWorkbenchComponent, IWorkbenchComponentHost};
use crate::workspace::IWorkspace;
use crate::worktree::IWorktree;

// ──────────────────────────────────────────────────────────────────────────
//  IWorkbenchComponent
// ──────────────────────────────────────────────────────────────────────────

/// 工作台组件能力扩展 —— 让 `dyn IValue` 可查询 `IWorkbenchComponent` 能力。
pub trait WorkbenchComponentAbilityExt {
    /// 若此值实现了 `IWorkbenchComponent`,返回引用;否则 `None`。
    fn as_workbench_component(&self) -> Option<&dyn IWorkbenchComponent>;
}

#[allow(unsafe_code)]
impl WorkbenchComponentAbilityExt for dyn IValue {
    fn as_workbench_component(&self) -> Option<&dyn IWorkbenchComponent> {
        let erased = rml_core::ability::query::<dyn IWorkbenchComponent>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorkbenchComponent>(erased) })
    }
}

/// 为实现 `IWorkbenchComponent` 的类型注册能力 cast 函数。
///
/// 业务自定义组件类型后,需在初始化时调用此函数注册,
/// 使 `as_workbench_component()` 查询生效,`EditorWorkbench` 可据此分类受理。
#[allow(unsafe_code)]
pub fn register_workbench_component_ability<T: IWorkbenchComponent + 'static>() {
    rml_core::ability::register::<T, dyn IWorkbenchComponent>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let v: &dyn IWorkbenchComponent = s;
            unsafe { rml_core::ability::erase(v) }
        })
    });
}

/// `dyn IContribution` 薄委托 —— trait upcast 到 `&dyn IValue` 后调用主 impl,
/// 使注册表返回的 `Arc<dyn IContribution>` 可直接调用 `as_workbench_component()`。
impl WorkbenchComponentAbilityExt for dyn rml_core::contribution::IContribution {
    fn as_workbench_component(&self) -> Option<&dyn IWorkbenchComponent> {
        let iv: &dyn IValue = self;
        iv.as_workbench_component()
    }
}

// ──────────────────────────────────────────────────────────────────────────
//  IWorkbenchComponentHost
// ──────────────────────────────────────────────────────────────────────────

/// 工作台组件宿主能力扩展 —— 让 `dyn IValue` 可查询 `IWorkbenchComponentHost` 能力。
///
/// 与 `WorkbenchComponentAbilityExt` 模式一致。`EditorWorkbench` impl
/// `IWorkbenchComponentHost` 后,经 `register_workbench_component_host_ability::<T>()`
/// 注册,`as_workbench_component_host()` 查询即可生效。
pub trait WorkbenchComponentHostAbilityExt {
    /// 若此值实现了 `IWorkbenchComponentHost`,返回引用;否则 `None`。
    fn as_workbench_component_host(&self) -> Option<&dyn IWorkbenchComponentHost>;
}

#[allow(unsafe_code)]
impl WorkbenchComponentHostAbilityExt for dyn IValue {
    fn as_workbench_component_host(&self) -> Option<&dyn IWorkbenchComponentHost> {
        let erased = rml_core::ability::query::<dyn IWorkbenchComponentHost>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorkbenchComponentHost>(erased) })
    }
}

/// 为实现 `IWorkbenchComponentHost` 的类型注册能力 cast 函数。
///
/// `EditorWorkbench` impl `IWorkbenchComponentHost` 后,需在 `#[ctor::ctor]` 中
/// 调用此函数注册,使 `as_workbench_component_host()` 查询生效。组件经此查询
/// 可取到 host 的 `document()` / `state()` 等共享 Entity。
#[allow(unsafe_code)]
pub fn register_workbench_component_host_ability<T: IWorkbenchComponentHost + 'static>() {
    rml_core::ability::register::<T, dyn IWorkbenchComponentHost>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let h: &dyn IWorkbenchComponentHost = s;
            unsafe { rml_core::ability::erase(h) }
        })
    });
}

// ──────────────────────────────────────────────────────────────────────────
//  IEditorCommand
// ──────────────────────────────────────────────────────────────────────────

/// 编辑器命令能力扩展 —— 让 `dyn IValue` 可查询 `IEditorCommand` 能力。
pub trait EditorCommandAbilityExt {
    /// 若此值实现了 `IEditorCommand`,返回命令引用;否则 `None`。
    fn as_editor_command(&self) -> Option<&dyn IEditorCommand>;
}

#[allow(unsafe_code)]
impl EditorCommandAbilityExt for dyn IValue {
    fn as_editor_command(&self) -> Option<&dyn IEditorCommand> {
        let erased = rml_core::ability::query::<dyn IEditorCommand>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IEditorCommand>(erased) })
    }
}

/// 为实现 `IEditorCommand` 的类型注册能力 cast 函数。
///
/// 命令实现后,需调用此函数注册,使 `as_editor_command()` 查询生效。
#[allow(unsafe_code)]
pub fn register_editor_command_ability<T: IEditorCommand + 'static>() {
    rml_core::ability::register::<T, dyn IEditorCommand>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let cmd: &dyn IEditorCommand = s;
            unsafe { rml_core::ability::erase(cmd) }
        })
    });
}

// ──────────────────────────────────────────────────────────────────────────
//  IWorktree
// ──────────────────────────────────────────────────────────────────────────

/// 工作树能力扩展 —— 让 `dyn IValue` 可查询 `IWorktree` 能力。
pub trait WorktreeAbilityExt {
    /// 若此值实现了 `IWorktree`,返回引用;否则 `None`。
    fn as_worktree(&self) -> Option<&dyn IWorktree>;
}

#[allow(unsafe_code)]
impl WorktreeAbilityExt for dyn IValue {
    fn as_worktree(&self) -> Option<&dyn IWorktree> {
        let erased = rml_core::ability::query::<dyn IWorktree>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorktree>(erased) })
    }
}

/// 为实现 `IWorktree` 的类型注册能力 cast 函数。
#[allow(unsafe_code)]
pub fn register_worktree_ability<T: IWorktree + 'static>() {
    rml_core::ability::register::<T, dyn IWorktree>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let wt: &dyn IWorktree = s;
            unsafe { rml_core::ability::erase(wt) }
        })
    });
}

// ──────────────────────────────────────────────────────────────────────────
//  IWorkspace
// ──────────────────────────────────────────────────────────────────────────

/// 工作空间能力扩展 —— 让 `dyn IValue` 可查询 `IWorkspace` 能力。
pub trait WorkspaceAbilityExt {
    /// 若此值实现了 `IWorkspace`,返回引用;否则 `None`。
    fn as_workspace(&self) -> Option<&dyn IWorkspace>;
}

#[allow(unsafe_code)]
impl WorkspaceAbilityExt for dyn IValue {
    fn as_workspace(&self) -> Option<&dyn IWorkspace> {
        let erased = rml_core::ability::query::<dyn IWorkspace>(self)?;
        Some(unsafe { rml_core::ability::restore::<dyn IWorkspace>(erased) })
    }
}

/// 为实现 `IWorkspace` 的类型注册能力 cast 函数。
#[allow(unsafe_code)]
pub fn register_workspace_ability<T: IWorkspace + 'static>() {
    rml_core::ability::register::<T, dyn IWorkspace>(|c| {
        let any: &dyn Any = c;
        any.downcast_ref::<T>().map(|s| {
            let ws: &dyn IWorkspace = s;
            unsafe { rml_core::ability::erase(ws) }
        })
    });
}
