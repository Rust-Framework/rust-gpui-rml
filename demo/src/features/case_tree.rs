//! 案例树节点贡献（数据类 `IContribution`）

use std::sync::Arc;

use gpui::{App, BorrowAppContext, SharedString};
use rml_app::contribution::{data_registerable, ContributionRegistryGlobal, Registerable};
use rml_core::contribution::{ContributionOptions, IContribution};

use crate::shell::hosts;

macro_rules! case_node {
    ($id:expr, $name_key:expr, $parent:expr, $order:expr) => {
        CaseNodeContribution {
            id: $id,
            name_key: $name_key,
            parent_id: $parent,
            order: $order,
        }
    };
}

#[derive(Clone)]
struct CaseNodeContribution {
    id: &'static str,
    name_key: &'static str,
    parent_id: Option<&'static str>,
    order: i32,
}

impl IContribution for CaseNodeContribution {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> SharedString {
        rml_core::i18n::t_static(self.name_key).into()
    }

    fn description(&self) -> SharedString {
        SharedString::default()
    }

    fn icon(&self) -> Option<SharedString> {
        None
    }
}

impl Registerable for CaseNodeContribution {
    fn into_entry(
        contribution: Arc<Self>,
        options: ContributionOptions,
    ) -> rml_core::contribution::ContributedEntry {
        data_registerable(contribution, options)
    }
}

fn register_node(cx: &mut App, node: CaseNodeContribution) {
    let mut opts = ContributionOptions::new().order(node.order);
    if let Some(parent) = node.parent_id {
        opts = opts.parent_id(parent);
    }
    let node = node.clone();
    cx.update_global::<ContributionRegistryGlobal, _>(|global, cx| {
        global
            .0
            .register(hosts::CASE_TREE, Arc::new(node), opts, cx);
    });
}

/// 注册案例树元数据贡献
pub fn register_case_tree(cx: &mut App) {
    register_node(
        cx,
        case_node!("cat.binding", "tree.cat.binding", None, 0),
    );
    register_node(
        cx,
        case_node!("binding.counter", "tree.case.counter", Some("cat.binding"), 1),
    );
    register_node(
        cx,
        case_node!("binding.two-way", "tree.case.two_way", Some("cat.binding"), 2),
    );
    register_node(
        cx,
        case_node!("cat.components", "tree.cat.components", None, 10),
    );
    register_node(
        cx,
        case_node!(
            "components.button",
            "tree.case.button",
            Some("cat.components"),
            11
        ),
    );
    register_node(cx, case_node!("cat.i18n", "tree.cat.i18n", None, 20));
    register_node(
        cx,
        case_node!("i18n.basic", "tree.case.i18n", Some("cat.i18n"), 21),
    );
}
