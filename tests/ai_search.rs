//! Quick unit tests for AI search behaviour.
//!
//! These are fast tests that verify basic invariants without running
//! full AI games. Use the criterion benchmarks (`cargo bench`) for
//! depth/quality regression coverage.

use twenty48::ai::best_move;
use twenty48::board::bitboard::Board;
// ---------------------------------------------------------------------------
// best_move returns Some legal move on non-terminal boards
// ---------------------------------------------------------------------------

#[test]
fn best_move_legal_on_simple_board() {
    // A simple board with enough room for expectimax to finish quickly:
    // one tile at (0,0), plenty of empty cells.
    let board = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
    let result = best_move(board);
    let dir = result.best_move;
    assert!(dir.is_some(), "best_move returned None on non-terminal board");
    let d = dir.unwrap();
    assert!(
        board.slide(d).is_some(),
        "returned direction {d:?} must be legal"
    );
}

#[test]
fn best_move_legal_on_mid_game_board() {
    // Another quick board: two tiles, still very few search nodes.
    let board = Board::from_array([[1, 2, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
    let result = best_move(board);
    let dir = result.best_move;
    assert!(dir.is_some(), "best_move returned None on non-terminal board");
    let d = dir.unwrap();
    assert!(
        board.slide(d).is_some(),
        "returned direction {d:?} must be legal"
    );
}

#[test]
fn best_move_terminal_returns_none() {
    let terminal = Board::from_array([[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]]);
    let result = best_move(terminal);
    assert!(result.best_move.is_none());
    assert_eq!(result.depth_reached, 0);
}