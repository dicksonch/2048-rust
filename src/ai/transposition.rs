//! Transposition table for expectimax search.

use crate::ai::heuristic::Eval;
use crate::ai::search::Depth;
use crate::board::Board;
use std::collections::HashMap;

/// Transposition table mapping `(board, depth)` to computed expected value.
pub struct TranspositionTable {
    map: HashMap<(Board, Depth), Eval>,
}

impl TranspositionTable {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Look up a cached value for `(board, depth)`.
    pub fn get(&self, board: Board, depth: Depth) -> Option<Eval> {
        self.map.get(&(board, depth)).copied()
    }

    /// Store a computed value for `(board, depth)`.
    pub fn insert(&mut self, board: Board, depth: Depth, value: Eval) {
        self.map.insert((board, depth), value);
    }

    /// Clear all cached entries (call between top-level searches).
    pub fn clear(&mut self) {
        self.map.clear();
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
}
