//! UI 组件封装

pub mod activity_bar;
pub mod alert;
pub mod alert_dialog;
pub mod avatar;
pub mod breadcrumb;
pub mod calendar;
pub mod card;
pub mod collapsible;
pub mod color_picker;
pub mod combobox;
pub mod date_picker;
pub mod form;
pub mod group_box;
pub mod hover_card;
pub mod link;
pub mod menu;
pub mod notification_trigger;
pub mod number_input;
pub mod otp_input;
pub mod pagination;
pub mod radio;
pub mod rating;
pub mod resizable;
pub mod select;
pub mod scroll;
pub mod settings;
pub mod sidebar;
pub mod sheet;
pub mod skeleton;
pub mod spinner;
pub mod status_bar;
pub mod stepper;
pub mod tab;
pub mod table;
pub mod tree;
pub mod virtual_list;

pub use activity_bar::{
    ActivityAct, ActivityBar, ActivityPanel, IActivityAct, IActivityPanel, VisualActivityPanel,
};
pub use alert::{Alert, AlertVariant};
pub use avatar::{Avatar, AvatarGroup};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem, BreadcrumbSibling};
pub use calendar::{Calendar, CalendarEvent, CalendarState, Date};
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
pub use tree::{Tree, TreeData};

// Phase 1 基础无状态组件 re-exports
pub use collapsible::Collapsible;
pub use color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
pub use combobox::{Combobox, ComboboxEvent, ComboboxState, StringComboboxEvent, StringComboboxState};
pub use date_picker::{DatePicker, DatePickerEvent, DatePickerState};
pub use form::{Field, FieldBuilder, Form};
pub use group_box::{GroupBox, GroupBoxVariants};
pub use hover_card::{HoverCard, HoverCardState};
pub use link::Link;
pub use notification_trigger::NotificationTrigger;
pub use number_input::{NumberInput, NumberInputEvent};
pub use otp_input::{OtpInput, OtpState};
pub use pagination::Pagination;
pub use radio::{Radio, RadioGroup};
pub use rating::Rating;
pub use resizable::{
    ResizablePanel, ResizablePanelEvent, ResizablePanelGroup, ResizableState, h_resizable,
    resizable_panel, v_resizable,
};
pub use settings::{
    AnySettingField, GroupBoxVariant, NumberFieldOptions, RenderOptions, SelectIndex,
    SettingField, SettingFieldElement, SettingFieldType, SettingGroup, SettingItem, SettingPage,
    Settings,
};
pub use sheet::Sheet;
pub use skeleton::Skeleton;
pub use select::{IndexPath, SearchableVec, Select, SelectEvent, SelectState, StringSelectEvent, StringSelectState};
pub use scroll::Scroll;
pub use sidebar::{
    Sidebar, SidebarCollapsible, SidebarEntry, SidebarFooter, SidebarHeader, SidebarMenu,
    SidebarMenuItem, SidebarToggleButton,
};
pub use spinner::Spinner;
pub use stepper::{Stepper, StepperItem};
pub use virtual_list::{VirtualList, VirtualListScrollHandle, h_virtual_list, v_virtual_list};
