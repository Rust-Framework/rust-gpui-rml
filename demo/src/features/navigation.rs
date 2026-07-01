//! 案例树导航桥接（SamplesPanel → MainWindow）

use std::sync::Mutex;

use gpui::App;

static ACTIVATE_HANDLER: Mutex<Option<Box<dyn Fn(String, &mut App) + Send + Sync>>> =
    Mutex::new(None);

pub fn set_case_activate_handler(f: impl Fn(String, &mut App) + Send + Sync + 'static) {
    if let Ok(mut guard) = ACTIVATE_HANDLER.lock() {
        *guard = Some(Box::new(f));
    }
}

pub fn activate_case(case_id: String, app: &mut App) {
    if let Ok(guard) = ACTIVATE_HANDLER.lock() {
        if let Some(f) = guard.as_ref() {
            f(case_id, app);
        }
    }
}
