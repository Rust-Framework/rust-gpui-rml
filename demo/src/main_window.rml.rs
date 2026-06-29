use rml::prelude::*;
use rml_ui::{IconName, MenuItem, StatusBarItem};

#[window]
#[derive(Default)]
pub struct MainWindow {
    pub count: i32,
}

impl MainWindow {
    #[computed]
    pub fn menu_items(&self) -> Vec<MenuItem> {
        vec![
            MenuItem::new("文件").submenu(vec![
                MenuItem::new("新建"),
                MenuItem::new("打开"),
                MenuItem::separator(),
                MenuItem::new("退出"),
            ]),
            MenuItem::new("编辑").submenu(vec![
                MenuItem::new("撤销"),
                MenuItem::new("重做"),
                MenuItem::separator(),
                MenuItem::new("剪切"),
                MenuItem::new("复制"),
                MenuItem::new("粘贴"),
            ]),
            MenuItem::new("视图").submenu(vec![
                MenuItem::new("放大"),
                MenuItem::new("缩小"),
                MenuItem::separator(),
                MenuItem::new("重置缩放"),
            ]),
            MenuItem::new("帮助").submenu(vec![MenuItem::new("关于")]),
        ]
    }

    #[computed]
    pub fn status_items(&self) -> Vec<StatusBarItem> {
        vec![StatusBarItem::new("就绪")]
    }

    #[command]
    pub fn on_click(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.count += 1;
        cx.notify();
    }
}
