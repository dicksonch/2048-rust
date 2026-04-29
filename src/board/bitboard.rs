//! 2048 bitboard: encoding, cell/row accessors, board utilities, display, and move
//! engine.
//!
//! # Layout
//!
//! The board is a `u64` where each of the 16 cells occupies exactly 4 bits (one nibble).
//! Cell `(r, c)` (row `r`, column `c`, both 0-indexed from top-left) lives at
//! bit position `4 * (4*r + c)` through `4 * (4*r + c) + 3`.
//!
//! So row `r` occupies bits `16*r` through `16*r + 15`, and each row is a `u16`
//! with cells packed left-to-right in nibbles 0..3.
//!
//! Each nibble stores a **log2 exponent**: 0 = empty cell, n = tile value 2^n.
//!
//! Example board (visual):
//! ```text
//! [ 2,  0,  4,  0 ]   →  row 0: nibbles [1, 0, 2, 0]  → u16 = 0x0201
//! [ 0,  8,  0, 16 ]   →  row 1: nibbles [0, 3, 0, 4]  → u16 = 0x4030
//! [ 0,  0,  0,  0 ]   →  row 2: nibbles [0, 0, 0, 0]  → u16 = 0x0000
//! [ 0,  0,  0,  0 ]   →  row 3: nibbles [0, 0, 0, 0]  → u16 = 0x0000
//! ```

use super::tables::{MOVE_LEFT, MOVE_RIGHT, SCORE_LEFT, SCORE_RIGHT};

/// A 2048 board packed as 16 4-bit nibbles in a `u64`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Board(pub u64);

/// A cell value: log2 exponent (0 = empty, n = tile 2^n).
pub type Cell = u8;

/// Four cells packed into a `u16` (nibble per cell), used for table indexing.
pub type Row = u16;

/// A move score (sum of merged tile values).
pub type Score = u32;

/// Direction for player moves.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// All four directions, in a fixed order.
    pub const ALL: [Direction; 4] = [
        Direction::Up,
        Direction::Down,
        Direction::Left,
        Direction::Right,
    ];
}

/// The empty board (all cells zero).
pub const EMPTY_BOARD: Board = Board(0);

impl Board {
    /// Bit shift for cell `(r, c)`.
    ///
    /// Cell `(r, c)` lives in bits `[shift, shift+3]` of the board `u64`.
    /// Layout: `shift = 4 * (4*r + c)`.
    #[inline]
    pub const fn cell_shift(r: usize, c: usize) -> u32 {
        4 * (4 * r as u32 + c as u32)
    }

    /// Read the cell at `(r, c)` as a log2 exponent.
    ///
    /// Returns 0 for an empty cell, `n` for tile value `2^n`.
    #[inline]
    pub fn get_cell(self, r: usize, c: usize) -> Cell {
        debug_assert!(r < 4 && c < 4, "cell indices out of range");
        ((self.0 >> Board::cell_shift(r, c)) & 0xF) as Cell
    }

    /// Return the tile value at `(r, c)` as a human-readable number.
    ///
    /// Returns 0 for an empty cell, otherwise `2^exp` where `exp` is the
    /// log2 exponent stored in the cell.
    #[inline]
    pub fn tile_value(self, r: usize, c: usize) -> u32 {
        1u32 << self.get_cell(r, c)
    }

    /// Return a new board with cell `(r, c)` set to `v` (log2 exponent).
    ///
    /// Clears the existing nibble at `(r, c)` and ORs in the new value.
    #[inline]
    pub fn set_cell(self, r: usize, c: usize, v: Cell) -> Board {
        debug_assert!(r < 4 && c < 4, "cell indices out of range");
        debug_assert!(v <= 15, "cell value exceeds nibble range");
        let shift = Board::cell_shift(r, c);
        Board((self.0 & !(0xF_u64 << shift)) | ((v as u64 & 0xF) << shift))
    }

    /// Extract row `r` (0..4) as a `u16` containing four 4-bit cells.
    ///
    /// Row `r` occupies bits `16*r` through `16*r + 15`.
    /// Within the `u16`, column `c` is in nibble `c` (bits `4c..4c+3`).
    ///
    /// Example: row 0 with cells `[1, 0, 2, 0]` → `0x0201`.
    #[inline]
    pub fn get_row(self, r: usize) -> Row {
        debug_assert!(r < 4, "row index out of range");
        ((self.0 >> (16 * r)) & 0xFFFF) as Row
    }

    /// Return a new board with row `r` replaced by `row`.
    ///
    /// Clears the 16 bits for row `r` and ORs in the new row value.
    #[inline]
    pub fn set_row(self, r: usize, row: Row) -> Board {
        debug_assert!(r < 4, "row index out of range");
        let shift = 16 * r;
        Board((self.0 & !(0xFFFF_u64 << shift)) | ((row as u64) << shift))
    }

