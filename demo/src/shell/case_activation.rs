//! Demo：案例树激活 → MainWindow 打开 Tab（应用层桥接，非框架 API）

use std::sync::Mutex;

use gpui::App;

static HANDLER: Mutex<Option<Box<dyn Fn(String, &mut App) + Send + Sync>>> = Mutex::new(None);

pub fn register_case_activation(handler: Box<dyn Fn(String, &mut App) + Send + Sync>) {
    *HANDLER.lock().unwrap() = Some(handler);
}

pub fn activate_case(case_id: String, cx: &mut App) {
    if let Ok(guard) = HANDLER.lock() {
        if let Some(handler) = guard.as_ref() {
            handler(case_id, cx);
        }
    }
}
