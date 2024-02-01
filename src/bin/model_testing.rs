#![allow(warnings)]
use diesel::{prelude::*, PgConnection};
use dotenvy::dotenv;
use lib::concurrency_setup::actix_actor_model::*;
use lib::concurrency_setup::tokio_actor_model::TradeStreamActorHandler;
use lib::concurrency_setup::tokio_actor_model::{
    MatchingEngineActor as MEA, MatchingEngineHandler, MatchingEngineMessage,
    OrderBookActorHandler, OrderBookStreamMessage as OBSM, SequencerActor as SA, SequencerHandler,
    SequencerMessage as SM, StateManagementMessage, TradeStreamActor as TSA,
    TradeStreamMessage as TSM,
};
use lib::schema::binancetrades::dsl::*;
use std::{collections::BinaryHeap, env};
use tracing_subscriber::Layer;
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*, registry::Registry};

use actix::prelude::*;
use actix_rt::{task::spawn_blocking, Arbiter, System};
use anyhow::Result;
use futures_util::{Stream, StreamExt};
use lib::models::BinanceTradesNewModel;
use lib::{
    client::{ws::*, ws_types::*},
    types::*,
    utils::*,
};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tracing_flame::FlameLayer;

/// NOTE: Frameworks to include: Critereon for benchmarking, fuschia stress testing, goose load testing(maybe?)

pub const MAINNET: &str = "wss://fstream.binance.com";

// General framework for testing,
// Read stream data (ob/trade) into memory prior to testing. Use criterion to iterate through x number of bench functions iterations.
// Each function call processes the data in some subset of time, whereby we can have control over
// load on message system, averaging msg_passed/ms, to test for backpressure issues. Increase load until
// failure point (determine what qualifies as failure). Another method to stress test: add more
// active stream connections (i.e. more incoming data) until system runs into quantifiable issues.
// write end-point verification into matching engine.
// Produce bad data to integrate into test data-set, and write custom functions to test
// sequencer state management/changes. After all of these are passed, write live testing/data
// verifications methods and grafana metric integration and test with live data. keep adding
// new streams until the system is at capacity.
//
#[tokio::main]
async fn main() -> Result<()> {
    let file_appender = tracing_appender::rolling::minutely(
        ".logs/tokio_model_logs",
        "concurrency_model_testing.log",
    );

    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    tracing_subscriber::fmt().with_writer(non_blocking).init();

    Ok(())
}
