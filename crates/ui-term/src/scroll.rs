//! Bridges alacritty scrollback to [`gpui_component::scroll::ScrollbarHandle`].
//!
//! GPUI scroll offsets use `0` at the top of the scrollable range and negative values
//! when scrolled toward the bottom — the same convention as [`gpui::ScrollHandle`].

use crate::event::GpuiEventProxy;
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::term::Term;
use gpui::{Pixels, Point, Size, px};
use gpui_component::scroll::ScrollbarHandle;
use parking_lot::Mutex;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// Matches `gpui_component::scroll::scrollbar::WIDTH` (overlay track width).
pub const SCROLLBAR_TRACK_WIDTH: Pixels = px(16.);

/// Scroll handle for terminal history, compatible with gpui-component scrollbars.
#[derive(Clone)]
pub struct TerminalScrollHandle {
    term: Arc<Mutex<Term<GpuiEventProxy>>>,
    cell_height: Arc<Mutex<Pixels>>,
    wake_tx: flume::Sender<()>,
    auto_follow: Arc<AtomicBool>,
}

impl TerminalScrollHandle {
    pub fn new(
        term: Arc<Mutex<Term<GpuiEventProxy>>>,
        wake_tx: flume::Sender<()>,
        auto_follow: Arc<AtomicBool>,
    ) -> Self {
        Self {
            term,
            cell_height: Arc::new(Mutex::new(px(14.0))),
            wake_tx,
            auto_follow,
        }
    }

    pub fn set_cell_height(&self, height: Pixels) {
        *self.cell_height.lock() = height;
    }

    fn cell_height(&self) -> Pixels {
        *self.cell_height.lock()
    }

    /// GPUI scroll Y for a given alacritty `display_offset` (0 = live end).
    fn scroll_y_for_display_offset(
        display_offset: usize,
        history: usize,
        cell_height: Pixels,
    ) -> Pixels {
        let lines_from_top = history.saturating_sub(display_offset);
        -cell_height * lines_from_top as f32
    }

    /// Alacritty `display_offset` from a GPUI scroll Y (`0` = top/oldest, negative = newer).
    fn display_offset_for_scroll_y(scroll_y: Pixels, history: usize, cell_height: Pixels) -> usize {
        if history == 0 || cell_height <= px(0.0) {
            return 0;
        }
        let lines_from_top = if scroll_y >= px(0.0) {
            0
        } else {
            (-scroll_y / cell_height).round().max(0.0) as usize
        };
        history.saturating_sub(lines_from_top)
    }

    fn apply_scroll_y(&self, scroll_y: Pixels) {
        let cell_height = self.cell_height();
        if cell_height <= px(0.0) {
            return;
        }

        let mut term = self.term.lock();
        let grid = term.grid();
        let history = grid.history_size();
        if history == 0 {
            return;
        }

        let target_offset =
            Self::display_offset_for_scroll_y(scroll_y, history, cell_height);
        let current = grid.display_offset();
        let delta = target_offset as i32 - current as i32;
        if delta != 0 {
            term.scroll_display(Scroll::Delta(delta));
            self.auto_follow
                .store(target_offset == 0, Ordering::Relaxed);
            let _ = self.wake_tx.send(());
        }
    }
}

impl ScrollbarHandle for TerminalScrollHandle {
    fn offset(&self) -> Point<Pixels> {
        let term = self.term.lock();
        let grid = term.grid();
        let history = grid.history_size();
        let display_offset = grid.display_offset();
        let cell_height = self.cell_height();

        let y = Self::scroll_y_for_display_offset(display_offset, history, cell_height);
        Point::new(px(0.0), y)
    }

    fn set_offset(&self, offset: Point<Pixels>) {
        self.apply_scroll_y(offset.y);
    }

    fn content_size(&self) -> Size<Pixels> {
        let term = self.term.lock();
        let grid = term.grid();
        let cell_height = self.cell_height();
        let total_lines = grid.history_size() + grid.screen_lines();
        Size::new(px(0.0), cell_height * total_lines as f32)
    }

    fn start_drag(&self) {
        self.auto_follow.store(false, Ordering::Relaxed);
    }

    fn end_drag(&self) {
        let at_bottom = self.term.lock().grid().display_offset() == 0;
        self.auto_follow.store(at_bottom, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalScrollHandle;
    use gpui::px;

    #[test]
    fn scroll_y_matches_gpui_convention() {
        let ch = px(20.0);
        let history = 10;

        assert_eq!(
            TerminalScrollHandle::display_offset_for_scroll_y(px(0.0), history, ch),
            history
        );
        assert_eq!(
            TerminalScrollHandle::display_offset_for_scroll_y(-ch * 10.0, history, ch),
            0
        );
        assert_eq!(
            TerminalScrollHandle::scroll_y_for_display_offset(history, history, ch),
            px(0.0)
        );
        assert_eq!(
            TerminalScrollHandle::scroll_y_for_display_offset(0, history, ch),
            -ch * 10.0
        );
    }
}
