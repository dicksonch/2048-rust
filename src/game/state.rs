//! Game state, rules, and move application.

use crate::board::bitboard::{Board, Direction, Score};
use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;

use super::spawn::spawn as do_spawn;

/// Returned when an illegal move direction is attempted.
#[derive(Debug)]
pub struct IllegalMove;

/// Full game state: board, cumulative score, move count, and seeded RNG.
///
/// All randomness is derived from the seed given at construction time.
#[derive(Clone, Debug)]
pub struct Game {
    /// Current board state.
    pub board: Board,
    /// Cumulative score.
    pub score: Score,
    /// Number of successful moves played.
    pub moves: u32,
    rng: ChaCha8Rng,
}

impl Game {
    /// Create a new game with the given seed.
    ///
    /// Each of the 16 cells has a 50% chance of starting with a tile of value 2.
    pub fn new(seed: u64) -> Self {
        let mut rng = ChaCha8Rng::seed_from_u64(seed);
        // Keep only the lowest bit of each nibble: 0 = empty, 1 = tile 2.
        let board = rng.next_u64() & 0x1111_1111_1111_1111_u64;
        Self {
            board: Board(board),
            score: 0,
            moves: 0,
            rng,
        }
    }

    /// Apply `dir` to the current board.
    ///
    /// On success: applies the move, spawns a new tile, updates score and move
    /// counter.
    /// On failure (illegal move): returns `Err(IllegalMove)` without side effects.
    pub fn play(&mut self, dir: Direction) -> Result<(), IllegalMove> {
        let (new_board, score_gained) = self.board.slide(dir).ok_or(IllegalMove)?;
        self.board = new_board;
        self.score += score_gained;
        self.moves += 1;
        // Spawn a new tile
        if let Some((b, _, _)) = do_spawn(self.board, &mut self.rng) {
            self.board = b;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legal_moves_all_on_empty() {
        let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let moves = b.legal_moves();
        assert!(moves.contains(&Direction::Right));
        assert!(moves.contains(&Direction::Down));
        assert_eq!(moves.len(), 2);
    }
}
