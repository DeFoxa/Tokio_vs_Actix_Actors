#![allow(warnings)]
mod client;
mod concurrency_setup;
pub mod models;
mod ob_model;
pub mod schema;
pub mod types;
mod utils;

use crate::concurrency_setup::actix_actor_model::*;
use crate::concurrency_setup::tokio_actor_model::{
    MatchingEngineActor as MEA, SequencerActor as SA, SequencerMessage as SM,
    TradeStreamActor as TSA, TradeStreamMessage as TSM,
};
use diesel::prelude::*;
use diesel::PgConnection;
use dotenvy::dotenv;
use schema::binancetrades::dsl::*;
use std::collections::BinaryHeap;
use std::env;
use tracing_subscriber::Layer;
use tracing_subscriber::{filter::EnvFilter, fmt, prelude::*, registry::Registry};

use crate::{
    client::{ws::*, ws_types::*},
    types::*,
    utils::*,
};
use actix::prelude::*;
use actix_rt::{task::spawn_blocking, Arbiter, System};
use anyhow::Result;
use concurrency_setup::*;
use futures_util::{Stream, StreamExt};
use models::BinanceTradesNewModel;
use serde_json::Value;
// use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, oneshot};
use tokio_tungstenite::tungstenite::Message;
use tracing_flame::FlameLayer;
//TODO: Fix the clones in actor_model

pub const MAINNET: &str = "wss://fstream.binance.com";

// #[actix_rt::main]
#[tokio::main]
async fn main() -> Result<()> {
    // let file_appender =
    //     tracing_appender::rolling::minutely(".logs", "concurrency_model_testing.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // tracing_subscriber::fmt().with_writer(non_blocking).init();
    let matching = MEA::new(SequencerMessage::TakerTrade);
    let (trade_sender, trade_receiver) = mpsc::channel(32);
    let (seq_sender, seq_receiver) = mpsc::channel(32);
    let seq_actor = SA::new(seq_receiver, matching);
    tokio::spawn(async move { seq_actor.run().await });

    Ok(())
}

fn establish_connection() -> PgConnection {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL not found, must be set");
    PgConnection::establish(&database_url)
        .unwrap_or_else(|_| panic!("error connecting to DB: {}", database_url))
}

// #[tokio::main]
// async fn _main() -> Result<()> {
//     Ok(())
// }

//
//Combined connection book
//

async fn book_data_to_db() -> Result<()> {
    let pool = create_db_pool();
    let db_actor = TradeStreamDBActor { pool: pool.clone() }.start();
    let book_db_actor = BookModelDbActor { pool: pool }.start();

    let (mut ws_state, Response) = Client::connect_combined_async(
        MAINNET,
        vec![
            &StreamNameGenerator::combined_stream_partial_book("ethusdt", "10").await,
            &StreamNameGenerator::combined_stream_trades_by_symbol("ethusdt").await,
        ],
    )
    .await?;

    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let other_event_type = value.get("e").and_then(Value::as_str);
                match other_event_type {
                    Some("aggTrade") => {
                        let trades = serde_json::from_value::<BinanceTrades>(value.clone())
                            .expect("error deserializing to binancetrades");

                        db_actor.try_send(TradeStreamDBMessage {
                            data: trades.to_db_model(),
                        });
                        // println!("{:?}", &trades);
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                        book_db_actor.try_send(BookModelDbMessage {
                            data: book.to_db_model(),
                        });
                        // println!("{:?}", &book);
                    }
                    _ => {
                        eprintln!("Error matching deserialized fields, no aggTrade or depthUpdate");
                    }
                }
            }
            _ => (),
        }
    })
    .await;
    Ok(())
}

//
// COMBINED
//

