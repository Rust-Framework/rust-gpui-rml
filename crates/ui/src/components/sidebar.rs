//! Sidebar 组件封装 —— 基于 gpui-component 的 Sidebar
//!
//! 声明式侧边栏容器，支持可折叠、header/footer 插槽。
//! 通过 `SidebarEntry` 枚举将泛型 `Sidebar<E>` 具象化为非泛型 `Sidebar`，
//! 支持子节点类型：`<SidebarMenu>` 和 `<SidebarMenuItem>`。
//!
//! ## 声明式语法
//!
//! ```rml
//! <Sidebar side="left" collapsible="icon" collapsed={is_collapsed}>
//!     <template slot="header">
//!         <SidebarHeader><Label>My App</Label></SidebarHeader>
//!     </template>
//!     <SidebarMenu>
//!         <SidebarMenuItem label="Home" icon="Home" active on_click={go_home} />
//!         <SidebarMenuItem label="Settings" icon="Settings" on_click={go_settings}>
//!             <SidebarMenuItem label="General" on_click={go_general} />
//!         </SidebarMenuItem>
//!     </SidebarMenu>
//!     <template slot="footer">
//!         <SidebarFooter><Label>v1.0</Label></SidebarFooter>
//!     </template>
//! </Sidebar>
//! ```

use gpui::{App, ElementId, IntoElement, Window};
use gpui_component::sidebar::{
    Sidebar as GpuiSidebar, SidebarItem,
};
use gpui_component::Collapsible;

/// 侧边栏条目枚举 —— 统一 `SidebarMenu` 和 `SidebarMenuItem` 为单一类型，
/// 使泛型 `Sidebar<E>` 具象化为 `Sidebar<SidebarEntry>`。
#[derive(Clone)]
pub enum SidebarEntry {
    Menu(SidebarMenu),
    Item(SidebarMenuItem),
}

impl Collapsible for SidebarEntry {
    fn is_collapsed(&self) -> bool {
        match self {
            SidebarEntry::Menu(m) => m.is_collapsed(),
            SidebarEntry::Item(i) => i.is_collapsed(),
        }
    }

    fn collapsed(self, collapsed: bool) -> Self {
        match self {
            SidebarEntry::Menu(m) => SidebarEntry::Menu(m.collapsed(collapsed)),
            SidebarEntry::Item(i) => SidebarEntry::Item(i.collapsed(collapsed)),
        }
    }
}

impl SidebarItem for SidebarEntry {
    fn render(
        self,
        id: impl Into<ElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> impl IntoElement {
        match self {
            SidebarEntry::Menu(m) => m.render(id, window, cx).into_any_element(),
            SidebarEntry::Item(i) => i.render(id, window, cx).into_any_element(),
        }
    }
}

/// RML Sidebar —— 非泛型侧边栏容器，内部使用 `SidebarEntry` 统一子节点类型。
pub type Sidebar = GpuiSidebar<SidebarEntry>;

pub use gpui_component::sidebar::{
    SidebarCollapsible, SidebarFooter, SidebarHeader, SidebarMenu, SidebarMenuItem,
    SidebarToggleButton,
};
