//! Application state machine for the 2048 TUI.
//!
//! `App` holds all mutable UI state: the current game, display mode, hint, undo
//! stack, and any transient message. `App` does not interact with the terminal;
//! that is the responsibility of `tui::mod`.

use crate::board::bitboard::{Board, Direction, Score};
use crate::game::Game;

/// Autoplay / human-play mode selector.
#[derive(Clone, Debug, PartialEq)]
pub enum Mode {
    /// Human is in control.
    Human,
    /// AI plays automatically (one move per ~50 ms tick).
    Auto,
}

/// Result of an AI hint request.
#[derive(Clone, Debug)]
pub enum HintInfo {
    /// A direction is suggested.
    Move { dir: Direction, depth: u8 },
    /// No moves are possible (board is terminal).
    NoMoves,
}

/// Full application state.
#[derive(Debug)]
pub struct App {
    /// Current game state.
    pub game: Game,
    /// Best score seen across resets this session.
    pub best_score: Score,
    /// Current play mode (human or AI).
    pub mode: Mode,
    /// Last computed AI hint, if any.
    pub hint: Option<HintInfo>,
    /// Undo stack: (board, score) pairs, capped at 64 entries.
    pub history: Vec<(Board, Score)>,
    /// Transient status message displayed in the info panel.
    pub message: Option<String>,
    /// The seed used for the current game (for display).
    pub seed: u64,
    /// Winning tile as log2 exponent (e.g. 2048 -> 11).
    /// `None` means no win condition — play continues indefinitely.
    pub win_tile_exp: Option<u8>,
}

impl App {
    /// Create a new `App` with the given seed, starting in human mode.
    /// `win_tile` is the winning tile as a log2 exponent (e.g. 2048 → `Some(11)`);
    /// `None` disables the win screen.
    pub fn new(seed: u64, win_tile_exp: Option<u8>) -> Self {
        Self {
            game: Game::new(seed),
            best_score: 0,
            mode: Mode::Human,
            hint: None,
            history: Vec::new(),
            message: None,
            seed,
            win_tile_exp,
        }
    }

    /// Attempt a move in the given direction.
    ///
    /// If legal: pushes current (board, score) to the history stack, applies the
    /// move, updates best_score, and clears any hint.
    /// If illegal: sets a brief message and does nothing else.
    pub fn try_move(&mut self, dir: Direction) {
        // Save state for undo
        let snapshot = (self.game.board, self.game.score);
        match self.game.play(dir) {
            Ok(_) => {
                if self.history.len() >= 64 {
                    self.history.remove(0);
                }
                self.history.push(snapshot);
                if self.game.score > self.best_score {
                    self.best_score = self.game.score;
                }
                self.hint = None;
                self.message = None;
            }
            Err(_) => {
                self.message = Some(format!("Illegal move: {dir:?}"));
            }
        }
    }

    /// Undo the last move, if any history is available.
    pub fn undo(&mut self) {
        if let Some((board, score)) = self.history.pop() {
            self.game.board = board;
            self.game.score = score;
            self.hint = None;
            self.message = Some("Undone".to_string());
        } else {
            self.message = Some("Nothing to undo".to_string());
        }
    }

    /// Reset the game with the same seed, clearing history and hints.
    pub fn reset(&mut self) {
        self.game = Game::new(self.seed);
        self.history.clear();
        self.hint = None;
        self.message = Some("Game reset".to_string());
    }

    /// Toggle between Human and Auto mode.
    pub fn toggle_auto(&mut self) {
        self.mode = match &self.mode {
            Mode::Human => Mode::Auto,
            Mode::Auto => Mode::Human,
        };
        self.hint = None;
    }

    /// Store a computed hint and clear any transient message.
    pub fn set_hint(&mut self, hint: HintInfo) {
        self.hint = Some(hint);
        self.message = None;
    }

    /// Return `true` if the current game is in a terminal state.
    pub fn is_game_over(&self) -> bool {
        !self.game.board.has_moves()
    }

    /// Return `true` if the player has reached (or exceeded) the winning tile.
    ///
    /// Returns `false` if `win_tile` is `None` (no win condition configured).
    pub fn is_game_won(&self) -> bool {
        if let Some(target_exp) = self.win_tile_exp {
            self.game.board.max_cell() >= target_exp
        } else {
            false
        }
    }

    /// Return configured winning tile as real value (e.g. 11 -> 2048).
    pub fn win_tile_value(&self) -> Option<u32> {
        self.win_tile_exp.map(|exp| 1u32 << exp)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn undo_restores_board_and_score() {
        let mut app = App::new(1, None);
        let board_before = app.game.board;
        let score_before = app.game.score;
        // Find a legal move
        let legal = app.game.board.legal_moves();
        assert!(!legal.is_empty());
        app.try_move(legal[0]);
        // State should have changed
        // (board may or may not change depending on the move, but history was pushed)
        app.undo();
        assert_eq!(app.game.board, board_before);
        assert_eq!(app.game.score, score_before);
    }

    #[test]
    fn illegal_move_does_not_push_history() {
        let mut app = App::new(0, None);
        // Single tile at top-left; Left and Up are illegal
        app.game.board =
            Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let history_len_before = app.history.len();
        app.try_move(Direction::Left);
        assert_eq!(app.history.len(), history_len_before);
    }

    #[test]
    fn reset_clears_history_and_hint() {
        let mut app = App::new(5, None);
        // Push something
        let legal = app.game.board.legal_moves();
        app.try_move(legal[0]);
        assert!(!app.history.is_empty());
        app.set_hint(HintInfo::Move {
            dir: Direction::Left,
            depth: 1,
        });
        app.reset();
        assert!(app.history.is_empty());
        assert!(app.hint.is_none());
    }
}