    /// Transpose the board: swap rows and columns.
    ///
    /// After transposing, the value that was at `(r, c)` is now at `(c, r)`.
    #[inline]
    pub fn transpose(self) -> Board {
        // Step 1: Transpose each 2x2 block, a1 fixed, swap a2 and a3
        let a1 = self.0 & 0xF0F0_0F0F_F0F0_0F0F_u64;
        let a2 = self.0 & 0x0000_F0F0_0000_F0F0_u64;
        let a3 = self.0 & 0x0F0F_0000_0F0F_0000_u64;
        let a = a1 | (a2 << 12) | (a3 >> 12);

        // Step 2: Transpose 2x2 blocks, b1 fixed, swap b2 and b3
        let b1 = a & 0xFF00_FF00_00FF_00FF_u64;
        let b2 = a & 0x00FF_00FF_0000_0000_u64;
        let b3 = a & 0x0000_0000_FF00_FF00_u64;
        Board(b1 | (b2 >> 24) | (b3 << 24))
    }

    /// Returns a 64-bit mask where the low bit of each nibble is set
    /// iff the corresponding board cell is empty. All other bits are 0.
    ///
    /// Concretely, bit `4*i` is 1 iff cell `i` (in row-major order) is empty.
    #[inline]
    fn empty_mask(self) -> u64 {
        let b = self.0;
        let any_bit_set = b | (b >> 1) | (b >> 2) | (b >> 3);
        !any_bit_set & 0x1111_1111_1111_1111
    }

    /// Count the number of empty cells on the board.
    #[inline]
    pub fn count_empties(self) -> u32 {
        self.empty_mask().count_ones()
    }

    /// Iterator over all empty cell positions `(row, col)`, in row-major order.
    #[inline]
    pub fn empties(self) -> impl Iterator<Item = (usize, usize)> {
        let mut mask = self.empty_mask();
        std::iter::from_fn(move || {
            if mask == 0 {
                return None;
            }
            // Lowest set bit is at position 4*i for some cell index i.
            let bit_pos = mask.trailing_zeros() as usize;
            mask &= mask - 1; // clear that bit
            let i = bit_pos / 4;
            Some((i / 4, i % 4))
        })
    }

    /// Return the maximum cell value (log2 exponent) on the board.
    #[inline]
    pub fn max_cell(self) -> Cell {
        let mut max_value = 0u8;
        let mut x = self.0;
        for _ in 0..16 {
            max_value = max_value.max((x & 0xF) as u8);
            x >>= 4;
        }
        max_value
    }

    /// Construct a board from a 4×4 array of log2 exponents.
    ///
    /// `cells[r][c]` is the exponent for cell `(r, c)`.
    /// This is a test helper; production code should use `EMPTY_BOARD` and `set_cell`.
    #[allow(clippy::needless_range_loop)]
    pub fn from_array(cells: [[Cell; 4]; 4]) -> Board {
        let mut b = EMPTY_BOARD;
        for r in 0..4 {
            for c in 0..4 {
                b = b.set_cell(r, c, cells[r][c]);
            }
        }
        b
    }

    /// Decompose a board into a 4×4 array of log2 exponents.
    ///
    /// Inverse of `from_array`.
    #[allow(clippy::needless_range_loop)]
    pub fn to_array(self) -> [[Cell; 4]; 4] {
        let mut cells = [[0u8; 4]; 4];
        for r in 0..4 {
            for c in 0..4 {
                cells[r][c] = self.get_cell(r, c);
            }
        }
        cells
    }

    // -----------------------------------------------------------------------
    // Move engine
    // -----------------------------------------------------------------------

    /// Apply a left slide to every row, returning `(new_board, score)`.
    #[inline]
    fn move_left(self) -> (Board, Score) {
        let ml = &*MOVE_LEFT;
        let sl = &*SCORE_LEFT;
        let mut b = self;
        let mut score: Score = 0;
        for r in 0..4 {
            let row = b.get_row(r) as usize;
            b = b.set_row(r, ml[row]);
            score += sl[row];
        }
        (b, score)
    }

    /// Apply a right slide to every row, returning `(new_board, score)`.
    #[inline]
    fn move_right(self) -> (Board, Score) {
        let mr = &*MOVE_RIGHT;
        let sr = &*SCORE_RIGHT;
        let mut b = self;
        let mut score: Score = 0;
        for r in 0..4 {
            let row = b.get_row(r) as usize;
            b = b.set_row(r, mr[row]);
            score += sr[row];
        }
        (b, score)
    }

