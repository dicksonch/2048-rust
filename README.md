# twenty48

A terminal-based 2048 game with an Expectimax AI, written in Rust.

The game uses a bitboard representation (a single `u64` with 16 nibbles) for maximum performance. The AI uses expectimax search with iterative deepening, a transposition table, and probability pruning.

## How to build

```bash
cargo build --release
```

## How to run

```bash
# Play interactively (human control, optional AI hint with 'H')
cargo run --release

# Play with a specific seed
cargo run --release -- --seed 42

# Set a win-condition tile (e.g. 4096). Omit for no win condition.
cargo run --release -- --win-tile 4096
```

## Controls

| Key     | Action                                          |
| ------- | ----------------------------------------------- |
| ← ↑ → ↓ | Move tiles                                      |
| `H`     | Toggle AI hint (shows best move & search depth) |
| `A`     | Toggle autoplay (AI plays automatically)        |
| `u`     | Undo last move                                  |
| `r`     | Reset game                                      |
| `q`     | Quit                                            |

## CLI Options

| Flag             | Description                                                                        |
| ---------------- | ---------------------------------------------------------------------------------- |
| `--seed <N>`     | Random seed for the game (default: random)                                         |
| `--win-tile <N>` | Tile value that triggers "You Win!" screen (e.g. 2048). Omit for no win condition. |

## Architecture

- **Board:** `u64` with 16 4-bit nibbles (log<sub>2</sub> tile values). Cell `(r,c)` at bit `4·(4r+c)`.
- **Row tables:** Precomputed 65536-entry `LazyLock` tables for left/right moves and scores.
- **Search:** Expectimax with iterative deepening, transposition table, and probability pruning. Max nodes consider all legal directions; chance nodes average over tile spawns (2 or 4) at each empty cell, weighted by probability.
