//! 双击检测共享工具 —— 跨组件复用（Tab / Tree 等）。
//!
//! 250ms 时间窗口内两次点击视为双击。状态经 `DblClickState`（App Global）跨帧存储,
//! key 由各组件自行命名（如 `tab-dbl-{ix}` / `tree-dbl-{ix}`）避免冲突。

use std::collections::HashMap;
use std::time::{Duration, Instant};

use gpui::App;

/// 双击检测时间窗口（ms）。
pub const DOUBLE_CLICK_WINDOW: Duration = Duration::from_millis(250);

/// 双击检测状态。经 App Global 跨帧记录每个节点上次点击时间。
#[derive(Default)]
pub struct DblClickState {
    last_clicks: HashMap<String, Option<Instant>>,
}

impl gpui::Global for DblClickState {}

impl DblClickState {
    /// 检测指定 key 是否构成双击,并更新点击时间。
    ///
    /// 双击后清空记录（避免三击误触发）。返回是否构成双击。
    pub fn check_and_update(&mut self, key: &str, now: Instant) -> bool {
        let prev = self.last_clicks.get(key).copied().unwrap_or(None);
        let is_dbl = is_double_click(prev, now);
        self.last_clicks
            .insert(key.to_string(), if is_dbl { None } else { Some(now) });
        is_dbl
    }
}

/// 从 App 获取或初始化 DblClickState,检测双击并更新。
///
/// 封装 `has_global` + `set_global` + `check_and_update` 三步,
/// 供 Tab / Tree 等组件统一调用。
pub fn check_double_click(cx: &mut App, key: &str, now: Instant) -> bool {
    if !cx.has_global::<DblClickState>() {
        cx.set_global(DblClickState::default());
    }
    cx.global_mut::<DblClickState>().check_and_update(key, now)
}

/// 判断是否构成双击。纯函数,便于单测。
pub fn is_double_click(prev: Option<Instant>, now: Instant) -> bool {
    match prev {
        Some(p) => now.duration_since(p) <= DOUBLE_CLICK_WINDOW,
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_double_click_none_prev_returns_false() {
        let now = Instant::now();
        assert!(!is_double_click(None, now));
    }

    #[test]
    fn is_double_click_within_window_returns_true() {
        let now = Instant::now();
        let prev = Some(now - Duration::from_millis(200));
        assert!(is_double_click(prev, now));
    }

    #[test]
    fn is_double_click_at_boundary_returns_true() {
        let now = Instant::now();
        let prev = Some(now - Duration::from_millis(250));
        assert!(is_double_click(prev, now));
    }

    #[test]
    fn is_double_click_beyond_window_returns_false() {
        let now = Instant::now();
        let prev = Some(now - Duration::from_millis(251));
        assert!(!is_double_click(prev, now));
    }

    #[test]
    fn check_and_update_first_click_not_double() {
        let mut state = DblClickState::default();
        let now = Instant::now();
        assert!(!state.check_and_update("test", now));
    }

    #[test]
    fn check_and_update_second_click_within_window_is_double() {
        let mut state = DblClickState::default();
        let t1 = Instant::now();
        let t2 = t1 + Duration::from_millis(100);
        state.check_and_update("test", t1);
        assert!(state.check_and_update("test", t2));
    }

    #[test]
    fn check_and_update_clears_after_double_click() {
        let mut state = DblClickState::default();
        let t1 = Instant::now();
        let t2 = t1 + Duration::from_millis(100);
        let t3 = t2 + Duration::from_millis(100);
        state.check_and_update("test", t1);
        assert!(state.check_and_update("test", t2)); // 双击
        assert!(!state.check_and_update("test", t3)); // 双击后清空,第三击不算
    }
}