    /// Apply an up slide (each column slides toward row 0).
    ///
    /// Transpose so that columns become rows, slide left, then transpose back.
    #[inline]
    fn move_up(self) -> (Board, Score) {
        let t = self.transpose();
        let (moved, score) = t.move_left();
        (moved.transpose(), score)
    }

    /// Apply a down slide (each column slides toward row 3).
    ///
    /// Transpose, slide right, transpose back.
    #[inline]
    fn move_down(self) -> (Board, Score) {
        let t = self.transpose();
        let (moved, score) = t.move_right();
        (moved.transpose(), score)
    }

    /// Slide all tiles in direction `dir`.
    ///
    /// Returns `Some((new_board, score_gained))` if the board changed, or `None`
    /// if the move is illegal (the board is already packed in that direction).
    #[inline]
    pub fn slide(self, dir: Direction) -> Option<(Board, Score)> {
        let (new_board, score) = match dir {
            Direction::Left => self.move_left(),
            Direction::Right => self.move_right(),
            Direction::Up => self.move_up(),
            Direction::Down => self.move_down(),
        };
        if new_board != self {
            Some((new_board, score))
        } else {
            None
        }
    }

    /// Returns `true` if at least one direction produces a board change.
    pub fn has_moves(self) -> bool {
        Direction::ALL.into_iter().any(|d| self.slide(d).is_some())
    }

    /// Returns all directions that produce a legal (board-changing) move.
    pub fn legal_moves(self) -> Vec<Direction> {
        Direction::ALL
            .into_iter()
            .filter(|&d| self.slide(d).is_some())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // from_array / to_array, get_row / set_row
    // -----------------------------------------------------------------------

    #[test]
    fn from_array_to_array_round_trip() {
        let arr: [[Cell; 4]; 4] = [[15, 14, 13, 12], [8, 9, 10, 11], [7, 6, 5, 4], [0, 1, 2, 3]];
        assert_eq!(Board::to_array(Board::from_array(arr)), arr);
        // Empty array case
        assert_eq!(Board::from_array([[0u8; 4]; 4]), EMPTY_BOARD);
        assert_eq!(EMPTY_BOARD, Board(0));
    }

    #[test]
    fn get_and_set_row() {
        let b = Board::from_array([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 0]]);
        assert_eq!(b.get_row(0), 0x4321_u16);
        assert_eq!(b.get_row(1), 0x8765_u16);
        assert_eq!(b.get_row(2), 0xCBA9_u16);
        assert_eq!(b.get_row(3), 0x0FED_u16);

        let row0 = b.get_row(0);
        let b2 = EMPTY_BOARD.set_row(0, row0);
        assert_eq!(b2.get_row(0), row0);
        assert_eq!(b2.get_row(1), 0);
        assert_eq!(b2.get_row(2), 0);
        assert_eq!(b2.get_row(3), 0);
    }

    // -----------------------------------------------------------------------
    // transpose
    // -----------------------------------------------------------------------

