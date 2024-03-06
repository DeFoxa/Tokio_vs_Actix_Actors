#![allow(unused)]
use actix::Actor;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use dotenvy::dotenv;
use futures_util::StreamExt;
use lib::concurrency_setup::actix_actor_model::*;
use lib::concurrency_setup::tokio_actor_model::TradeStreamActorHandler;
use lib::concurrency_setup::tokio_actor_model::{
    MatchingEngineActor as MEA, MatchingEngineHandler, MatchingEngineMessage,
    OrderBookActorHandler, OrderBookStreamMessage as OBSM, SequencerActor as SA, SequencerHandler,
    SequencerMessage as SM, StateManagementMessage, TradeStreamActor as TSA,
    TradeStreamMessage as TSM,
};
use lib::schema::binancetrades::dsl::*;
use serde_json::Value;
use tungstenite::Message;
// use lib::{schema::binancetrades, types::*};
use anyhow::Result;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
    utils::*,
};

use serde::Deserialize;
use std::env;
use std::io::stdout;

use crossterm::{
    event::{self, Event::Key, KeyCode::Char, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use ratatui::{
    // prelude::{CrosstermBackend, Frame, Stylize, Terminal},
    prelude::*,
    widgets::Paragraph,
};

pub const MAINNET: &str = "wss://fstream.binance.com";

fn main() -> Result<()> {
    start_up()?;
    let status = run();
    shutdown()?;
    status?;
    Ok(())
}

struct App {
    counter: i64,
    should_quit: bool,
}

fn ui(app: &App, frame: &mut Frame) {
    // frame.render_widget(
    //     Paragraph::new(format!("Counter: {}", app.counter)),
    //     frame.size(),
    // );

    let layout: Rect = Layout::default()
        .direction(Direction::Vertical)
        .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(frame.size());
    frame.render_widget(
        Paragraph::new(
            format!("Counter: {}", app.counter).block(Block::new().borders(Borders::ALL)),
        ),
        layout,
    );
    // for i in 0..10 {
    //     let area = Rect::new(0, i, frame.size().width, 3);
    //     frame.render_widget(Paragraph::new(format!("Counter: {}", app.counter)), area);
    // }
}

fn update(app: &mut App) -> Result<()> {
    if event::poll(std::time::Duration::from_millis(250))? {
        if let Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press {
                match key.code {
                    Char('j') => app.counter += 1,
                    Char('k') => app.counter -= 1,
                    Char('q') => app.should_quit = true,
                    _ => {}
                }
            }
        }
    }

    Ok(())
}

fn run() -> Result<()> {
    let mut t = Terminal::new(CrosstermBackend::new(std::io::stderr()))?;
    let mut app = App {
        counter: 0,
        should_quit: false,
    };

    loop {
        t.draw(|f| ui(&app, f))?;
        update(&mut app);
        if app.should_quit {
            break;
        }
    }
    Ok(())
}
fn start_up() -> Result<()> {
    enable_raw_mode()?;
    execute!(std::io::stderr(), EnterAlternateScreen)?;
    Ok(())
}
fn shutdown() -> Result<()> {
    execute!(std::io::stderr(), LeaveAlternateScreen)?;
    disable_raw_mode()?;
    Ok(())
}
