//! `#[contributehost]` 窗口自动绑定：订阅注册表变更并驱动 ViewModel 刷新

use gpui::{BorrowAppContext, Context, Render};
use rml_core::contribution::IContributionHostId;

use super::global::ContributionRegistryGlobal;

/// 标注了 `#[contributehost]` 的 ViewModel 可实现此 trait 以在贡献变更时刷新绑定。
///
/// 默认实现仅 `cx.notify()`；主窗口等需映射控件数据的 ViewModel 应覆盖
/// [`on_contributions_changed`](Self::on_contributions_changed)。
pub trait ContributionHostView: IContributionHostId + Render + 'static {
    fn on_contributions_changed(&mut self, cx: &mut Context<Self>) {
        let _ = cx;
    }

    /// 首次 render 时由 codegen 自动调用（无需在 `on_loaded` 中手动绑定）。
    fn attach_contribution_host(this: &mut Self, cx: &mut Context<Self>) {
        if this.__rml_contribution_attached() {
            return;
        }
        this.__rml_set_contribution_attached(true);

        let weak = cx.weak_entity();
        let host_id = Self::ID;
        cx.update_global::<ContributionRegistryGlobal, _>(|global, _| {
            global.0.subscribe_host(
                host_id,
                Box::new(move |app| {
                    if let Some(entity) = weak.upgrade() {
                        entity.update(app, |this, cx| {
                            this.on_contributions_changed(cx);
                            cx.notify();
                        });
                    }
                }),
            );
        });

        this.on_contributions_changed(cx);
        cx.notify();
    }

    #[doc(hidden)]
    fn __rml_contribution_attached(&self) -> bool;

    #[doc(hidden)]
    fn __rml_set_contribution_attached(&mut self, attached: bool);
}

/// codegen 在 `#[contributehost]` 窗口首次 render 时调用
pub fn attach_host_view<T: ContributionHostView>(this: &mut T, cx: &mut Context<T>) {
    T::attach_contribution_host(this, cx);
}
