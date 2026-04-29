//! AI module: heuristic evaluation and optimized search.

pub mod heuristic;
pub mod search;
pub mod transposition;

pub use search::{best_move, SearchResult};
