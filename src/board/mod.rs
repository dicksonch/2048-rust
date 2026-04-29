//! Board module: bitboard representation, move engine, and display.

pub mod bitboard;
pub mod tables;

pub use bitboard::{Board, Cell, Direction, Row, EMPTY_BOARD};