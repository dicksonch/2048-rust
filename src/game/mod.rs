//! Game module: game state, move application, and spawn logic.

pub mod spawn;
pub mod state;

pub use state::{Game, IllegalMove};
