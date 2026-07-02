//! UI 组件封装

pub mod activity_bar;
pub mod dialog_window;
pub mod menu;
pub mod status_bar;
pub mod tab;
pub mod tree;

pub use activity_bar::{
    ActivityAct, ActivityActs, ActivityBar, ActivityPanel, ActivityPanels, IActivityAct,
    IActivityPanel,
};
pub use dialog_window::{DialogDragState, DialogTitleBar};
pub use menu::{
    IMenuItem, Menu, MenuBar, MenuItem, MenuItems, configure_menu_bar_popup, menu_bar_button,
    render_menu_bar_from_items,
};
pub use status_bar::{
    IStatusBarItem, NativeStatusBar, StatusBar, StatusBarAlign, StatusBarItem, StatusBarItems,
};
pub use tab::{Tab, TabBar, TabVariant};
pub use tree::Tree;
