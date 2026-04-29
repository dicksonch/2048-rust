//! Terminal setup/teardown and the main event loop.
//!
//! Entry point: `run(seed, win_tile)`.
//!
//! # AI integration
//! Pressing `H` calls `ai::best_move`.
//! In Auto mode, `ai::best_move` is called on each loop iteration.

use std::error::Error;
use std::io;
use std::time::Duration;

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

use crate::ai::best_move as ai_search;
use crate::board::bitboard::{Board, Direction};
use crate::board::tables::warm_tables;
use crate::tui::app::{App, HintInfo, Mode};
use crate::tui::theme::warm_styles;
use crate::tui::ui;

// ---------------------------------------------------------------------------
// Input — key bindings
// ---------------------------------------------------------------------------

/// Actions returned by the input handler to drive the event loop.
#[derive(Debug, PartialEq)]
enum Action {
    /// No action needed.
    None,
    /// Quit the application.
    Quit,
    /// Move in the given direction.
    Move(Direction),
    /// Request an AI hint.
    RequestHint,
    /// Undo last move.
    Undo,
    /// Reset the game.
    Reset,
    /// Toggle autoplay mode.
    ToggleAuto,
    /// Terminal was resized; redraw with new dimensions.
    Resize,
}

/// Translate a terminal event into an `Action` intent.
///
/// Pure translation — no side effects. The event loop performs all state
/// changes by dispatching the returned `Action`.
fn handle_input(event: Event) -> Action {
    match event {
        Event::Resize(_, _) => Action::Resize,
        Event::Key(key) => handle_key(key),
        _ => Action::None,
    }
}

/// Pure key → action mapping. No application state is touched here.
fn handle_key(key: KeyEvent) -> Action {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => Action::Quit,
        KeyCode::Left => Action::Move(Direction::Left),
        KeyCode::Right => Action::Move(Direction::Right),
        KeyCode::Up => Action::Move(Direction::Up),
        KeyCode::Down => Action::Move(Direction::Down),
        KeyCode::Char('H') => Action::RequestHint,
        KeyCode::Char('A') => Action::ToggleAuto,
        KeyCode::Char('u') => Action::Undo,
        KeyCode::Char('r') => Action::Reset,
        _ => Action::None,
    }
}

// ---------------------------------------------------------------------------
// Event loop
// ---------------------------------------------------------------------------

/// Run the TUI. Always starts in human mode; press `A` to toggle autoplay.
///
/// `seed` is passed to `App::new` with the optional `win_tile_exp`.
/// `win_tile_exp` is the winning tile as a log2 exponent (e.g. 2048 → `Some(11)`);
/// `None` disables the win condition.
pub fn run(seed: u64, win_tile_exp: Option<u8>) -> Result<(), Box<dyn Error>> {
    warm_tables();
    warm_styles();

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = run_loop(&mut terminal, seed, win_tile_exp);

    // Terminal teardown (always, even on error)
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen,)?;
    terminal.show_cursor()?;

    result
}

/// Inner event loop (separate so teardown is guaranteed).
fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    seed: u64,
    win_tile_exp: Option<u8>,
) -> Result<(), Box<dyn Error>> {
    let mut app = App::new(seed, win_tile_exp);

    // Poll timeout — also defines the auto-play step interval.
    let poll_timeout = Duration::from_millis(50);

    loop {
        // Draw frame
        terminal.draw(|f| ui::render(f, &app))?;

        // Poll for input
        if event::poll(poll_timeout)? {
            let ev = event::read()?;
            let action = handle_input(ev);

            match action {
                Action::Resize => terminal.autoresize()?,
                Action::Quit => break,
                Action::Move(dir) if app.mode == Mode::Human => app.try_move(dir),
                Action::RequestHint => {
                    app.set_hint(compute_move(app.game.board));
                }
                Action::ToggleAuto => app.toggle_auto(),
                Action::Undo => app.undo(),
                Action::Reset => app.reset(),
                Action::Move(_) => {} // ignore manual moves in auto mode
                Action::None => {}
            }
        }

        // Auto-play: step on each loop iteration
        if app.mode == Mode::Auto && !app.is_game_over() && !app.is_game_won() {
            match compute_move(app.game.board) {
                HintInfo::Move { dir, .. } => app.try_move(dir),
                HintInfo::NoMoves => {} // board is terminal, loop will exit naturally
            }
        }
    }

    Ok(())
}

/// Run the AI search and return a hint.
fn compute_move(board: Board) -> HintInfo {
    let result = ai_search(board);
    match result.best_move {
        Some(dir) => HintInfo::Move {
            dir,
            depth: result.depth_reached,
        },
        None => HintInfo::NoMoves,
    }
}
