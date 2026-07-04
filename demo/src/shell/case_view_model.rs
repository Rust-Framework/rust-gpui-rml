//! 案例视图模型 —— 解包 (IVisualContribution, ContributionOptions) 为类型化结构。
//!
//! 供 MainWindow.cases 集合持有，ActivityPanel Tree 直接消费。
//! 替代旧 shell_chrome.rs::map_case_tree_items 投影函数。

use std::collections::HashMap;
use std::sync::Arc;

use gpui::SharedString;
use rml_core::contribution::{ContributionOptions, IContribution, VisualAbilityExt};
use rml_core::i18n::t_static;
use rml_ui::TreeItem;

/// 案例视图模型：解包 case 类视觉贡献的元数据 + 贡献引用。
///
/// 持有 `Arc<dyn IContribution>`（经 `as_visual()` 提取视图引用），
/// 镜像 `VisualActivityPanel` 模式 —— 不直接持有 `Arc<dyn IVisualContribution>`
/// （`Arc<dyn IContribution>` 无法 upcast 到 `Arc<dyn IVisualContribution>`）。
#[derive(Clone)]
pub struct CaseViewModel {
    pub id: SharedString,
    pub name: SharedString,
    pub group: Option<SharedString>,
    pub order: i32,
    contribution: Arc<dyn IContribution>,
}

impl CaseViewModel {
    /// 从贡献条目构造；非 case 槽位或非视觉贡献返回 None。
    pub fn from_contribution(
        c: Arc<dyn IContribution>,
        opts: ContributionOptions,
    ) -> Option<Self> {
        if opts.effective_slot() != Some("case") {
            return None;
        }
        c.as_visual()?;
        Some(Self {
            id: c.id().into(),
            name: c.name(),
            group: opts.group,
            order: opts.order,
            contribution: c,
        })
    }

    /// 渲染案例视图（委托给底层 IVisualContribution）。
    pub fn render(&self, window: &mut gpui::Window, cx: &mut gpui::App) -> gpui::AnyElement {
        self.contribution
            .as_visual()
            .expect("CaseViewModel requires IVisualContribution")
            .render(window, cx)
    }

    /// 动态名称（委托给底层贡献的 name()，反映当前 locale）。
    /// 供 CaseWorkbench::name() 使用 —— tab 标题随 locale 切换刷新。
    pub fn contribution_name(&self) -> SharedString {
        self.contribution.name()
    }

    /// 按 group 分组 → 按 order 排序 → 构建 TreeItem 层级。
    /// 顶层分组节点 expanded，子节点为案例。
    pub fn build_tree_items(cases: &[Self]) -> Vec<TreeItem> {
        let mut by_group: HashMap<Option<String>, Vec<&CaseViewModel>> = HashMap::new();
        for c in cases {
            by_group
                .entry(c.group.as_ref().map(|s| s.to_string()))
                .or_default()
                .push(c);
        }

        let mut groups: Vec<(Option<String>, i32)> = by_group
            .iter()
            .map(|(g, items)| (g.clone(), items.iter().map(|c| c.order).min().unwrap_or(0)))
            .collect();
        groups.sort_by_key(|(_, o)| *o);

        let mut result: Vec<TreeItem> = Vec::new();
        for (group, _) in groups {
            let mut siblings = by_group.get(&group).cloned().unwrap_or_default();
            siblings.sort_by_key(|c| c.order);

            match group {
                Some(g) => {
                    let group_id = format!("group.{}", g);
                    let group_name = t_static(&format!("tree.group.{}", g));
                    let mut item = TreeItem::new(group_id, group_name).expanded(true);
                    for c in siblings {
                        item = item.child(TreeItem::new(c.id.clone(), c.name.clone()));
                    }
                    result.push(item);
                }
                None => {
                    for c in siblings {
                        result.push(TreeItem::new(c.id.clone(), c.name.clone()));
                    }
                }
            }
        }
        result
    }
}
