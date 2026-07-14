//! ExplorerPanel —— Arc Studio 文件资源管理器面板。
//!
//! `#[component]` RML 组件,渲染多根文件树。
//! - `.rml` 模板: `<Tree ref="tree_state" items={tree_items} on-activate="on_file_activate" />`
//! - 手动 `impl IContribution`(id/name/icon 元数据)
//! - 手动 `impl IVisual`(经 `get_or_create_entity` 复用 Entity,委托 `Render::render`)
//! - `impl ILifecycle::on_loaded` → `refresh_tree`
//! - `#[command] on_file_activate` —— 解析路径 → URI → IWorkbenchManager::open
//!
//! 经 DI 获取 `IWorkspaceManager` 列出工作空间,每个工作空间构建为 TreeData 根节点,
//! 递归展开子目录(跳过 target/node_modules/.git/dist/build)。

use std::path::{Path, PathBuf};
use std::sync::{Arc, Once};

use gpui::AnyElement;
use rml::prelude::*;
use rml_app::contribution::get_or_create_entity;
use rml_core::contribution::IconSpec;
use rml_core::context::ServiceProviderExt;
use rml_core::workbench::IWorkbenchManager;
use rml_ui::{TreeData, TreeState};
use studio_core::worktree::EntryKind;
use studio_core::workspace::{IWorkspace, IWorkspaceManager};

/// 文件树递归构建时跳过的目录名(性能优化 + 避免噪音)。
const SKIP_DIRS: &[&str] = &["target", "node_modules", ".git", "dist", "build"];

/// Arc Studio 文件资源管理器面板。
///
/// `#[component]` 生成 `impl IModel + IViewModel + IComponent`(RML 框架契约),
/// 经 `include!` 引入 RML 编译器生成的 `impl Render` 驱动 `.rml` 模板。
///
/// 手动 `impl IContribution + IVisual + ILifecycle` 补充元数据 + 渲染入口 + 生命周期
/// (因 `#[contribute]` 被项目规范拒绝 —— 生成 `contribution_entries` 污染业务代码)。
#[component]
#[derive(Default)]
pub struct ExplorerPanel {
    tree_state: ElementRef<TreeState>,
    tree_items: Vec<TreeData>,
}

impl IContribution for ExplorerPanel {
    fn id(&self) -> &str {
        "explorer"
    }
    fn name(&self) -> SharedString {
        "Explorer".into()
    }
    fn icon(&self) -> Option<IconSpec> {
        Some(IconSpec::named("Folder"))
    }
}

impl IVisual for ExplorerPanel {
    fn render(&self, window: &mut Window, cx: &mut App) -> AnyElement {
        let entity = get_or_create_entity::<ExplorerPanel>(cx);
        entity.update(cx, |this, ctx| this.render(window, ctx).into_any_element())
    }
}

impl ILifecycle for ExplorerPanel {
    fn on_loaded(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_tree(cx);
    }
}

impl ExplorerPanel {
    /// 刷新文件树:经 DI 获取 IWorkspaceManager → list() → 构建多根 TreeData。
    fn refresh_tree(&mut self, cx: &mut Context<Self>) {
        let workspaces = cx
            .get_trait::<dyn IWorkspaceManager>()
            .map(|mgr| mgr.list())
            .unwrap_or_default();

        self.tree_items = workspaces.iter().map(build_workspace_root).collect();
        cx.notify();
    }

    /// 双击文件节点:解析路径 → URI → IWorkbenchManager::open 打开 Tab。
    #[command]
    pub fn on_file_activate(&mut self, item_id: &SharedString, cx: &mut Context<Self>) {
        let path = PathBuf::from(item_id.to_string());

        let Some(workspace_mgr) = cx.get_trait::<dyn IWorkspaceManager>() else {
            return;
        };
        let Some(workbench_mgr) = cx.get_trait::<dyn IWorkbenchManager>() else {
            return;
        };

        let Some(ws) = workspace_mgr
            .list()
            .into_iter()
            .find(|ws| path.starts_with(ws.root()))
        else {
            return;
        };

        if let Ok(rel) = path.strip_prefix(ws.root()) {
            let uri = ws.resolve(rel);
            workbench_mgr.open(&uri);
        }
    }
}

/// 构建工作空间根节点 —— label = "name (branch)",递归展开子目录。
fn build_workspace_root(ws: &Arc<dyn IWorkspace>) -> TreeData {
    let name = ws.name().to_string();
    let label = match ws.branch() {
        Some(branch) => format!("{name} ({branch})"),
        None => name,
    };
    let id = ws.root().to_string_lossy().into_owned();
    let children = build_entry_tree(ws, None, 0);
    TreeData::new(id, label).children(children).expanded(true)
}

/// 递归构建目录子树(跳过 SKIP_DIRS,深度限制 3 层避免大仓库性能问题)。
fn build_entry_tree(ws: &Arc<dyn IWorkspace>, dir: Option<&Path>, depth: usize) -> Vec<TreeData> {
    if depth > 3 {
        return Vec::new();
    }
    ws.entries(dir)
        .into_iter()
        .filter(|e| !should_skip(&e.name))
        .map(|e| {
            let abs_path = ws.root().join(&e.path);
            let id = abs_path.to_string_lossy().into_owned();
            let children = if e.kind == EntryKind::Directory {
                build_entry_tree(ws, Some(&e.path), depth + 1)
            } else {
                Vec::new()
            };
            TreeData::new(id, e.name.to_string()).children(children)
        })
        .collect()
}

fn should_skip(name: &str) -> bool {
    SKIP_DIRS.contains(&name)
}

// ──────────────────────────────────────────────────────────────────────────
//  能力注册:ExplorerPanel 需注册 IContribution + IVisual 能力 cast,
//  使 VisualActivityPanel::new(c).as_visual() 查询生效。
// ──────────────────────────────────────────────────────────────────────────

static ABILITY_REGISTERED: Once = Once::new();

pub fn register_explorer_abilities() {
    ABILITY_REGISTERED.call_once(|| {
        register_contribution_ability::<ExplorerPanel>();
        register_visual_ability::<ExplorerPanel>();
    });
}
