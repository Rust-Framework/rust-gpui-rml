//! UI 组件封装

pub mod activity_bar;
pub mod alert;
pub mod alert_dialog;
pub mod avatar;
pub mod breadcrumb;
pub mod card;
pub mod collapsible;
pub mod group_box;
pub mod link;
pub mod menu;
pub mod pagination;
pub mod radio;
pub mod skeleton;
pub mod spinner;
pub mod status_bar;
pub mod tab;
pub mod table;
pub mod tree;

pub use activity_bar::{
    ActivityAct, ActivityBar, ActivityPanel, IActivityAct, IActivityPanel, VisualActivityPanel,
};
pub use alert::{Alert, AlertVariant};
pub use avatar::{Avatar, AvatarGroup};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem, BreadcrumbSibling};
pub use card::{Card, CardVariant};
pub use alert_dialog::{
    AlertDialog, DialogAction, DialogButtonProps, DialogClose, DialogContent, DialogDescription,
    DialogFooter, DialogHeader, DialogTitle,
};
pub use menu::{MenuBar, configure_menu_bar_popup, menu_bar_button};
pub use status_bar::{NativeStatusBar, StatusBarAlign};
pub use tab::{Tab, TabBar, TabItem, Tabs, TabVariant};
pub use table::{
    CellTemplate, DefaultTableDelegate, FooterTemplate, HeaderTemplate, Table, TableColumn,
    TableDelegate, TableRow,
};
pub use tree::Tree;

// Phase 1 基础无状态组件 re-exports
pub use collapsible::Collapsible;
pub use group_box::{GroupBox, GroupBoxVariants};
pub use link::Link;
pub use pagination::Pagination;
pub use radio::{Radio, RadioGroup};
pub use skeleton::Skeleton;
pub use spinner::Spinner;
