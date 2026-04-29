//! Tile spawn logic for 2048.
//!
//! After each successful move, one empty cell is chosen uniformly at random
//! and filled with tile 2 (log2 = 1, probability `P_TILE_2`) or tile 4
//! (log2 = 2, probability `P_TILE_4`).

use crate::board::{Board, Cell};
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Probability of spawning tile 2 (value 1) versus tile 4 (value 2).
pub const P_TILE_2: f64 = 0.5;
/// Probability of spawning tile 4 (value 2) — derived from `P_TILE_2`.
pub const P_TILE_4: f64 = 1.0 - P_TILE_2;

/// Attempt to spawn a new tile on an empty cell.
///
/// Picks an empty cell uniformly at random from all empty positions, then places:
/// - value `1` (tile 2) with probability `P_TILE_2`, or
/// - value `2` (tile 4) with probability `P_TILE_4`.
///
/// Returns `Some((new_board, (row, col), value))` or `None` if the board is full.
pub fn spawn(board: Board, rng: &mut ChaCha8Rng) -> Option<(Board, (usize, usize), Cell)> {
    let empty_cells: Vec<(usize, usize)> = board.empties().collect();
    if empty_cells.is_empty() {
        return None;
    }
    let idx = rng.gen_range(0..empty_cells.len());
    let (r, c) = empty_cells[idx];
    let value: Cell = if rng.gen::<f64>() < P_TILE_2 { 1 } else { 2 };
    let new_board = board.set_cell(r, c, value);
    Some((new_board, (r, c), value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::board::EMPTY_BOARD;
    use rand::SeedableRng;

    #[test]
    fn spawn_on_empty_board_succeeds() {
        let mut rng = ChaCha8Rng::seed_from_u64(42);
        let result = spawn(EMPTY_BOARD, &mut rng);
        assert!(result.is_some());
        let (board, _pos, val) = result.unwrap();
        assert!(val == 1 || val == 2);
        // Board now has exactly one non-empty cell
        assert_eq!(board.count_empties(), 15);
    }

    #[test]
    fn spawn_on_full_board_returns_none() {
        // Fill every cell
        let full = Board::from_array([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [1, 2, 3, 4]]);
        let mut rng = ChaCha8Rng::seed_from_u64(1);
        assert!(spawn(full, &mut rng).is_none());
    }

    #[test]
    fn spawn_probability_matches_p_tile_4() {
        let mut rng = ChaCha8Rng::seed_from_u64(0);
        let mut fours = 0usize;
        let n = 100_000;
        for _ in 0..n {
            let (_, _, val) = spawn(EMPTY_BOARD, &mut rng).unwrap();
            if val == 2 {
                fours += 1;
            }
        }
        let rate = fours as f64 / n as f64;
        // Should be ~P_TILE_4; allow ±1%
        assert!(
            (rate - P_TILE_4).abs() < 0.01,
            "4-tile rate was {rate}, expected {}",
            P_TILE_4
        );
    }

    #[test]
    fn empties_counts_correct() {
        let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 2]]);
        let empty: Vec<_> = b.empties().collect();
        assert_eq!(empty.len(), 14);
    }
}
