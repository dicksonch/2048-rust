//! ratatui rendering for 2048.
//!
//! `render(frame, app)` draws the full UI into the given `Frame`.

use ratatui::{
    layout::{Alignment, Constraint, Direction as LayoutDir, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::tui::app::HintInfo::*;
use crate::tui::app::{App, HintInfo, Mode};
use crate::tui::theme::tile_style;

/// Render the full TUI into `frame`.
pub fn render(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let rows = Layout::default()
        .direction(LayoutDir::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(10),
            Constraint::Length(1),
        ])
        .split(area);

    render_title(frame, app, rows[0]);

    let cols = Layout::default()
        .direction(LayoutDir::Horizontal)
        .constraints([Constraint::Length(30), Constraint::Min(20)])
        .split(rows[1]);

    render_board(frame, app, cols[0]);
    render_info(frame, app, cols[1]);
    render_help(frame, rows[2]);
}

fn render_title(frame: &mut Frame, app: &App, area: Rect) {
    let mode_tag = match &app.mode {
        Mode::Human => "",
        Mode::Auto => "  [AUTO]",
    };
    let title = format!(" 2048  seed: {} {} ", app.seed, mode_tag);
    let p = Paragraph::new(title).style(
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    );
    frame.render_widget(p, area);
}

fn render_board(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default().borders(Borders::ALL).title(" Board ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let cell_height = inner.height / 4;
    let cell_width = inner.width / 4;
    if cell_height == 0 || cell_width == 0 {
        return;
    }

    for r in 0..4_usize {
        for c in 0..4_usize {
            let cell_area = Rect {
                x: inner.x + (c as u16) * cell_width,
                y: inner.y + (r as u16) * cell_height,
                width: cell_width,
                height: cell_height,
            };
            let exp = app.game.board.get_cell(r, c);
            let style = tile_style(exp);
            let val = app.game.board.tile_value(r, c);
            let val_str = if val == 0 {
                String::new()
            } else {
                val.to_string()
            };

            let cell_block = Block::default().borders(Borders::ALL).style(style);
            let cell_inner = cell_block.inner(cell_area);
            frame.render_widget(cell_block, cell_area);

            let text_line = Line::from(Span::styled(val_str, style.add_modifier(Modifier::BOLD)));
            let p = Paragraph::new(text_line).alignment(Alignment::Center);
            frame.render_widget(p, cell_inner);
        }
    }
}

/// Format the current hint as a display string (or placeholder).
fn hint_text(hint: &Option<HintInfo>) -> String {
    match hint {
        None => "--".to_string(),
        Some(Move { dir, depth }) => format!("{:?} (d{})", dir, depth),
        Some(NoMoves) => "No moves possible".to_string(),
    }
}

/// Build status lines for the info panel (game-over / game-won messages).
fn status_lines(app: &App) -> Vec<Line<'static>> {
    let style = Style::default()
        .fg(Color::LightRed)
        .add_modifier(Modifier::BOLD);
    if app.is_game_won() {
        let win_tile = app.win_tile_value().unwrap_or(2048);
        vec![
            Line::from(Span::styled(format!(" Game won at {}!", win_tile), style)),
            Line::from(Span::styled(" Please restart.".to_string(), style)),
        ]
    } else if app.is_game_over() {
        vec![
            Line::from(Span::styled(" Game over.".to_string(), style)),
            Line::from(Span::styled(" Please restart.".to_string(), style)),
        ]
    } else {
        Vec::new()
    }
}

fn render_info(frame: &mut Frame, app: &App, area: Rect) {
    let mode_text = match &app.mode {
        Mode::Human => "Human",
        Mode::Auto => "Auto",
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" Score: {}", app.game.score),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::raw(format!(" Best:  {}", app.best_score))),
        Line::from(Span::raw(format!(" Moves: {}", app.game.moves))),
        Line::from(""),
        Line::from(Span::raw(format!(" Mode:  {}", mode_text))),
        Line::from(""),
        Line::from(Span::styled(
            format!(" Hint:  {}", hint_text(&app.hint)),
            Style::default().fg(Color::Cyan),
        )),
    ];

    // Append message and status lines
    let status = status_lines(app);
    let has_message = app.message.is_some();
    if has_message || !status.is_empty() {
        lines.push(Line::from(""));
    }
    if let Some(msg) = &app.message {
        lines.push(Line::from(Span::styled(
            format!(" {}", msg),
            Style::default().fg(Color::Yellow),
        )));
    }
    lines.extend(status);

    let block = Block::default().borders(Borders::ALL).title(" Info ");
    let p = Paragraph::new(lines).block(block);
    frame.render_widget(p, area);
}

fn render_help(frame: &mut Frame, area: Rect) {
    let help = " arrows: move  H: hint  A: auto  u: undo  r: reset  q: quit ";
    let p = Paragraph::new(help).style(Style::default().fg(Color::DarkGray));
    frame.render_widget(p, area);
}