    #[test]
    fn transpose_correctness() {
        let b = Board::from_array([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 0]]);
        let t = b.transpose();
        let expected =
            Board::from_array([[1, 5, 9, 13], [2, 6, 10, 14], [3, 7, 11, 15], [4, 8, 12, 0]]);
        assert_eq!(t, expected);
    }

    // -----------------------------------------------------------------------
    // count_empty + max_cell
    // -----------------------------------------------------------------------

    #[test]
    fn count_empty_cases() {
        assert_eq!(EMPTY_BOARD.count_empties(), 16);
        let one_tile = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert_eq!(one_tile.count_empties(), 15);
        let two_tiles = Board::from_array([[1, 0, 0, 0], [2, 0, 0, 0], [3, 0, 0, 0], [4, 0, 0, 5]]);
        assert_eq!(two_tiles.count_empties(), 11);
        let full = Board::from_array([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [1, 2, 3, 4]]);
        assert_eq!(full.count_empties(), 0);
    }

    #[test]
    fn max_cell_cases() {
        assert_eq!(EMPTY_BOARD.max_cell(), 0);
        let b = Board::from_array([[1, 2, 3, 4], [5, 6, 7, 8], [9, 10, 11, 12], [13, 14, 15, 0]]);
        assert_eq!(b.max_cell(), 15);
        for v in 1u8..=15 {
            let b = EMPTY_BOARD.set_cell(2, 2, v);
            assert_eq!(b.max_cell(), v, "max_cell with single tile value {v}");
        }
    }

    // -----------------------------------------------------------------------
    // empties
    // -----------------------------------------------------------------------

    #[test]
    fn empties_counts_correct() {
        let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 2]]);
        let empty: Vec<_> = b.empties().collect();
        assert_eq!(empty.len(), 14);
    }

    // -----------------------------------------------------------------------
    // slide tests — canonical move cases (table-driven)
    // -----------------------------------------------------------------------

    /// A slide case: `input` slid in `dir` yields `expected` board and `score`.
    struct SlideCase {
        input: [[Cell; 4]; 4],
        dir: Direction,
        expected: [[Cell; 4]; 4],
        score: u32,
    }

    #[test]
    fn slide_cases() {
        let cases = vec![
            // Left — basic merge
            SlideCase {
                input: [[1, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Left,
                expected: [[2, 2, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 8,
            },
            // Left — chain merge [2,2,4,4] → [4,8,0,0]
            SlideCase {
                input: [[1, 1, 2, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Left,
                expected: [[2, 3, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 12,
            },
            // Left — middle merge [4,2,2,4] → [4,4,4,0]
            SlideCase {
                input: [[2, 1, 1, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Left,
                expected: [[2, 2, 2, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 4,
            },
            // Left — spaced merge [2,0,2,4] → [4,4,0,0]
            SlideCase {
                input: [[1, 0, 1, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Left,
                expected: [[2, 2, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 4,
            },
            // Left — single tile slide
            SlideCase {
                input: [[0, 0, 0, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Left,
                expected: [[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 0,
            },
            // Left — triple merge rightmost [2,2,2,0] → [4,2,0,0]
            SlideCase {
                input: [[1, 1, 1, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Left,
                expected: [[2, 1, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 4,
            },
            // Right — spaced merge
            SlideCase {
                input: [[0, 1, 0, 1], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Right,
                expected: [[0, 0, 0, 2], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 4,
            },
            // Up — single tile move
            SlideCase {
                input: [[0, 0, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Up,
                expected: [[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 0,
            },
            // Up — column merge
            SlideCase {
                input: [[1, 0, 0, 0], [1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                dir: Direction::Up,
                expected: [[2, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]],
                score: 4,
            },
            // Down — column merge
            SlideCase {
                input: [[0, 0, 0, 0], [0, 0, 0, 0], [1, 0, 0, 0], [1, 0, 0, 0]],
                dir: Direction::Down,
                expected: [[0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [2, 0, 0, 0]],
                score: 4,
            },
        ];

        for c in &cases {
            let (new_b, score) = Board::from_array(c.input).slide(c.dir).unwrap();
            let expected = Board::from_array(c.expected);
            assert_eq!(
                new_b, expected,
                "failed for case: {:?} {:?}",
                c.input, c.dir
            );
            assert_eq!(
                score, c.score,
                "score mismatch for case: {:?} {:?}",
                c.input, c.dir
            );
        }
    }

    #[test]
    fn illegal_move_returns_none() {
        // Single tile at top-left: Left and Up are illegal
        let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert!(b.slide(Direction::Left).is_none());
        assert!(b.slide(Direction::Up).is_none());
        // Right and Down are legal
        assert!(b.slide(Direction::Right).is_some());
        assert!(b.slide(Direction::Down).is_some());
    }

    // -----------------------------------------------------------------------
    // has_any_move / legal_moves
    // -----------------------------------------------------------------------

    #[test]
    fn has_move_and_terminal() {
        // Single tile at top-left: has moves, not terminal
        let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        assert!(b.has_moves());

        // Full board with no adjacent equal tiles: terminal
        let full_no_merge =
            Board::from_array([[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 1]]);
        assert!(!full_no_merge.has_moves());

        // Full board with one possible merge: has moves
        let full_merge =
            Board::from_array([[1, 2, 1, 2], [2, 1, 2, 1], [1, 2, 1, 2], [2, 1, 2, 2]]);
        assert!(full_merge.has_moves());
    }

    #[test]
    fn legal_moves_on_sparse_board() {
        let b = Board::from_array([[1, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0], [0, 0, 0, 0]]);
        let moves = b.legal_moves();
        assert!(moves.contains(&Direction::Right));
        assert!(moves.contains(&Direction::Down));
        assert_eq!(moves.len(), 2);
    }

    /// Up/Down via transpose + left/right equivalence.
    #[test]
    fn up_down_via_transpose() {
        let b = Board::from_array([[0, 0, 0, 0], [1, 0, 0, 0], [2, 0, 0, 0], [0, 0, 0, 0]]);
        let up_result = b.slide(Direction::Up).unwrap().0;
        let t = b.transpose();
        let (left, _) = t.slide(Direction::Left).unwrap();
        assert_eq!(up_result, left.transpose());

        let down_result = b.slide(Direction::Down).unwrap().0;
        let t = b.transpose();
        let (right, _) = t.slide(Direction::Right).unwrap();
        assert_eq!(down_result, right.transpose());
    }
}