async fn stream_data_to_matching_engine() -> Result<()> {
    let matching_engine_addr = MatchingEngineActor { data: 1 }.start();
    let seq_addr = SequencerActor {
        queue: BinaryHeap::new(),
        matching_engine_addr,
        last_ob_update: Instant::now(),
        is_processing_paused: false,
    }
    .start();

    let trade_stream_actor = TradeStreamActor {
        sequencer_addr: seq_addr.clone(),
    }
    .start();
    let book_actor = BookModelStreamActor {
        sequencer_addr: seq_addr,
    }
    .start();

    let (mut ws_state, Response) = Client::connect_combined_async(
        MAINNET,
        vec![
            &StreamNameGenerator::combined_stream_partial_book("ethusdt", "10").await,
            &StreamNameGenerator::combined_stream_trades_by_symbol("ethusdt").await,
        ],
    )
    .await?;

    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let other_event_type = value.get("e").and_then(Value::as_str);
                match other_event_type {
                    Some("aggTrade") => {
                        let trades = serde_json::from_value::<BinanceTrades>(value.clone())
                            .expect("error deserializing to binancetrades");

                        trade_stream_actor.do_send(TradeStreamMessage { data: trades });
                        // println!("{:?}", &trades);
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                        book_actor.do_send(BookModelStreamMessage { data: book });
                        // println!("{:?}", &book);
                    }
                    _ => {
                        eprintln!("Error matching deserialized fields, no aggTrade or depthUpdate");
                    }
                }
            }
            _ => (),
        }
    })
    .await;
    Ok(())
}
//single ws connection
async fn trade_stream_connection() -> Result<()> {
    let (mut ws_state, Response) =
        Client::connect_with_stream_name(&StreamNameGenerator::trades_by_symbol("btcusdt").await)
            .await?;
    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let trades: BinanceTrades = serde_json::from_value(value).expect("some error 2");
                //NOTE: Add message handling below or return trades as Result
                // db_actor.do_send(TradeStreamDBMessage {
                //     data: trades.to_db_model(),
                // });
            }
            _ => (),
        }
    })
    .await;
    Ok(())
}
async fn book_stream_connection() -> Result<()> {
    let (mut ws_state, Response) =
        Client::connect_with_stream_name(&StreamNameGenerator::partial_book("ethusdt", "10").await)
            .await?;
    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let book: BinancePartialBook = serde_json::from_value(value).expect("some error 2");

                //NOTE: Add message handling below or return trades as Result
                // db_actor.do_send(TradeStreamDBMessage {
                //     data: trades.to_db_model(),
                // });
            }
            _ => (),
        }
    })
    .await;
    Ok(())
}
// async fn inst_sequence_addr() -> Result<SequenceActor> {
//     let matching_engine_addr = MatchingEngineActor { data: 1 }.start();
//      let sequencer_addr = SequencerActor {
//         queue: BinaryHeap::new(),
//         matching_engine_addr,
//         last_ob_update: Instant::now(),
//         is_processing_paused: false,
//     }.start();
//      Ok(sequence_addr)
// }
// DATABASE TESTING CODE
// let test = BinanceTradesModel {
//        id: 1,
//        event_type: Some("testing".to_string()),
//        event_time: Some(1234),
//        symbol: Some("TEST_BTC".to_string()),
//        aggegate_id: Some(1),
//        price: Some(46999.01),
//        quantity: Some(1.0),
//        first_trade_id: Some(2),
//        last_trade_id: Some(4),
//        trade_timestamp: Some(56789),
//        is_buyer_mm: Some(false),
//    };
//    let connection = &mut establish_connection;
//    let entry = diesel::insert_into(binancetrades)
//        .values(test)
//        .execute(&mut connection())?;
//

// Tracing Flamegraph implementation for use in fn main
// let fmt_layer: tracing_subscriber::fmt::Layer<Registry> = fmt::Layer::default();
// let (flame_layer, _guard) = FlameLayer::with_file("./tracing.folded").unwrap();
// tracing_subscriber::registry()
//     .with(fmt::layer())
//     .with(flame_layer)
//     .init();
//

// Actix actor model main implementation below
// swap code below into main or keep actix function that is called from main
// async fn actix() -> result<()> {
// let addr = TradeStreamActor.start();
//     let (mut ws_state, Response) =
//         Client::connect_with_stream_name(&StreamNameGenerator::trades_by_symbol("btcusdt").await)
//             .await?;
//     let (write, read) = ws_state.socket.split();
//     read.for_each(|message| async {
//         match message {
//             Ok(Message::Text(text)) => {
//                 let value: Value = serde_json::from_str(&text).expect("some error 1");
//                 let trades: BinanceTrades = serde_json::from_value(value).expect("some error 2");
//                 addr.try_send(TradeStreamMessage { data: trades });
//             }
//             _ => (),
//         }
//     })
//     .await;
//     // system.run().unwrap();
//     Ok(())
//
// }
