use std::collections::HashMap;

use gpui::SharedString;
use rml::prelude::*;
use rml_core::i18n::{t_static, I18nState};

use crate::shell::MainWindowRef;

#[contribute(
    host_id = "demo.shell",
    id = "welcome",
    kind = "case",
    order = 0,
)]
#[component]
#[derive(Default)]
pub struct WelcomeCase {
    pub items: Vec<CaseNavItem>,
    pub grouped_items: Vec<CaseNavItemGroup>,
}

#[derive(Clone)]
pub struct CaseNavItem {
    pub id: SharedString,
    pub name: SharedString,
    pub group: Option<SharedString>,
    pub order: i32,
}

#[derive(Clone)]
pub struct CaseNavItemGroup {
    pub label: SharedString,
    pub items: Vec<CaseNavItem>,
}

impl IContribution for WelcomeCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("shell.welcome")
    }
}

impl ILifecycle for WelcomeCase {
    fn on_loaded(&mut self, window: &mut gpui::Window, cx: &mut Context<Self>) {
        // WelcomeCase 由 MainWindow::active_view 渲染，首次 render 时 MainWindow
        // 正在被更新，直接 main.read(cx) 会触发 re-entrant panic。
        // 使用 defer_in 将 refresh_items 推迟到当前 effect cycle 结束后执行。
        cx.defer_in(window, |this, _window, cx| {
            this.refresh_items(cx);
            cx.notify();
        });
        cx.observe_global::<I18nState>(|this, cx| {
            this.refresh_items(cx);
            cx.notify();
        })
        .detach();
    }
}

impl WelcomeCase {
    /// 从 MainWindow.cases 拷贝案例元数据快照（id/name/group/order），
    /// 并预计算分组结果到 `grouped_items` 字段。
    /// locale 切换时由 observe_global 重新调用，name 随 contribution_name() 刷新。
    fn refresh_items(&mut self, cx: &mut Context<Self>) {
        if let Some(main) = cx
            .get_service::<MainWindowRef>()
            .and_then(|r| r.0.upgrade())
        {
            let cases = main.read(cx).cases.clone();
            self.items = cases
                .iter()
                .filter(|c| c.id.as_ref() != Self::CONTRIBUTION_ID)
                .map(|c| CaseNavItem {
                    id: c.id.clone(),
                    name: c.contribution_name(),
                    group: c.group.clone(),
                    order: c.order,
                })
                .collect();
        }
        self.grouped_items = Self::compute_grouped_items(&self.items);
        self.__rml_bump_version("items");
        self.__rml_bump_version("grouped_items");
    }

    /// 按 group 分组 → 按 group 内最小 order 排序组 → 组内按 order 排序。
    /// 与 CaseViewModel::build_tree_items 投影逻辑一致，但不依赖 TreeData。
    fn compute_grouped_items(items: &[CaseNavItem]) -> Vec<CaseNavItemGroup> {
        let mut by_group: HashMap<Option<String>, Vec<CaseNavItem>> = HashMap::new();
        for item in items {
            by_group
                .entry(item.group.as_ref().map(|s| s.to_string()))
                .or_default()
                .push(item.clone());
        }

        let mut groups: Vec<(Option<String>, i32)> = by_group
            .iter()
            .map(|(g, items)| {
                let min_order = items.iter().map(|c| c.order).min().unwrap_or(0);
                (g.clone(), min_order)
            })
            .collect();
        groups.sort_by_key(|(_, o)| *o);

        groups
            .into_iter()
            .map(|(g, _)| {
                let mut items = by_group.remove(&g).unwrap_or_default();
                items.sort_by_key(|c| c.order);

                let label = match g.as_deref() {
                    Some("binding") => t_static("tree.group.binding"),
                    Some("components") => t_static("tree.group.components"),
                    Some("i18n") => t_static("tree.group.i18n"),
                    Some("menu") => t_static("tree.group.menu"),
                    Some("framework") => t_static("tree.group.framework"),
                    Some(other) => SharedString::from(other.to_string()),
                    None => t_static("shell.welcome"),
                };

                CaseNavItemGroup { label, items }
            })
            .collect()
    }

    /// 命令式构建单个分组的渲染树：标题 + 卡片行。
    /// 由模板 `<component each={group in grouped_items} content={self.render_group(group, _window, cx)} />` 调用。
    /// 直接使用 GPUI 方法而非 CSS class，因为 `class="..."` 仅在 codegen 编译期处理。
    pub fn render_group(
        &self,
        group: &CaseNavItemGroup,
        _window: &mut gpui::Window,
        cx: &mut Context<Self>,
    ) -> gpui::AnyElement {
        use gpui::{
            div, px, FontWeight, IntoElement, ParentElement, StatefulInteractiveElement, Styled,
        };
        use rml_ui::Card;

        let cards: Vec<gpui::AnyElement> = group
            .items
            .iter()
            .enumerate()
            .map(|(idx, item)| {
                let case_id = item.id.clone();
                Card::new(("welcome_card", idx))
                    .hoverable(true)
                    .w(px(160.))
                    .child(
                        div()
                            .text_size(px(14.))
                            .font_weight(FontWeight::MEDIUM)
                            .child(item.name.clone()),
                    )
                    .on_click(cx.listener(move |this, _: &gpui::ClickEvent, _window, cx| {
                        this.open_case(case_id.clone(), cx);
                    }))
                    .into_any_element()
            })
            .collect();

        div()
            .flex()
            .flex_col()
            .gap(px(8.))
            .w_full()
            .max_w(px(960.))
            .child(div().child(group.label.clone()))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(px(12.))
                    .children(cards),
            )
            .into_any_element()
    }

    /// 卡片点击 → 委托 MainWindow::open_case 打开对应 Tab。
    pub fn open_case(&mut self, case_id: SharedString, cx: &mut Context<Self>) {
        if let Some(main) = cx
            .get_service::<MainWindowRef>()
            .and_then(|r| r.0.upgrade())
        {
            main.update(cx, |mw, cx| {
                mw.open_case(case_id.to_string(), cx);
            });
        }
    }
}
