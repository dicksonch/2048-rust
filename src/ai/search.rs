//! Optimized expectimax search for 2048.
//!
//! The game tree alternates between **max nodes** (player chooses a move) and
//! **chance nodes** (the game spawns a random tile). This module implements a
//! depth-limited expectimax search with three optimizations: iterative deepening,
//! a transposition table, and probability pruning.
//!
//! # Game tree structure
//!
//! ```text
//!  Max node     ← player picks a direction (up / down / left / right)
//!  ├─ Chance    ← new tile spawns: pick empty cell × tile value (2 / 4)
//!  │  ├─ Max    ← opponent (nature) chooses nothing, but
//!  │  │  ⋮           the player's next move is a new max node
//!  │  └─ Max
//!  ├─ Chance
//!  │  ├─ Max
//!  │  │  ⋮
//!  │  └─ Max
//!  └─ … (other directions)
//! ```
//!
//! At a **max node** we try every legal direction, apply the slide, add the
//! immediate score gained, then recurse into a chance node at `depth - 1`.
//! We return the direction with the highest expected value.
//!
//! At a **chance node** we iterate over all empty cells, spawning tile 2
//! (log₂ value = 1, probability `P_TILE_2` = 0.5) or tile 4 (log₂ = 2,
//! probability `P_TILE_4` = 0.5) at each one. The result is the probability-
//! weighted average of the downstream max nodes.
//!
//! # Iterative deepening
//!
//! `best_move` runs the search repeatedly at increasing depths (1, 2, …, up to
//! `MAX_DEPTH` = 10). After each completed depth the transposition table is
//! cleared and the best move / value from that depth is recorded. If the time
//! budget (`TIME_BUDGET_MS` = 20 ms) is exhausted, the loop exits and returns the
//! result from the **last fully-completed** depth. This guarantees a reasonable
//! move even when interrupted.
//!
//! # Transposition table
//!
//! A `HashMap<(Board, Depth), Eval>` caches computed values for max nodes.
//! Before recursing at a max node we probe the table: on hit we return the
//! cached value without expanding the subtree. On miss we compute, then store.
//!
//! **Chance nodes are NOT cached** because their value depends on the cumulative
//! probability along the path (used for pruning). The same board at the same
//! depth can yield different values depending on how likely it was to reach.
//!
//! The table is cleared at the **start** of each iterative-deepening depth to
//! keep memory bounded, not at the end (so earlier work within the same depth
//! benefits siblings).
//!
//! # Probability pruning
//!
//! Each chance-node call receives the cumulative probability of reaching that
//! node. If that probability drops below `PROB_THRESHOLD` (0.001) the node is
//! not expanded — we return the static heuristic immediately. This cuts off
//! branches that contribute negligible expected value while preserving the
//! overall score.
//!
//! # Leaf evaluation
//!
//! When search reaches `depth = 0` or a terminal board, the static heuristic
//! [`evaluate`] is returned. See [`crate::ai::heuristic`] for the two-component
//! formula (empty-cell utility + best snake score).
//!

use crate::ai::heuristic::{evaluate, Eval};
use crate::ai::transposition::TranspositionTable;
use crate::board::{Board, Direction};
use crate::game::spawn::{P_TILE_2, P_TILE_4};
use std::time::Instant;

/// Search depth measured in plies.
pub type Depth = u8;

// ---------------------------------------------------------------------------
// Global search configuration — tuned for ~20ms per move on a mid-game board.
// ---------------------------------------------------------------------------

/// Time budget in milliseconds.
const TIME_BUDGET_MS: u32 = 20;
/// Hard cap on search depth in plies.
const MAX_DEPTH: Depth = 10;
/// Skip chance branches below this cumulative probability.
const PROB_THRESHOLD: f64 = 0.001;

// ---------------------------------------------------------------------------
// Result types
// ---------------------------------------------------------------------------

/// Result returned by the top-level search.
pub struct SearchResult {
    /// Best move found, or `None` if the board is terminal.
    pub best_move: Option<Direction>,
    /// Deepest completed search depth.
    pub depth_reached: Depth,
}

/// Top-level search: iterative deepening with time budget.
pub fn best_move(board: Board) -> SearchResult {
    if !board.has_moves() {
        return SearchResult {
            best_move: None,
            depth_reached: 0,
        };
    }

    let start = Instant::now();
    let mut tt = TranspositionTable::new();
    let mut best_dir = None;
    let mut depth_reached = 0u8;

    for depth in 1..=MAX_DEPTH {
        if TIME_BUDGET_MS > 0 && start.elapsed().as_millis() as u32 >= TIME_BUDGET_MS {
            break;
        }
        tt.clear();
        let (dir, _val) = search_max(board, depth, 1.0, &mut tt, true);

        if let Some(d) = dir {
            best_dir = Some(d);
            depth_reached = depth;
        }
    }

    SearchResult {
        best_move: best_dir,
        depth_reached,
    }
}

/// Max node: choose the direction maximizing expected value. Uses TT.
///
/// When `is_root` is true, also returns the best direction and skips the
/// transposition table (root nodes are never revisited at the same depth).
fn search_max(
    board: Board,
    depth: Depth,
    prob: f64,
    tt: &mut TranspositionTable,
    is_root: bool,
) -> (Option<Direction>, Eval) {
    if depth == 0 || !board.has_moves() {
        return (None, evaluate(board));
    }
    if !is_root {
        // Probe TT (skip for root — (board, depth) is unique at the root)
        if let Some(cached) = tt.get(board, depth) {
            return (None, cached);
        }
    }
    let mut best_dir = None;
    let mut best = 0.0f64;
    for &dir in &Direction::ALL {
        if let Some((new_board, score_gained)) = board.slide(dir) {
            let val = score_gained as f64 + search_chance(new_board, depth - 1, prob, tt);
            if val > best {
                best = val;
                best_dir = Some(dir);
            }
        }
    }
    if !is_root {
        tt.insert(board, depth, best);
    }
    (best_dir, best)
}

/// Chance node: weighted average over all empty-cell spawns with probability pruning.
fn search_chance(board: Board, depth: Depth, prob: f64, tt: &mut TranspositionTable) -> Eval {
    if depth == 0 {
        return evaluate(board);
    }
    if PROB_THRESHOLD > 0.0 && prob < PROB_THRESHOLD {
        return evaluate(board);
    }
    let n_empty = board.count_empties();
    if n_empty == 0 {
        return search_max(board, depth, prob, tt, false).1;
    }
    let p_each = 1.0 / n_empty as f64;
    let p2 = P_TILE_2 * p_each;
    let p4 = P_TILE_4 * p_each;

    // Cumulative probability for deeper pruning (spread across tile types).
    let pp2 = prob * p2;
    let pp4 = prob * p4;

    let mut sum = 0.0f64;
    for (r, c) in board.empties() {
        let b2 = board.set_cell(r, c, 1);
        let b4 = board.set_cell(r, c, 2);
        // Spawn probability weights are unconditional at this level.
        // cum_prob flows deeper only for pruning — search_max returns an
        // unweighted expected value (all downstream scaling is contained
        // inside its own chance nodes).
        sum += p2 * search_max(b2, depth - 1, pp2, tt, false).1;
        sum += p4 * search_max(b4, depth - 1, pp4, tt, false).1;
    }
    sum
}
