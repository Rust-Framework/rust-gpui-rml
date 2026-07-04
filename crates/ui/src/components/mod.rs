//! UI 组件封装

pub mod activity_bar;
pub mod alert_dialog;
pub mod avatar;
pub mod card;
pub mod menu;
pub mod status_bar;
pub mod tab;
pub mod table;
pub mod tree;

pub use activity_bar::{
    ActivityAct, ActivityBar, ActivityPanel, IActivityAct, IActivityPanel, VisualActivityPanel,
};
pub use avatar::{Avatar, AvatarGroup};
pub use card::{Card, CardVariant};
pub use alert_dialog::{
    AlertDialog, DialogAction, DialogButtonProps, DialogClose, DialogContent, DialogDescription,
    DialogFooter, DialogHeader, DialogTitle,
};
pub use menu::{
    IMenuItem, Menu, MenuBar, MenuItem, configure_menu_bar_popup, menu_bar_button,
    render_menu_bar_from_items,
};
pub use status_bar::{IStatusBarItem, NativeStatusBar, StatusBar, StatusBarAlign, StatusBarItem};
pub use tab::{Tab, TabBar, TabItem, TabVariant};
pub use table::{
    CellTemplate, DefaultTableDelegate, FooterTemplate, HeaderTemplate, Table, TableColumn,
    TableDelegate, TableRow,
};
pub use tree::Tree;
