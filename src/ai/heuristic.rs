//! 2-component board evaluation for 2048:
//!
//! 1. **Empty-cell utility** — A 17-entry lookup table indexed by empty-cell
//!    count (0..=16). Values follow a halving-the-gap series up to 200:
//!
//!    ```text
//!    n:   0    1    2    3    4    5    6    7    8    9+
//!    v:   0  100  150  175  188  194  197  198  199  200
//!    ```
//!
//!    Scaled by `W_EMPTY = 25_000` so range is 0–5 M.
//!
//! 2. **Snake score** — Best-of-8 D₄ orientation of a row-major zigzag path.
//!    The canonical snake traverses the 4×4 board like a typewriter with
//!    alternating row direction:
//!
//!    ```text
//!    ┌────┬────┬────┬────┐   Each cell contributes
//!    │  0 │  1 │  2 │  3 │   `2^(tile_exp + weight_exp)`
//!    ├────┼────┼────┼────┤   where weight_exp = 15 - position
//!    │  7 │  6 │  5 │  4 │   so the head (pos 0) gets
//!    ├────┼────┼────┼────┤   the heaviest weight 2¹⁵.
//!    │  8 │  9 │ 10 │ 11 │
//!    ├────┼────┼────┼────┤   All 8 D₄ symmetries are tried
//!    │ 15 │ 14 │ 13 │ 12 │   and the maximum is taken.
//!    └────┴────┴────┴────┘
//!    ```
//!
//!    Values are powers of two throughout — no multiplication or f32 tables.
//!
//! Final: `evaluate(board) = empty_utility + best_snake_score`.
use crate::board::Board;

/// Floating-point score produced by the board evaluation function.
pub type Eval = f64;

// ---------------------------------------------------------------------------
// Empty-cell utility lookup (17 entries, n = 0 ..= 16)
// ---------------------------------------------------------------------------

/// Scale factor for empty-cell utility (`EMPTY_UTIL` max × W_EMPTY = 5M).
pub const W_EMPTY: u64 = 25_000;

/// Halving-the-gap utility values (0..=200) indexed by empty-cell count.
static EMPTY_UTIL: [u64; 17] = [
    0,   // 0
    100, // 1
    150, // 2
    175, // 3
    188, // 4
    194, // 5
    197, // 6
    198, // 7
    199, // 8
    200, // 9+
    200, 200, 200, 200, 200, 200, 200,
];

// ---------------------------------------------------------------------------
// Snake weight exponents: 8 D₄ orientations × 16 cells
// ---------------------------------------------------------------------------
//
// weight(k) = 2^WEIGHT_EXPS[orientation][cell_index].
// Canonical snake (top-left, row-major zigzag):
//   pos:  0  1  2  3   4  5  6  7   8  9 10 11  12 13 14 15
//   exp: 15 14 13 12   8  9 10 11   7  6  5  4   0  1  2  3
// Remaining 7 orientations are rotations / flips / transposes of this.
const WEIGHT_EXPS: [[u8; 16]; 8] = [
    // 0: top-left, row-major
    [15, 14, 13, 12, 8, 9, 10, 11, 7, 6, 5, 4, 0, 1, 2, 3],
    // 1: top-right, row-major (horizontal flip of 0)
    [12, 13, 14, 15, 11, 10, 9, 8, 4, 5, 6, 7, 3, 2, 1, 0],
    // 2: bottom-left, row-major (vertical flip of 0)
    [0, 1, 2, 3, 7, 6, 5, 4, 8, 9, 10, 11, 15, 14, 13, 12],
    // 3: bottom-right, row-major (180° of 0)
    [3, 2, 1, 0, 4, 5, 6, 7, 11, 10, 9, 8, 12, 13, 14, 15],
    // 4: top-left, column-major (transpose of 0)
    [15, 8, 7, 0, 14, 9, 6, 1, 13, 10, 5, 2, 12, 11, 4, 3],
    // 5: top-right, column-major (horizontal flip of 4)
    [0, 7, 8, 15, 1, 6, 9, 14, 2, 5, 10, 13, 3, 4, 11, 12],
    // 6: bottom-left, column-major (vertical flip of 4)
    [12, 11, 4, 3, 13, 10, 5, 2, 14, 9, 6, 1, 15, 8, 7, 0],
    // 7: bottom-right, column-major (180° of 4)
    [3, 4, 11, 12, 2, 5, 10, 13, 1, 6, 9, 14, 0, 7, 8, 15],
];

// ---------------------------------------------------------------------------
// Public evaluation function
// ---------------------------------------------------------------------------

/// Evaluate `board`: higher = better for the player.
pub fn evaluate(board: Board) -> Eval {
    // Empty utility: one table lookup, one integer multiply.
    let empty_score = W_EMPTY * EMPTY_UTIL[board.count_empties() as usize];

    // Snake score: best orientation of 2^(tile_exp + weight_exp) via shift.
    let mut best_snake = 0u64;
    for weights in &WEIGHT_EXPS {
        let mut score = 0u64;
        for (i, &weight_exp) in weights.iter().enumerate() {
            let tile_exp = board.get_cell(i / 4, i % 4);
            if tile_exp > 0 {
                // tile_exp ∈ [1,15], weight_exp ∈ [0,15], sum ≤ 30 — fits u64
                score += 1u64 << (tile_exp as u32 + weight_exp as u32);
            }
        }
        if score > best_snake {
            best_snake = score;
        }
    }

    (empty_score + best_snake) as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::EMPTY_BOARD;

    #[test]
    fn evaluate_empty_board_is_finite_and_positive() {
        let v = evaluate(EMPTY_BOARD);
        assert!(v.is_finite(), "non-finite on empty board");
        assert_eq!(v, 5_000_000.0, "unexpected empty board value: {v}");
    }

    #[test]
    fn evaluate_terminal_board_no_panic() {
        let b = Board::from_array([[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]]);
        assert!(evaluate(b).is_finite());
    }

    #[test]
    fn snake_board_scores_higher_than_random_shuffle() {
        let snake = Board::from_array([[11, 10, 9, 8], [5, 6, 7, 4], [3, 2, 1, 0], [0, 0, 0, 0]]);
        let shuffled =
            Board::from_array([[5, 3, 9, 0], [10, 0, 6, 8], [1, 11, 2, 0], [7, 0, 4, 0]]);
        assert!(
            evaluate(snake) > evaluate(shuffled),
            "snake={} shuffled={}",
            evaluate(snake),
            evaluate(shuffled)
        );
    }

    #[test]
    fn mirror_snake_boards_score_equally() {
        let s0 = Board::from_array([[8, 7, 6, 5], [1, 2, 3, 4], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let s1 = Board::from_array([[5, 6, 7, 8], [4, 3, 2, 1], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert!(
            (evaluate(s0) - evaluate(s1)).abs() < 1.0,
            "v0={} v1={}",
            evaluate(s0),
            evaluate(s1)
        );
    }

    #[test]
    fn max_tile_at_corner_beats_center() {
        let corner = Board::from_array([[11, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let center = Board::from_array([[0, 0, 0, 0], [0, 11, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let diff = evaluate(corner) - evaluate(center);
        assert!(diff > 60_000_000.0, "expected diff > 60M, got {diff}");
    }

    #[test]
    fn empty_util_is_non_decreasing() {
        for n in 0..16 {
            assert!(
                EMPTY_UTIL[n] <= EMPTY_UTIL[n + 1],
                "not non-decreasing at n={n}"
            );
        }
    }
}
