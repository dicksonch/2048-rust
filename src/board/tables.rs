//! Precomputed row move/score lookup tables.
//!
//! For every possible 16-bit row value (0..=65535), we precompute:
//! - The result of sliding all tiles left (or right).
//! - The score gained by that move (sum of merged tile values).
//!
//! Tables are built once at startup via `std::sync::LazyLock`.
//!
//! # Row encoding
//!
//! A `Row` (`u16`) holds four 4-bit cells: cell 0 in bits 0..3 (column 0 = leftmost),
//! cell 1 in bits 4..7, cell 2 in bits 8..11, cell 3 in bits 12..15.
//! Each nibble is a log2 exponent (0 = empty, n = tile 2^n).
//!
//! # Move semantics (left)
//!
//! 1. **Compact:** shift all non-zero cells to the left, preserving order.
//! 2. **Merge once:** scan left to right; if two adjacent cells are equal, merge
//!    them into one cell with value `v+1` (i.e., double the tile) and clear the
//!    right cell. A cell produced by merging cannot merge again.
//! 3. **Compact again:** shift non-zero cells left to fill the gap.

use super::bitboard::{Row, Score};
use std::sync::LazyLock;

/// Result of sliding every possible row left.
pub static MOVE_LEFT: LazyLock<[Row; 65536]> = LazyLock::new(build_move_left);
/// Result of sliding every possible row right.
pub static MOVE_RIGHT: LazyLock<[Row; 65536]> = LazyLock::new(build_move_right);
/// Score gained by sliding each row left.
pub static SCORE_LEFT: LazyLock<[Score; 65536]> = LazyLock::new(build_score_left);
/// Score gained by sliding each row right.
pub static SCORE_RIGHT: LazyLock<[Score; 65536]> = LazyLock::new(build_score_right);

/// Force initialization of all row tables.
///
/// Call this from `main` before starting the TUI so that startup latency is
/// deterministic and does not spike during the first move.
pub fn warm_tables() {
    let _ = &*MOVE_LEFT;
    let _ = &*MOVE_RIGHT;
    let _ = &*SCORE_LEFT;
    let _ = &*SCORE_RIGHT;
}

// ---------------------------------------------------------------------------
// Internal: compute a single left-move on a row given as four nibbles.
// Returns (result_row, score_gained).
// ---------------------------------------------------------------------------

/// Extract the four nibbles of a row into a fixed-size array.
///
/// `cells[0]` = leftmost (column 0), `cells[3]` = rightmost.
#[inline(always)]
fn unpack(row: Row) -> [u8; 4] {
    [
        (row & 0xF) as u8,
        ((row >> 4) & 0xF) as u8,
        ((row >> 8) & 0xF) as u8,
        ((row >> 12) & 0xF) as u8,
    ]
}

/// Pack four nibbles back into a `Row`.
#[inline(always)]
fn pack(cells: [u8; 4]) -> Row {
    (cells[0] as Row)
        | ((cells[1] as Row) << 4)
        | ((cells[2] as Row) << 8)
        | ((cells[3] as Row) << 12)
}

/// Apply a left slide to four cells, returning (result_cells, score).
///
/// Algorithm:
/// 1. Compact: collect non-zero cells into a contiguous prefix.
/// 2. Merge: scan left to right; merge adjacent equal cells once.
/// 3. Compact again: fill from the left.
fn slide_left_cells(cells: [u8; 4]) -> ([u8; 4], Score) {
    // Step 1: compact (remove zeros)
    let mut buf = [0u8; 4];
    let mut n = 0usize;
    for &v in &cells {
        if v != 0 {
            buf[n] = v;
            n += 1;
        }
    }

    // Step 2: merge adjacent equal cells (left to right, each cell merges at most once)
    let mut score: Score = 0;
    let mut i = 0;
    while i + 1 < n {
        if buf[i] == buf[i + 1] {
            let merged = buf[i] + 1; // log2(2^n + 2^n) = n+1
            score += 1u32 << merged; // actual tile value of the merged tile
            buf[i] = merged;
            // Shift remaining cells left to fill the gap
            for j in i + 1..n - 1 {
                buf[j] = buf[j + 1];
            }
            buf[n - 1] = 0;
            n -= 1;
            // Do NOT increment i: the merged cell cannot merge again,
            // but we advance past it naturally by i+1 in the next iter.
            i += 1;
        } else {
            i += 1;
        }
    }

    ([buf[0], buf[1], buf[2], buf[3]], score)
}

