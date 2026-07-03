//! 能力查询基础设施 —— mopa 模式实现 trait object 间 downcast。
//!
//! 核心思路：
//! - trait upcasting：`&dyn IContribution` 直接 coerce 到 `&dyn Any`（因 `IContribution: Any`）
//! - `downcast_ref::<T>()` 还原具体类型，再 trait upcast 到 `&dyn Ability`
//! - 全局 `HashMap<(TypeId, TypeId), CastFn>` 按 (concrete_type_id, ability_trait_id) 索引 cast 函数
//! - cast 函数 transmute `&dyn Ability` 为 `ErasedAbility`（擦除 fat pointer）
//! - `restore::<A>` 将 `ErasedAbility` 还原为 `&dyn A`
//!
//! unsafe 仅存在于 `erase`/`restore` 两个函数，封装在本模块内。

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::{LazyLock, RwLock};

use crate::contribution::IContribution;

/// 擦除后的能力 fat pointer（data + vtable）。
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ErasedAbility {
    data: *const (),
    vtable: *const (),
}

/// cast 函数类型：`&dyn IContribution` → `Option<ErasedAbility>`。
pub type CastFn = fn(&dyn IContribution) -> Option<ErasedAbility>;

static ABILITY_REGISTRY: LazyLock<RwLock<HashMap<(TypeId, TypeId), CastFn>>> =
    LazyLock::new(|| RwLock::new(HashMap::new()));

/// 注册能力 cast 函数（幂等，重复注册同一 key 等价于覆盖）。
///
/// 由 `#[contribute]` 宏在 `__rml_register_*` 中调用。
pub fn register<T: 'static, A: ?Sized + 'static>(cast_fn: CastFn) {
    let key = (TypeId::of::<T>(), TypeId::of::<A>());
    ABILITY_REGISTRY.write().unwrap().insert(key, cast_fn);
}

/// 查询能力：返回擦除后的 fat pointer，由 `restore::<A>` 还原。
pub fn query<A: ?Sized + 'static>(c: &dyn IContribution) -> Option<ErasedAbility> {
    // trait upcast：&dyn IContribution → &dyn Any（因 IContribution: Any）
    let any: &dyn Any = c;
    let concrete_id = any.type_id();
    let ability_id = TypeId::of::<A>();
    let registry = ABILITY_REGISTRY.read().unwrap();
    let cast_fn = registry.get(&(concrete_id, ability_id))?;
    cast_fn(c)
}

/// 擦除：`&dyn A` → `ErasedAbility`。
///
/// # Safety
/// `a` 必须是合法的 `&dyn A` fat pointer。`A: 'static` 保证 vtable 静态有效。
#[allow(unsafe_code)]
pub unsafe fn erase<A: ?Sized + 'static>(a: &A) -> ErasedAbility {
    let ptr: *const A = a;
    // fat pointer (data + vtable) 与 ErasedAbility 布局一致
    unsafe { std::mem::transmute_copy::<*const A, ErasedAbility>(&ptr) }
}

/// 还原：`ErasedAbility` → `&dyn A`。
///
/// # Safety
/// `erased` 必须由 `erase::<A>` 产生，且 `A` 与还原目标一致。
#[allow(unsafe_code)]
pub unsafe fn restore<'a, A: ?Sized + 'static>(erased: ErasedAbility) -> &'a A {
    let ptr: *const A = unsafe { std::mem::transmute_copy::<ErasedAbility, *const A>(&erased) };
    unsafe { &*ptr }
}

// ──────────────────────────────────────────────────────────────────────────
//  单元测试
// ──────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::{CallContext, CommandAbilityExt, ICommand};
    use crate::contribution::IContribution;
    use gpui::SharedString;

    struct TestCmd;
    impl IContribution for TestCmd {
        fn id(&self) -> &str {
            "test.cmd"
        }
        fn name(&self) -> SharedString {
            "test".into()
        }
    }
    impl ICommand for TestCmd {
        fn execute(&self, _ctx: &mut CallContext) {}
    }

    #[test]
    #[allow(unsafe_code)]
    fn ability_query_returns_some_when_registered() {
        register::<TestCmd, dyn ICommand>(|c| {
            let any: &dyn Any = c;
            any.downcast_ref::<TestCmd>().map(|s| {
                let cmd: &dyn ICommand = s;
                unsafe { erase(cmd) }
            })
        });
        let cmd = TestCmd;
        let c: &dyn IContribution = &cmd;
        assert!(c.as_command().is_some());
    }

    #[test]
    fn ability_query_returns_none_when_not_registered() {
        struct Unregistered;
        impl IContribution for Unregistered {
            fn id(&self) -> &str {
                "unreg"
            }
            fn name(&self) -> SharedString {
                "u".into()
            }
        }
        let u = Unregistered;
        let c: &dyn IContribution = &u;
        assert!(c.as_command().is_none());
    }
}
