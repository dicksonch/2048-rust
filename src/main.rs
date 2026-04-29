//! Entry point for the twenty48 application.

use clap::Parser;

#[derive(Parser)]
#[command(name = "twenty48", about = "2048 with Expectimax AI")]
struct Cli {
    /// Random seed for the game (default: random)
    #[arg(long)]
    seed: Option<u64>,

    /// Tile value that triggers "You Win!" screen (e.g. 2048). Omit for no win condition.
    #[arg(long)]
    win_tile: Option<u32>,
}

fn main() {
    let cli = Cli::parse();
    let seed = cli.seed.unwrap_or(rand::random());
    eprintln!("Starting game with seed: {seed}");
    let win_tile_exp = cli.win_tile.map(|v| v.ilog2() as u8);
    if let Err(e) = twenty48::tui::run(seed, win_tile_exp) {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