fn build_move_left() -> [Row; 65536] {
    let mut table = [0u16; 65536];
    for row in 0u32..65536 {
        let (result, _) = slide_left_cells(unpack(row as Row));
        table[row as usize] = pack(result);
    }
    table
}

fn build_score_left() -> [Score; 65536] {
    let mut table = [0u32; 65536];
    for row in 0u32..65536 {
        let (_, score) = slide_left_cells(unpack(row as Row));
        table[row as usize] = score;
    }
    table
}

/// Reverse a row (flip left↔right).
///
/// Column 0 becomes column 3, column 1 becomes column 2, etc.
#[inline]
fn reverse_row(row: Row) -> Row {
    let c = unpack(row);
    pack([c[3], c[2], c[1], c[0]])
}

fn build_move_right() -> [Row; 65536] {
    let mut table = [0u16; 65536];
    for row in 0u32..65536 {
        let rev = reverse_row(row as Row);
        let (result, _) = slide_left_cells(unpack(rev));
        table[row as usize] = reverse_row(pack(result));
    }
    table
}

fn build_score_right() -> [Score; 65536] {
    let mut table = [0u32; 65536];
    for row in 0u32..65536 {
        let rev = reverse_row(row as Row);
        let (_, score) = slide_left_cells(unpack(rev));
        table[row as usize] = score;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_move_cases() {
        // exponents: 1=tile2, 2=tile4, 3=tile8
        // [1,1,1,1] left → [2,2,0,0], score 8
        let row = pack([1, 1, 1, 1]);
        assert_eq!(unpack(MOVE_LEFT[row as usize]), [2, 2, 0, 0]);
        assert_eq!(SCORE_LEFT[row as usize], 8);

        // [1,1,2,2] left → [2,3,0,0], score 12
        let row = pack([1, 1, 2, 2]);
        assert_eq!(unpack(MOVE_LEFT[row as usize]), [2, 3, 0, 0]);
        assert_eq!(SCORE_LEFT[row as usize], 12);

        // [2,1,1,2] left → [2,2,2,0], score 4
        let row = pack([2, 1, 1, 2]);
        assert_eq!(unpack(MOVE_LEFT[row as usize]), [2, 2, 2, 0]);
        assert_eq!(SCORE_LEFT[row as usize], 4);

        // [1,0,1,2] left → [2,2,0,0], score 4
        let row = pack([1, 0, 1, 2]);
        assert_eq!(unpack(MOVE_LEFT[row as usize]), [2, 2, 0, 0]);
        assert_eq!(SCORE_LEFT[row as usize], 4);

        // [0,0,0,1] left → [1,0,0,0], score 0
        let row = pack([0, 0, 0, 1]);
        assert_eq!(unpack(MOVE_LEFT[row as usize]), [1, 0, 0, 0]);
        assert_eq!(SCORE_LEFT[row as usize], 0);

        // [1,1,1,0] left → [2,1,0,0], score 4
        let row = pack([1, 1, 1, 0]);
        assert_eq!(unpack(MOVE_LEFT[row as usize]), [2, 1, 0, 0]);
        assert_eq!(SCORE_LEFT[row as usize], 4);

        // [0,1,0,1] right → [0,0,0,2], score 4
        let row = pack([0, 1, 0, 1]);
        assert_eq!(unpack(MOVE_RIGHT[row as usize]), [0, 0, 0, 2]);
        assert_eq!(SCORE_RIGHT[row as usize], 4);
    }

    #[test]
    fn reverse_row_involution() {
        for row in (0..65536u32).step_by(7) {
            assert_eq!(reverse_row(reverse_row(row as Row)), row as Row);
        }
    }
}
