//! UI 组件封装

pub mod activity_bar;
pub mod alert;
pub mod alert_dialog;
pub mod avatar;
pub mod breadcrumb;
pub mod calendar;
pub mod card;
pub mod chat;
pub mod collapsible;
pub mod color_picker;
pub mod combobox;
pub mod date_picker;
pub mod dock;
pub mod form;
pub mod grid;
pub mod group_box;
pub mod hover_card;
pub mod key_binding;
pub mod link;
pub mod markdown;
pub mod menu;
pub mod notification_trigger;
pub mod number_input;
pub mod otp_input;
pub mod pagination;
pub mod radio;
pub mod rating;
pub mod resizable;
pub mod scroll;
pub mod select;
pub mod settings;
pub mod sheet;
pub mod shortcut_scope;
pub mod sidebar;
pub mod skeleton;
pub mod spinner;
pub mod status_bar;
pub mod stepper;
pub mod tab;
pub mod table;
pub mod theme_switcher;
pub mod tree;
pub mod virtual_list;

pub use activity_bar::{
    ActivityAct, ActivityBar, ActivityPanel, IActivityAct, IActivityPanel, VisualActivityPanel,
    get_activity_panels, register_activity_panel,
};
pub use alert::{Alert, AlertVariant};
pub use alert_dialog::{
    AlertDialog, DialogAction, DialogButtonProps, DialogClose, DialogContent, DialogDescription,
    DialogFooter, DialogHeader, DialogTitle,
};
pub use avatar::{Avatar, AvatarGroup};
pub use breadcrumb::{Breadcrumb, BreadcrumbItem, BreadcrumbSibling};
pub use calendar::{Calendar, CalendarEvent, CalendarState, Date};
pub use card::{Card, CardVariant};
pub use menu::{configure_menu_bar_popup, menu_bar_button, MenuBar};
pub use status_bar::{NativeStatusBar, StatusBarAlign};
pub use tab::{Tab, TabBar, TabItem, TabVariant, Tabs};
pub use table::{
    CellTemplate, DefaultTableDelegate, FooterTemplate, HeaderTemplate, Table, TableColumn,
    TableDelegate, TableRow,
};
pub use tree::{Tree, TreeData};

// Phase 1 基础无状态组件 re-exports
pub use collapsible::Collapsible;
pub use color_picker::{ColorPicker, ColorPickerEvent, ColorPickerState};
pub use combobox::{
    Combobox, ComboboxEvent, ComboboxState, StringComboboxEvent, StringComboboxState,
};
pub use date_picker::{DatePicker, DatePickerEvent, DatePickerState};
pub use dock::{
    register_panel, DockArea, DockEvent, DockItem, DockPlacement, Panel, PanelControl, PanelEvent,
    PanelStyle, PanelView, SimplePanel, StackPanel, TabPanel,
};
pub use form::{Field, FieldBuilder, Form};
pub use grid::{Grid, GridItem};
pub use group_box::{GroupBox, GroupBoxVariants};
pub use hover_card::{HoverCard, HoverCardState};
pub use key_binding::KeyBinding;
pub use link::Link;
pub use markdown::Markdown;
pub use notification_trigger::NotificationTrigger;
pub use number_input::{NumberInput, NumberInputEvent};
pub use otp_input::{OtpInput, OtpState};
pub use pagination::Pagination;
pub use radio::{Radio, RadioGroup};
pub use rating::Rating;
pub use resizable::{
    h_resizable, resizable_panel, v_resizable, ResizablePanel, ResizablePanelEvent,
    ResizablePanelGroup, ResizableState,
};
pub use scroll::Scroll;
pub use select::{
    IndexPath, SearchableVec, Select, SelectEvent, SelectState, StringSelectEvent,
    StringSelectState,
};
pub use settings::{
    AnySettingField, GroupBoxVariant, NumberFieldOptions, RenderOptions, SelectIndex, SettingField,
    SettingFieldElement, SettingFieldType, SettingGroup, SettingItem, SettingPage, Settings,
};
pub use sheet::Sheet;
pub use shortcut_scope::ShortcutScope;
pub use sidebar::{
    Sidebar, SidebarCollapsible, SidebarEntry, SidebarFooter, SidebarHeader, SidebarMenu,
    SidebarMenuItem, SidebarToggleButton,
};
pub use skeleton::Skeleton;
pub use spinner::Spinner;
pub use stepper::{Stepper, StepperItem};
pub use theme_switcher::ThemeSwitcher;
pub use virtual_list::{h_virtual_list, v_virtual_list, VirtualList, VirtualListScrollHandle};

pub use chat::{
    render_content, ChatAttachment, ChatBubble, ChatConfig, ChatConversation, ChatError, ChatEvent,
    ChatInput, ChatInputEvent, ChatMessage, ChatMessageAction, ChatMetadata, ChatPanel, ChatRequest,
    ChatRole, ChatStreamEvent, ChatToolCall, IChatBackend, MessageActionItem, MessageListEvent,
    MessageListView, ModelInfo, RenderMode,
};
