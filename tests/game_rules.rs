//! Integration tests for game rules: spawn, terminal detection, Game struct.

use twenty48::board::bitboard::Board;
use twenty48::board::bitboard::Direction;
use twenty48::game::Game;

// ---------------------------------------------------------------------------
// Determinism
// ---------------------------------------------------------------------------

#[test]
fn same_moves_same_outcome_across_clones() {
    let move_seq = [
        Direction::Left,
        Direction::Up,
        Direction::Right,
        Direction::Down,
        Direction::Left,
        Direction::Left,
    ];
    let mut g1 = Game::new(7);
    let mut g2 = Game::new(7);
    for &d in &move_seq {
        let r1 = g1.play(d);
        let r2 = g2.play(d);
        assert_eq!(r1.is_ok(), r2.is_ok(), "legality must agree at each step");
    }
    assert_eq!(g1.board, g2.board);
    assert_eq!(g1.score, g2.score);
    assert_eq!(g1.moves, g2.moves);
}

#[test]
fn different_seeds_give_different_boards() {
    let g1 = Game::new(1);
    let g2 = Game::new(2);
    assert_ne!(g1.board, g2.board);
}

// ---------------------------------------------------------------------------
// Initial state
// ---------------------------------------------------------------------------

#[test]
fn new_game_score_and_moves_are_zero() {
    let g = Game::new(99);
    assert_eq!(g.score, 0);
    assert_eq!(g.moves, 0);
}

#[test]
fn new_game_is_not_terminal() {
    for seed in 0..10u64 {
        let g = Game::new(seed);
        assert!(
            g.board.has_moves(),
            "fresh game should not be terminal (seed {seed})"
        );
    }
}

#[test]
fn regression_seeds_all_start_with_some_tiles() {
    for &seed in &[0, 1, 2, 3, 4] {
        let g = Game::new(seed);
        let empty = g.board.count_empties();
        assert!(
            empty > 0 && empty < 16,
            "seed {seed} has {empty} empty cells (expected some but not all)"
        );
    }
}

// ---------------------------------------------------------------------------
// Terminal detection
// ---------------------------------------------------------------------------

#[test]
fn terminal_board_detected_correctly() {
    // Checkerboard: no merges, no empty cells
    let terminal_board =
        Board::from_array([[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]]);
    let mut g = Game::new(0);
    g.board = terminal_board;
    assert!(!g.board.has_moves());
    assert!(g.board.legal_moves().is_empty());
}

#[test]
fn board_with_one_merge_is_not_terminal() {
    // Has exactly one merge available (row 0 has two adjacent 1s)
    let b = Board::from_array([[1, 1, 2, 3], [4, 5, 6, 7], [8, 9, 10, 11], [3, 7, 11, 12]]);
    let mut g = Game::new(0);
    g.board = b;
    assert!(g.board.has_moves());
    assert!(!g.board.legal_moves().is_empty());
}

// ---------------------------------------------------------------------------
// play() legality
// ---------------------------------------------------------------------------

#[test]
fn illegal_move_returns_err_and_no_side_effects() {
    // Single tile at top-left; Left and Up are illegal
    let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
    let mut g = Game::new(0);
    g.board = b;
    let score_before = g.score;
    let moves_before = g.moves;
    let board_before = g.board;

    assert!(g.play(Direction::Left).is_err());
    assert!(g.play(Direction::Up).is_err());
    assert_eq!(g.score, score_before);
    assert_eq!(g.moves, moves_before);
    assert_eq!(g.board, board_before);
}

#[test]
fn valid_move_increments_move_counter() {
    let mut g = Game::new(1);
    let moves_before = g.moves;
    // Find a legal direction
    let legal = g.board.legal_moves();
    assert!(!legal.is_empty());
    g.play(legal[0]).expect("should be legal");
    assert_eq!(g.moves, moves_before + 1);
}

#[test]
fn valid_move_changes_board() {
    let mut g = Game::new(5);
    let board_before = g.board;
    let legal = g.board.legal_moves();
    g.play(legal[0]).expect("legal");
    assert_ne!(
        g.board, board_before,
        "board must change after a legal move"
    );
}

// ---------------------------------------------------------------------------
// Score accumulation
// ---------------------------------------------------------------------------

#[test]
fn score_increases_on_merge() {
    // Build a board where Left produces a definite merge
    let b = Board::from_array([[1, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
    let mut g = Game::new(0);
    g.board = b;
    let score_before = g.score;
    g.play(Direction::Left).expect("legal");
    assert!(g.score > score_before, "score should increase after merge");
    // [1,1] → merged into [2] = 2^2 = 4; score gain = 4
    assert_eq!(g.score, score_before + 4);
}
