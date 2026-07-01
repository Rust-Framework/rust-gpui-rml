//! 按贡献 id 缓存 GPUI Entity

use std::any::{Any, TypeId};
use std::collections::HashMap;

use gpui::{AnyElement, AppContext, Entity, IntoElement, ParentElement, Render, Styled};

use crate::contribution::{ComponentEntityCache, ContributionRenderContext};

/// 默认组件 Entity 缓存
#[derive(Default)]
pub struct ComponentEntityCacheImpl {
    entries: HashMap<String, (TypeId, Box<dyn Any + Send + Sync>)>,
}

impl ComponentEntityCacheImpl {
    pub fn new() -> Self {
        Self::default()
    }
}

impl ComponentEntityCache for ComponentEntityCacheImpl {
    fn render_view<V>(
        &mut self,
        contribution_id: &str,
        view: V,
        ctx: &mut ContributionRenderContext<'_>,
    ) -> AnyElement
    where
        V: Render + Send + Sync + 'static,
    {
        let entity = if let Some((type_id, boxed)) = self.entries.get(contribution_id) {
            if *type_id == TypeId::of::<V>() {
                boxed.downcast_ref::<Entity<V>>().unwrap().clone()
            } else {
                let entity = ctx.cx.new(|_| view);
                self.entries.insert(
                    contribution_id.to_string(),
                    (TypeId::of::<V>(), Box::new(entity.clone())),
                );
                entity
            }
        } else {
            let entity = ctx.cx.new(|_| view);
            self.entries.insert(
                contribution_id.to_string(),
                (TypeId::of::<V>(), Box::new(entity.clone())),
            );
            entity
        };

        gpui::div()
            .size_full()
            .child(entity)
            .into_any_element()
    }

    fn pre_register<T: gpui::Render + Send + Sync + 'static>(
        &mut self,
        contribution_id: &str,
        entity: gpui::Entity<T>,
    ) {
        self.entries.insert(
            contribution_id.to_string(),
            (TypeId::of::<T>(), Box::new(entity)),
        );
    }

    fn clear(&mut self, contribution_id: &str) {
        self.entries.remove(contribution_id);
    }

    fn clear_all(&mut self) {
        self.entries.clear();
    }
}
