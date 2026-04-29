//! Color theme for 2048 tile values.
//!
//! Maps a log2 cell exponent (0 = empty, 1 = tile 2, …, 15 = tile 32768)
//! to a ratatui `Style`. Background colors are interpolated from
//! light yellow (exponent 1) to deep red-orange (exponent ≥ 15).
//!
//! # Gradient
//!
//! Start: rgb(255, 255, 128) — light yellow   (tile 2)
//! End:   rgb(220,  20,   0) — deep red-orange (tile 32768)
//!
//! For exponent n in 1..=15 (k = n-1):
//!   R = 255 - 35k / 14
//!   G = 255 - 235k / 14
//!   B = 128 - 128k / 14
//!
//! Text is Black for n ≤ 4 (bright backgrounds) and White for n ≥ 5 (dark backgrounds).
//!
//! Styles are precomputed once at first access via `std::sync::LazyLock`,
//! consistent with `board::tables`.

use crate::board::bitboard::Cell;
use ratatui::style::{Color, Style};
use std::sync::LazyLock;

/// Start color channels — tile 2.
const S_R: u8 = 255;
const S_G: u8 = 255;
const S_B: u8 = 128;
/// End color channels — tile 32768.
const E_R: u8 = 220;
const E_G: u8 = 20;
const E_B: u8 = 0;

/// Precomputed tile styles for cells 0..=15.
static STYLES: LazyLock<[Style; 16]> =
    LazyLock::new(|| std::array::from_fn(|i| compute_style(i as u8)));

/// Force initialisation of the style table.
///
/// Call once at startup (alongside `board::tables::warm_tables`) so the
/// one-time integer division happens deterministically rather than during rendering.
pub fn warm_styles() {
    let _ = &*STYLES;
}

/// Return the display `Style` for a cell with the given log2 exponent.
///
/// `cell == 0` means empty; `cell == n` means tile value `2^n`.
///
/// This is an O(1) lookup into a precomputed table — no floating-point
/// arithmetic at render time.
pub fn tile_style(cell: Cell) -> Style {
    STYLES[cell as usize]
}

/// Compute the style for a single cell exponent (used at table-build time only).
fn compute_style(cell: Cell) -> Style {
    match cell {
        0 => Style::default()
            .fg(Color::DarkGray)
            .bg(Color::Rgb(40, 40, 40)),
        n @ 1..=14 => {
            // Integer lerp: S - (S - E) * k / 14   (k = n - 1)
            let k = (n - 1) as i32;
            let r = S_R as i32 - ((S_R as i32 - E_R as i32) * k) / 14;
            let g = S_G as i32 - ((S_G as i32 - E_G as i32) * k) / 14;
            let b = S_B as i32 - ((S_B as i32 - E_B as i32) * k) / 14;
            let bg = Color::Rgb(r as u8, g as u8, b as u8);
            let fg = if n <= 4 { Color::Black } else { Color::White };
            Style::default().fg(fg).bg(bg)
        }
        _ => Style::default(),
    }
}
