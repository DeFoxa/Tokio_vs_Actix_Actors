#![allow(warnings)]
mod client;
mod concurrency_setup;
mod ob_model;
mod types;
mod utils;
use crate::concurrency_setup::actor_model::*;

use crate::concurrency_setup::actor_model::{
    MatchingEngineActor, SequencerActor, TradeStreamActor, TradeStreamMessage,
};
use std::collections::BinaryHeap;

use crate::{
    client::{ws::*, ws_types::*},
    types::*,
};
use actix::prelude::*;
use actix_rt::{task::spawn_blocking, Arbiter, System};
use anyhow::Result;
use concurrency_setup::*;
use futures_util::{Stream, StreamExt};
use serde_json::Value;
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio_tungstenite::tungstenite::Message;

//TODO: Fix the clones in actor_model

pub const MAINNET: &str = "wss://fstream.binance.com";

#[actix_rt::main]
async fn main() -> Result<()> {
    // let trade_addr = TradeStreamActor {
    //     sequencer_addr: sequencer_addr,
    // }
    // .start();
    let matching_engine_addr = MatchingEngineActor { data: 1 }.start();
    let sequencer_addr = SequencerActor {
        queue: BinaryHeap::new(),
        matching_engine_addr,
        last_ob_update: Instant::now(),
        is_processing_paused: false,
    }
    .start();

    let trade_addr = TradeStreamActor {
        sequencer_addr: sequencer_addr.clone(),
    }
    .start();
    let book_addr = BookModelStreamActor {
        sequencer_addr: sequencer_addr.clone(),
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

    // let (mut ws_state, Response) =
    //     Client::connect_with_stream_name(&StreamNameGenerator::trades_by_symbol("btcusdt").await)
    //         .await?;
    let (write, read) = ws_state.socket.split();
    read.for_each(|message| async {
        match message {
            Ok(Message::Text(text)) => {
                let value: Value = serde_json::from_str(&text).expect("some error 1");
                let event_type = value.get("e").and_then(Value::as_str);
                match event_type {
                    Some("aggTrade") => {
                        let trades = serde_json::from_value::<BinanceTrades>(value.clone())
                            .expect("error deserializing to binancetrades");

                        trade_addr.do_send(TradeStreamMessage { data: trades });
                        // println!("{:?}", trades);
                    }
                    Some("depthUpdate") => {
                        let book = serde_json::from_value::<BinancePartialBook>(value.clone())
                            .expect("error deserializing to PartialBook");
                        book_addr.do_send(BookModelStreamMessage { data: book });
                        // println!("{:?}", book);
                    }
                    _ => {
                        eprintln!("Error matching deserialized fields, no aggTrade or depthUpdate");
                    }
                }

                // let trades: BinanceTrades = serde_json::from_value(value).expect("some error 2");
                // trade_addr.do_send(TradeStreamMessage { data: trades });
            }
            _ => (),
        }
    })
    .await;
    // system.run().unwrap();
    Ok(())
}

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
