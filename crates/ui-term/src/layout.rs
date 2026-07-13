//! Terminal pointer interaction: layout cache, scroll bar, coordinate mapping.

use alacritty_terminal::index::{Column, Line, Point as AlacPoint};
use alacritty_terminal::term::viewport_to_point;
use crate::scroll::SCROLLBAR_TRACK_WIDTH;
use gpui::{Bounds, Pixels, Point, px};

/// Width reserved so cell grid does not sit under the overlay scrollbar.
pub const SCROLLBAR_WIDTH: Pixels = SCROLLBAR_TRACK_WIDTH;

/// Last measured terminal content geometry (updated each paint).
#[derive(Clone, Debug, Default)]
pub struct TerminalLayout {
    pub bounds: Bounds<Pixels>,
    pub origin: Point<Pixels>,
    pub cell_width: Pixels,
    pub cell_height: Pixels,
    pub cols: usize,
    pub rows: usize,
    pub content_width: Pixels,
    pub content_height: Pixels,
}

impl TerminalLayout {
    pub fn is_valid(&self) -> bool {
        self.cell_width > px(0.0) && self.cell_height > px(0.0) && self.cols > 0 && self.rows > 0
    }

    /// Map a window position to a viewport-relative grid point, if inside the grid.
    pub fn pixel_to_viewport(&self, position: Point<Pixels>) -> Option<AlacPoint> {
        if !self.is_valid() {
            return None;
        }
        if position.x < self.origin.x
            || position.y < self.origin.y
            || position.x >= self.origin.x + self.content_width
            || position.y >= self.origin.y + self.content_height
        {
            return None;
        }

        let col = ((position.x - self.origin.x) / self.cell_width)
            .floor()
            .max(0.0) as usize;
        let row = ((position.y - self.origin.y) / self.cell_height)
            .floor()
            .max(0.0) as i32;

        if col >= self.cols || row < 0 || row as usize >= self.rows {
            return None;
        }

        Some(AlacPoint::new(Line(row), Column(col)))
    }

    pub fn viewport_to_grid(&self, display_offset: usize, viewport: AlacPoint) -> AlacPoint {
        viewport_to_point(
            display_offset,
            alacritty_terminal::index::Point::new(viewport.line.0 as usize, viewport.column),
        )
    }

}
